//! Module: ids::tree
//!
//! Responsibility: identify declared Tree topology and concrete Fleet-scoped Trees.
//! Does not own: Tree placement, Registry authority, lifecycle, or ID generation.
//! Boundary: declared IDs are bounded canonical names; generated IDs use lowercase hex.

use crate::impl_storable_bounded;
use candid::CandidType;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::{borrow::Borrow, fmt, str::FromStr};
use thiserror::Error as ThisError;

const TREE_NAME_MAX_BYTES: usize = 40;

///
/// TreeSpecId
///
/// App-scoped identity of one declared permitted Tree topology.
///

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct TreeSpecId(String);

impl TreeSpecId {
    #[must_use]
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for TreeSpecId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl AsRef<str> for TreeSpecId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for TreeSpecId {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl CandidType for TreeSpecId {
    fn _ty() -> candid::types::Type {
        candid::types::TypeInner::Text.into()
    }

    fn idl_serialize<S>(&self, serializer: S) -> Result<(), S::Error>
    where
        S: candid::types::Serializer,
    {
        serializer.serialize_text(self.as_str())
    }
}

impl FromStr for TreeSpecId {
    type Err = TreeSpecIdParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        validate_tree_name(value).map_err(TreeSpecIdParseError::from)?;
        Ok(Self(value.to_string()))
    }
}

impl TryFrom<String> for TreeSpecId {
    type Error = TreeSpecIdParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        validate_tree_name(&value).map_err(TreeSpecIdParseError::from)?;
        Ok(Self(value))
    }
}

impl<'de> Deserialize<'de> for TreeSpecId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::try_from(value).map_err(de::Error::custom)
    }
}

impl_storable_bounded!(TreeSpecId, 64, false);

///
/// TreeGroupId
///
/// Fleet-scoped identity of one App-declared independently scaled Tree Group.
///

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct TreeGroupId(String);

impl TreeGroupId {
    #[must_use]
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for TreeGroupId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl AsRef<str> for TreeGroupId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for TreeGroupId {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl CandidType for TreeGroupId {
    fn _ty() -> candid::types::Type {
        candid::types::TypeInner::Text.into()
    }

    fn idl_serialize<S>(&self, serializer: S) -> Result<(), S::Error>
    where
        S: candid::types::Serializer,
    {
        serializer.serialize_text(self.as_str())
    }
}

impl FromStr for TreeGroupId {
    type Err = TreeGroupIdParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        validate_tree_name(value).map_err(TreeGroupIdParseError::from)?;
        Ok(Self(value.to_string()))
    }
}

impl TryFrom<String> for TreeGroupId {
    type Error = TreeGroupIdParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        validate_tree_name(&value).map_err(TreeGroupIdParseError::from)?;
        Ok(Self(value))
    }
}

impl<'de> Deserialize<'de> for TreeGroupId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::try_from(value).map_err(de::Error::custom)
    }
}

impl_storable_bounded!(TreeGroupId, 64, false);

///
/// TreeId
///
/// Generated durable Fleet-scoped identity of one concrete Canister Tree.
///

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TreeId([u8; 32]);

impl TreeId {
    /// Construct an ID from bytes produced by the host's cryptographic generator.
    ///
    /// The bytes do not become authoritative until an installation journal
    /// durably records them before Tree Root creation.
    #[must_use]
    pub const fn from_generated_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for TreeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl CandidType for TreeId {
    fn _ty() -> candid::types::Type {
        candid::types::TypeInner::Text.into()
    }

    fn idl_serialize<S>(&self, serializer: S) -> Result<(), S::Error>
    where
        S: candid::types::Serializer,
    {
        serializer.serialize_text(&self.to_string())
    }
}

impl FromStr for TreeId {
    type Err = TreeIdParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != 64 {
            return Err(TreeIdParseError::Length(value.len()));
        }
        if !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(TreeIdParseError::CanonicalHex);
        }

        let mut bytes = [0; 32];
        for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
            bytes[index] = (decode_nibble(pair[0]) << 4) | decode_nibble(pair[1]);
        }
        Ok(Self(bytes))
    }
}

impl Serialize for TreeId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for TreeId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(de::Error::custom)
    }
}

///
/// TreeSpecIdParseError
///
/// Typed rejection for an invalid Tree Spec identifier.
///

#[derive(Clone, Debug, Eq, PartialEq, ThisError)]
pub enum TreeSpecIdParseError {
    #[error("Tree Spec ID must not be empty")]
    Empty,

    #[error("Tree Spec ID must not exceed {max_bytes} bytes, got {actual_bytes}")]
    TooLong {
        max_bytes: usize,
        actual_bytes: usize,
    },

    #[error("Tree Spec ID must use only ASCII letters, numbers, '-' or '_'")]
    InvalidCharacters,
}

///
/// TreeGroupIdParseError
///
/// Typed rejection for an invalid Tree Group identifier.
///

#[derive(Clone, Debug, Eq, PartialEq, ThisError)]
pub enum TreeGroupIdParseError {
    #[error("Tree Group ID must not be empty")]
    Empty,

    #[error("Tree Group ID must not exceed {max_bytes} bytes, got {actual_bytes}")]
    TooLong {
        max_bytes: usize,
        actual_bytes: usize,
    },

    #[error("Tree Group ID must use only ASCII letters, numbers, '-' or '_'")]
    InvalidCharacters,
}

///
/// TreeIdParseError
///
/// Typed rejection for a non-canonical concrete Tree identity.
///

#[derive(Clone, Debug, Eq, PartialEq, ThisError)]
pub enum TreeIdParseError {
    #[error("Tree ID must contain exactly 64 characters, got {0}")]
    Length(usize),

