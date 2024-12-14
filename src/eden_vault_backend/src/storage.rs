use crate::{state::event::{Event, EventType}, user::User};
use ic_stable_structures::{
    log::Log as StableLog,
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    storable::{Bound, Storable},
    DefaultMemoryImpl,
    Vec as StableVec
};
use std::borrow::Cow;
use std::cell::RefCell;

const OLD_LOG_INDEX_MEMORY_ID: MemoryId = MemoryId::new(0);
const OLD_LOG_DATA_MEMORY_ID: MemoryId = MemoryId::new(1);

const LOG_INDEX_MEMORY_ID: MemoryId = MemoryId::new(4);
const LOG_DATA_MEMORY_ID: MemoryId = MemoryId::new(5);

const VEC_DATA_MEMORY_ID: MemoryId = MemoryId::new(6);

pub type VMem = VirtualMemory<DefaultMemoryImpl>;
type EventLog = StableLog<Event, VMem, VMem>;
type UsersVec = StableVec<User, VMem>;

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

    static USERS: RefCell<UsersVec> = MEMORY_MANAGER
        .with(|m| 
            RefCell::new(
                StableVec::new(
                    m.borrow().get(VEC_DATA_MEMORY_ID)
                ).expect("failed to initialize stable vec")
            )
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

    static OLD_EVENTS: RefCell<EventLog> = MEMORY_MANAGER
    .with(|m|
            RefCell::new(
                StableLog::init(
                    m.borrow().get(OLD_LOG_INDEX_MEMORY_ID),
                    m.borrow().get(OLD_LOG_DATA_MEMORY_ID)
                ).expect("failed to initialize stable log")
            )
    );
}

pub fn push_user(user: &User) {
    USERS
        .with(|users| {
            users.borrow().push(
                user
            )
        })
        .expect("recording an event should succeed");
}

pub fn with_users_iter<F, R>(f: F) -> R
where
    F: for<'a> FnOnce(Box<dyn Iterator<Item = User> + 'a>) -> R,
{
    USERS.with(|user| f(Box::new(user.borrow().iter())))
}

pub fn migrate_event(payload: &Event) {
    EVENTS
        .with(|events| {
            events.borrow().append(payload)
        })
        .expect("recording an event should succeed");
}

/// Appends the event to the event log.
pub fn record_old_event(payload: EventType) {
    OLD_EVENTS
        .with(|events| {
            events.borrow().append(&Event {
                timestamp: ic_cdk::api::time(),
                payload,
            })
        })
        .expect("recording an event should succeed");
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

/// Returns the total number of events in the audit log.
pub fn total_old_event_count() -> u64 {
    OLD_EVENTS.with(|events| events.borrow().len())
}

pub fn with_event_iter<F, R>(f: F) -> R
where
    F: for<'a> FnOnce(Box<dyn Iterator<Item = Event> + 'a>) -> R,
{
    EVENTS.with(|events| f(Box::new(events.borrow().iter())))
}

pub fn with_old_event_iter<F, R>(f: F) -> R
where
    F: for<'a> FnOnce(Box<dyn Iterator<Item = Event> + 'a>) -> R,
{
    OLD_EVENTS.with(|events| f(Box::new(events.borrow().iter())))
}