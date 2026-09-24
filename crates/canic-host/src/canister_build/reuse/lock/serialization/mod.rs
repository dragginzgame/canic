//! Module: canister_build::reuse::lock::serialization
//!
//! Responsibility: require explicit nullable fields in current owner metadata.
//! Does not own: lock authority or recovery.
//! Boundary: omitted fields reject; explicit null remains an unavailable observation.

use serde::{Deserialize, Deserializer};

pub(super) fn required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}