    #[error("Tree ID must contain only lowercase hexadecimal characters")]
    CanonicalHex,
}

#[derive(Clone, Copy)]
enum TreeNameIssue {
    Empty,
    TooLong {
        max_bytes: usize,
        actual_bytes: usize,
    },
    InvalidCharacters,
}

impl From<TreeNameIssue> for TreeSpecIdParseError {
    fn from(issue: TreeNameIssue) -> Self {
        match issue {
            TreeNameIssue::Empty => Self::Empty,
            TreeNameIssue::TooLong {
                max_bytes,
                actual_bytes,
            } => Self::TooLong {
                max_bytes,
                actual_bytes,
            },
            TreeNameIssue::InvalidCharacters => Self::InvalidCharacters,
        }
    }
}

impl From<TreeNameIssue> for TreeGroupIdParseError {
    fn from(issue: TreeNameIssue) -> Self {
        match issue {
            TreeNameIssue::Empty => Self::Empty,
            TreeNameIssue::TooLong {
                max_bytes,
                actual_bytes,
            } => Self::TooLong {
                max_bytes,
                actual_bytes,
            },
            TreeNameIssue::InvalidCharacters => Self::InvalidCharacters,
        }
    }
}

fn validate_tree_name(value: &str) -> Result<(), TreeNameIssue> {
    if value.is_empty() {
        return Err(TreeNameIssue::Empty);
    }
    if value.len() > TREE_NAME_MAX_BYTES {
        return Err(TreeNameIssue::TooLong {
            max_bytes: TREE_NAME_MAX_BYTES,
            actual_bytes: value.len(),
        });
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(TreeNameIssue::InvalidCharacters);
    }
    Ok(())
}

fn decode_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => unreachable!("canonical hex was validated before decoding"),
    }
}

#[cfg(test)]
mod tests {
    use crate::cdk::structures::storable::Storable;

    use super::*;

    #[test]
    fn declared_tree_ids_are_bounded_canonical_names() {
        let spec = "users".parse::<TreeSpecId>().expect("Tree Spec ID");
        let group = "users-primary"
            .parse::<TreeGroupId>()
            .expect("Tree Group ID");

        assert_eq!(spec.as_str(), "users");
        assert_eq!(group.as_str(), "users-primary");
        std::assert_matches!("".parse::<TreeSpecId>(), Err(TreeSpecIdParseError::Empty));
        std::assert_matches!(
            "bad/name".parse::<TreeGroupId>(),
            Err(TreeGroupIdParseError::InvalidCharacters)
        );
        std::assert_matches!(
            "a".repeat(TREE_NAME_MAX_BYTES + 1).parse::<TreeSpecId>(),
            Err(TreeSpecIdParseError::TooLong { .. })
        );
    }

    #[test]
    fn declared_tree_ids_validate_serde_and_candid_input() {
        let mut invalid_bytes = Vec::new();
        ciborium::ser::into_writer("bad/name", &mut invalid_bytes)
            .expect("serialize invalid Tree Spec ID");

        assert!(ciborium::de::from_reader::<TreeSpecId, _>(invalid_bytes.as_slice()).is_err());

        let candid_bytes = candid::encode_one("projects").expect("encode Tree Group ID");
        let group = candid::decode_one::<TreeGroupId>(&candid_bytes).expect("decode Tree Group ID");
        let invalid_candid = candid::encode_one("bad/name").expect("encode invalid Tree Group ID");

        assert_eq!(group.as_str(), "projects");
        assert!(candid::decode_one::<TreeGroupId>(&invalid_candid).is_err());
    }

    #[test]
    fn declared_tree_ids_fit_their_stable_bound() {
        let spec = "a"
            .repeat(TREE_NAME_MAX_BYTES)
            .parse::<TreeSpecId>()
            .expect("maximum Tree Spec ID");
        let group = "b"
            .repeat(TREE_NAME_MAX_BYTES)
            .parse::<TreeGroupId>()
            .expect("maximum Tree Group ID");
        let spec_bytes = spec.to_bytes();
        let group_bytes = group.to_bytes();

        assert!(spec_bytes.len() <= 64);
        assert!(group_bytes.len() <= 64);
        assert_eq!(TreeSpecId::from_bytes(spec_bytes), spec);
        assert_eq!(TreeGroupId::from_bytes(group_bytes), group);
    }

    #[test]
    fn tree_id_uses_exact_canonical_text() {
        let tree_id = TreeId::from_generated_bytes([0xab; 32]);
        let text = "ab".repeat(32);

        assert_eq!(tree_id.to_string(), text);
        assert_eq!(text.parse::<TreeId>(), Ok(tree_id));

        let mut serde_bytes = Vec::new();
        ciborium::ser::into_writer(&tree_id, &mut serde_bytes).expect("serialize Tree ID");
        let decoded: TreeId =
            ciborium::de::from_reader(serde_bytes.as_slice()).expect("decode Tree ID");
        assert_eq!(decoded, tree_id);

        let candid_bytes = candid::encode_one(tree_id).expect("encode Tree ID");
        let decoded: TreeId = candid::decode_one(&candid_bytes).expect("decode Tree ID");
        let invalid_candid =
            candid::encode_one("A000000000000000000000000000000000000000000000000000000000000000")
                .expect("encode invalid Tree ID");

        assert_eq!(decoded, tree_id);
        assert!(candid::decode_one::<TreeId>(&invalid_candid).is_err());
    }

    #[test]
    fn tree_id_rejects_noncanonical_text() {
        std::assert_matches!("ab".parse::<TreeId>(), Err(TreeIdParseError::Length(2)));
        std::assert_matches!(
            "A000000000000000000000000000000000000000000000000000000000000000".parse::<TreeId>(),
            Err(TreeIdParseError::CanonicalHex)
        );
    }
}
