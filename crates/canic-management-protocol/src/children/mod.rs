//! Two independently retained child slots under one management owner.

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

/// Stable numeric identity scoped to one management owner, never a canister principal.
/// Admission order and configuration sorting cannot change it. The current protocol
/// uses identities 0 and 1 for ordinary requests and separate identities for project grants.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ChildId(u8);

impl ChildId {
    /// Original allocation, including journals migrated from one-child executables.
    pub const FIRST: Self = Self(0);
    /// Additional allocation with independent authority and evidence.
    pub const SECOND: Self = Self(1);
    /// Qualified child identities in permanent custody order.
    /// This is the current protocol bound, not an operator capacity setting.
    pub const ALL: [Self; 2] = [Self::FIRST, Self::SECOND];

    /// Dedicated child identities reserved for controller-approved project pools.
    pub const PROJECT: [Self; 2] = [Self(2), Self(3)];

    /// Construct an ordinary management request identity; unsupported values refuse.
    #[must_use]
    pub const fn new(value: usize) -> Option<Self> {
        if value < Self::ALL.len() {
            Some(Self(value as u8))
        } else {
            None
        }
    }

    /// Stable numeric identity, also its position in historical custody arrays.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

// These names and discriminants belong to retained Wire requests, approval
// digests and Store pins. Runtime identities must not rewrite their encoding.
#[derive(Deserialize, Serialize)]
enum RecordId {
    First,
    Second,
    FirstProject,
    SecondProject,
}

impl Serialize for ChildId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let record = match self.0 {
            0 => RecordId::First,
            1 => RecordId::Second,
            2 => RecordId::FirstProject,
            3 => RecordId::SecondProject,
            _ => return Err(serde::ser::Error::custom("unsupported child identity")),
        };
        record.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ChildId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match RecordId::deserialize(deserializer)? {
            RecordId::First => Self::FIRST,
            RecordId::Second => Self::SECOND,
            RecordId::FirstProject => Self::PROJECT[0],
            RecordId::SecondProject => Self::PROJECT[1],
        })
    }
}

/// Exact child scope is part of the authenticated request, not ambient execution state.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Request<T> {
    /// Stable target slot.
    #[serde(deserialize_with = "ordinary_child")]
    pub slot: ChildId,
    /// Existing creation or upgrade contract.
    pub value: T,
}

fn ordinary_child<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<ChildId, D::Error> {
    let slot = ChildId::deserialize(deserializer)?;
    ChildId::new(slot.index()).ok_or_else(|| serde::de::Error::custom("ordinary child required"))
}

/// Admit creation in the selected slot.
pub const STAGE: u64 = 31;
/// Read the selected creation journal.
pub const STATUS: u64 = 32;
/// Reconcile the exact selected creation approval.
pub const RECONCILE: u64 = 33;
/// Admit an upgrade in the selected slot.
pub const STAGE_UPGRADE: u64 = 34;
/// Read the selected upgrade approval, including its retained history.
pub const OBSERVE_UPGRADE: u64 = 35;
/// Reconcile the exact selected upgrade approval.
pub const RECONCILE_UPGRADE: u64 = 36;
/// Read both complete journals, including empty slots.
pub const CUSTODY: u64 = 37;
/// Test-build-only interruption of one selected child.
pub const INJECT: u64 = 38;
/// Complete owner custody in stable slot order; includes both empty and occupied slots.
pub type Custody = [super::custody::Custody; 2];
/// Bounded observation of both child histories.
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 16_384,
    allocation: 2 * super::custody::LIMITS.allocation,
    nodes: 2 * super::custody::LIMITS.nodes,
    depth: super::custody::LIMITS.depth + 1,
};

/// Combined creation/current-upgrade Wasm bytes across both slots. The original
/// two-bucket artifact lease leaves 64 bytes for storage and payload framing.
pub const MAX_ARTIFACT_BYTES: usize = 1_048_512;
