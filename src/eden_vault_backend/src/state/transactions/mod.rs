#[cfg(test)]
mod tests;

use crate::endpoints::{EthTransaction, RetrieveEthStatus, TxFinalizedStatus, WithdrawalStatus};
use crate::eth_rpc::Hash;
use crate::eth_rpc_client::responses::TransactionReceipt;
use crate::eth_rpc_client::responses::TransactionStatus;
use crate::lifecycle::EthereumNetwork;
use crate::map::MultiKeyMap;
use crate::numeric::{
    CkTokenAmount, Erc20Value, GasAmount, LedgerMintIndex, TransactionCount, TransactionNonce, Wei,
};
use crate::state::event::EventType;
use crate::state::read_state;
use crate::tx::{
    Eip1559TransactionRequest, FinalizedEip1559Transaction, GasFeeEstimate, ResubmissionStrategy,
    SignedEip1559TransactionRequest, SignedTransactionRequest, TransactionRequest,
};
use candid::{Nat, Principal};
use ic_ethereum_types::Address;
use icrc_ledger_types::icrc1::account::Account;
use minicbor::{Decode, Encode};
use std::cmp::min;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum WithdrawalSearchParameter {
    ByWithdrawalId(Nat),
    ByRecipient(Address),
    BySenderAccount(Account),
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum WithdrawalRequest {
    CkErc20(Erc20WithdrawalRequest),
}

impl WithdrawalRequest {
    pub fn get_withdrawal_id(&self) -> Nat {
        match self {
            WithdrawalRequest::CkErc20(request) => request.id.clone(),
        }
    }
    pub fn created_at(&self) -> Option<u64> {
        match self {
            WithdrawalRequest::CkErc20(request) => Some(request.created_at),
        }
    }

    /// Address to which the funds are to be sent to.
    pub fn payee(&self) -> Address {
        match self {
            WithdrawalRequest::CkErc20(request) => request.destination,
        }
    }

    pub fn from(&self) -> Principal {
        match self {
            WithdrawalRequest::CkErc20(request) => request.from,
        }
    }

    pub fn from_subaccount(&self) -> &Option<Subaccount> {
        match self {
            WithdrawalRequest::CkErc20(request) => &request.from_subaccount,
        }
    }

    pub fn into_accepted_withdrawal_request_event(self) -> EventType {
        match self {
            WithdrawalRequest::CkErc20(request) => {
                EventType::AcceptedErc20WithdrawalRequest(request)
            }
        }
    }

    pub fn match_parameter(&self, parameter: &WithdrawalSearchParameter) -> bool {
        use WithdrawalSearchParameter::*;
        match parameter {
            ByWithdrawalId(index) => &self.get_withdrawal_id() == index,
            ByRecipient(address) => &self.payee() == address,
            BySenderAccount(Account { owner, subaccount }) => {
                &self.from() == owner && self.from_subaccount() == &subaccount.map(Subaccount)
            }
        }
    }
}

impl From<Erc20WithdrawalRequest> for WithdrawalRequest {
    fn from(value: Erc20WithdrawalRequest) -> Self {
        WithdrawalRequest::CkErc20(value)
    }
}

/// Ethereum withdrawal request issued by the user.
#[derive(Clone, Eq, PartialEq, Decode, Encode)]
pub struct EthWithdrawalRequest {
    /// The ETH amount that the receiver will get, not accounting for the Ethereum transaction fees.
    #[n(0)]
    pub withdrawal_amount: Wei,
    /// The address to which the minter will send ETH.
    #[n(1)]
    pub destination: Address,
    /// The transaction ID of the ckETH burn operation.
    #[cbor(n(2), with = "crate::cbor::nat")]
    pub ledger_burn_index: Nat,
    /// The owner of the account from which the minter burned ckETH.
    #[cbor(n(3), with = "crate::cbor::principal")]
    pub from: Principal,
    /// The subaccount from which the minter burned ckETH.
    #[n(4)]
    pub from_subaccount: Option<Subaccount>,
    /// The IC time at which the withdrawal request arrived.
    #[n(5)]
    pub created_at: Option<u64>,
}

/// ERC-20 withdrawal request issued by the user.
#[derive(Clone, Eq, PartialEq, Decode, Encode)]
pub struct Erc20WithdrawalRequest {
    /// Amount of burn ckETH that can be used to pay for the Ethereum transaction fees.
    #[n(0)]
    pub max_transaction_fee: Wei,
    /// The ERC-20 amount that the receiver will get.
    #[n(1)]
    pub withdrawal_amount: Erc20Value,
    /// The recipient's address of the sent ERC-20 tokens.
    #[n(2)]
    pub destination: Address,
    /// The transaction ID of the ckETH burn operation on the ckETH ledger.
    /// The owner of the account from which the minter burned ckETH.
    #[cbor(n(3), with = "crate::cbor::principal")]
    pub from: Principal,
    /// The subaccount from which the minter burned ckETH.
    #[n(4)]
    pub from_subaccount: Option<Subaccount>,
    /// The IC time at which the withdrawal request arrived.
    #[n(5)]
    pub created_at: u64,
    /// The transaction ID.
    #[cbor(n(6), with = "crate::cbor::nat")]
    pub id: Nat,
}

impl From<&WithdrawalRequest> for ReimbursementIndex {
    fn from(value: &WithdrawalRequest) -> Self {
        match value {
            WithdrawalRequest::CkErc20(request) => ReimbursementIndex::CkErc20 {
                withdrawal_id: request.id.clone(),
            },
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Decode, Encode)]
pub enum ReimbursementIndex {
    #[n(1)]
    CkErc20 {
        #[cbor(n(0), with = "crate::cbor::nat")]
        withdrawal_id: Nat,
    },
}

impl ReimbursementIndex {
    pub fn id(&self) -> Nat {
        match self {
            Self::CkErc20 { withdrawal_id } => withdrawal_id.clone(),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Debug, Decode, Encode)]
pub struct ReimbursementRequest {
    /// Burn index on the ledger that should be reimbursed.
    #[cbor(n(0), with = "crate::cbor::nat")]
    pub ledger_burn_index: Nat,
    /// The amount that should be reimbursed in the smallest denomination.
    #[n(1)]
    pub reimbursed_amount: CkTokenAmount,
    #[cbor(n(2), with = "crate::cbor::principal")]
    pub to: Principal,
    #[n(3)]
    pub to_subaccount: Option<Subaccount>,
    /// Transaction hash of the failed ETH transaction.
    /// We use this hash to link the mint reimbursement transaction
    /// on the ledger with the failed ETH transaction.
    #[n(4)]
    pub transaction_hash: Option<Hash>,
}

#[derive(Clone, Eq, PartialEq, Debug, Decode, Encode)]
pub struct Reimbursed {
    #[cbor(n(0), with = "crate::cbor::id")]
    pub reimbursed_in_block: LedgerMintIndex,
    #[cbor(n(1), with = "crate::cbor::nat")]
    pub burn_in_block: Nat,
    /// The amount reimbursed in the smallest denomination.
    #[n(2)]
    pub reimbursed_amount: CkTokenAmount,
    #[n(3)]
    pub transaction_hash: Option<Hash>,
}

pub type ReimbursedResult = Result<Reimbursed, ReimbursedError>;

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum ReimbursedError {
    /// Whether reimbursement was minted or not is unknown,
    /// most likely because there was an unexpected panic in the callback.
    /// The reimbursement request is quarantined to avoid any double minting and
    /// will not be further processed without manual intervention.
    Quarantined,
}

#[derive(Clone, Eq, PartialEq, Decode, Encode)]
#[cbor(transparent)]
pub struct Subaccount(#[cbor(n(0), with = "minicbor::bytes")] pub [u8; 32]);

impl Subaccount {
    pub fn to_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Debug for Subaccount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", hex::encode(self.0))
    }
}

struct DebugPrincipal<'a>(&'a Principal);

impl fmt::Debug for DebugPrincipal<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for EthWithdrawalRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let EthWithdrawalRequest {
            withdrawal_amount,
            destination,
            ledger_burn_index,
            from,
            from_subaccount,
            created_at,
        } = self;
        f.debug_struct("EthWithdrawalRequest")
            .field("withdrawal_amount", withdrawal_amount)
            .field("destination", destination)
            .field("ledger_burn_index", ledger_burn_index)
            .field("from", &DebugPrincipal(from))
            .field("from_subaccount", from_subaccount)
            .field("created_at", created_at)
            .finish()
    }
}

