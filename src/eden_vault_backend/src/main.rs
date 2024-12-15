use crate::checked_amount::CheckedAmountOf;
use candid::{Nat, Principal};
use eden_vault_backend::address::{validate_address_as_destination, AddressValidationError};
use eden_vault_backend::user::{does_user_already_exist, get_user_by, CreateNewUser, GetUserBy, User, UserError, UserStats};
use eden_vault_backend::checked_amount;
use eden_vault_backend::deposit::scrape_logs;
use eden_vault_backend::endpoints::ckerc20::{
    RetrieveErc20Request, TransferErc20Error, WithdrawErc20Arg, WithdrawErc20Error
};
use eden_vault_backend::endpoints::events::{GetEventsArg, GetEventsResult};
use eden_vault_backend::endpoints::{
    RetrieveEthStatus, WithdrawalDetail, WithdrawalSearchParameter,
};
use eden_vault_backend::eth_logs::{EventSource, ReceivedErc20Event};
use eden_vault_backend::guard::retrieve_withdraw_guard;
use eden_vault_backend::lifecycle::MinterArg;
use eden_vault_backend::logs::INFO;
use eden_vault_backend::numeric::{Erc20Value, Erc20Tag, Wei, LedgerBurnIndex};
use eden_vault_backend::state::audit::{process_event, EventType, Event};
use eden_vault_backend::state::transactions::{Erc20WithdrawalRequest, ReimbursementIndex, Subaccount};
use eden_vault_backend::state::{
    lazy_call_ecdsa_public_key, mutate_state, read_state, transactions, State, STATE,
};
use eden_vault_backend::storage::{push_user, with_event_iter};
use eden_vault_backend::tx::lazy_refresh_gas_fee_estimate;
use eden_vault_backend::withdraw::{
    process_retrieve_eth_requests, CKERC20_WITHDRAWAL_TRANSACTION_GAS_LIMIT,
};
use eden_vault_backend::{
    state, storage, PROCESS_ETH_RETRIEVE_TRANSACTIONS_INTERVAL, SCRAPING_ETH_LOGS_INTERVAL,
};
use ic_canister_log::log;
use ic_cdk::{init, post_upgrade, pre_upgrade, query, update};
use ic_ethereum_types::Address;
use std::convert::TryFrom;
use std::str::FromStr;
use std::time::Duration;
use eden_vault_backend::user::OptionUser;

pub const SEPOLIA_TEST_CHAIN_ID: u64 = 11155111;

fn validate_caller_not_anonymous() -> candid::Principal {
    let principal = ic_cdk::caller();
    if principal == candid::Principal::anonymous() {
        panic!("anonymous principal is not allowed");
    }
    principal
}

fn setup_timers() {
    ic_cdk_timers::set_timer(Duration::from_secs(0), || {
        // Initialize the minter's public key to make the address known.
        ic_cdk::spawn(async {
            let _ = lazy_call_ecdsa_public_key().await;
        })
    });
    // Start scraping logs immediately after the install, then repeat with the interval.
    ic_cdk_timers::set_timer(Duration::from_secs(0), || ic_cdk::spawn(scrape_logs()));
    ic_cdk_timers::set_timer_interval(SCRAPING_ETH_LOGS_INTERVAL, || ic_cdk::spawn(scrape_logs()));
    ic_cdk_timers::set_timer_interval(PROCESS_ETH_RETRIEVE_TRANSACTIONS_INTERVAL, || {
        ic_cdk::spawn(process_retrieve_eth_requests())
    });
}

#[init]
fn init(arg: MinterArg) {
    match arg {
        MinterArg::InitArg(init_arg) => {
            log!(INFO, "[init]: initialized minter with arg: {:?}", init_arg);
            STATE.with(|cell| {
                storage::record_event(EventType::Init(init_arg.clone()));
                *cell.borrow_mut() =
                    Some(State::try_from(init_arg).expect("BUG: failed to initialize minter"))
            });
        }
        MinterArg::UpgradeArg(_) => {
            ic_cdk::trap("cannot init canister state with upgrade args");
        }
    }
    setup_timers();
}

