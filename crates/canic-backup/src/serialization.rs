//! Module: serialization
//!
//! Responsibility: enforce exact current JSON field presence at Serde boundaries.
//! Does not own: artifact validation, schema versions, or persistence policy.
//! Boundary: serialized contracts use these helpers when `null` is valid but omission is not.

use serde::{Deserialize, Deserializer};

/// Deserialize a required field whose explicit value may be `null`.
pub fn required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}
