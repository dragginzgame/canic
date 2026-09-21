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
    Backoff,
    ConfiguredCanisters,
    EffectPreparation,
    EffectReconciliation,
    EffectSubmission,
    IndependentSubmission,
    EstateFunding,
    FleetSnapshot,
    LedgerFee,
    OperatorBalance,
    Planning,
    PoolBalances,
    ProtocolActions,
    ProtocolReadiness,
    RootManagement,
    TerminalInventory,
}

/// Paired stage boundary; an unmatched start denotes interrupted diagnostic work.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FleetObservationTiming {
    pub span_id: u64,
    pub parent_span_id: Option<u64>,
    pub stage: FleetObservationStage,
    /// Enclosing inclusive stage, if any; child counts must not be added to its totals.
    pub parent_stage: Option<FleetObservationStage>,
    pub elapsed_millis: u128,
    /// Logical host transport attempts, including failures; excludes internal IC hops.
    pub remote_call_attempts: u64,
    /// Local ICP identity Principal lookups, including failures.
    pub identity_lookup_attempts: u64,
    /// Inclusive local identity subprocess time, including startup and compatibility checks.
    pub identity_lookup_millis: u64,
    /// Responses served from the current observation cache, including repeated consumers.
    pub cached_read_hits: u64,
    /// None marks the start; Some marks completion, including a typed failure boundary.
    pub succeeded: Option<bool>,
}
