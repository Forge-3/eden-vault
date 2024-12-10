use crate::eth_rpc_client::responses::TransactionReceipt;
use crate::state::{transactions, transactions::EthWithdrawalRequest};
use crate::tx::{SignedEip1559TransactionRequest, TransactionPrice};
use candid::{CandidType, Deserialize, Nat, Principal};
use icrc_ledger_types::icrc1::account::Account;
use minicbor::{Decode, Encode};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

pub mod ckerc20;

#[derive(Clone, Eq, PartialEq, Debug, CandidType, Deserialize)]
pub struct Eip1559TransactionPriceArg {
    pub ckerc20_ledger_id: Principal,
}

#[derive(Clone, Eq, PartialEq, Debug, CandidType, Deserialize)]
pub struct Eip1559TransactionPrice {
    pub gas_limit: Nat,
    pub max_fee_per_gas: Nat,
    pub max_priority_fee_per_gas: Nat,
    pub max_transaction_fee: Nat,
    pub timestamp: Option<u64>,
}

impl From<TransactionPrice> for Eip1559TransactionPrice {
    fn from(value: TransactionPrice) -> Self {
        Self {
            gas_limit: value.gas_limit.into(),
            max_fee_per_gas: value.max_fee_per_gas.into(),
            max_priority_fee_per_gas: value.max_priority_fee_per_gas.into(),
            max_transaction_fee: value.max_transaction_fee().into(),
            timestamp: None,
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, CandidType, Deserialize)]
pub struct CkErc20Token {
    pub ckerc20_token_symbol: String,
    pub erc20_contract_address: String,
    pub ledger_canister_id: Principal,
}

impl From<crate::erc20::CkErc20Token> for CkErc20Token {
    fn from(value: crate::erc20::CkErc20Token) -> Self {
        Self {
            ckerc20_token_symbol: value.ckerc20_token_symbol.to_string(),
            erc20_contract_address: value.erc20_contract_address.to_string(),
            ledger_canister_id: value.ckerc20_ledger_id,
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, CandidType, Deserialize)]
pub struct Erc20Balance {
    pub erc20_contract_address: String,
    pub balance: Nat,
}

#[derive(Clone, Eq, PartialEq, Debug, CandidType, Deserialize)]
pub struct MinterInfo {
    pub minter_address: Option<String>,
    #[deprecated(note = "use eth_helper_contract_address instead")]
    pub smart_contract_address: Option<String>,
    pub eth_helper_contract_address: Option<String>,
    pub erc20_helper_contract_address: Option<String>,
    pub supported_ckerc20_tokens: Option<Vec<CkErc20Token>>,
    pub minimum_withdrawal_amount: Option<Nat>,
    pub ethereum_block_height: Option<CandidBlockTag>,
    pub last_observed_block_number: Option<Nat>,
    pub eth_balance: Option<Nat>,
    pub last_gas_fee_estimate: Option<GasFeeEstimate>,
    pub erc20_balances: Option<Vec<Erc20Balance>>,
    pub last_eth_scraped_block_number: Option<Nat>,
    pub last_erc20_scraped_block_number: Option<Nat>,
    pub cketh_ledger_id: Option<Principal>,
}

#[derive(Clone, Eq, PartialEq, Debug, CandidType, Deserialize)]
pub struct GasFeeEstimate {
    pub max_fee_per_gas: Nat,
    pub max_priority_fee_per_gas: Nat,
    pub timestamp: u64,
}

#[derive(Clone, Eq, PartialEq, Hash, Debug, CandidType, Deserialize)]
pub struct EthTransaction {
    pub transaction_hash: String,
}

impl From<&SignedEip1559TransactionRequest> for EthTransaction {
    fn from(value: &SignedEip1559TransactionRequest) -> Self {
        Self {
            transaction_hash: value.hash().to_string(),
        }
    }
}

impl From<&TransactionReceipt> for EthTransaction {
    fn from(receipt: &TransactionReceipt) -> Self {
        Self {
            transaction_hash: receipt.transaction_hash.to_string(),
        }
    }
}

#[derive(Clone, PartialEq, Debug, CandidType, Deserialize)]
pub struct RetrieveEthRequest {
    pub block_index: Nat,
}

#[derive(Clone, Eq, PartialEq, Debug, Default, CandidType, Decode, Deserialize, Encode)]
#[cbor(index_only)]
pub enum CandidBlockTag {
    /// The latest mined block.
    #[default]
    #[cbor(n(0))]
    Latest,
    /// The latest safe head block.
    /// See
    /// <https://www.alchemy.com/overviews/ethereum-commitment-levels#what-are-ethereum-commitment-levels>
    #[cbor(n(1))]
    Safe,
    /// The latest finalized block.
    /// See
    /// <https://www.alchemy.com/overviews/ethereum-commitment-levels#what-are-ethereum-commitment-levels>
    #[cbor(n(2))]
    Finalized,
}

impl From<EthWithdrawalRequest> for RetrieveEthRequest {
    fn from(value: EthWithdrawalRequest) -> Self {
        Self {
            block_index: Nat::from(value.ledger_burn_index),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Hash, Debug, CandidType, Deserialize)]
pub enum RetrieveEthStatus {
    NotFound,
    Pending,
    TxCreated,
    TxSent(EthTransaction),
    TxFinalized(TxFinalizedStatus),
}

#[derive(Clone, Eq, PartialEq, Hash, Debug, CandidType, Deserialize)]
pub enum TxFinalizedStatus {
    Success {
        transaction_hash: String,
        effective_transaction_fee: Option<Nat>,
    },
    PendingReimbursement(EthTransaction),
    Reimbursed {
        transaction_hash: String,
        reimbursed_amount: Nat,
        reimbursed_in_block: Nat,
    },
}

impl Display for RetrieveEthStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RetrieveEthStatus::NotFound => write!(f, "Not Found"),
            RetrieveEthStatus::Pending => write!(f, "Pending"),
            RetrieveEthStatus::TxCreated => write!(f, "Created"),
            RetrieveEthStatus::TxSent(tx) => write!(f, "Sent({})", tx.transaction_hash),
            RetrieveEthStatus::TxFinalized(tx_status) => match tx_status {
                TxFinalizedStatus::Success {
                    transaction_hash, ..
                } => write!(f, "Confirmed({})", transaction_hash),
                TxFinalizedStatus::PendingReimbursement(tx) => {
                    write!(f, "PendingReimbursement({})", tx.transaction_hash)
                }
                TxFinalizedStatus::Reimbursed {
                    reimbursed_in_block,
                    transaction_hash,
                    reimbursed_amount,
                } => write!(
                    f,
                    "Failure({}, reimbursed: {} Wei in block: {})",
                    transaction_hash, reimbursed_amount, reimbursed_in_block
                ),
            },
        }
    }
}

