use minicbor_serde::{from_slice, to_vec};
use serde::{Serialize, de::DeserializeOwned};
use std::fmt::Debug;
use thiserror::Error as ThisError;

///
/// Serialize/Deserialize
/// for consistent use of minicbor (super efficient, no_std)
///

///
/// SerializeError
///

#[derive(Debug, ThisError)]
pub enum SerializeError {
    #[error("serialize error: {0}")]
    Serialize(String),

    #[error("deserialize error: {0}")]
    Deserialize(String),
}

// serialize
pub fn serialize<T>(t: &T) -> Result<Vec<u8>, SerializeError>
where
    T: Serialize,
{
    let bytes = to_vec(t).map_err(|e| SerializeError::Serialize(e.to_string()))?;

    Ok(bytes)
}

// deserialize
pub fn deserialize<T>(bytes: &[u8]) -> Result<T, SerializeError>
where
    T: DeserializeOwned,
{
    let t: T = from_slice(bytes).map_err(|e| SerializeError::Deserialize(e.to_string()))?;

    Ok(t)
}
