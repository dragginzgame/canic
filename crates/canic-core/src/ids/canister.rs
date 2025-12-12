//!
//! Strongly-typed identifiers representing canister roles within the project.
//! Provides string-backed wrappers with storage traits and helpers for config
//! parsing while avoiding repeated `Cow` boilerplate around the codebase.
//!

use crate::memory::impl_storable_bounded;
use candid::CandidType;
use derive_more::Display;
use serde::{Deserialize, Serialize};
use std::{borrow::Borrow, borrow::Cow, str::FromStr};

///
/// CanisterRole
///
/// A human-readable identifier for a canister role/type (e.g., "root", "example").
///
/// Stored as `Cow<'static, str>` so known constants can be zero‑copy while
/// dynamic values allocate only when needed.
///

#[derive(
    CandidType, Clone, Debug, Eq, Ord, Display, PartialOrd, Deserialize, Serialize, PartialEq, Hash,
)]
#[serde(transparent)]
pub struct CanisterRole(pub Cow<'static, str>);

impl CanisterRole {
    pub const ROOT: Self = Self(Cow::Borrowed("root"));

    #[must_use]
    pub const fn new(s: &'static str) -> Self {
        Self(Cow::Borrowed(s))
    }

    #[must_use]
    pub const fn owned(s: String) -> Self {
        Self(Cow::Owned(s))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns true if this type represents the built-in ROOT canister.
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.0.as_ref() == "root"
    }

    /// Convert into an owned string (avoids an extra allocation for owned variants).
    #[must_use]
    pub fn into_string(self) -> String {
        self.0.into_owned()
    }
}

impl FromStr for CanisterRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::owned(s.to_string()))
    }
}

impl From<&'static str> for CanisterRole {
    fn from(s: &'static str) -> Self {
        Self(Cow::Borrowed(s))
    }
}

impl From<&String> for CanisterRole {
    fn from(s: &String) -> Self {
        Self(Cow::Owned(s.clone()))
    }
}

impl From<String> for CanisterRole {
    fn from(s: String) -> Self {
        Self(Cow::Owned(s))
    }
}

impl From<CanisterRole> for String {
    fn from(ct: CanisterRole) -> Self {
        ct.into_string()
    }
}

impl AsRef<str> for CanisterRole {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for CanisterRole {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl_storable_bounded!(CanisterRole, 64, false);

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::CanisterRole;
    #[test]
    fn basic_traits_and_utils() {
        let a = CanisterRole::ROOT;
        assert!(a.is_root());
        assert_eq!(a.as_str(), "root");
        let b: CanisterRole = "example".into();
        assert_eq!(b.as_str(), "example");
        let s: String = b.clone().into();
        assert_eq!(s, "example");
        assert_eq!(b.as_ref(), "example");
    }
}
