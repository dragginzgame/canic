//! Module: fleet_ensure::model::serialization
//!
//! Responsibility: require explicit nullable fields in current durable Fleet records.
//! Does not own: schema alternatives, defaults or predecessor decoding.
//! Boundary: null is a current value; omitted authority is never inferred.

use serde::{Deserialize, Deserializer};

pub(super) fn required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}
