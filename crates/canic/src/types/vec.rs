//!
//! Bounded string wrappers that integrate with stable structures and enforce
//! maximum lengths at construction time. These appear in configs and memory
//! tables where size caps matter.
//!

use crate::impl_storable_bounded;
use candid::CandidType;
use derive_more::{Deref, DerefMut, Display};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

///
/// BoundedString
///
/// String wrapper enforcing a compile-time maximum length, with serde and
/// storage trait implementations.
///

#[derive(
    CandidType,
    Clone,
    Debug,
    Deref,
    DerefMut,
    Deserialize,
    Display,
    Eq,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
)]
pub struct BoundedString<const N: u32>(pub String);

#[allow(clippy::cast_possible_truncation)]
impl<const N: u32> BoundedString<N> {
    #[must_use]
    pub fn new(s: impl Into<String>) -> Self {
        let s: String = s.into();
        let slen = s.len();

        assert!(
            slen as u32 <= N,
            "String '{s}' too long for BoundedString<{N}> ({slen} bytes)",
        );

        Self(s)
    }
}

impl<const N: u32> AsRef<str> for BoundedString<N> {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

pub type BoundedString8 = BoundedString<8>;
pub type BoundedString16 = BoundedString<16>;
pub type BoundedString32 = BoundedString<32>;
pub type BoundedString64 = BoundedString<64>;
pub type BoundedString128 = BoundedString<128>;
pub type BoundedString256 = BoundedString<256>;

impl_storable_bounded!(BoundedString8, 8, false);
impl_storable_bounded!(BoundedString16, 16, false);
impl_storable_bounded!(BoundedString32, 32, false);
impl_storable_bounded!(BoundedString64, 64, false);
impl_storable_bounded!(BoundedString128, 128, false);
impl_storable_bounded!(BoundedString256, 256, false);

// Fallible Into<String> back
impl<const N: u32> From<BoundedString<N>> for String {
    fn from(b: BoundedString<N>) -> Self {
        b.0
    }
}

impl<const N: u32> TryFrom<String> for BoundedString<N> {
    type Error = String;

    #[allow(clippy::cast_possible_truncation)]
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() as u32 <= N {
            Ok(Self(value))
        } else {
            Err(format!("String too long for BoundedString<{N}>"))
        }
    }
}

impl<const N: u32> TryFrom<&str> for BoundedString<N> {
    type Error = String;

    #[allow(clippy::cast_possible_truncation)]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.len() as u32 <= N {
            Ok(Self(value.to_string()))
        } else {
            Err(format!("String too long for BoundedString<{N}>"))
        }
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::structures::Storable,
        core::{deserialize, serialize},
    };

    #[test]
    fn create_within_bounds() {
        let s = "hello".to_string();
        let b = BoundedString16::new(s.clone());
        assert_eq!(b.0, s);
    }

    #[test]
    fn create_at_exact_limit() {
        let s = "a".repeat(16);
        let b = BoundedString16::new(s.clone());
        assert_eq!(b.0, s);
    }

    #[test]
    fn ordering_and_equality() {
        let a = BoundedString16::new("abc".to_string());
        let b = BoundedString16::new("abc".to_string());
        let c = BoundedString16::new("def".to_string());

        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a < c); // "abc" < "def"
    }

    #[test]
    fn serialize_and_deserialize_roundtrip() {
        let original = BoundedString32::new("roundtrip test".to_string());

        // to bytes
        let bytes = serialize(&original).unwrap();

        // back
        let decoded: BoundedString32 = deserialize(&bytes).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn storable_impl_to_bytes_and_from_bytes() {
        let original = BoundedString64::new("hello world".to_string());

        let bytes = serialize(&original).unwrap();
        let cow = std::borrow::Cow::Owned(bytes);

        // Storable trait methods
        let stored = <BoundedString64 as Storable>::from_bytes(cow);

        assert_eq!(original, stored);

        let owned = original.clone().into_bytes();
        assert_eq!(deserialize::<BoundedString64>(&owned).unwrap(), original);
    }
}
