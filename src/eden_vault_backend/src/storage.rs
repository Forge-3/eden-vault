use crate::state::event::{Event, EventType};
use ic_stable_structures::{
    log::Log as StableLog,
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    storable::{Bound, Storable},
    DefaultMemoryImpl,
};
use std::borrow::Cow;
use std::cell::RefCell;

const LOG_INDEX_MEMORY_ID: MemoryId = MemoryId::new(0);
const LOG_DATA_MEMORY_ID: MemoryId = MemoryId::new(1);

type VMem = VirtualMemory<DefaultMemoryImpl>;
type EventLog = StableLog<Event, VMem, VMem>;

impl Storable for Event {
    fn to_bytes(&self) -> Cow<[u8]> {
        let mut buf = vec![];
        minicbor::encode(self, &mut buf).expect("event encoding should always succeed");
        Cow::Owned(buf)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        minicbor::decode(bytes.as_ref())
            .unwrap_or_else(|e| panic!("failed to decode event bytes {}: {e}", hex::encode(bytes)))
    }

    const BOUND: Bound = Bound::Unbounded;
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    /// The log of the ckETH state modifications.
    static EVENTS: RefCell<EventLog> = MEMORY_MANAGER
        .with(|m|
              RefCell::new(
                  StableLog::init(
                      m.borrow().get(LOG_INDEX_MEMORY_ID),
                      m.borrow().get(LOG_DATA_MEMORY_ID)
                  ).expect("failed to initialize stable log")
              )
        );
}

/// Appends the event to the event log.
pub fn record_event(payload: EventType) {
    EVENTS
        .with(|events| {
            events.borrow().append(&Event {
                timestamp: ic_cdk::api::time(),
                payload,
            })
        })
        .expect("recording an event should succeed");
}

/// Returns the total number of events in the audit log.
pub fn total_event_count() -> u64 {
    EVENTS.with(|events| events.borrow().len())
}

pub fn with_event_iter<F, R>(f: F) -> R
where
    F: for<'a> FnOnce(Box<dyn Iterator<Item = Event> + 'a>) -> R,
{
    EVENTS.with(|events| f(Box::new(events.borrow().iter())))
}

/// Appends the event to the event log.
pub fn retry_tx_events() {
    EVENTS
        .with(|events| {
            let events = events.borrow().iter().filter(|event| {
                match &event.payload {
                    EventType::Init(_) => true,
                    EventType::Upgrade(_) => true,
                    EventType::InvalidDeposit { event_source: _, reason: _ } => true,
                    EventType::SyncedToBlock { block_number: _ } => true,
                    EventType::CreatedTransaction { withdrawal_id: _, transaction: _ } => false,
                    EventType::SignedTransaction { withdrawal_id: _, transaction: _ } => false,
                    EventType::ReplacedTransaction { withdrawal_id: _, transaction: _ } => true,
                    EventType::FinalizedTransaction { withdrawal_id: _, transaction_receipt: _ } => true,
                    EventType::AcceptedErc20Deposit(_) => true,
                    EventType::AcceptedErc20WithdrawalRequest(_) => true,
                    EventType::MintedCkErc20 { event_source: _, principal: _, amount: _ } => true,
                    EventType::SyncedErc20ToBlock { block_number: _ } => true,
                    EventType::QuarantinedDeposit { event_source: _ } => true,
                    EventType::QuarantinedReimbursement { index: _ } => true,
                    EventType::SkippedBlockForContract { contract_address: _, block_number: _ } => true,
                    EventType::Erc20TransferCompleted { from: _, to: _, amount: _ } => true,
                }
            }).collect::<Vec<_>>();
        });
}