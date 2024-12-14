use candid::{CandidType, Principal};
use ic_ethereum_types::Address;
use ic_stable_structures::{storable::Bound, Storable, Vec as StableVec};
use minicbor::{Decode, Encode};
use serde::Deserialize;
use std::{borrow::Cow, str::FromStr};

use crate::storage::with_users_iter;

// hex::encode(self.id)

#[derive(Clone, Eq, PartialEq, Debug, Decode, Encode)]
pub struct User {
    #[n(0)]
     id: [u8; 12],
     #[cbor(n(1), with = "crate::cbor::principal")]
     principal: Principal,
     #[n(2)]
     eth_address: Address
}

impl User {
    pub fn new(id: [u8; 12], principal: Principal, address_string: String) -> Self {
        let eth_address = Address::from_str(&address_string)
        .unwrap_or_else(|e| ic_cdk::trap(&format!("invalid recipient address: {:?}", e)));

        Self {
            id,
            principal,
            eth_address
        }
    }

    pub fn get_id(&self) -> [u8; 12] {
        self.id
    }

    pub fn get_principal(&self) -> Principal {
        self.principal
    }
    
    pub fn get_eth_address(&self) -> Address {
        self.eth_address
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
        max_size: 12,
        is_fixed_size: true,
    };
}

pub fn does_user_already_exist(user: &User) -> bool {
    with_users_iter(|mut user_iter| 
        user_iter.any(|u| 
            u.get_id() == user.get_id() || 
                u.get_principal() == user.get_principal() || 
                u.get_eth_address() == user.get_eth_address()
        )
    )
}

pub enum GetUserBy {
    Principal(Principal),
    Id([u8; 12]),
    EthAddress(Address)
}

pub fn get_user_by(get_by: GetUserBy) -> Option<User> {
    match get_by {
        GetUserBy::Principal(principal) => with_users_iter(|mut user_iter| 
            user_iter.find(|user| user.get_principal() == principal)
        ),
        GetUserBy::Id(id) => with_users_iter(|mut user_iter| 
            user_iter.find(|user| user.get_id() == id)
        ),
        GetUserBy::EthAddress(address) => with_users_iter(|mut user_iter| 
            user_iter.find(|user| user.get_eth_address() == address)
        ),
    }
}

#[derive(Clone, PartialEq, Debug, CandidType, Deserialize)]
pub enum UserError {
    CallerNotFound(Principal),
    RecipientNotFound(String)
}