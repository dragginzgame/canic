//! Stable child identities and the permanent-service and placement custody contracts.

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

/// Retained creation evidence for one currently assigned child.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InstalledChild {
    /// Slot in this Root's installation journals.
    pub slot: ChildId,
    /// Principal returned by the management canister.
    pub canister: String,
    /// Assigned service or placement role when supplied at installation.
    pub role: Option<String>,
    /// Immediate application parent when the install binding identifies it.
    pub parent: Option<String>,
    /// Last recorded creation phase. Current availability requires a separate observation.
    pub phase: crate::Phase,
}

/// Controller query over at most 32 journal slots per response.
#[cfg(feature = "endpoints")]
pub struct ListChildren;
#[cfg(feature = "endpoints")]
impl canic::method::Contract for ListChildren {
    const NAME: &'static str = "list_children";
    const ID: u64 = ic_wire::method_id("canic.root.list_children.v1");
    type Input = Option<ChildId>;
    type Output = (Vec<InstalledChild>, Option<ChildId>);
}

/// Public query for creation or upgrade evidence in a selected slot.
pub const READ_STATUS: u64 = ic_wire::method_id("canic.root.child_status.v1");

/// Select the public child journal. An absent upgrade approval selects its latest entry.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum StatusRequest {
    /// Current creation evidence for a service or placement allocation.
    Creation(ChildId),
    /// Latest or exact upgrade evidence for a permanent service.
    Upgrade(Request<Option<crate::upgrade::Approval>>),
}

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

/// A Root-local identity. Placement slots occupy a bounded namespace below service slots.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ChildId(u32);

impl ChildId {
    /// Number of placement slots across both allocation policies.
    pub const SLOT_COUNT: usize = 16;
    /// First permanent service.
    pub const FIRST: Self = Self(Self::SLOT_COUNT as u32);
    /// Second permanent service.
    pub const SECOND: Self = Self(Self::FIRST.0 + 1);
    /// Shared-shard identities in HRW order.
    pub const SHARDS: [Self; 8] = [
        Self(0),
        Self(1),
        Self(2),
        Self(3),
        Self(4),
        Self(5),
        Self(6),
        Self(7),
    ];
    /// Dedicated instance identities in claim order.
    pub const INSTANCES: [Self; 8] = [
        Self(8),
        Self(9),
        Self(10),
        Self(11),
        Self(12),
        Self(13),
        Self(14),
        Self(15),
    ];

    /// Permanent service ordinal.
    #[must_use]
    pub fn service_index(self) -> Option<usize> {
        self.index().checked_sub(Self::SLOT_COUNT)
    }

    /// Select a permanent service by ordinal.
    #[must_use]
    pub fn service(index: usize) -> Option<Self> {
        Some(Self(u32::try_from(index).ok()?.checked_add(Self::FIRST.0)?))
    }

    /// Select an allocation from its placement slot.
    #[must_use]
    pub fn placement(index: usize) -> Option<Self> {
        (index < Self::SLOT_COUNT).then_some(Self(index as u32))
    }

    /// Reopen either namespace from its retained raw slot.
    #[must_use]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// Raw slot in retained allocation evidence.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

// The discriminator precedes the slot so superseded positional approvals refuse.
const ID_DOMAIN: u8 = 73;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Slot {
    Service(u32),
    Placement(u32),
}
impl Serialize for ChildId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            let slot = match self.0.checked_sub(Self::FIRST.0) {
                Some(ordinal) => Slot::Service(ordinal),
                None => Slot::Placement(self.0),
            };
            slot.serialize(serializer)
        } else {
            (ID_DOMAIN, self.0).serialize(serializer)
        }
    }
}
impl<'de> Deserialize<'de> for ChildId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            return match Slot::deserialize(deserializer)? {
                Slot::Service(ordinal) => Self::service(ordinal as usize),
                Slot::Placement(index) => Self::placement(index as usize),
            }
            .ok_or_else(|| serde::de::Error::custom("invalid child slot"));
        }
        let (domain, raw) = <(u8, u32)>::deserialize(deserializer)?;
        if domain != ID_DOMAIN {
            return Err(serde::de::Error::custom("unknown child identity"));
        }
        Ok(Self(raw))
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
    slot.service_index()
        .map(|_| slot)
        .ok_or_else(|| serde::de::Error::custom("ordinary child required"))
}

/// Read the selected creation journal.
pub const STATUS: u64 = 32;
/// Read every permanent-service journal, including empty slots.
pub const CUSTODY: u64 = 37;
/// Test-build-only interruption of one selected child.
pub const INJECT: u64 = 38;
/// Complete owner custody in stable slot order; includes both empty and occupied slots.
pub type Custody = Vec<super::custody::Custody>;
/// Bounded observation of all permanent service histories.
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 65_536,
    allocation: 65_536 * 1024,
    nodes: 65_536 * 8,
    depth: super::custody::LIMITS.depth + 1,
};