#[derive(CandidType, Deserialize)]
pub struct WithdrawalArg {
    pub amount: Nat,
    pub recipient: String,
}

#[derive(PartialEq, Debug, CandidType, Deserialize)]
pub enum WithdrawalError {
    AmountTooLow { min_withdrawal_amount: Nat },
    InsufficientFunds { balance: Nat },
    InsufficientAllowance { allowance: Nat },
    RecipientAddressBlocked { address: String },
    TemporarilyUnavailable(String),
}

#[derive(Clone, Eq, PartialEq, Debug, CandidType, Deserialize)]
pub enum WithdrawalSearchParameter {
    ByWithdrawalId(Nat),
    ByRecipient(String),
    BySenderAccount(Account),
}

impl TryFrom<WithdrawalSearchParameter> for transactions::WithdrawalSearchParameter {
    type Error = String;

    fn try_from(parameter: WithdrawalSearchParameter) -> Result<Self, String> {
        use WithdrawalSearchParameter::*;
        match parameter {
            ByWithdrawalId(index) => Ok(Self::ByWithdrawalId(index)),
            ByRecipient(address) => Ok(Self::ByRecipient(ic_ethereum_types::Address::from_str(
                &address,
            )?)),
            BySenderAccount(account) => Ok(Self::BySenderAccount(account)),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Hash, Debug, CandidType, Deserialize)]
pub struct WithdrawalDetail {
    pub withdrawal_id: Nat,
    pub recipient_address: String,
    pub from: Principal,
    pub from_subaccount: Option<[u8; 32]>,
    pub token_symbol: String,
    pub withdrawal_amount: Nat,
    pub max_transaction_fee: Option<Nat>,
    pub status: WithdrawalStatus,
    pub withdraw_vault_fee_value: Nat,
}

#[derive(Clone, Eq, PartialEq, Hash, Debug, CandidType, Deserialize)]
pub enum WithdrawalStatus {
    Pending,
    TxCreated,
    TxSent(EthTransaction),
    TxFinalized(TxFinalizedStatus),
}

#[derive(Clone, PartialEq, Debug, CandidType, Deserialize)]
pub struct AddCkErc20Token {
    pub chain_id: Nat,
    pub address: String,
    pub ckerc20_token_symbol: String,
    pub ckerc20_ledger_id: Principal,
}

pub mod events {
    use crate::lifecycle::init::InitArg;
    use crate::lifecycle::upgrade::UpgradeArg;
    use candid::{CandidType, Deserialize, Nat, Principal};
    use serde_bytes::ByteBuf;

