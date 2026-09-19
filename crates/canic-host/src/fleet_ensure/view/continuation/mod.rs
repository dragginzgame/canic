//! Module: fleet_ensure::view::continuation
//!
//! Responsibility: distinguish reviewed work from dependent live discovery.
//! Boundary: these projections never authorize an effect or enlarge a debit bound.

use serde::Serialize;

/// Operator preview of the current review and its unresolved continuation work.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ContinuationForecast {
    pub authority: ContinuationAuthority,
    pub reviewed_actions: usize,
    pub maximum_successor_actions: Option<u32>,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub maximum_operator_debit_cycles: u128,
    pub imports: Vec<ContinuationImport>,
    pub dependent_funding: Vec<DependentPoolFunding>,
    pub requires_live_discovery: Vec<ContinuationDiscovery>,
}

/// The authority available for future protocol work; extra paid effects always need review.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinuationAuthority {
    Complete,
    SeparateReview,
    WithinReviewedProtocolBounds,
}

/// Known import target, with readiness deliberately left to current Root evidence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ContinuationImport {
    pub root: String,
    pub canister: String,
    pub principal: Option<String>,
    pub state: ContinuationImportState,
}

/// Distinguish an already reviewed import from an initialization-dependent candidate.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinuationImportState {
    PostInitializationObservation,
    ReviewedReconciliation,
}

/// A known dependent top-up estimate that is not included in continuation authority.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DependentPoolFunding {
    pub root: String,
    pub principal: String,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub amount_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub ledger_fee_cycles: u128,
}

/// Missing evidence that prevents a complete remaining-work or funding quote.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinuationDiscovery {
    PoolReadiness,
    PoolFundingAndCapacity,
    PublicationAndProvisioning,
}