fn emit_preupgrade_events() {
    read_state(|s| {
        storage::record_event(EventType::SyncedToBlock {
            block_number: s.last_scraped_block_number,
        });
        storage::record_event(EventType::SyncedErc20ToBlock {
            block_number: s.last_erc20_scraped_block_number,
        });
    });
}

#[pre_upgrade]
fn pre_upgrade() {
    emit_preupgrade_events();
}

#[post_upgrade]
fn post_upgrade(minter_arg: Option<MinterArg>) {
    use eden_vault_backend::lifecycle;
    match minter_arg {
        Some(MinterArg::InitArg(_)) => {
            ic_cdk::trap("cannot upgrade canister state with init args");
        }
        Some(MinterArg::UpgradeArg(upgrade_args)) => lifecycle::post_upgrade(Some(upgrade_args)),
        None => lifecycle::post_upgrade(None),
    }
    setup_timers();
}

#[update]
async fn minter_address() -> String {
    state::minter_address().await.to_string()
}

#[update]
async fn retrieve_eth_status(block_index: u64) -> RetrieveEthStatus {
    let _ledger_burn_index = LedgerBurnIndex::new(block_index);
    let ledger_burn_index_nat = Nat::from(block_index as u128);
    read_state(|s| {
        s.eth_transactions
            .transaction_status(&ledger_burn_index_nat)
    })
}

#[query]
fn withdrawal_status(parameter: WithdrawalSearchParameter) -> Vec<WithdrawalDetail> {
    use transactions::WithdrawalRequest::*;
    let parameter = transactions::WithdrawalSearchParameter::try_from(parameter).unwrap();
    read_state(|s| {
        s.eth_transactions
            .withdrawal_status(&parameter)
            .into_iter()
            .map(|(request, status, tx)| WithdrawalDetail {
                withdrawal_id: request.get_withdrawal_id(),
                recipient_address: request.payee().to_string(),
                token_symbol: match request {
                    CkErc20(_r) => s.ckerc20_tokens.1.to_string(),
                },
                withdrawal_amount: match request {
                    CkErc20(r) => r.withdrawal_amount.into(),
                },
                max_transaction_fee: match (request, tx) {
                    (CkErc20(r), _) => Some(r.max_transaction_fee.into()),
                },
                from: request.from(),
                from_subaccount: request
                    .from_subaccount()
                    .clone()
                    .map(|subaccount| subaccount.0),
                status,
                from_user_id: get_user_by(
                    GetUserBy::Principal(request.from())
                ).get_user_id()
            })
            .collect()
    })
}

#[update]
async fn withdraw_erc20(
    WithdrawErc20Arg { amount, recipient }: WithdrawErc20Arg,
) -> Result<RetrieveErc20Request, WithdrawErc20Error> {
    let caller = validate_caller_not_anonymous();
    let admin = read_state(|s| s.admin);

    if admin != caller {
        let _caller_as_user = get_user_by(GetUserBy::Principal(caller))
            .ok_or(WithdrawErc20Error::CallerNotFound(caller))?;
    }
    let _guard = retrieve_withdraw_guard(caller).unwrap_or_else(|e| {
        ic_cdk::trap(&format!(
            "Failed retrieving guard for principal {}: {:?}",
            caller, e
        ))
    });

    let destination = validate_address_as_destination(&recipient).map_err(|e| match e {
        AddressValidationError::Invalid { .. } | AddressValidationError::NotSupported(_) => {
            ic_cdk::trap(&e.to_string())
        }
        AddressValidationError::Blocked(address) => WithdrawErc20Error::RecipientAddressBlocked {
            address: address.to_string(),
        },
    })?;

    let ckerc20_withdrawal_amount =
        Erc20Value::try_from(amount).expect("ERROR: failed to convert Nat to u256");


        let withdraw_fee = read_state(|s| s.withdraw_fee_value);
        let total_amount_needed = ckerc20_withdrawal_amount
            .checked_add(withdraw_fee)
            .expect("BUG: Overflow when calculating total amount needed");
        let caller_balance = read_state(|s| s.erc20_balances.balance_of(&caller));
        if caller_balance < total_amount_needed {
            return Err(WithdrawErc20Error::InsufficientFunds {
                available: Nat::from(caller_balance),
                required: Nat::from(total_amount_needed),
            });
        }

    let ckerc20_tokens = read_state(|s| s.ckerc20_tokens.clone());
    let erc20_tx_fee = estimate_erc20_transaction_fee().await.ok_or_else(|| {
        WithdrawErc20Error::TemporarilyUnavailable("Failed to retrieve current gas fee".to_string())
    })?;

    log!(
        INFO,
        "[withdraw_erc20]: burning {} {}",
        ckerc20_withdrawal_amount,
        ckerc20_tokens.1
    );

    let withdrawal_request = Erc20WithdrawalRequest {
        max_transaction_fee: erc20_tx_fee,
        withdrawal_amount: ckerc20_withdrawal_amount,
        destination,
        from: caller,
        from_subaccount: None,
        created_at: ic_cdk::api::time(),
        id: read_state(|s| s.next_withdrawal_id()).into(),
    };
    log!(
        INFO,
        "[withdraw_erc20]: queuing withdrawal request {:?}",
        withdrawal_request
    );
    mutate_state(|s| {
        process_event(
            s,
            EventType::AcceptedErc20WithdrawalRequest(withdrawal_request.clone()),
        );
    });
    Ok(RetrieveErc20Request::from(withdrawal_request))
}

