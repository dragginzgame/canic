//! Module: ids::subnet
//!
//! Responsibility: subnet role identifiers shared across Canic layers.
//! Does not own: placement policy, subnet registry state, or authorization.
//! Boundary: provides stable, bounded subnet role names for storage and DTOs.

use crate::{cdk::candid::CandidType, impl_storable_bounded};
use serde::{Deserialize, Serialize};
use std::{
    borrow::{Borrow, Cow},
    fmt,
    str::FromStr,
};

///
/// SubnetRole
///
/// A human-readable identifier for a subnet type.
///
/// Stored as `Cow<'static, str>` so known constants can be zero-copy while
/// dynamic values allocate only when needed.
/// Owned by ids and shared across config, storage, DTOs, and workflows.
///

const PRIME_ROLE: &str = "prime";

#[derive(
    CandidType, Clone, Debug, Eq, Ord, PartialOrd, Deserialize, Serialize, PartialEq, Hash,
)]
#[serde(transparent)]
pub struct SubnetRole(pub Cow<'static, str>);

impl SubnetRole {
    pub const PRIME: Self = Self(Cow::Borrowed(PRIME_ROLE));

    /// Create a borrowed static subnet role.
    #[must_use]
    pub const fn new(s: &'static str) -> Self {
        Self(Cow::Borrowed(s))
    }

    /// Create an owned subnet role.
    #[must_use]
    pub const fn owned(s: String) -> Self {
        Self(Cow::Owned(s))
    }

    /// Return the subnet role as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Return whether this role is the built-in prime subnet role.
    #[must_use]
    pub fn is_prime(&self) -> bool {
        self.0.as_ref() == PRIME_ROLE
    }

    /// Convert into an owned string (avoids an extra allocation for owned variants).
    #[must_use]
    pub fn into_string(self) -> String {
        self.0.into_owned()
    }
}

impl FromStr for SubnetRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::owned(s.to_string()))
    }
}

impl From<&'static str> for SubnetRole {
    fn from(s: &'static str) -> Self {
        Self(Cow::Borrowed(s))
    }
}

impl From<&String> for SubnetRole {
    fn from(s: &String) -> Self {
        Self(Cow::Owned(s.clone()))
    }
}

impl From<String> for SubnetRole {
    fn from(s: String) -> Self {
        Self(Cow::Owned(s))
    }
}

impl From<SubnetRole> for String {
    fn from(ct: SubnetRole) -> Self {
        ct.into_string()
    }
}

impl AsRef<str> for SubnetRole {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for SubnetRole {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for SubnetRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl_storable_bounded!(SubnetRole, 64, false);

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_traits_and_utils() {
        let a = SubnetRole::PRIME;
        assert!(a.is_prime());
        assert_eq!(a.as_str(), "prime");
        let b: SubnetRole = "example".into();
        assert_eq!(b.as_str(), "example");
        let s: String = b.clone().into();
        assert_eq!(s, "example");
        assert_eq!(b.as_ref(), "example");
    }
}
