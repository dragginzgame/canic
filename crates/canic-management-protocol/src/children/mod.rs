//! Stable child identities and the permanent-service and placement custody contracts.

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

/// Public query for the latest or an exact upgrade in one explicit slot.
pub const UPGRADE_STATUS: u64 = ic_wire::method_id("child_upgrade_status");

/// Controller update to read one creation or exact upgrade journal.
pub const OBSERVE_STATUS: u64 = ic_wire::method_id("canic.root.observe_child_status.v1");

/// Select the retained record and its original child scope.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ObserveStatus {
    /// Creation evidence for a service or placement allocation.
    Creation(ChildId),
    /// Upgrade evidence for a permanent service and its exact approval.
    Upgrade(Request<crate::upgrade::Approval>),
}

/// The journal selected by the controller.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ObservedStatus {
    /// Creation progress and retained effect evidence.
    Creation(crate::Status),
    /// Upgrade progress and retained effect evidence.
    Upgrade(crate::upgrade::Status),
}

/// Stable numeric identity scoped to one management owner, never a canister principal.
/// Admission order and configuration sorting cannot change it. The current protocol
/// preserves the original two service identities and assigns separate slots for mixed-fleet growth.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ChildId(u32);

impl ChildId {
    /// Original allocation, including journals migrated from one-child executables.
    pub const FIRST: Self = Self(0);
    /// Additional allocation with independent authority and evidence.
    pub const SECOND: Self = Self(1);
    /// Historical service slots, in custody order. Fresh Roots extend this prefix.
    pub const HISTORICAL_SERVICES: [Self; 8] = [
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
        if self.index() >= Self::SLOT_COUNT {
            Some(self.index() - Self::SLOT_COUNT + Self::HISTORICAL_SERVICES.len())
        } else {
            Self::HISTORICAL_SERVICES
                .iter()
                .position(|&slot| slot == self)
        }
    }

    /// Map a permanent service ordinal into its own namespace, beyond the frozen placement IDs.
    #[must_use]
    pub fn service(index: usize) -> Option<Self> {
        if let Some(id) = Self::HISTORICAL_SERVICES.get(index) {
            return Some(*id);
        }
        let raw = index.checked_add(Self::SLOT_COUNT - Self::HISTORICAL_SERVICES.len())?;
        Some(Self(u32::try_from(raw).ok()?))
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
    /// Size of the original mixed service and placement namespace.
    pub const SLOT_COUNT: usize = 24;

    /// Reopen a placement identity held in the Root journals.
    #[must_use]
    pub const fn placement(index: usize) -> Option<Self> {
        if index < Self::SLOT_COUNT {
            Some(Self(index as u32))
        } else {
            None
        }
    }

    /// Construct an ordinary management request identity; unsupported values refuse.
    #[must_use]
    pub fn new(value: usize) -> Option<Self> {
        let slot = Self(u32::try_from(value).ok()?);
        slot.service_index().map(|_| slot)
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
    Service(u32),
}

impl Serialize for ChildId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let record = match self.0 {
            0 => RecordId::First,
            1 => RecordId::Second,
            2 => RecordId::FirstProject,
            3 => RecordId::SecondProject,
            id if self.index() < Self::SLOT_COUNT => RecordId::Placement(id as u8),
            id => RecordId::Service(id),
        };
        record.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ChildId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match RecordId::deserialize(deserializer)? {
            RecordId::Service(id) if id >= Self::SLOT_COUNT as u32 => Self(id),
            RecordId::Service(_) => {
                return Err(serde::de::Error::custom("overlapping service identity"));
            }
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
pub type Custody = Vec<super::custody::Custody>;
/// Bounded observation of all permanent service histories.
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: ChildId::HISTORICAL_SERVICES.len() * super::custody::LIMITS.bytes,
    allocation: ChildId::HISTORICAL_SERVICES.len() * super::custody::LIMITS.allocation,
    nodes: ChildId::HISTORICAL_SERVICES.len() * super::custody::LIMITS.nodes,
    depth: super::custody::LIMITS.depth + 1,
};

/// Combined creation/current-upgrade Wasm bytes across permanent slots. The original
/// two-bucket artifact lease leaves 64 bytes for storage and payload framing.
pub const MAX_ARTIFACT_BYTES: usize = 1_048_512;
