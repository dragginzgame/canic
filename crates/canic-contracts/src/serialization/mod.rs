//! Module: serialization
//!
//! Responsibility: select the current owned stable-value identity and Wire budgets.
//! Does not own: codecs, individual record bounds or commitment domains.
//! Boundary: storage uses bounded native Wire; external protocols and fixed evidence are separate.

#[cfg(test)]
mod tests;

use ic_wire::{Error, Limits, native};
use serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned};
use thiserror::Error as ThisError;

// The owning memory slot selects the record schema. This marker identifies the
// storage representation independently of RPC frames and external evidence.
const STORAGE_HEADER: [u8; 4] = *b"CSW\x01";

/// Maximum traversal allowances for one owned stable value, including its marker.
/// Narrower record, map and checkpoint bounds remain enforced by their owners.
pub const STORAGE_LIMITS: Limits = Limits {
    bytes: 64 * 1024 * 1024,
    allocation: 1024 * 1024 * 1024,
    nodes: 64 * 1024 * 1024,
    depth: 128,
};

/// Typed Wire encoding or admission failure at the owned storage boundary.
#[derive(Debug, ThisError)]
pub enum SerializeError {
    #[error("serialize error: {0}")]
    Serialize(#[source] Error),

    #[error("deserialize error: {0}")]
    Deserialize(#[source] Error),
}

/// Encode one current stable value, including native full-range u128 fields.
pub fn serialize<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, SerializeError> {
    native::encode(&(STORAGE_HEADER, value), STORAGE_LIMITS).map_err(SerializeError::Serialize)
}

/// Admit the storage identity before decoding exactly one complete Wire value.
pub fn deserialize<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, SerializeError> {
    if !bytes.starts_with(&STORAGE_HEADER) {
        return Err(SerializeError::Deserialize(Error::Identity));
    }
    native::decode::<([u8; 4], T)>(bytes, STORAGE_LIMITS)
        .map(|(_, value)| value)
        .map_err(SerializeError::Deserialize)
}

/// Require a present current-contract field whose explicit value may be absent.
pub fn required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}
