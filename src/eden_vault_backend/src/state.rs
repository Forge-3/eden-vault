use std::cell::RefCell;
use ic_cdk::api::management_canister::ecdsa::EcdsaPublicKeyResponse;
use crate::types::{Address, EthereumNetwork, Wei};

thread_local! {
    pub static STATE: RefCell<Option<State>> = RefCell::default();
}

#[derive(Clone, PartialEq, Debug)]
pub struct State {
    pub ethereum_network: EthereumNetwork,
    pub ecdsa_key_name: String,
    pub erc20_helper_contract_address: Option<Address>,
    pub ecdsa_public_key: Option<EcdsaPublicKeyResponse>,
    pub cketh_minimum_withdrawal_amount: Wei,
    pub ethereum_block_height: BlockTag,
    pub first_scraped_block_number: BlockNumber,
    pub last_scraped_block_number: BlockNumber,
    pub last_erc20_scraped_block_number: BlockNumber,
    pub last_observed_block_number: Option<BlockNumber>,
    pub events_to_mint: BTreeMap<EventSource, ReceivedEvent>,
    pub minted_events: BTreeMap<EventSource, MintedEvent>,
    pub invalid_events: BTreeMap<EventSource, InvalidEventReason>,
    pub eth_transactions: EthTransactions,
    pub skipped_blocks: BTreeMap<Address, BTreeSet<BlockNumber>>,

    /// Current balance of ETH held by the minter.
    /// Computed based on audit events.
    pub eth_balance: EthBalance,

    /// Current balance of ERC-20 tokens held by the minter.
    /// Computed based on audit events.
    pub erc20_balances: Erc20Balances,

    /// Per-principal lock for pending withdrawals
    pub pending_withdrawal_principals: BTreeSet<Principal>,

    /// Locks preventing concurrent execution timer tasks
    pub active_tasks: HashSet<TaskType>,

    /// Number of HTTP outcalls since the last upgrade.
    /// Used to correlate request and response in logs.
    pub http_request_counter: u64,

    pub last_transaction_price_estimate: Option<(u64, GasFeeEstimate)>,

    /// Canister ID of the EVM RPC canister that
    /// handles communication with Ethereum
    pub evm_rpc_id: Option<Principal>,
}