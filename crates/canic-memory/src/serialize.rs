//!
//! serde_cbor-powered serialization helpers ensuring deterministic codecs across
//! canisters. Provides a thin wrapper with shared error handling for CBOR
//! round-trips in stable structures.
//!

use serde::{Serialize, de::DeserializeOwned};
use serde_cbor::{from_slice, to_vec};
use std::fmt::Debug;
use thiserror::Error as ThisError;

///
/// SerializeError
///
/// Error variants wrapping CBOR serialization or deserialization failures
/// so callers can bubble them up uniformly.
///

#[derive(Debug, ThisError)]
pub enum SerializeError {
    #[error("serialize error: {0}")]
    Serialize(String),

    #[error("deserialize error: {0}")]
    Deserialize(String),
}

///
/// Serialize a value into CBOR bytes using serde_cbor.
///
pub fn serialize<T>(t: &T) -> Result<Vec<u8>, SerializeError>
where
    T: Serialize,
{
    let bytes = to_vec(t).map_err(|e| SerializeError::Serialize(e.to_string()))?;

    Ok(bytes)
}

///
/// Deserialize CBOR bytes into a value using serde_cbor.
///
pub fn deserialize<T>(bytes: &[u8]) -> Result<T, SerializeError>
where
    T: DeserializeOwned,
{
    let t: T = from_slice(bytes).map_err(|e| SerializeError::Deserialize(e.to_string()))?;

    Ok(t)
}
