//! Read-only facts available before selecting or compiling a release.

use crate::fleet_ensure::model::FleetEnsureCompletion;
use serde::Serialize;

/// Snapshot for early operator checks, never deployment or payment authority.
#[derive(Debug, Serialize)]
pub struct FleetReadiness {
    pub environment: String,
    pub fleet: String,
    pub operator: String,
    pub cycles_ledger: String,
    pub network_identity: String,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub available_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::option_u128_text")]
    pub estimated_required_cycles: Option<u128>,
    #[serde(with = "crate::fleet_ensure::model::option_u128_text")]
    pub estimated_shortfall_cycles: Option<u128>,
    pub retained_operation: Option<RetainedReadinessOperation>,
    pub blockers: Vec<ReadinessBlocker>,
}

/// Retained operation identity and completion, without changing its journal.
#[derive(Debug, Serialize)]
pub struct RetainedReadinessOperation {
    pub operation_id: String,
    pub plan_sha256: String,
    pub completion: FleetEnsureCompletion,
}

/// A condition that should be resolved before starting a new build for this Fleet.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessBlocker {
    RetainedOperation,
    EstimatedFundingShortfall,
}
