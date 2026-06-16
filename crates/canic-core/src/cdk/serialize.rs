//! Module: cdk::serialize
//!
//! Responsibility: serde-CBOR helpers for stable storage encoding.
//! Does not own: individual stable schema bounds or migration policy.
//! Boundary: maps serde encode/decode failures into typed Canic errors.

use serde::{Serialize, de::DeserializeOwned};
use serde_cbor::{from_slice, to_vec};
use thiserror::Error as ThisError;

///
/// SerializeError
///
/// Typed error returned by Canic stable serialization helpers.
///

#[derive(Debug, ThisError)]
pub enum SerializeError {
    #[error("serialize error: {0}")]
    Serialize(String),

    #[error("deserialize error: {0}")]
    Deserialize(String),
}

/// Serialize one value to CBOR bytes.
pub fn serialize<T>(value: &T) -> Result<Vec<u8>, SerializeError>
where
    T: Serialize,
{
    to_vec(value).map_err(|err| SerializeError::Serialize(err.to_string()))
}

/// Deserialize one value from CBOR bytes.
pub fn deserialize<T>(bytes: &[u8]) -> Result<T, SerializeError>
where
    T: DeserializeOwned,
{
    from_slice(bytes).map_err(|err| SerializeError::Deserialize(err.to_string()))
}