async fn estimate_erc20_transaction_fee() -> Option<Wei> {
    lazy_refresh_gas_fee_estimate()
        .await
        .map(|gas_fee_estimate| {
            gas_fee_estimate
                .to_price(CKERC20_WITHDRAWAL_TRANSACTION_GAS_LIMIT)
                .max_transaction_fee()
        })
}

#[query]
fn is_address_blocked(address_string: String) -> bool {
    let address = Address::from_str(&address_string)
        .unwrap_or_else(|e| ic_cdk::trap(&format!("invalid recipient address: {:?}", e)));
    eden_vault_backend::blocklist::is_blocked(&address)
}

#[update]
async fn get_canister_status() -> ic_cdk::api::management_canister::main::CanisterStatusResponse {
    ic_cdk::api::management_canister::main::canister_status(
        ic_cdk::api::management_canister::main::CanisterIdRecord {
            canister_id: ic_cdk::id(),
        },
    )
    .await
    .expect("failed to fetch canister status")
    .0
}

#[query]
fn get_events(arg: GetEventsArg) -> GetEventsResult {
    use eden_vault_backend::endpoints::events::{
        AccessListItem, ReimbursementIndex as CandidReimbursementIndex,
        TransactionReceipt as CandidTransactionReceipt,
        TransactionStatus as CandidTransactionStatus, UnsignedTransaction,
        Event as CandidEvent,
    };
    use eden_vault_backend::eth_rpc_client::responses::TransactionReceipt;
    use eden_vault_backend::tx::Eip1559TransactionRequest;
    use serde_bytes::ByteBuf;
    use eden_vault_backend::endpoints::events::EventSource as CandidEventSource;

    const MAX_EVENTS_PER_RESPONSE: u64 = 100;

    fn map_event_source(
        EventSource {
            transaction_hash,
            log_index,
        }: EventSource,
    ) -> CandidEventSource {
        CandidEventSource {
            transaction_hash: transaction_hash.to_string(),
            log_index: log_index.into(),
        }
    }

    fn map_reimbursement_index(index: ReimbursementIndex) -> CandidReimbursementIndex {
        match index {
            ReimbursementIndex::CkErc20 {
                withdrawal_id,
            } => CandidReimbursementIndex::CkErc20 {
                withdrawal_id,

            },
        }
    }

    fn map_unsigned_transaction(tx: Eip1559TransactionRequest) -> UnsignedTransaction {
        UnsignedTransaction {
            chain_id: tx.chain_id.into(),
            nonce: tx.nonce.into(),
            max_priority_fee_per_gas: tx.max_priority_fee_per_gas.into(),
            max_fee_per_gas: tx.max_fee_per_gas.into(),
            gas_limit: tx.gas_limit.into(),
            destination: tx.destination.to_string(),
            value: tx.amount.into(),
            data: ByteBuf::from(tx.data),
            access_list: tx
                .access_list
                .0
                .iter()
                .map(|item| AccessListItem {
                    address: item.address.to_string(),
                    storage_keys: item
                        .storage_keys
                        .iter()
                        .map(|key| ByteBuf::from(key.0.to_vec()))
                        .collect(),
                })
                .collect(),
        }
    }

    fn map_transaction_receipt(receipt: TransactionReceipt) -> CandidTransactionReceipt {
        use eden_vault_backend::eth_rpc_client::responses::TransactionStatus;
        CandidTransactionReceipt {
            block_hash: receipt.block_hash.to_string(),
            block_number: receipt.block_number.into(),
            effective_gas_price: receipt.effective_gas_price.into(),
            gas_used: receipt.gas_used.into(),
            status: match receipt.status {
                TransactionStatus::Success => CandidTransactionStatus::Success,
                TransactionStatus::Failure => CandidTransactionStatus::Failure,
            },
            transaction_hash: receipt.transaction_hash.to_string(),
        }
    }

    fn map_event(Event { timestamp, payload }: Event) -> CandidEvent {
        use eden_vault_backend::endpoints::events::EventPayload as EP;
        CandidEvent {
            timestamp,
            payload : match payload {
                EventType::Init(args) => EP::Init(args),
                EventType::Upgrade(args) => EP::Upgrade(args),
                EventType::AcceptedErc20Deposit(ReceivedErc20Event {
                    transaction_hash,
                    block_number,
                    log_index,
                    from_address,
                    value,
                    principal,
                    erc20_contract_address,
                }) => EP::AcceptedErc20Deposit {
                    transaction_hash: transaction_hash.to_string(),
                    block_number: block_number.into(),
                    log_index: log_index.into(),
                    from_address: from_address.to_string(),
                    value: value.into(),
                    to_user_id: get_user_by(GetUserBy::Principal(principal)).get_user_id(),
                    principal,
                    erc20_contract_address: erc20_contract_address.to_string(),
                },
                EventType::InvalidDeposit {
                    event_source,
                    reason,
                } => EP::InvalidDeposit {
                    event_source: map_event_source(event_source),
                    reason,
                },
                EventType::SyncedToBlock { block_number } => EP::SyncedToBlock {
                    block_number: block_number.into(),
                },
                EventType::SyncedErc20ToBlock { block_number } => EP::SyncedErc20ToBlock {
                    block_number: block_number.into(),
                },
                EventType::CreatedTransaction {
                    withdrawal_id,
                    transaction,
                } => EP::CreatedTransaction {
                    withdrawal_id,
                    transaction: map_unsigned_transaction(transaction),
                },
                EventType::SignedTransaction {
                    withdrawal_id,
                    transaction,
                } => EP::SignedTransaction {
                    withdrawal_id,
                    raw_transaction: transaction.raw_transaction_hex(),
                },
                EventType::ReplacedTransaction {
                    withdrawal_id,
                    transaction,
                } => EP::ReplacedTransaction {
                    withdrawal_id,
                    transaction: map_unsigned_transaction(transaction),
                },
                EventType::FinalizedTransaction {
                    withdrawal_id,
                    transaction_receipt,
                } => EP::FinalizedTransaction {
                    details: withdrawal_status(WithdrawalSearchParameter::ByWithdrawalId(withdrawal_id.clone())),
                    withdrawal_id,
                    transaction_receipt: map_transaction_receipt(transaction_receipt),
                },
                EventType::SkippedBlockForContract {
                    contract_address,
                    block_number,
                } => EP::SkippedBlock {
                    contract_address: Some(contract_address.to_string()),
                    block_number: block_number.into(),
                },
                EventType::AcceptedErc20WithdrawalRequest(Erc20WithdrawalRequest {
                    max_transaction_fee,
                    withdrawal_amount,
                    destination,
                    from,
                    from_subaccount,
                    created_at,
                    id
                }) => EP::AcceptedErc20WithdrawalRequest {
                    max_transaction_fee: max_transaction_fee.into(),
                    withdrawal_amount: withdrawal_amount.into(),
                    destination: destination.to_string(),
                    from,
                    from_subaccount: from_subaccount.map(Subaccount::to_bytes),
                    created_at,
                    withdrawal_id: id,
                    from_user_id: get_user_by(GetUserBy::Principal(from)).get_user_id(),
                },
                EventType::MintedCkErc20 {
                    event_source,
                    principal,
                    amount,
                } => EP::MintedCkErc20 {
                    event_source: map_event_source(event_source),
                    principal,
                    amount: amount.into(),
                    to_user_id: get_user_by(GetUserBy::Principal(principal)).get_user_id(),
                },
                EventType::QuarantinedDeposit { event_source } => EP::QuarantinedDeposit {
                    event_source: map_event_source(event_source),
                },
                EventType::QuarantinedReimbursement { index } => EP::QuarantinedReimbursement {
                    index: map_reimbursement_index(index),
                },
                EventType::Erc20TransferCompleted { from, to, amount } => EP::Erc20TransferCompleted {
                    from,
                    from_user_id: 
                        get_user_by(
                            GetUserBy::Principal(from)
                        ).get_user_id(),
                    to,
                    to_user_id:  
                        get_user_by(
                            GetUserBy::Principal(to)
                        ).get_user_id(),
                    amount: amount.into(),
                },
            },
        }
    }

    let events = storage::with_event_iter(|it| {
        it.skip(arg.start as usize)
            .take(arg.length.min(MAX_EVENTS_PER_RESPONSE) as usize)
            .map(map_event)
            .collect()
    });

    GetEventsResult {
        events,
        total_event_count: storage::total_event_count(),
    }
}

