use crate::state::transactions::Erc20WithdrawalRequest;
use candid::{CandidType, Deserialize, Nat, Principal};

#[derive(CandidType, Deserialize)]
pub struct WithdrawErc20Arg {
    pub amount: Nat,
    pub recipient: String,
}

#[derive(CandidType, Debug, Clone, PartialEq, Eq)]
pub struct RetrieveErc20Request {
    pub max_transaction_fee: Nat,
    pub withdrawal_amount: Nat,
    pub destination: String,
    pub from: Principal,
    pub created_at: u64,
    pub id: Nat,
}

impl From<Erc20WithdrawalRequest> for RetrieveErc20Request {
    fn from(request: Erc20WithdrawalRequest) -> Self {
        RetrieveErc20Request {
            withdrawal_amount: request.withdrawal_amount.try_into().unwrap(),
            destination: request.destination.to_string(),
            max_transaction_fee: request.max_transaction_fee.try_into().unwrap(),
            from: request.from,
            created_at: request.created_at,
            id: request.id.try_into().unwrap(),
        }
    }
}

#[derive(Clone, PartialEq, Debug, CandidType, Deserialize)]
pub enum WithdrawErc20Error {
    TokenNotSupported {
        supported_tokens: Vec<crate::endpoints::CkErc20Token>,
    },
    RecipientAddressBlocked {
        address: String,
    },
    CkEthLedgerError {
        error: LedgerError,
    },
    CkErc20LedgerError {
        cketh_block_index: Nat,
        error: LedgerError,
    },
    TemporarilyUnavailable(String),
    InsufficientFunds { 
        available: Nat,
        required: Nat,
    },
    CallerNotFound(Principal),
}

#[derive(Clone, PartialEq, Debug, CandidType, Deserialize)]
pub enum TransferErc20Error {
    CallerNotFound(Principal),
    RecipientNotFound(String),
    InsufficientFunds { 
        available: Nat,
        required: Nat,
    },
}

#[derive(Clone, PartialEq, Debug, CandidType, Deserialize)]
pub enum LedgerError {
    InsufficientFunds {
        balance: Nat,
        failed_burn_amount: Nat,
        token_symbol: String,
        ledger_id: Principal,
    },
    AmountTooLow {
        minimum_burn_amount: Nat,
        failed_burn_amount: Nat,
        token_symbol: String,
        ledger_id: Principal,
    },
    InsufficientAllowance {
        allowance: Nat,
        failed_burn_amount: Nat,
        token_symbol: String,
        ledger_id: Principal,
    },
    TemporarilyUnavailable(String),
}
