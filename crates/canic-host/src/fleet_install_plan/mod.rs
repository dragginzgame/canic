//! Module: fleet_install_plan
//!
//! Responsibility: expose immutable pre-effect multi-root Fleet installation planning.
//! Does not own: placement selection, Canister creation, installation, or Registry mutation.
//! Boundary: callers supply exact resolved placement/funding input before external effects.

mod authority;
mod decision;
mod initial_placement_policy;
mod model;
mod operator_evidence;
mod persistence;
mod preflight;
#[cfg(test)]
mod tests;

/// Exact Cycles Ledger fee charged for one `create_canister` call.
pub const CYCLES_LEDGER_CREATE_CANISTER_FEE_CYCLES: u128 = 100_000_000;

pub(crate) use authority::load_fresh_fleet_recovery_decision_authority;
pub use authority::{
    FreshFleetDecisionAuthorityError, FreshFleetDecisionAuthorityRequest,
    load_fresh_fleet_decision_authority,
};
pub(crate) use decision::compile_fresh_fleet_deployment_plan_with_operator_debit;
pub use decision::{
    FRESH_FLEET_DEPLOYMENT_PLAN_SCHEMA_VERSION, compile_fresh_fleet_deployment_plan,
    fresh_fleet_maximum_operator_debit,
};
pub(crate) use initial_placement_policy::required_initial_pool_asset_cycles;
pub use model::{
    FleetInstallPlan, FleetInstallPlanError, FleetInstallPlanRequest, FreshFleetCanisterCountsV1,
    FreshFleetCatalogEvidenceV1, FreshFleetDecisionAuthorityV1, FreshFleetDeploymentPlanError,
    FreshFleetDeploymentPlanRequest, FreshFleetDeploymentPlanV1, FreshFleetExpectedArtifactV1,
    FreshFleetFundingPayerV1, FreshFleetFundingRequirementV1, FreshFleetOperatorFundingEvidenceV1,
    FreshFleetPreflightEffectsV1, FreshFleetPreflightError, FreshFleetPreflightRequest,
    FreshFleetPreflightV1, FreshFleetReleaseSourceV1, FreshFleetSubnetRootPlanV1,
    PersistedFleetInstallPlan, PersistedFleetSubnetRootReleaseSet, PlannedCanisterCreationFunding,
    PlannedComponentGroupPlacementAssignment, PlannedFleetCoordinator, PlannedFleetSubnetRoot,
    PlannedFleetSubnetRootInput, PlannedSubnetPlacementCostEvidence,
};
pub use operator_evidence::{
    FreshFleetOperatorEvidenceError, observe_fresh_fleet_operator_funding,
};
pub(crate) use persistence::load_retained_fleet_install_plan;
pub use persistence::{compile_and_persist_fleet_install_plan, load_persisted_fleet_install_plan};
pub(crate) use preflight::compile_retained_fleet_preflight;
pub use preflight::{FRESH_FLEET_PREFLIGHT_SCHEMA_VERSION, compile_fresh_fleet_preflight};

#[cfg(test)]
use persistence::{FLEET_INSTALL_PLAN_FILE, fleet_install_plan_path, root_release_set_path};