#[cfg(feature = "debug_checks")]
#[query]
fn check_audit_log() {
    use eden_vault_backend::state::audit::replay_events;

    emit_preupgrade_events();

    read_state(|s| {
        replay_events()
            .is_equivalent_to(s)
            .expect("replaying the audit log should produce an equivalent state")
    })
}

/// Returns the amount of heap memory in bytes that has been allocated.
#[cfg(target_arch = "wasm32")]
pub fn heap_memory_size_bytes() -> usize {
    const WASM_PAGE_SIZE_BYTES: usize = 65536;
    core::arch::wasm32::memory_size(0) * WASM_PAGE_SIZE_BYTES
}

#[cfg(not(any(target_arch = "wasm32")))]
pub fn heap_memory_size_bytes() -> usize {
    0
}

/// Checks the real candid interface against the one declared in the did file
#[test]
fn check_candid_interface_compatibility() {
    fn source_to_str(source: &candid_parser::utils::CandidSource) -> String {
        match source {
            candid_parser::utils::CandidSource::File(f) => {
                std::fs::read_to_string(f).unwrap_or_else(|_| "".to_string())
            }
            candid_parser::utils::CandidSource::Text(t) => t.to_string(),
        }
    }

    fn check_service_equal(
        new_name: &str,
        new: candid_parser::utils::CandidSource,
        old_name: &str,
        old: candid_parser::utils::CandidSource,
    ) {
        let new_str = source_to_str(&new);
        let old_str = source_to_str(&old);
        match candid_parser::utils::service_equal(new, old) {
            Ok(_) => {}
            Err(e) => {
                eprintln!(
                    "{} is not compatible with {}!\n\n\
            {}:\n\
            {}\n\n\
            {}:\n\
            {}\n",
                    new_name, old_name, new_name, new_str, old_name, old_str
                );
                panic!("{:?}", e);
            }
        }
    }

    candid::export_service!();

    let new_interface = __export_service();

    // check the public interface against the actual one
    let old_interface = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("cketh_minter.did");

    check_service_equal(
        "actual ledger candid interface",
        candid_parser::utils::CandidSource::Text(&new_interface),
        "declared candid interface in cketh_minter.did file",
        candid_parser::utils::CandidSource::File(old_interface.as_path()),
    );
}

