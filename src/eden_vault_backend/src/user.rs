use candid::{CandidType, Nat, Principal};
use ic_stable_structures::{storable::Bound, Storable};
use minicbor::{Decode, Encode};
use serde::Deserialize;
use std::borrow::Cow;

use crate::storage::with_users_iter;

// hex::encode(self.id)

#[derive(Clone, Eq, PartialEq, Debug, Decode, Encode)]
pub struct User {
    #[n(0)]
     id: [u8; 12],
     #[cbor(n(1), with = "crate::cbor::principal")]
     principal: Principal,
}

impl User {
    pub fn new(id: [u8; 12], principal: Principal) -> Self {
        Self {
            id,
            principal,
        }
    }

    pub fn get_id(&self) -> [u8; 12] {
        self.id
    }

    pub fn get_principal(&self) -> Principal {
        self.principal
    }
}


impl Storable for User {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        let mut buf = vec![];
        minicbor::encode(self, &mut buf).expect("event encoding should always succeed");
        Cow::Owned(buf)
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        minicbor::decode(bytes.as_ref())
            .unwrap_or_else(|e| panic!("failed to decode event bytes {}: {e}", hex::encode(bytes)))
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: 45,
        is_fixed_size: false,
    };
}

pub fn does_user_already_exist(user: &User) -> bool {
    with_users_iter(|mut user_iter| 
        user_iter.any(|u| 
            u.get_id() == user.get_id() || 
                u.get_principal() == user.get_principal()
        )
    )
}

pub enum GetUserBy {
    Principal(Principal),
    Id([u8; 12])
}

pub fn get_user_by(get_by: GetUserBy) -> Option<User> {
    match get_by {
        GetUserBy::Principal(principal) => with_users_iter(|mut user_iter| 
            user_iter.find(|user| user.get_principal() == principal)
        ),
        GetUserBy::Id(id) => with_users_iter(|mut user_iter| 
            user_iter.find(|user| user.get_id() == id)
        ),
    }
}

pub trait OptionUser<User> {
    fn get_user_id(&self) -> Option<String>;
}

impl OptionUser<User> for Option<User> {
    fn get_user_id(&self) -> Option<String> {
        match self {
            Some(user) => Some(hex::encode(user.get_id())),
            None => None,
        }
    }
}


#[derive(Clone, PartialEq, Debug, CandidType, Deserialize)]
pub enum UserError {
    CallerNotFound(Principal),
    RecipientNotFound(String),
    UserAlreadyExists,
    NotAdmin,
    UserIsAdmin
}

#[derive(CandidType, Deserialize)]
pub struct CreateNewUser {
    pub principal: Principal,
    pub user_id:[u8; 12]
}

#[derive(CandidType, Deserialize)]
pub struct UserStats {
    pub deposit_count: Nat,
    pub started_withdrawals: Nat,
    pub transfers_from: Nat,
    pub transfers_in: Nat,
    pub ended_withdrawals: Nat,
    pub user_balance: Nat,
}