impl fmt::Debug for Erc20WithdrawalRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let Erc20WithdrawalRequest {
            max_transaction_fee,
            withdrawal_amount,
            destination,
            from,
            from_subaccount,
            created_at,
            id,
        } = self;
        f.debug_struct("Erc20WithdrawalRequest")
            .field("max_transaction_fee", max_transaction_fee)
            .field("withdrawal_amount", withdrawal_amount)
            .field("destination", destination)
            .field("from", &DebugPrincipal(from))
            .field("from_subaccount", from_subaccount)
            .field("created_at", created_at)
            .field("id", id)
            .finish()
    }
}

/// State machine holding Ethereum transactions issued by the minter.
/// Overall the transaction lifecycle is as follows:
/// 1. The user's withdrawal request is enqueued and processed in a FIFO order.
/// 2. A transaction is created by either consuming a withdrawal request
///    (the first time a transaction is created for that nonce and burn index)
///    or re-submitting an already sent transaction for that nonce and burn index.
/// 3. The transaction is signed via threshold ECDSA and recorded by either consuming the
///    previously created transaction or re-submitting an already sent transaction as is.
/// 4. The transaction is sent to Ethereum. There may have been multiple
///    sent transactions for that nonce and burn index in case of resubmissions.
/// 5. For a given nonce (and burn index), at most one sent transaction is finalized.
///    The others sent transactions for that nonce were never mined and can be discarded.
/// 6. If a given transaction fails the minter will reimburse the user who requested the
///    withdrawal with the corresponding amount minus fees.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct EthTransactions {
    pub(in crate::state) pending_withdrawal_requests: VecDeque<WithdrawalRequest>,
    // Processed withdrawal requests (transaction created, sent, or finalized).
    pub(in crate::state) processed_withdrawal_requests: BTreeMap<Nat, WithdrawalRequest>,
    pub(in crate::state) created_tx: MultiKeyMap<TransactionNonce, Nat, TransactionRequest>,
    pub(in crate::state) sent_tx: MultiKeyMap<TransactionNonce, Nat, Vec<SignedTransactionRequest>>,
    pub(in crate::state) finalized_tx:
        MultiKeyMap<TransactionNonce, Nat, FinalizedEip1559Transaction>,
    pub(in crate::state) next_nonce: TransactionNonce,

    pub(in crate::state) maybe_reimburse: BTreeSet<Nat>,
    pub(in crate::state) reimbursement_requests: BTreeMap<ReimbursementIndex, ReimbursementRequest>,
    pub(in crate::state) reimbursed: BTreeMap<ReimbursementIndex, ReimbursedResult>,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum CreateTransactionError {
    InsufficientTransactionFee {
        withdrawal_id: Nat,
        allowed_max_transaction_fee: Wei,
        actual_max_transaction_fee: Wei,
    },
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum ResubmitTransactionError {
    InsufficientTransactionFee {
        withdrawal_id: Nat,
        transaction_nonce: TransactionNonce,
        allowed_max_transaction_fee: Wei,
        max_transaction_fee: Wei,
    },
}

impl EthTransactions {
    pub fn new(next_nonce: TransactionNonce) -> Self {
        Self {
            pending_withdrawal_requests: VecDeque::new(),
            processed_withdrawal_requests: BTreeMap::new(),
            created_tx: MultiKeyMap::default(),
            sent_tx: MultiKeyMap::default(),
            finalized_tx: MultiKeyMap::default(),
            next_nonce,
            maybe_reimburse: Default::default(),
            reimbursement_requests: Default::default(),
            reimbursed: Default::default(),
        }
    }

    pub fn next_transaction_nonce(&self) -> TransactionNonce {
        self.next_nonce
    }

    pub fn update_next_transaction_nonce(&mut self, new_nonce: TransactionNonce) {
        self.next_nonce = new_nonce;
    }

    pub fn reimbursement_requests_iter(
        &self,
    ) -> impl Iterator<Item = (&ReimbursementIndex, &ReimbursementRequest)> {
        self.reimbursement_requests.iter()
    }

    pub fn reimbursed_transactions_iter(
        &self,
    ) -> impl Iterator<Item = (&ReimbursementIndex, &ReimbursedResult)> {
        self.reimbursed.iter()
    }

    fn find_reimbursed_transaction_by_get_withdrawal_id(
        &self,
        searched_burn_index: &Nat,
    ) -> Option<&ReimbursedResult> {
        self.reimbursed
            .iter()
            .find_map(|(index, value)| match index {
                ReimbursementIndex::CkErc20 { withdrawal_id, .. }
                    if withdrawal_id == searched_burn_index =>
                {
                    Some(value)
                }
                _ => None,
            })
    }

    pub fn record_withdrawal_request<R: Into<WithdrawalRequest>>(&mut self, request: R) {
        let request = request.into();
        let withdrawal_id = request.get_withdrawal_id();
        if self
            .pending_withdrawal_requests
            .iter()
            .any(|r| r.get_withdrawal_id() == withdrawal_id)
            || self.created_tx.contains_alt(&withdrawal_id)
            || self.sent_tx.contains_alt(&withdrawal_id)
            || self.finalized_tx.contains_alt(&withdrawal_id)
        {
            panic!("BUG: duplicate ckETH ledger burn index {withdrawal_id}");
        }
        self.pending_withdrawal_requests.push_back(request);
    }

    /// Move an existing withdrawal request to the back of the queue.
    pub fn reschedule_withdrawal_request<R: Into<WithdrawalRequest>>(&mut self, request: R) {
        let request = request.into();
        assert_eq!(
            self.pending_withdrawal_requests
                .iter()
                .filter(|r| r.get_withdrawal_id() == request.get_withdrawal_id())
                .count(),
            1,
            "BUG: expected exactly one withdrawal request with ckETH ledger burn index {}",
            request.get_withdrawal_id()
        );
        self.remove_withdrawal_request(&request);
        self.record_withdrawal_request(request);
    }

    pub fn record_created_transaction(
        &mut self,
        withdrawal_id: Nat,
        transaction: Eip1559TransactionRequest,
    ) {
        let withdrawal_request = self
            .pending_withdrawal_requests
            .iter()
            .find(|req| req.get_withdrawal_id() == withdrawal_id)
            .cloned()
            .unwrap_or_else(|| panic!("BUG: withdrawal request {withdrawal_id} not found"));
        assert!(
            self.pending_withdrawal_requests
                .contains(&withdrawal_request),
            "BUG: withdrawal request not found"
        );
        match &withdrawal_request {
            WithdrawalRequest::CkErc20(_req) => {
                assert_eq!(
                    Wei::ZERO,
                    transaction.amount,
                    "BUG: ERC-20 transaction amount should be zero"
                );
            }
        }
        let nonce = self.next_nonce;
        assert_eq!(transaction.nonce, nonce, "BUG: transaction nonce mismatch");
        self.next_nonce = self
            .next_nonce
            .checked_increment()
            .expect("Transaction nonce overflow");
        self.remove_withdrawal_request(&withdrawal_request);
        let transaction_request = TransactionRequest {
            transaction,
            resubmission: match &withdrawal_request {
                WithdrawalRequest::CkErc20(ckerc20) => ResubmissionStrategy::GuaranteeEthAmount {
                    allowed_max_transaction_fee: ckerc20.max_transaction_fee,
                },
            },
        };
        assert_eq!(
            self.created_tx.try_insert(
                nonce,
                withdrawal_request.get_withdrawal_id(),
                transaction_request
            ),
            Ok(())
        );
        assert_eq!(
            self.processed_withdrawal_requests
                .insert(withdrawal_id.clone(), withdrawal_request),
            None
        );
        assert!(self.maybe_reimburse.insert(withdrawal_id));
    }

    pub fn record_signed_transaction(
        &mut self,
        signed_transaction: SignedEip1559TransactionRequest,
    ) {
        let created_tx = self
            .created_tx
            .get(&signed_transaction.nonce())
            .expect("BUG: missing created transaction");
        assert_eq!(
            created_tx.as_ref(),
            signed_transaction.transaction(),
            "BUG: mismatch between sent transaction and created transaction"
        );
        let signed_tx = created_tx.clone_resubmission_strategy(signed_transaction);
        let (nonce, ledger_burn_index, _created_tx) = self
            .created_tx
            .remove_entry(&signed_tx.as_ref().nonce())
            .expect("BUG: missing created transaction");
        if let Some(sent_tx) = self.sent_tx.get_mut(&nonce) {
            sent_tx.push(signed_tx);
        } else {
            assert_eq!(
                self.sent_tx
                    .try_insert(nonce, ledger_burn_index, vec![signed_tx]),
                Ok(())
            );
        }
    }

    /// Create transactions to resubmit corresponding to already sent transactions
    /// with nonces greater than the latest mined transaction nonce:
    /// * the resubmitted transaction will need to be re-signed if its transaction fee was increased
    /// * the resubmitted transaction can be resent as is if its transaction fee was not increased
    ///
    /// We stop on the first error since if a transaction with nonce n could not be resubmitted
    /// (e.g., the transaction amount does not cover the new fees),
    /// then the next transactions with nonces n+1, n+2, ... are blocked anyway
    /// and trying to resubmit them would only artificially increase their transaction fees.
    pub fn create_resubmit_transactions(
        &self,
        latest_transaction_count: TransactionCount,
        current_gas_fee: GasFeeEstimate,
    ) -> Vec<Result<(Nat, Eip1559TransactionRequest), ResubmitTransactionError>> {
        // If transaction count at block height H is c > 0, then transactions with nonces
        // 0, 1, ..., c - 1 were mined. If transaction count is 0, then no transactions were mined.
        // The nonce of the first pending transaction is then exactly c.
        let first_pending_tx_nonce: TransactionNonce = latest_transaction_count.change_units();
        let mut transactions_to_resubmit = Vec::new();
        for (nonce, withdrawal_id, signed_tx) in self
            .sent_tx
            .iter()
            .filter(|(nonce, _withdrawal_id, _signed_tx)| *nonce >= &first_pending_tx_nonce)
        {
            let last_signed_tx = signed_tx.last().expect("BUG: empty sent transactions list");
            match last_signed_tx.resubmit(current_gas_fee.clone()) {
                Ok(Some(new_tx)) => {
                    transactions_to_resubmit.push(Ok((withdrawal_id.clone(), new_tx)));
                }
                Ok(None) => {
                    // the transaction fee is still up-to-date but because the transaction did not get included,
                    // we re-send it as is to be sure that it remains known to the mempool and hopefully be included at some point.
                    // Since we always re-send the last non-included transactions in sent_tx, there is nothing to do.
                }
                Err(crate::tx::ResubmitTransactionError::InsufficientTransactionFee {
                    allowed_max_transaction_fee,
                    actual_max_transaction_fee,
                }) => {
                    transactions_to_resubmit.push(Err(
                        ResubmitTransactionError::InsufficientTransactionFee {
                            withdrawal_id: withdrawal_id.clone(),
                            transaction_nonce: *nonce,
                            allowed_max_transaction_fee,
                            max_transaction_fee: actual_max_transaction_fee,
                        },
                    ));
                    return transactions_to_resubmit;
                }
            }
        }
        transactions_to_resubmit
    }

    pub fn record_resubmit_transaction(&mut self, new_tx: Eip1559TransactionRequest) {
        let nonce = new_tx.nonce;
        let (ledger_burn_index, last_sent_tx) =
            Self::expect_last_sent_tx_entry(&self.sent_tx, &nonce);
        assert!(equal_ignoring_fee_and_amount(last_sent_tx.as_ref().transaction(), &new_tx),
                "BUG: mismatch between last sent transaction {last_sent_tx:?} and the transaction to resubmit {new_tx:?}");
        Self::cleanup_failed_resubmitted_transactions(&mut self.created_tx, &nonce);
        let new_tx = last_sent_tx.clone_resubmission_strategy(new_tx);
        assert_eq!(
            self.created_tx
                .try_insert(nonce, ledger_burn_index.clone(), new_tx),
            Ok(())
        );
    }

    pub fn sent_transactions_to_finalize(
        &self,
        finalized_transaction_count: &TransactionCount,
    ) -> BTreeMap<Hash, Nat> {
        let first_non_finalized_tx_nonce: TransactionNonce =
            finalized_transaction_count.change_units();
        let mut transactions = BTreeMap::new();
        for (_nonce, index, sent_txs) in self
            .sent_tx
            .iter()
            .filter(|(nonce, _burn_index, _signed_txs)| *nonce < &first_non_finalized_tx_nonce)
        {
            for sent_tx in sent_txs {
                if let Some(prev_index) =
                    transactions.insert(sent_tx.as_ref().hash(), index.clone())
                {
                    assert_eq!(prev_index, *index,
                               "BUG: duplicate transaction hash {} for burn indices {prev_index} and {index}", sent_tx.as_ref().hash());
                }
            }
        }
        transactions
    }

    pub fn record_finalized_transaction(
        &mut self,
        ledger_burn_index: Nat,
        receipt: TransactionReceipt,
    ) {
        let sent_tx = self
            .sent_tx
            .get_alt(&ledger_burn_index)
            .expect("BUG: missing sent transactions")
            .iter()
            .find(|sent_tx| sent_tx.as_ref().hash() == receipt.transaction_hash)
            .expect("ERROR: no transaction matching receipt");
        let finalized_tx = sent_tx
            .as_ref()
            .clone()
            .try_finalize(receipt.clone())
            .expect("ERROR: invalid transaction receipt");

        let nonce = sent_tx.as_ref().nonce();
        {
            self.sent_tx.remove_entry(&nonce);
            Self::cleanup_failed_resubmitted_transactions(&mut self.created_tx, &nonce);
        }
        assert_eq!(
            self.finalized_tx
                .try_insert(nonce, ledger_burn_index.clone(), finalized_tx.clone()),
            Ok(())
        );

        assert!(
            self.maybe_reimburse.remove(&ledger_burn_index.clone()),
            "failed to remove entry from maybe_reimburse with block index: {ledger_burn_index}",
        );

        let request = self.processed_withdrawal_requests
            .get(&ledger_burn_index)
            .expect("failed to find entry from processed_withdrawal_requests with block index: {ledger_burn_index}");
        let index = ReimbursementIndex::from(request);
        match &request {
            WithdrawalRequest::CkErc20(request) => {
                if receipt.status == TransactionStatus::Failure {
                    self.record_reimbursement_request(
                        index,
                        ReimbursementRequest {
                            ledger_burn_index: request.id.clone(),
                            reimbursed_amount: request.withdrawal_amount.change_units(),
                            to: request.from,
                            to_subaccount: request.from_subaccount.clone(),
                            transaction_hash: Some(receipt.transaction_hash),
                        },
                    );
                }
            }
        }
    }

    pub fn record_reimbursement_request(
        &mut self,
        index: ReimbursementIndex,
        request: ReimbursementRequest,
    ) {
        assert_eq!(
            self.maybe_reimburse.get(&index.id()),
            None,
            "BUG: withdrawal request still in maybe_reimburse could lead to double minting!"
        );
        assert_eq!(
            self.reimbursed.get(&index),
            None,
            "BUG: reimbursement request was already processed"
        );
        assert_eq!(
            self.reimbursement_requests.insert(index.clone(), request),
            None,
            "BUG: reimbursement request for withdrawal {index:?} already exists"
        );
    }

    /// Quarantine the reimbursement request identified by its index to prevent double minting.
    /// WARNING!: It's crucial that this method does not panic,
    /// since it's called inside the clean-up callback, when an unexpected panic did occur before.
    pub fn record_quarantined_reimbursement(&mut self, index: ReimbursementIndex) {
        self.reimbursement_requests.remove(&index);
        self.reimbursed
            .insert(index, Err(ReimbursedError::Quarantined));
    }

    pub fn withdrawal_status(
        &self,
        parameter: &WithdrawalSearchParameter,
    ) -> Vec<(
        &WithdrawalRequest,
        WithdrawalStatus,
        Option<&Eip1559TransactionRequest>,
    )> {
        // Pending requests matching the given search parameter
        let pending = self.pending_withdrawal_requests.iter().filter_map(|r| {
            r.match_parameter(parameter)
                .then_some((r, WithdrawalStatus::Pending, None))
        });

        // Processed withdrawal requests matching the given search parameter.
        let processed = self
            .processed_withdrawal_requests
            .values()
            .filter(|r| r.match_parameter(parameter))
            .map(
                |request| match self.processed_transaction_status(&request.get_withdrawal_id()) {
                    (RetrieveEthStatus::TxCreated, Some(tx)) => {
                        (request, WithdrawalStatus::TxCreated, Some(tx))
                    }
                    (RetrieveEthStatus::TxSent(sent), Some(tx)) => {
                        (request, WithdrawalStatus::TxSent(sent), Some(tx))
                    }
                    (RetrieveEthStatus::TxFinalized(status), Some(tx)) => {
                        (request, WithdrawalStatus::TxFinalized(status), Some(tx))
                    }
                    _ => {
                        panic!("Status of processed request is not found {:?}", request)
                    }
                },
            );

        pending.chain(processed).collect()
    }

    pub fn transaction_status(&self, burn_index: &Nat) -> RetrieveEthStatus {
        if self
            .pending_withdrawal_requests
            .iter()
            .any(|r| &r.get_withdrawal_id() == burn_index)
        {
            return RetrieveEthStatus::Pending;
        }
        self.processed_transaction_status(burn_index).0
    }

    fn processed_transaction_status(
        &self,
        burn_index: &Nat,
    ) -> (RetrieveEthStatus, Option<&Eip1559TransactionRequest>) {
        if let Some(tx) = self.created_tx.get_alt(burn_index) {
            return (RetrieveEthStatus::TxCreated, Some(tx.as_ref()));
        }

        if let Some(tx) = self.sent_tx.get_alt(burn_index).and_then(|txs| txs.last()) {
            return (
                RetrieveEthStatus::TxSent(EthTransaction::from(tx.as_ref())),
                Some(tx.as_ref().as_ref()),
            );
        }

        if let Some(tx) = self.finalized_tx.get_alt(burn_index) {
            if let Some(Ok(reimbursed)) =
                self.find_reimbursed_transaction_by_get_withdrawal_id(burn_index)
            {
                return (
                    RetrieveEthStatus::TxFinalized(TxFinalizedStatus::Reimbursed {
                        reimbursed_in_block: reimbursed.reimbursed_in_block.get().into(),
                        transaction_hash: tx.transaction_hash().to_string(),
                        reimbursed_amount: reimbursed.reimbursed_amount.into(),
                    }),
                    Some(tx.as_ref()),
                );
            }
            if tx.transaction_status() == &TransactionStatus::Failure {
                return (
                    RetrieveEthStatus::TxFinalized(TxFinalizedStatus::PendingReimbursement(
                        EthTransaction {
                            transaction_hash: tx.transaction_hash().to_string(),
                        },
                    )),
                    Some(tx.as_ref()),
                );
            }

            return (
                RetrieveEthStatus::TxFinalized(TxFinalizedStatus::Success {
                    transaction_hash: tx.transaction_hash().to_string(),
                    effective_transaction_fee: Some(tx.effective_transaction_fee().into()),
                }),
                Some(tx.as_ref()),
            );
        }

        (RetrieveEthStatus::NotFound, None)
    }

    pub fn withdrawal_requests_batch(&self, requested_batch_size: usize) -> Vec<WithdrawalRequest> {
        // The number of pending transaction nonces is counted and not the number of pending transactions
        // because a nonce may be associated with several distinct transactions (due to re-submission and dynamic fees).
        // However, once a nonce is chosen for a withdrawal request, it's in our interest that the corresponding transaction be finalized asap.
        // Limiting the number of transactions would be counter-productive.
        const MAX_NUM_PENDING_TRANSACTION_NONCES: usize = 1000;
        let unique_pending_transaction_nonces: BTreeSet<_> =
            self.created_tx.keys().chain(self.sent_tx.keys()).collect();
        let actual_batch_size = min(
            MAX_NUM_PENDING_TRANSACTION_NONCES
                .saturating_sub(unique_pending_transaction_nonces.len()),
            requested_batch_size,
        );
        self.withdrawal_requests_iter()
            .take(actual_batch_size)
            .cloned()
            .collect()
    }

    pub fn withdrawal_requests_iter(&self) -> impl Iterator<Item = &WithdrawalRequest> {
        self.pending_withdrawal_requests.iter()
    }

    pub fn withdrawal_requests_len(&self) -> usize {
        self.pending_withdrawal_requests.len()
    }

    pub fn maybe_reimburse_requests_iter(&self) -> impl Iterator<Item = &WithdrawalRequest> {
        self.processed_withdrawal_requests
            .iter()
            .filter_map(|(index, request)| {
                if self.maybe_reimburse.contains(index) {
                    Some(request)
                } else {
                    None
                }
            })
    }

    pub fn transactions_to_sign_iter(
        &self,
    ) -> impl Iterator<Item = (&TransactionNonce, &Nat, &Eip1559TransactionRequest)> {
        self.created_tx
            .iter()
            .map(|(nonce, ledger_burn_index, tx)| (nonce, ledger_burn_index, tx.as_ref()))
    }

    pub fn transactions_to_sign_batch(
        &self,
        batch_size: usize,
    ) -> Vec<(Nat, Eip1559TransactionRequest)> {
        self.transactions_to_sign_iter()
            .take(batch_size)
            .map(|(_nonce, withdrawal_id, tx)| (withdrawal_id.clone(), tx.clone()))
            .collect()
    }

    pub fn transactions_to_send_batch(
        &self,
        latest_transaction_count: TransactionCount,
        batch_size: usize,
    ) -> Vec<SignedEip1559TransactionRequest> {
        let first_pending_tx_nonce: TransactionNonce = latest_transaction_count.change_units();
        self.sent_tx
            .iter()
            .filter_map(move |(nonce, ledger_burn_index, txs)| {
                txs.last()
                    .map(|tx| (nonce, ledger_burn_index, tx))
                    .filter(|(nonce, _ledger_burn_index, _tx)| *nonce >= &first_pending_tx_nonce)
            })
            .take(batch_size)
            .map(|(_nonce, _index, tx)| tx.as_ref())
            .cloned()
            .collect()
    }

    pub fn sent_transactions_iter(
        &self,
    ) -> impl Iterator<
        Item = (
            &TransactionNonce,
            &Nat,
            Vec<&SignedEip1559TransactionRequest>,
        ),
    > {
        self.sent_tx
            .iter()
            .map(|(nonce, index, txs)| (nonce, index, txs.iter().map(|tx| tx.as_ref()).collect()))
    }

    pub fn get_finalized_transaction(
        &self,
        burn_index: &Nat,
    ) -> Option<&FinalizedEip1559Transaction> {
        self.finalized_tx.get_alt(burn_index)
    }

    pub fn get_processed_withdrawal_request(&self, burn_index: &Nat) -> Option<&WithdrawalRequest> {
        self.processed_withdrawal_requests.get(burn_index)
    }

    pub fn finalized_transactions_iter(
        &self,
    ) -> impl Iterator<Item = (&TransactionNonce, &Nat, &FinalizedEip1559Transaction)> {
        self.finalized_tx.iter()
    }

    pub fn is_sent_tx_empty(&self) -> bool {
        self.sent_tx.is_empty()
    }

    pub fn has_pending_requests(&self) -> bool {
        !self.pending_withdrawal_requests.is_empty()
            || !self.created_tx.is_empty()
            || !self.sent_tx.is_empty()
    }

    fn remove_withdrawal_request(&mut self, request: &WithdrawalRequest) {
        self.pending_withdrawal_requests.retain(|r| r != request);
    }

    fn expect_last_sent_tx_entry<'a>(
        sent_tx: &'a MultiKeyMap<TransactionNonce, Nat, Vec<SignedTransactionRequest>>,
        nonce: &TransactionNonce,
    ) -> (&'a Nat, &'a SignedTransactionRequest) {
        let (ledger_burn_index, sent_txs) = sent_tx
            .get_entry(nonce)
            .expect("BUG: sent transaction not found");
        let last_sent_tx = sent_txs.last().expect("BUG: empty sent transactions list");
        (ledger_burn_index, last_sent_tx)
    }

    fn cleanup_failed_resubmitted_transactions(
        created_tx: &mut MultiKeyMap<TransactionNonce, Nat, TransactionRequest>,
        nonce: &TransactionNonce,
    ) {
        use crate::logs::INFO;
        use ic_canister_log::log;

        if let Some((_nonce, _index, prev_resubmitted_tx)) = created_tx.remove_entry(nonce) {
            log!(INFO, "[cleanup_failed_resubmitted_transactions]: removing previously resubmitted transaction {prev_resubmitted_tx:?} that failed to progress");
        }
    }

    /// Checks whether two transaction state machines are equivalent.
    pub fn is_equivalent_to(&self, other: &Self) -> Result<(), String> {
        use ic_utils_ensure::ensure_eq;

        fn sorted_requests(requests: &VecDeque<WithdrawalRequest>) -> Vec<WithdrawalRequest> {
            let mut buf: Vec<_> = requests.iter().cloned().collect();
            buf.sort_unstable_by_key(|req| req.get_withdrawal_id());
            buf
        }

        // We can reorder request in `reschedule_withdrawal_request`. The audit log won't
        // reflect this change, so we must sort the queues before comparing them.
        ensure_eq!(
            sorted_requests(&self.pending_withdrawal_requests),
            sorted_requests(&other.pending_withdrawal_requests)
        );
        ensure_eq!(self.created_tx, other.created_tx);
        ensure_eq!(self.sent_tx, other.sent_tx);
        ensure_eq!(self.finalized_tx, other.finalized_tx);
        ensure_eq!(self.next_nonce, other.next_nonce);

        ensure_eq!(self.maybe_reimburse, other.maybe_reimburse);
        ensure_eq!(self.reimbursement_requests, other.reimbursement_requests);
        ensure_eq!(self.reimbursed, other.reimbursed);

        Ok(())
    }

    pub fn oldest_incomplete_withdrawal_timestamp(&self) -> Option<u64> {
        self.withdrawal_requests_iter()
            .chain(self.maybe_reimburse_requests_iter())
            .flat_map(|req| req.created_at().into_iter())
            .min()
    }
}

