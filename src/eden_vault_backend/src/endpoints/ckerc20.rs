use crate::state::transactions::Erc20WithdrawalRequest;
use candid::{CandidType, Deserialize, Nat, Principal};

#[derive(CandidType, Deserialize)]
pub struct WithdrawErc20Arg {
    pub amount: Nat,
    pub recipient: String,
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
