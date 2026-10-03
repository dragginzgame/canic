//! Provisioning recovery facts retain journal ownership without granting release authority.

use crate::fleet_ensure::view::release::FleetReleaseProvisioningView;
use candid::Principal;
use std::collections::BTreeSet;

/// Separate journals may use the same operation ID without sharing a recovery owner.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ReleaseProvisioningOwner {
    Provisioning,
    DirectorySynchronization,
}

/// Owner-qualified identity of one retained operation or active pointer.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ReleaseProvisioningIdentity {
    pub owner: ReleaseProvisioningOwner,
    pub operation_id: [u8; 32],
}

/// Recorded stage; absent delivery does not establish that lower-level paid work is settled.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseProvisioningState {
    Accepted,
    Provisioned,
    Publishing,
    Published,
    Activating,
    RuntimesActive,
    DirectoryPlanned,
    DirectorySynchronizing,
    DirectorySynchronized,
}

/// Original identity, stage and explicit delivery intent supplied to pure policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseProvisioningFacts {
    pub identity: ReleaseProvisioningIdentity,
    pub plan_hash: [u8; 32],
    pub state: ReleaseProvisioningState,
    pub delivery_in_flight: Option<Principal>,
}

/// Complete projected journal and active pointers from one authenticated Root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseRootProvisioningFacts {
    pub root: Principal,
    pub active: BTreeSet<ReleaseProvisioningIdentity>,
    pub operations: Vec<ReleaseProvisioningFacts>,
}

/// Remaining owner work; unfinished installations need not be completed before release.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseProvisioningDisposition {
    /// The existing owner must account for its lower-level work before state can be discarded.
    OwnerReconciliation,
    /// Retained delivery intent must be reconciled without blind redispatch.
    DeliveryReconciliation { recipient: Principal },
    /// Terminal aggregate history; this is not proof of current custody or quiescence.
    RecordedCompletion,
}

/// Exact original facts and their recovery meaning, independent of failure counters or deadlines.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseProvisioningAssessment {
    pub facts: ReleaseProvisioningFacts,
    pub disposition: ReleaseProvisioningDisposition,
}

/// Missing active records remain explicit; stale pointers never revive terminal operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseRootProvisioningAssessment {
    pub root: Principal,
    pub operations: Vec<ReleaseProvisioningAssessment>,
    pub unmatched_active: BTreeSet<ReleaseProvisioningIdentity>,
}

/// All original pages and derived assessments, without reset or spending authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetReleaseProvisioningAssessment {
    pub evidence: FleetReleaseProvisioningView,
    pub roots: Vec<ReleaseRootProvisioningAssessment>,
}
