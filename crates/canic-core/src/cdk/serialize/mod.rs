//! Runtime use of the canonical shared byte codecs.

pub(crate) use canic_contracts::serialization::required_option;
pub use canic_contracts::serialization::{SerializeError, deserialize, serialize};

pub(crate) mod canonical;

/// Preserve Wire budget refusal at the fallible Storage codec boundary.
#[must_use]
pub const fn storable_error(error: SerializeError) -> ic_storage::storable::CodecError {
    let (SerializeError::Serialize(source) | SerializeError::Deserialize(source)) = error;
    match source {
        ic_wire::Error::Limit => ic_storage::storable::CodecError::Bounds,
        ic_wire::Error::Allocation
        | ic_wire::Error::Malformed
        | ic_wire::Error::Trailing
        | ic_wire::Error::Identity => {
            ic_storage::storable::CodecError::Invalid("Wire stable value")
        }
    }
}
