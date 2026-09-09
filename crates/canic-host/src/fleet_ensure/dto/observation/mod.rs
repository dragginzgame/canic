//! Module: fleet_ensure::dto::observation
//!
//! Responsibility: expose bounded timing evidence for existing observation stages.
//! Does not own: planning authority, cached state, retries, or remote effects.
//! Boundary: elapsed time and logical transport attempts are diagnostic only.

use serde::Serialize;

/// Existing observation boundary whose elapsed cost is measured.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetObservationStage {
    ConfiguredCanisters,
    EstateFunding,
    LedgerFee,
    OperatorBalance,
    PoolBalances,
    ProtocolActions,
    ProtocolReadiness,
    RootManagement,
    TerminalInventory,
}

/// Completed stage measurement, emitted on success and failure.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FleetObservationTiming {
    pub stage: FleetObservationStage,
    pub elapsed_millis: u128,
    pub remote_call_attempts: u64,
    pub succeeded: bool,
}
