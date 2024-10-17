use candid::{CandidType};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Display, Formatter, LowerHex, UpperHex};
use std::str::FromStr;

#[derive(
    Copy, Clone, Eq, PartialEq, Hash, Debug, Default, CandidType, Decode, Deserialize, Encode,
)]
#[cbor(index_only)]
pub enum EthereumNetwork {
    #[n(1)]
    Mainnet,
    #[n(11155111)]
    #[default]
    Sepolia,
}
impl EthereumNetwork {
    pub fn chain_id(&self) -> u64 {
        match self {
            EthereumNetwork::Mainnet => 1,
            EthereumNetwork::Sepolia => 11155111,
        }
    }
}

impl TryFrom<u64> for EthereumNetwork {
    type Error = String;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(EthereumNetwork::Mainnet),
            11155111 => Ok(EthereumNetwork::Sepolia),
            _ => Err("Unknown Ethereum Network".to_string()),
        }
    }
}

impl Display for EthereumNetwork {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EthereumNetwork::Mainnet => write!(f, "Ethereum Mainnet"),
            EthereumNetwork::Sepolia => write!(f, "Ethereum Testnet Sepolia"),
        }
    }
}

#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Decode, Deserialize, Encode, Serialize,
)]
#[serde(transparent)]
#[cbor(transparent)]
pub struct Address(
    #[serde(with = "crate::serde_data")]
    #[cbor(n(0), with = "minicbor::bytes")]
    [u8; 20],
);

impl Address {
    /// Ethereum zero address.
    ///
    /// ```
    /// let address = ic_ethereum_types::Address::ZERO;
    /// assert_eq!(address.to_string(), "0x0000000000000000000000000000000000000000");
    /// assert_eq!(address.into_bytes(), [0u8; 20]);
    /// ```
    pub const ZERO: Self = Self([0u8; 20]);

    /// Create a new Ethereum address from raw bytes.
    pub const fn new(bytes: [u8; 20]) -> Self {
        Self(bytes)
    }

    /// Convert an Ethereum address into a 20-byte array.
    pub const fn into_bytes(self) -> [u8; 20] {
        self.0
    }
}

impl LowerHex for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "0x{}", hex::encode(self.0))
    }
}

impl UpperHex for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "0x{}", hex::encode_upper(self.0))
    }
}

/// Parse an address from a 32-byte array with left zero padding.
impl TryFrom<&[u8; 32]> for Address {
    type Error = String;

    fn try_from(value: &[u8; 32]) -> Result<Self, Self::Error> {
        let (leading_zeroes, address_bytes) = value.split_at(12);
        if !leading_zeroes.iter().all(|leading_zero| *leading_zero == 0) {
            return Err(format!(
                "address has leading non-zero bytes: {:?}",
                leading_zeroes
            ));
        }
        Ok(Address::new(
            <[u8; 20]>::try_from(address_bytes).expect("vector has correct length"),
        ))
    }
}

/// Convert a 20-byte address to 32-byte array, with left zero padding.
impl From<&Address> for [u8; 32] {
    fn from(address: &Address) -> Self {
        let bytes = address.as_ref();
        let pad = 32 - bytes.len();
        let mut padded: [u8; 32] = [0; 32];
        padded[pad..32].copy_from_slice(bytes);
        padded
    }
}

impl FromStr for Address {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with("0x") {
            return Err("address doesn't start with '0x'".to_string());
        }
        let mut bytes = [0u8; 20];
        hex::decode_to_slice(&s[2..], &mut bytes)
            .map_err(|e| format!("address is not hex: {}", e))?;
        Ok(Self(bytes))
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

/// Display address using [EIP-55](https://eips.ethereum.org/EIPS/eip-55).
impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut addr_chars = [0u8; 20 * 2];
        hex::encode_to_slice(self.0, &mut addr_chars)
            .expect("bug: failed to encode an address as hex");

        let checksum = keccak(&addr_chars[..]);
        let mut cs_nibbles = [0u8; 32 * 2];
        for i in 0..32 {
            cs_nibbles[2 * i] = checksum[i] >> 4;
            cs_nibbles[2 * i + 1] = checksum[i] & 0x0f;
        }
        write!(f, "0x")?;
        for (a, cs) in addr_chars.iter().zip(cs_nibbles.iter()) {
            let ascii_byte = if *cs >= 0x08 {
                a.to_ascii_uppercase()
            } else {
                *a
            };
            write!(f, "{}", char::from(ascii_byte))?;
        }
        Ok(())
    }
}

fn keccak(bytes: &[u8]) -> [u8; 32] {
    ic_sha3::Keccak256::hash(bytes)
}

pub enum WeiTag {}
pub type Wei = CheckedAmountOf<WeiTag>;

