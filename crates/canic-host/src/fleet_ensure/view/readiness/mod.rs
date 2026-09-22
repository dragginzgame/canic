//! Read-only facts available before selecting or compiling a release.

use crate::fleet_ensure::model::FleetEnsureCompletion;
use serde::Serialize;

///
/// FleetReadiness
///
/// Snapshot for early operator checks, never deployment or payment authority.
///

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
    pub observed_at_unix_ms: u64,
    pub completed_at_unix_ms: u64,
    pub funding: PrebuildFundingReadiness,
}

///
/// RetainedReadinessOperation
///
/// Retained operation identity and completion, without changing its journal.
///

#[derive(Debug, Serialize)]
pub struct RetainedReadinessOperation {
    pub operation_id: String,
    pub plan_sha256: String,
    pub completion: FleetEnsureCompletion,
    pub terminal_review_required: bool,
}

///
/// ReadinessBlocker
///
/// A condition that should be resolved before starting a new build for this Fleet.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessBlocker {
    RetainedOperation,
    RetainedTerminalReview,
    EstimatedFundingShortfall,
    RootNativeShortfall,
    StartupFundingPolicy,
}

///
/// PrebuildFundingReadiness
///
/// Observations and estimates available without compiling or selecting artifacts.
///

#[derive(Debug, Serialize)]
pub struct PrebuildFundingReadiness {
    pub desired_sha256: Option<String>,
    pub app_config_sha256: Option<String>,
    pub roots: Vec<RootFundingReadiness>,
    #[serde(with = "crate::fleet_ensure::model::option_u128_text")]
    pub per_step_execution_allowance_cycles: Option<u128>,
    pub conversion: Option<ReadinessConversionQuote>,
    pub unresolved: Vec<ReadinessUnresolved>,
}

///
/// RootFundingReadiness
///
/// A Root's native balance and configuration floor, excluding unbuilt execution work.
///

#[derive(Debug, Serialize)]
pub struct RootFundingReadiness {
    pub root: String,
    pub principal: Option<String>,
    #[serde(with = "crate::fleet_ensure::model::option_u128_text")]
    pub available_native_cycles: Option<u128>,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub configured_minimum_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::option_u128_text")]
    pub startup_minimum_cycles: Option<u128>,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub required_native_floor_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::option_u128_text")]
    pub floor_shortfall_cycles: Option<u128>,
    pub unfunded_role: Option<crate::fleet_ensure::model::StartupRoleShortfall>,
    pub unavailable: Option<RootReadinessUnavailable>,
}

///
/// RootReadinessUnavailable
///
/// Why an existing native balance cannot be claimed from this observation.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RootReadinessUnavailable {
    PrincipalUnresolved,
    AuthorityMismatch,
    ObservationFailed,
    BalanceUnavailable,
}

///
/// ReadinessConversionQuote
///
/// Advisory conversion of the caller's operator-Ledger shortfall, never a Root top-up quote.
///

#[derive(Debug, Serialize)]
pub struct ReadinessConversionQuote {
    pub cmc: String,
    pub icp_ledger: String,
    pub rate: crate::fleet_ensure::view::operator_mint::OperatorMintRateQuote,
    pub estimated_mint_e8s: Option<u64>,
    pub estimated_total_icp_debit_e8s: Option<u64>,
}

///
/// ReadinessUnresolved
///
/// Unresolved inputs must not be mistaken for zero costs or deployment approval.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessUnresolved {
    DesiredNotSelected,
    StartupConfiguration,
    ArtifactExecutionReserve,
    PoolAndCurrentGrantUsage,
    SelectedPlanOperatorDebit,
    ConversionNotRequested,
    ConversionObservationFailed,
    ConversionAmountUnavailable,
    OperatorIcpBalance,
    FreshPlanAdmission,
}