/// Creates an EIP-1559 transaction for the given withdrawal request.
/// The transaction fees are paid by the beneficiary,
/// meaning that the fees will be deducted from the withdrawal amount.
///
/// # Errors
/// * `CreateTransactionError::InsufficientTransactionFee` if the ETH withdrawal amount does not cover the transaction fee.
pub fn create_transaction(
    withdrawal_request: &WithdrawalRequest,
    nonce: TransactionNonce,
    gas_fee_estimate: GasFeeEstimate,
    gas_limit: GasAmount,
    ethereum_network: EthereumNetwork,
) -> Result<Eip1559TransactionRequest, CreateTransactionError> {
    assert!(
        gas_limit > GasAmount::ZERO,
        "BUG: gas limit should be non-zero"
    );
    match withdrawal_request {
        WithdrawalRequest::CkErc20(request) => {
            // The transaction fee is already paid and must be at most
            // the `max_transaction_fee` in the withdrawal request, which, given a gas limit, gives us an upper bound on
            // the `max_fee_per_gas`. We allocate the maximum from the beginning to minimize
            // transaction resubmissions: even if the `base_fee_per_gas` increases considerably,
            // the transaction could still make it as long as `transaction.max_fee_per_gas >=  block.base_fee_per_gas`,
            // since the `priority_fee_per_gas` received by the miner is capped to (see https://eips.ethereum.org/EIPS/eip-1559)
            // min(transaction.max_priority_fee_per_gas, transaction.max_fee_per_gas - block.base_fee_per_gas).
            let request_max_fee_per_gas = request
                .max_transaction_fee
                .into_wei_per_gas(gas_limit)
                .expect("BUG: gas_limit should be non-zero");
            let actual_min_max_fee_per_gas = gas_fee_estimate.min_max_fee_per_gas();
            if actual_min_max_fee_per_gas > request_max_fee_per_gas {
                return Err(CreateTransactionError::InsufficientTransactionFee {
                    allowed_max_transaction_fee: request.max_transaction_fee,
                    actual_max_transaction_fee: actual_min_max_fee_per_gas
                        .transaction_cost(gas_limit)
                        .unwrap_or(Wei::MAX),
                    withdrawal_id: request.id.clone(),
                });
            }

            let erc20_contract_address = read_state(|s| s.ckerc20_tokens.0);

            Ok(Eip1559TransactionRequest {
                chain_id: ethereum_network.chain_id(),
                nonce,
                max_priority_fee_per_gas: gas_fee_estimate.max_priority_fee_per_gas,
                max_fee_per_gas: request_max_fee_per_gas,
                gas_limit,
                destination: erc20_contract_address,
                amount: Wei::ZERO,
                data: TransactionCallData::Erc20Transfer {
                    to: request.destination,
                    value: request.withdrawal_amount,
                }
                .encode(),
                access_list: Default::default(),
            })
        }
    }
}

