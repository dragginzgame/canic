use crate::impl_storable_bounded;
use candid::CandidType;
use derive_more::Display;
use serde::{Deserialize, Serialize};
use std::{borrow::Borrow, borrow::Cow, str::FromStr};

/// A human-readable identifier for a canister role/type (e.g., "root", "example").
///
/// Stored as `Cow<'static, str>` so known constants can be zero‑copy while
/// dynamic values allocate only when needed.
#[derive(
    CandidType, Clone, Debug, Eq, Ord, Display, PartialOrd, Deserialize, Serialize, PartialEq, Hash,
)]
#[serde(transparent)]
pub struct CanisterType(pub Cow<'static, str>);

impl CanisterType {
    pub const ROOT: Self = Self(Cow::Borrowed("root"));

    #[must_use]
    pub const fn new(s: &'static str) -> Self {
        Self(Cow::Borrowed(s))
    }

    #[must_use]
    pub fn owned(s: String) -> Self {
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

impl FromStr for CanisterType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::owned(s.to_string()))
    }
}

impl From<&'static str> for CanisterType {
    fn from(s: &'static str) -> Self {
        Self(Cow::Borrowed(s))
    }
}

impl From<String> for CanisterType {
    fn from(s: String) -> Self {
        Self(Cow::Owned(s))
    }
}

impl From<CanisterType> for String {
    fn from(ct: CanisterType) -> Self {
        ct.into_string()
    }
}

impl AsRef<str> for CanisterType {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for CanisterType {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl_storable_bounded!(CanisterType, 48, false);

#[cfg(test)]
mod tests {
    use super::CanisterType;
    #[test]
    fn basic_traits_and_utils() {
        let a = CanisterType::new("root");
        assert!(a.is_root());
        assert_eq!(a.as_str(), "root");
        let b: CanisterType = "example".into();
        assert_eq!(b.as_str(), "example");
        let s: String = b.clone().into();
        assert_eq!(s, "example");
        assert_eq!(b.as_ref(), "example");
    }
}
