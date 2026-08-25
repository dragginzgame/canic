//! Module: install_root::fleet_component_provisioning_journal
//!
//! Responsibility: expose host recovery authority for fresh Component provisioning.
//! Does not own: Coordinator/root state machines, placement derivation, or catalog validation.
//! Boundary: typed transitions, authority validation and durable persistence have separate owners.

mod model;
mod persistence;
#[cfg(test)]
pub(super) mod tests;
mod transition;
mod validation;

#[cfg(test)]
use model::FleetComponentProvisioningInstallJournalError;
pub(super) use model::{
    FleetComponentProvisioningInstallPhase, FleetComponentProvisioningTerminalEvidence,
    JOURNAL_SCHEMA_VERSION, PlanFleetComponentProvisioningInstallRequest,
    ResolvedFleetComponentProvisioningInstall,
};
pub(super) use persistence::terminal_evidence as terminal_component_provisioning_evidence;
pub(super) use transition::{
    begin_component_provisioning_advance, begin_component_provisioning_preparation,
    begin_fleet_catalog_publication, complete_fleet_component_provisioning_install,
    plan_fleet_component_provisioning_install, record_component_provisioning_advanced,
    record_component_provisioning_observed, record_fleet_catalog_published,
};

#[cfg(test)]
use validation::advance_request;