// First 4 bytes of keccak256(transfer(address,uint256))
const ERC_20_TRANSFER_FUNCTION_SELECTOR: [u8; 4] = hex_literal::hex!("a9059cbb");

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum TransactionCallData {
    Erc20Transfer { to: Address, value: Erc20Value },
}

impl TransactionCallData {
    /// Encode the transaction call data to interact with an Ethereum smart contract.
    /// See the [Contract ABI Specification](https://docs.soliditylang.org/en/develop/abi-spec.html#contract-abi-specification).
    pub fn encode(&self) -> Vec<u8> {
        match self {
            TransactionCallData::Erc20Transfer { to, value } => {
                let mut data = Vec::with_capacity(68);
                data.extend(ERC_20_TRANSFER_FUNCTION_SELECTOR);
                data.extend(<[u8; 32]>::from(to));
                data.extend(value.to_be_bytes());
                data
            }
        }
    }

    pub fn decode<T: AsRef<[u8]>>(data: T) -> Result<Self, String> {
        let data = data.as_ref();
        match data.get(0..4) {
            Some(selector) if selector == ERC_20_TRANSFER_FUNCTION_SELECTOR => {
                if data.len() != 68 {
                    return Err("Invalid data length".to_string());
                }
                let address = <[u8; 32]>::try_from(&data[4..36]).unwrap();
                let to = Address::try_from(&address).unwrap();

                let value = <[u8; 32]>::try_from(&data[36..]).unwrap();
                let value = Erc20Value::from_be_bytes(value);

                Ok(TransactionCallData::Erc20Transfer { to, value })
            }
            Some(selector) => Err(format!(
                "Unknown function selector 0x{:?}",
                hex::encode(selector)
            )),
            None => Err("missing function selector".to_string()),
        }
    }
}

/// Returns true if the two transactions are equal ignoring the transaction fee and amount.
/// The following fields are ignored:
/// * `max_fee_per_gas`
/// * `max_priority_fee_per_gas`
/// * `amount` (because the cost of the transaction is paid by the beneficiary and so influencing the fee does influence the transaction amount)
fn equal_ignoring_fee_and_amount(
    lhs: &Eip1559TransactionRequest,
    rhs: &Eip1559TransactionRequest,
) -> bool {
    let mut rhs_with_lhs_fee_and_amount = rhs.clone();
    rhs_with_lhs_fee_and_amount.max_fee_per_gas = lhs.max_fee_per_gas;
    rhs_with_lhs_fee_and_amount.max_priority_fee_per_gas = lhs.max_priority_fee_per_gas;
    rhs_with_lhs_fee_and_amount.amount = lhs.amount;

    lhs == &rhs_with_lhs_fee_and_amount
}
