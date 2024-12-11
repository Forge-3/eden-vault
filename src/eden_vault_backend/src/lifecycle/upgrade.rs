use crate::endpoints::CandidBlockTag;
use crate::logs::INFO;
use crate::state::audit::{process_event, replay_events, EventType};
use crate::state::{mutate_state, read_state};
use crate::state::STATE;
use crate::storage::total_event_count;
use candid::{CandidType, Deserialize, Nat, Principal};
use ic_canister_log::log;
use minicbor::{Decode, Encode};

#[derive(Clone, Eq, PartialEq, Debug, Default, CandidType, Decode, Deserialize, Encode)]
pub struct UpgradeArg {
    #[cbor(n(0), with = "crate::cbor::nat::option")]
    pub next_transaction_nonce: Option<Nat>,
    #[cbor(n(1), with = "crate::cbor::nat::option")]
    pub minimum_withdrawal_amount: Option<Nat>,
    #[n(3)]
    pub ethereum_block_height: Option<CandidBlockTag>,
    #[n(5)]
    pub erc20_helper_contract_address: Option<String>,
    #[cbor(n(6), with = "crate::cbor::nat::option")]
    pub last_erc20_scraped_block_number: Option<Nat>,
    #[cbor(n(7), with = "crate::cbor::principal::option")]
    pub evm_rpc_id: Option<Principal>,
    #[n(8)]
    pub ckerc20_token_address: Option<String>,
    #[n(9)]
    pub ckerc20_token_symbol: Option<String>,
    #[cbor(n(10), with = "crate::cbor::nat::option")]
    pub withdraw_fee_value: Option<Nat>,
}

pub fn post_upgrade(upgrade_args: Option<UpgradeArg>) {
    let start = ic_cdk::api::instruction_counter();

    STATE.with(|cell| {
        *cell.borrow_mut() = Some(replay_events());
    });
    if let Some(args) = upgrade_args {
        mutate_state(|s| process_event(s, EventType::Upgrade(args)))
    }

    read_state(|s| {
        let withdraw_count = s.withdraw_count.clone();
        log!(
            INFO,
            "[upgrade]: withdraw_count {withdraw_count}"
        );
    });


    let end = ic_cdk::api::instruction_counter();

    let event_count = total_event_count();
    let instructions_consumed = end - start;

    log!(
        INFO,
        "[upgrade]: replaying {event_count} events consumed {instructions_consumed} instructions ({} instructions per event on average)",
        instructions_consumed / event_count
    );
}
