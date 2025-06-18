use candid::CandidType;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::fmt::Debug;
use thiserror::Error as ThisError;

///
/// Serialize/Deserialize
/// specific about the type of serializer it uses
///

///
/// SerializeError
///

#[derive(CandidType, Debug, Deserialize, Serialize, ThisError)]
pub enum SerializeError {
    #[error("serialize error: {0}")]
    Serialize(String),

    #[error("deserialize error: {0}")]
    Deserialize(String),
}

/// Serialize using `serde_cbor` (non-canonical)
pub fn serialize<T>(value: &T) -> Result<Vec<u8>, SerializeError>
where
    T: Serialize,
{
    serde_cbor::to_vec(value).map_err(|e| SerializeError::Serialize(e.to_string()))
}

/// Deserialize using `serde_cbor`
pub fn deserialize<T>(bytes: &[u8]) -> Result<T, SerializeError>
where
    T: DeserializeOwned,
{
    serde_cbor::from_slice(bytes).map_err(|e| SerializeError::Deserialize(e.to_string()))
}
