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
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FleetEnsureProgressState {
    Advancing,
    AwaitingProgress,
    PrerequisiteComplete,
    FundingRequired,
    ReviewRequired {
        reason: FleetEnsureSuccessorReviewReason,
    },
    Complete,
}

/// Bounded progress event bound to the exact reviewed operation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FleetEnsureProgress {
    pub operation_id: String,
    pub plan_sha256: String,
    pub phase: FleetEnsurePhase,
    pub state: FleetEnsureProgressState,
    pub applied_effects: u32,
    pub reviewed_effects: usize,
}
