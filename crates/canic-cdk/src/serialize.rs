//!
//! serde_cbor-powered serialization helpers ensuring deterministic codecs across
//! canisters. Provides a thin wrapper with shared error handling for CBOR
//! round-trips in stable structures.
//!

use serde::{Serialize, de::DeserializeOwned};
use serde_cbor::{from_slice, to_vec};
use thiserror::Error as ThisError;

///
/// SerializeError
///
/// Error variants wrapping CBOR serialization or deserialization failures.
#[derive(Debug, ThisError)]
pub enum SerializeError {
    /// CBOR serialization failed.
    #[error("serialize error: {0}")]
    Serialize(String),

    /// CBOR deserialization failed.
    #[error("deserialize error: {0}")]
    Deserialize(String),
}

///
/// Serialize a value into CBOR bytes using serde_cbor.
///
pub fn serialize<T>(value: &T) -> Result<Vec<u8>, SerializeError>
where
    T: Serialize,
{
    to_vec(value).map_err(|err| SerializeError::Serialize(err.to_string()))
}

///
/// Deserialize CBOR bytes into a value using serde_cbor.
///
pub fn deserialize<T>(bytes: &[u8]) -> Result<T, SerializeError>
where
    T: DeserializeOwned,
{
    from_slice(bytes).map_err(|err| SerializeError::Deserialize(err.to_string()))
}
