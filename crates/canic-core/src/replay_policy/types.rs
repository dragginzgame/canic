//! Module: replay_policy::types
//!
//! Responsibility: define the public replay-policy manifest data shapes.
//! Does not own: endpoint execution, replay storage, or workflow guards.
//! Boundary: passive policy metadata consumed by manifests and release checks.

///
/// EndpointKind
///
/// Boundary classification for a manifested Canic endpoint.
/// Owned by replay policy and used by endpoint manifest rows.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EndpointKind {
    Query,
    Update,
}

///
/// ReplayPolicy
///
/// Replay behavior classification recorded for endpoint and command surfaces.
/// Owned by replay policy and consumed by release checks and replay workflows.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplayPolicy {
    QueryOrReadOnly,
    ResponseIdempotent {
        command_kind: &'static str,
    },
    ReplayProtected {
        command_kind: &'static str,
        requires_operation_id: bool,
    },
    MonotonicTransition {
        command_kind: &'static str,
    },
    SnapshotConvergent {
        command_kind: &'static str,
    },
    CommandDispatch {
        command_kind: &'static str,
        command_manifest: &'static str,
    },
    IntentionallyNonIdempotent {
        command_kind: &'static str,
        reason: &'static str,
    },
}

///
/// CostClass
///
/// Cost and quota family attached to a replay-policy entry.
/// Owned by replay policy and mapped into cost guard configuration.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CostClass {
    None,
    RootCanisterSignaturePrepare,
    IssuerCanisterSignaturePrepare,
    ManagementDeployment,
    ValueTransfer,
    DurablePublish,
}

///
/// ReplayImplementationStatus
///
/// Release-readiness state for a manifest entry.
/// Owned by replay policy and consumed by release-blocker tests.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplayImplementationStatus {
    Implemented,
    ReleaseBlocker,
}

///
/// EndpointReplayPolicy
///
/// Replay manifest row for a Canic endpoint.
/// Owned by replay policy and stored in the endpoint replay manifest.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EndpointReplayPolicy {
    pub endpoint: &'static str,
    pub endpoint_kind: EndpointKind,
    pub replay_policy: ReplayPolicy,
    pub implementation_status: ReplayImplementationStatus,
    pub cost_class: CostClass,
    pub quota_policy: Option<&'static str>,
    pub cycle_reserve_policy: Option<&'static str>,
}

///
/// PoolAdminCommandReplayPolicy
///
/// Replay manifest row for a `PoolAdminCommand` variant.
/// Owned by replay policy and stored in the pool-admin command manifest.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PoolAdminCommandReplayPolicy {
    pub variant: &'static str,
    pub replay_policy: ReplayPolicy,
    pub implementation_status: ReplayImplementationStatus,
    pub cost_class: CostClass,
    pub quota_policy: Option<&'static str>,
    pub cycle_reserve_policy: Option<&'static str>,
}

///
/// RootCapabilityCommandReplayPolicy
///
/// Replay manifest row for a `RootCapabilityCommand` variant.
/// Owned by replay policy and stored in the root-capability command manifest.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootCapabilityCommandReplayPolicy {
    pub variant: &'static str,
    pub replay_policy: ReplayPolicy,
    pub implementation_status: ReplayImplementationStatus,
    pub cost_class: CostClass,
    pub quota_policy: Option<&'static str>,
    pub cycle_reserve_policy: Option<&'static str>,
}
