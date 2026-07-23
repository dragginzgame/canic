//! Module: cdk::types::string
//!
//! Responsibility: bounded string wrappers for runtime keys and stable structures.
//! Does not own: field-specific validation or DTO compatibility policy.
//! Boundary: enforces byte-size caps at construction and stable decoding.

use crate::cdk::structures::{Storable, storable::Bound};
use candid::CandidType;
use serde::{Deserialize, Serialize, de::Deserializer};
use std::{
    borrow::Cow,
    convert::TryFrom,
    fmt::{self, Display},
    ops::Deref,
};
use thiserror::Error as ThisError;

///
/// BoundedString
///
/// String wrapper enforcing a compile-time maximum byte length across
/// construction, Serde/Candid decoding, and stable storage.
///

#[derive(CandidType, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct BoundedString<const N: u32>(String);

impl<const N: u32> BoundedString<N> {
    /// Build a bounded string when the input fits within the byte limit.
    pub fn try_new(s: impl Into<String>) -> Result<Self, BoundedStringError> {
        let s: String = s.into();
        let actual_bytes = s.len();

        if u64::try_from(actual_bytes).is_ok_and(|length| length <= u64::from(N)) {
            Ok(Self(s))
        } else {
            Err(BoundedStringError::TooLong {
                max_bytes: N,
                actual_bytes,
            })
        }
    }

    /// Build a bounded string and panic when the input exceeds the byte limit.
    ///
    /// # Panics
    ///
    /// Panics when the input string is longer than `N` bytes.
    #[must_use]
    pub fn new(s: impl Into<String>) -> Self {
        Self::try_new(s).unwrap_or_else(|err| panic!("{err}"))
    }

    /// Borrow the bounded value as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the bounded wrapper and return its validated string.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

///
/// BoundedStringError
///
/// Typed construction failure for a string that exceeds its byte bound.
/// Owned by the bounded value and preserved by callers that validate input.
///

#[derive(Clone, Debug, Eq, PartialEq, ThisError)]
pub enum BoundedStringError {
    #[error("bounded string is {actual_bytes} bytes; maximum is {max_bytes}")]
    TooLong { max_bytes: u32, actual_bytes: usize },
}

impl<'de, const N: u32> Deserialize<'de> for BoundedString<N> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::try_new(value).map_err(serde::de::Error::custom)
    }
}

impl<const N: u32> AsRef<str> for BoundedString<N> {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<const N: u32> Deref for BoundedString<N> {
    type Target = str;

    // Expose immutable string behavior without permitting bound-breaking mutation.
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const N: u32> Display for BoundedString<N> {
    // Render the bounded wrapper exactly like its inner string.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

pub type BoundedString64 = BoundedString<64>;
pub type BoundedString128 = BoundedString<128>;

// Convert the bounded wrapper back into its owned string.
impl<const N: u32> From<BoundedString<N>> for String {
    fn from(b: BoundedString<N>) -> Self {
        b.0
    }
}

impl<const N: u32> TryFrom<String> for BoundedString<N> {
    type Error = BoundedStringError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl<const N: u32> TryFrom<&str> for BoundedString<N> {
    type Error = BoundedStringError;

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

    /// Decode a bounded UTF-8 string from stable memory.
    ///
    /// # Panics
    ///
    /// Panics when stable bytes exceed the declared bound or are not valid UTF-8.
    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let bytes = bytes.into_owned();
        assert!(
            u64::try_from(bytes.len()).is_ok_and(|length| length <= u64::from(N)),
            "stable BoundedString<{N}> is {} bytes; maximum is {N}",
            bytes.len()
        );
        let value = String::from_utf8(bytes)
            .unwrap_or_else(|err| panic!("stable BoundedString<{N}> is not UTF-8: {err}"));

        Self(value)
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_within_bounds() {
        let s = "hello".to_string();
        let b = BoundedString::<16>::new(s.clone());
        assert_eq!(b.as_str(), s);
    }

    #[test]
    fn create_at_exact_limit() {
        let s = "a".repeat(16);
        let b = BoundedString::<16>::new(s.clone());
        assert_eq!(b.as_str(), s);
    }

    #[test]
    fn ordering_and_equality() {
        let a = BoundedString::<16>::new("abc".to_string());
        let b = BoundedString::<16>::new("abc".to_string());
        let c = BoundedString::<16>::new("def".to_string());

        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a < c);
    }

    #[test]
    fn try_new_preserves_typed_length_cause() {
        let err = BoundedString::<16>::try_new("a".repeat(17)).unwrap_err();
        assert_eq!(
            err,
            BoundedStringError::TooLong {
                max_bytes: 16,
                actual_bytes: 17,
            }
        );
    }

    #[test]
    fn serde_decode_enforces_the_bound() {
        let bounded = BoundedString::<16>::new("bounded");
        let bounded_encoded = crate::cdk::serialize::serialize(&bounded)
            .expect("bounded string serializes through its existing shape");
        assert_eq!(
            crate::cdk::serialize::deserialize::<BoundedString<16>>(&bounded_encoded)
                .expect("valid bounded string deserializes"),
            bounded
        );

        let encoded =
            crate::cdk::serialize::serialize(&"a".repeat(17)).expect("string fixture serializes");

        assert!(
            crate::cdk::serialize::deserialize::<BoundedString<16>>(&encoded).is_err(),
            "derived boundary decoding must not bypass construction validation"
        );
    }

    #[test]
    fn candid_decode_enforces_the_bound() {
        let bounded = BoundedString::<16>::new("bounded");
        let bounded_encoded = candid::encode_one(&bounded).expect("bounded string Candid-encodes");
        assert_eq!(
            candid::decode_one::<BoundedString<16>>(&bounded_encoded)
                .expect("valid bounded string Candid-decodes"),
            bounded
        );

        let oversized = candid::encode_one("a".repeat(17)).expect("string Candid-encodes");
        assert!(
            candid::decode_one::<BoundedString<16>>(&oversized).is_err(),
            "Candid decoding must not bypass construction validation"
        );
    }

    #[test]
    #[should_panic(expected = "stable BoundedString<16> is 17 bytes; maximum is 16")]
    fn stable_decode_rejects_overlong_bytes() {
        let _ = BoundedString::<16>::from_bytes(Cow::Owned(vec![b'a'; 17]));
    }

    #[test]
    #[should_panic(expected = "stable BoundedString<16> is not UTF-8")]
    fn stable_decode_rejects_invalid_utf8() {
        let _ = BoundedString::<16>::from_bytes(Cow::Owned(vec![0xff]));
    }
}
