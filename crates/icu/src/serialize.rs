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

// serialize
pub fn serialize<T>(value: &T) -> Result<Vec<u8>, SerializeError>
where
    T: Serialize,
{
    postcard::to_stdvec(value).map_err(|e| SerializeError::Serialize(e.to_string()))
}

// deserialize
pub fn deserialize<T>(bytes: &[u8]) -> Result<T, SerializeError>
where
    T: DeserializeOwned,
{
    postcard::from_bytes(bytes).map_err(|e| SerializeError::Deserialize(e.to_string()))

    /*
    from_reader(bytes).map_err(|e| {
        // attempt to deserialize into a more generic Value for debugging
        match from_reader::<Value, _>(bytes) {
            Ok(value) => {
                SerializeError::Deserialize(format!("failed to deserialize: {e} ({value:?})"))
            }
            Err(debug_error) => SerializeError::Deserialize(format!(
                "failed to deserialize: {e}. DEBUG FAILED {debug_error}"
            )),
        }
    })*/
}
