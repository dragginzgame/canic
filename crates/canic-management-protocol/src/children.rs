//! Two independently retained child slots under one management owner.
use serde::{Deserialize, Serialize};

/// Stable slot identity. Slot order never changes when another child is admitted.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Slot {
    /// The original child, including journals migrated from one-child executables.
    First,
    /// One additional child with independent authority and evidence.
    Second,
}

impl Slot {
    /// Qualified child identities in permanent custody order.
    /// This is the current protocol bound, not an operator capacity setting.
    pub const ALL: [Self; 2] = [Self::First, Self::Second];

    /// Array position in the bounded owner journal.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::First => 0,
            Self::Second => 1,
        }
    }
}

/// Exact child scope is part of the authenticated request, not ambient execution state.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Request<T> {
    /// Stable target slot.
    pub slot: Slot,
    /// Existing creation or upgrade contract.
    pub value: T,
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
