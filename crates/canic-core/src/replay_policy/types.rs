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
        command_kind: ReplayCommandKindLabel,
    },
    ReplayProtected {
        command_kind: ReplayCommandKindLabel,
        requires_operation_id: bool,
    },
    MonotonicTransition {
        command_kind: ReplayCommandKindLabel,
    },
    SnapshotConvergent {
        command_kind: ReplayCommandKindLabel,
    },
    CommandDispatch {
        command_kind: ReplayCommandKindLabel,
        command_manifest: ReplayCommandManifestLabel,
    },
    IntentionallyNonIdempotent {
        command_kind: ReplayCommandKindLabel,
        reason: &'static str,
    },
}

///
/// ReplayCommandKindLabel
///
/// Static manifest-owned replay command kind label.
/// Runtime replay storage still uses `model::replay::CommandKind`.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReplayCommandKindLabel(&'static str);

impl ReplayCommandKindLabel {
    #[must_use]
    pub const fn new(label: &'static str) -> Self {
        Self(label)
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

///
/// ReplayCommandManifestLabel
///
/// Static manifest-owned replay command manifest label.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReplayCommandManifestLabel(&'static str);

impl ReplayCommandManifestLabel {
    #[must_use]
    pub const fn new(label: &'static str) -> Self {
        Self(label)
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

///
/// ReplayQuotaPolicyLabel
///
/// Static manifest-owned replay quota policy label.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReplayQuotaPolicyLabel(&'static str);

impl ReplayQuotaPolicyLabel {
    #[must_use]
    pub const fn new(label: &'static str) -> Self {
        Self(label)
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

///
/// ReplayCycleReservePolicyLabel
///
/// Static manifest-owned replay cycle-reserve policy label.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReplayCycleReservePolicyLabel(&'static str);

impl ReplayCycleReservePolicyLabel {
    #[must_use]
    pub const fn new(label: &'static str) -> Self {
        Self(label)
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
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
    RootChainKeySigning,
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
    pub quota_policy: Option<ReplayQuotaPolicyLabel>,
    pub cycle_reserve_policy: Option<ReplayCycleReservePolicyLabel>,
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
    pub quota_policy: Option<ReplayQuotaPolicyLabel>,
    pub cycle_reserve_policy: Option<ReplayCycleReservePolicyLabel>,
}

///
/// RootCapabilityCommandReplayPolicy
///
/// Replay manifest row for an internal root-capability command variant.
/// Owned by replay policy and stored in the root-capability command manifest.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootCapabilityCommandReplayPolicy {
    pub variant: &'static str,
    pub replay_policy: ReplayPolicy,
    pub implementation_status: ReplayImplementationStatus,
    pub cost_class: CostClass,
    pub quota_policy: Option<ReplayQuotaPolicyLabel>,
    pub cycle_reserve_policy: Option<ReplayCycleReservePolicyLabel>,
}