#[update]
async fn set_admin(_: Principal) -> Result<String, String> {
    return Err("Call not implemented".to_string());
}

#[query]
async fn erc20_my_balance() -> Result<Nat, UserError> {
    let caller = validate_caller_not_anonymous();
    let admin = read_state(|s| s.admin);

    if admin != caller {
        let _caller_as_user = get_user_by(GetUserBy::Principal(caller))
            .ok_or(UserError::CallerNotFound(caller))?;
    }
    Ok(read_state(|s| s.erc20_balances.balance_of(&caller).try_into().unwrap()))
}

#[query]
async fn erc20_balance_of(principal: Principal) -> Nat {
    read_state(|s| s.erc20_balances.balance_of(&principal).try_into().unwrap())
}

#[query]
async fn get_user_erc20_stats(principal: Principal) -> UserStats{
    let mut deposit_count: Nat = 0u8.into();
    let mut started_withdrawals: Nat = 0u8.into();
    let mut transfers_from: Nat = 0u8.into();
    let mut transfers_in: Nat = 0u8.into();
    let mut ended_withdrawals: Nat = 0u8.into();

    with_event_iter(|events| {
        let events_iter = events.into_iter();
        for event in events_iter {
            match &event.payload {
                EventType::AcceptedErc20Deposit(deposit) => {
                    if deposit.principal == principal {
                        deposit_count+=1u8
                    }
                },
                EventType::AcceptedErc20WithdrawalRequest(withdrawal) => {
                    if withdrawal.from == principal {
                        started_withdrawals+=1u8
                    }
                }
                EventType::Erc20TransferCompleted{from, to, amount: _} => {
                    if from == &principal{
                        transfers_from+=1u8
                    } else if to == &principal{
                        transfers_in+=1u8
                    }
                }
                EventType::FinalizedTransaction{ withdrawal_id, transaction_receipt: _ } => {
                    let tx = read_state(|s| 
                        s.eth_transactions
                            .get_processed_withdrawal_request(withdrawal_id)
                            .expect("Tx is not Finalized?")
                            .clone()
                    );
                    if tx.from() == principal {
                        ended_withdrawals+=1u8
                    }
                }
                _=> (),
            } 
        }
    });
    let user_balance: Nat = read_state(|s| s.erc20_balances.balance_of(&principal).try_into().unwrap());

    return UserStats {
         deposit_count,
         started_withdrawals,
         transfers_from,
         transfers_in,
         ended_withdrawals,
         user_balance,
    }

}

