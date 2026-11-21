//! Stable child identities and the permanent-service and placement custody contracts.

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

/// Stable numeric identity scoped to one management owner, never a canister principal.
/// Admission order and configuration sorting cannot change it. The current protocol
/// preserves the original two service identities and assigns separate slots for mixed-fleet growth.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ChildId(u8);

impl ChildId {
    /// Original allocation, including journals migrated from one-child executables.
    pub const FIRST: Self = Self(0);
    /// Additional allocation with independent authority and evidence.
    pub const SECOND: Self = Self(1);
    /// Slots addressable by ordinary management requests, in custody order.
    pub const ALL: [Self; crate::stored::MAX_SUBNET_CHILDREN as usize] = [
        Self::FIRST,
        Self::SECOND,
        Self(16),
        Self(17),
        Self(18),
        Self(19),
        Self(20),
        Self(21),
    ];

    /// Position in permanent-service custody. Placement-only identities have no position.
    #[must_use]
    pub fn service_index(self) -> Option<usize> {
        Self::ALL.iter().position(|&slot| slot == self)
    }

    /// Original dedicated children retained by the four-child Root format.
    pub const PROJECT: [Self; 2] = [Self(2), Self(3)];

    /// Shared-shard identities in HRW candidate order. A pool grant admits a prefix.
    pub const SHARDS: [Self; 8] = [
        Self(22),
        Self(23),
        Self(4),
        Self(5),
        Self(6),
        Self(7),
        Self(8),
        Self(9),
    ];
    /// Dedicated instance identities in claim order. A pool grant admits a prefix.
    pub const INSTANCES: [Self; 8] = [
        Self(2),
        Self(3),
        Self(10),
        Self(11),
        Self(12),
        Self(13),
        Self(14),
        Self(15),
    ];
    /// Number of distinct physical identities in the Root journal.
    pub const SLOT_COUNT: usize = 24;

    /// Reopen a placement identity held in the Root journals.
    #[must_use]
    pub const fn placement(index: usize) -> Option<Self> {
        if index < Self::SLOT_COUNT {
            Some(Self(index as u8))
        } else {
            None
        }
    }

    /// Construct an ordinary management request identity; unsupported values refuse.
    #[must_use]
    pub fn new(value: usize) -> Option<Self> {
        Self::ALL.into_iter().find(|slot| slot.index() == value)
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
    Placement(u8),
}

impl Serialize for ChildId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let record = match self.0 {
            0 => RecordId::First,
            1 => RecordId::Second,
            2 => RecordId::FirstProject,
            3 => RecordId::SecondProject,
            id => RecordId::Placement(id),
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
            RecordId::Placement(id) => Self::placement(usize::from(id))
                .filter(|_| id >= 4)
                .ok_or_else(|| serde::de::Error::custom("unsupported placement identity"))?,
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

/// Address an existing allocation for exact reconciliation, or a qualification fault.
/// The receiving reconciliation handler checks the original approval and source.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AllocationRequest<T> {
    /// Root-local service or placement identity.
    pub slot: ChildId,
    /// Original operation authority or test fault.
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
/// Read every permanent-service journal, including empty slots.
pub const CUSTODY: u64 = 37;
/// Test-build-only interruption of one selected child.
pub const INJECT: u64 = 38;
/// Complete owner custody in stable slot order; includes both empty and occupied slots.
pub type Custody = [super::custody::Custody; ChildId::ALL.len()];
/// Bounded observation of all permanent service histories.
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: ChildId::ALL.len() * super::custody::LIMITS.bytes,
    allocation: ChildId::ALL.len() * super::custody::LIMITS.allocation,
    nodes: ChildId::ALL.len() * super::custody::LIMITS.nodes,
    depth: super::custody::LIMITS.depth + 1,
};

/// Combined creation/current-upgrade Wasm bytes across permanent slots. The original
/// two-bucket artifact lease leaves 64 bytes for storage and payload framing.
pub const MAX_ARTIFACT_BYTES: usize = 1_048_512;
