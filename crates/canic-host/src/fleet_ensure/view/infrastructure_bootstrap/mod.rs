//! Actual infrastructure custody observed through the operator's management authority.

use crate::fleet_ensure::model::capacity_import::survey::CapacityImportSampleRecord;
use canic_core::dto::fleet_registry::FleetRegistry;
use std::collections::BTreeMap;

/// A complete source sample; missing Coordinator identity is permitted only for reviewed creation.
#[derive(Clone, Debug)]
pub struct InfrastructureBootstrapObservation {
    pub held_sources: BTreeMap<
        String,
        crate::fleet_ensure::model::infrastructure_bootstrap::InfrastructureBootstrapCustodyRecord,
    >,
    pub canisters: BTreeMap<String, Option<CapacityImportSampleRecord>>,
    pub coordinator_registry: Option<FleetRegistry>,
    pub operator_cycles: u128,
    pub ledger_fee_cycles: u128,
}

/// Publication and the execution report produced by this invocation, if network work ran.
/// A completed publication replays locally without inventing newly applied effects.
pub(in crate::fleet_ensure) struct InfrastructureBootstrapApplyView {
    pub publication: crate::fleet_ensure::model::infrastructure_bootstrap::InfrastructureBootstrapPublicationRecord,
    pub execution: Option<crate::fleet_ensure::model::FleetEnsureReport>,
}
