//! Module: fleet_ensure::dto::progress
//!
//! Responsibility: describe bounded operator-visible Fleet convergence progress.
//! Does not own: authority, persistence, decisions, or effects.
//! Boundary: progress is informational; the retained journal owns completion.

use crate::fleet_ensure::model::FleetEnsureSuccessorReviewReason;
use serde::Serialize;

/// Named phase currently being advanced or verified by Fleet Ensure.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetEnsurePhase {
    Infrastructure,
    ImportReconciliation,
    ControlPlane,
    WorkloadProvisioning,
    PoolReadiness,
    TerminalVerification,
    Complete,
}

/// Whether the named phase is advancing, waiting or requires operator action.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FleetEnsureProgressState {
    Advancing,
    AwaitingProgress {
        /// Time spent awaiting this effect or terminal check in this invocation.
        elapsed_seconds: u64,
        provisioning: Option<FleetProvisioningProgress>,
    },
    PrerequisiteComplete,
    FundingRequired,
    ReviewRequired {
        reason: FleetEnsureSuccessorReviewReason,
        review: Option<Box<crate::fleet_ensure::model::FleetSuccessorReview>>,
    },
    Complete,
}

/// Bounded informational projection of the existing Coordinator observation.
/// Counts describe Roots, not individual Component activation or funding authority.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FleetProvisioningProgress {
    /// Reviewed component occurrences with only owner-observed progress.
    pub components: Vec<FleetComponentProgress>,
    /// Latest retryable Root failure from this observation, not a new retry decision.
    pub pending_root_failure:
        Option<canic_core::dto::component_provisioning::FleetComponentProvisioningRootFailure>,
    pub phase: canic_core::dto::component_provisioning::FleetComponentProvisioningPhase,
    pub root_batch_count: u32,
    pub accepted_root_count: u32,
    pub provisioned_root_count: u32,
    pub directory_confirmed_root_count: u32,
    pub directory_confirmation_root_count: u32,
    pub runtime_activated_root_count: u32,
    pub component_count: u32,
}

/// A reviewed occurrence stays distinct across repeated placements and Roots.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FleetComponentProgress {
    pub component_spec: String,
    pub deployment: String,
    pub placement: u32,
    pub member_path: Vec<String>,
    pub root: candid::Principal,
    pub state: FleetComponentProgressState,
    /// This occurrence is at the owner's current sequential cursor, not proof of failure.
    pub current: bool,
}

/// Positive evidence comes from exact per-member cursors, never aggregate Root counts.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetComponentProgressState {
    Unknown,
    Reserved,
    Claimed,
    Installed,
    Registered,
    Published,
    RuntimePending,
    Active,
}

/// Bounded progress event bound to the exact reviewed operation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FleetEnsureProgress {
    /// First unfinished reviewed action, not proof that its remote execution has started.
    pub next_action: Option<FleetEnsureActionProgress>,
    pub operation_id: String,
    pub plan_sha256: String,
    pub phase: FleetEnsurePhase,
    pub state: FleetEnsureProgressState,
    pub applied_effects: u32,
    pub reviewed_effects: usize,
}

/// Bounded host projection of reviewed work; carries no payload or execution authority.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FleetEnsureActionProgress {
    pub kind: FleetEnsureActionKind,
    pub target: String,
}

/// Informational action identity projected from the maintained reviewed action contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetEnsureActionKind {
    ActivateRegistry,
    ActivateRegistryMirror,
    AdoptStore,
    BootstrapStore,
    Create,
    Delete,
    DeleteSnapshot,
    Fund,
    FundEstate,
    Install,
    JoinRoot,
    MaintainPoolReadiness,
    ObservePoolReadiness,
    PrepareComponentRegistry,
    PrepareStoreFixture,
    Protocol,
    ProvisionComponents,
    PublishStoreChunk,
    PublishStoreFixtureChunk,
    ReconcilePoolAsset,
    SetControllers,
    Start,
    Stop,
    SynchronizeRegistry,
    Transfer,
    Uninstall,
}
