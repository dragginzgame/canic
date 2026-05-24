use serde::{Serialize, de::DeserializeOwned};
use serde_cbor::{from_slice, to_vec};
use thiserror::Error as ThisError;

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

pub fn serialize<T>(value: &T) -> Result<Vec<u8>, SerializeError>
where
    T: Serialize,
{
    to_vec(value).map_err(|err| SerializeError::Serialize(err.to_string()))
}

pub fn deserialize<T>(bytes: &[u8]) -> Result<T, SerializeError>
where
    T: DeserializeOwned,
{
    from_slice(bytes).map_err(|err| SerializeError::Deserialize(err.to_string()))
}
