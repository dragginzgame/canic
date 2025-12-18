//!
//! Bounded string wrappers that integrate with stable structures and enforce
//! maximum lengths at construction time. These appear in configs and memory
//! tables where size caps matter.
//!

use candid::CandidType;
use canic_cdk::structures::{Storable, storable::Bound};
use derive_more::{Deref, DerefMut, Display};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, convert::TryFrom};

///
/// BoundedString
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
    pub fn try_new(s: impl Into<String>) -> Result<Self, String> {
        let s: String = s.into();

        #[allow(clippy::cast_possible_truncation)]
        if s.len() as u32 <= N {
            Ok(Self(s))
        } else {
            Err(format!("String too long for BoundedString<{N}>"))
        }
    }

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
        Self::try_new(value)
    }
}

impl<const N: u32> TryFrom<&str> for BoundedString<N> {
    type Error = String;

    #[allow(clippy::cast_possible_truncation)]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl<const N: u32> Storable for BoundedString<N> {
    const BOUND: Bound = Bound::Bounded {
        max_size: N,
        is_fixed_size: false,
    };

    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Borrowed(self.0.as_bytes())
    }

    fn into_bytes(self) -> Vec<u8> {
        self.0.into_bytes()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let bytes = bytes.as_ref();

        assert!(
            bytes.len() <= N as usize,
            "Stored string exceeds BoundedString<{N}> bound"
        );

        let s = std::str::from_utf8(bytes)
            .expect("Stored BoundedString is not valid UTF-8")
            .to_string();

        Self(s)
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

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
    fn try_new_is_fallible() {
        let err = BoundedString16::try_new("a".repeat(17)).unwrap_err();
        assert!(err.contains("BoundedString<16>"));
    }
}