    use super::WithdrawalDetail;

    #[derive(Clone, Debug, CandidType, Deserialize)]
    pub struct GetEventsArg {
        pub start: u64,
        pub length: u64,
    }

    #[derive(Clone, Debug, CandidType, Deserialize)]
    pub struct GetEventsResult {
        pub events: Vec<Event>,
        pub total_event_count: u64,
    }

    #[derive(Clone, Eq, PartialEq, Debug, CandidType, Deserialize)]
    pub struct Event {
        pub timestamp: u64,
        pub payload: EventPayload,
    }

    #[derive(Clone, Eq, PartialEq, Debug, CandidType, Deserialize)]
    pub struct EventSource {
        pub transaction_hash: String,
        pub log_index: Nat,
    }

    #[derive(Clone, Eq, PartialEq, Debug, CandidType, Deserialize)]
    pub enum ReimbursementIndex {
        CkEth {
            ledger_burn_index: Nat,
        },
        CkErc20 {
            withdrawal_id: Nat,
        },
    }

    #[derive(Clone, Eq, PartialEq, Debug, CandidType, Deserialize)]
    pub struct AccessListItem {
        pub address: String,
        pub storage_keys: Vec<ByteBuf>,
    }

    #[derive(Clone, Eq, PartialEq, Debug, CandidType, Deserialize)]
    pub struct UnsignedTransaction {
        pub chain_id: Nat,
        pub nonce: Nat,
        pub max_priority_fee_per_gas: Nat,
        pub max_fee_per_gas: Nat,
        pub gas_limit: Nat,
        pub destination: String,
        pub value: Nat,
        pub data: ByteBuf,
        pub access_list: Vec<AccessListItem>,
    }

    #[derive(Clone, Eq, PartialEq, Debug, CandidType, Deserialize)]
    pub enum TransactionStatus {
        Success,
        Failure,
    }

    #[derive(Clone, Eq, PartialEq, Debug, CandidType, Deserialize)]
    pub struct TransactionReceipt {
        pub block_hash: String,
        pub block_number: Nat,
        pub effective_gas_price: Nat,
        pub gas_used: Nat,
        pub status: TransactionStatus,
        pub transaction_hash: String,
    }

    #[derive(Clone, Eq, PartialEq, Debug, CandidType, Deserialize)]
    pub enum EventPayload {
        Init(InitArg),
        Upgrade(UpgradeArg),
        AcceptedErc20Deposit {
            transaction_hash: String,
            block_number: Nat,
            log_index: Nat,
            from_address: String,
            value: Nat,
            principal: Principal,
            erc20_contract_address: String,
        },
        InvalidDeposit {
            event_source: EventSource,
            reason: String,
        },
        SyncedToBlock {
            block_number: Nat,
        },
        SyncedErc20ToBlock {
            block_number: Nat,
        },
        CreatedTransaction {
            withdrawal_id: Nat,
            transaction: UnsignedTransaction,
        },
        SignedTransaction {
            withdrawal_id: Nat,
            raw_transaction: String,
        },
        ReplacedTransaction {
            withdrawal_id: Nat,
            transaction: UnsignedTransaction,
        },
        FinalizedTransaction {
            withdrawal_id: Nat,
            transaction_receipt: TransactionReceipt,
            details: Vec<WithdrawalDetail>
        },
        SkippedBlock {
            contract_address: Option<String>,
            block_number: Nat,
        },
        AcceptedErc20WithdrawalRequest {
            max_transaction_fee: Nat,
            withdrawal_amount: Nat,
            destination: String,
            from: Principal,
            from_subaccount: Option<[u8; 32]>,
            created_at: u64,
            withdrawal_id: Nat,
            withdraw_vault_fee_value: Nat, 
        },
        MintedCkErc20 {
            event_source: EventSource,
            principal: Principal,
            amount: Nat,
        },
        QuarantinedDeposit {
            event_source: EventSource,
        },
        QuarantinedReimbursement {
            index: ReimbursementIndex,
        },
        Erc20TransferCompleted {
            from: Principal,
            to: Principal,
            amount: Nat,
        },
    }
}
