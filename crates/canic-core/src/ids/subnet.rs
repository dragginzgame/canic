//! Module: ids::subnet
//!
//! Responsibility: App-declared Subnet Slot identifiers shared across Canic layers.
//! Does not own: placement policy, subnet registry state, or authorization.
//! Boundary: provides stable, bounded Subnet Slot names for config and generated state.

use crate::{cdk::candid::CandidType, impl_storable_bounded};
use serde::{Deserialize, Serialize};
use std::{
    borrow::{Borrow, Cow},
    fmt,
    str::FromStr,
};

///
/// SubnetSlotId
///
/// A human-readable identifier for an App-declared logical Subnet Slot.
///
/// Stored as `Cow<'static, str>` so known constants can be zero-copy while
/// dynamic values allocate only when needed.
/// Owned by ids and shared across config, storage, DTOs, and workflows.
///

const DEFAULT_SLOT: &str = "default";

#[derive(
    CandidType, Clone, Debug, Eq, Ord, PartialOrd, Deserialize, Serialize, PartialEq, Hash,
)]
#[serde(transparent)]
pub struct SubnetSlotId(pub Cow<'static, str>);

impl SubnetSlotId {
    pub const DEFAULT: Self = Self(Cow::Borrowed(DEFAULT_SLOT));

    /// Create a borrowed static Subnet Slot identifier.
    #[must_use]
    pub const fn new(s: &'static str) -> Self {
        Self(Cow::Borrowed(s))
    }

    /// Create an owned Subnet Slot identifier.
    #[must_use]
    pub const fn owned(s: String) -> Self {
        Self(Cow::Owned(s))
    }

    /// Return the Subnet Slot identifier as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Return whether this is the built-in default workload slot.
    #[must_use]
    pub fn is_default(&self) -> bool {
        self.0.as_ref() == DEFAULT_SLOT
    }

    /// Convert into an owned string (avoids an extra allocation for owned variants).
    #[must_use]
    pub fn into_string(self) -> String {
        self.0.into_owned()
    }
}

impl FromStr for SubnetSlotId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::owned(s.to_string()))
    }
}

impl From<&'static str> for SubnetSlotId {
    fn from(s: &'static str) -> Self {
        Self(Cow::Borrowed(s))
    }
}

impl From<&String> for SubnetSlotId {
    fn from(s: &String) -> Self {
        Self(Cow::Owned(s.clone()))
    }
}

impl From<String> for SubnetSlotId {
    fn from(s: String) -> Self {
        Self(Cow::Owned(s))
    }
}

impl From<SubnetSlotId> for String {
    fn from(ct: SubnetSlotId) -> Self {
        ct.into_string()
    }
}

impl AsRef<str> for SubnetSlotId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for SubnetSlotId {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for SubnetSlotId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl_storable_bounded!(SubnetSlotId, 64, false);

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_traits_and_utils() {
        let a = SubnetSlotId::DEFAULT;
        assert!(a.is_default());
        assert_eq!(a.as_str(), "default");
        let b: SubnetSlotId = "example".into();
        assert_eq!(b.as_str(), "example");
        let s: String = b.clone().into();
        assert_eq!(s, "example");
        assert_eq!(b.as_ref(), "example");
    }
}
