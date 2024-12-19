use crate::{state::event::{Event, EventType}, user::User};
use ic_stable_structures::{
    log::Log as StableLog,
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    storable::{Bound, Storable},
    DefaultMemoryImpl,
    Cell,
    Vec as StableVec
};
use std::borrow::Cow;
use std::cell::RefCell;

const OLD_LOG_INDEX_MEMORY_ID: MemoryId = MemoryId::new(0);
const OLD_LOG_DATA_MEMORY_ID: MemoryId = MemoryId::new(1);

const LOG_INDEX_MEMORY_ID: MemoryId = MemoryId::new(4);
const LOG_DATA_MEMORY_ID: MemoryId = MemoryId::new(5);

const VEC_DATA_MEMORY_ID: MemoryId = MemoryId::new(7);
const USER_CELL_DATA_MEMORY_ID: MemoryId = MemoryId::new(8);

pub type VMem = VirtualMemory<DefaultMemoryImpl>;
type EventLog = StableLog<Event, VMem, VMem>;
type UsersNextSalt = Cell<u64, VMem>;
type QueueIndex = Cell<u64, VMem>;
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

    static USERS_SALT: RefCell<UsersNextSalt> = MEMORY_MANAGER
    .with(|m| 
        RefCell::new(
            Cell::init(
                m.borrow().get(USER_CELL_DATA_MEMORY_ID),
                198
            ).expect("failed to initialize stable cell")
        )
    );

    static QUEUE_INDEX: RefCell<QueueIndex> = MEMORY_MANAGER
    .with(|m| 
        RefCell::new(
            Cell::init(
                m.borrow().get(USER_CELL_DATA_MEMORY_ID),
                370
            ).expect("failed to initialize stable cell")
        )
    );

    static USERS: RefCell<UsersVec> = MEMORY_MANAGER
    .with(|m| 
        RefCell::new(
            StableVec::init(
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

//QUEUE_INDEX

pub fn get_current_queue_index()-> u64 {
    QUEUE_INDEX.with(|salt| salt.borrow().get().clone())
}

pub fn set_current_queue_index(index: u64) -> u64 {
    QUEUE_INDEX.with(|salt| salt.borrow_mut().set(index).expect("failed to set index"));
    QUEUE_INDEX.with(|salt| salt.borrow().get().clone())
}

pub fn get_next_user_salt() -> u64 {
    USERS_SALT.with(|salt| {
        salt.borrow().get().clone()
    })
}

pub fn inc_user_salt() -> u64 {
    USERS_SALT.with(|salt| {
        let curr = salt.borrow().get().clone();
        salt.borrow_mut().set(curr+1).expect("failed to write archive salt")
    })
}

pub fn users_len() -> u64 {
    USERS
        .with(|users| {
            users.borrow().len()
        })
}

pub fn push_user(user: &User) {
    USERS
        .with(|users| {
            users.borrow().push(
                user
            )
        })
        .expect("recording an user should succeed");
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