#[query]
async fn erc20_balance() -> Nat {
    read_state(|s| s.erc20_balances.get_erc20_balance().try_into().unwrap())
}

#[update]
async fn erc20_transfer(receiver: Principal, amount: Nat) -> Result<(), TransferErc20Error> {
    let caller = validate_caller_not_anonymous();
    let admin = read_state(|s| s.admin);

    if admin != caller {
        let _caller_as_user = get_user_by(GetUserBy::Principal(caller))
            .ok_or(TransferErc20Error::CallerNotFound(caller))?;
    }
    if admin != receiver {
        let _receiver_as_user = get_user_by(GetUserBy::Principal(receiver))
            .ok_or(TransferErc20Error::CallerNotFound(receiver))?;
    }

    let checked_amount = CheckedAmountOf::<Erc20Tag>::try_from(amount.clone())
        .expect("ERROR: failed to convert Nat to u256");

    read_state(|s| {
        let caller_balance = s.erc20_balances.balance_of(&caller);
        if caller_balance < checked_amount {
            return Err(TransferErc20Error::InsufficientFunds{
                available: caller_balance.into(),
                required: amount,
            });
        }
        Ok(())
    })?;
    mutate_state(|s| {
        process_event(
            s,
            EventType::Erc20TransferCompleted {
                from: caller,
                to: receiver,
                amount: checked_amount,
            },
        );
        Ok(())
    })
}

#[query]
async fn smart_contract_address() -> String {
    read_state(|s| s.erc20_helper_contract_address.clone())
    .map(|a| a.to_string())
    .unwrap_or("N/A".to_string())
}

#[update]
fn create_new_user(    principal: Principal,
    user_id:[u8; 12]) -> Result<(), UserError>{
    let caller = validate_caller_not_anonymous();
    let admin = read_state(|s| s.admin);

    if caller != admin {
        return Err(UserError::NotAdmin)
    }
    if principal == admin {
        return Err(UserError::UserIsAdmin)
    }
    let user = User::new(user_id, principal);
    if does_user_already_exist(&user) {
        return Err(UserError::UserAlreadyExists)
    }
    push_user(&user);
    Ok(())
}

ic_cdk::export_candid!();

fn main() {}
