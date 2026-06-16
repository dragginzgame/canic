//! Module: replay_policy::types
//! Responsibility: define the public replay-policy manifest data shapes.
//! Boundary: owns passive policy metadata types, not endpoint execution.

///
/// EndpointKind
///
/// Boundary classification for a manifested Canic endpoint.
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
