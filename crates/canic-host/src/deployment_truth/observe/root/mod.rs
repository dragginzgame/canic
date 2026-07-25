use super::super::*;
use super::inventory::LocalInventoryRequest;
use super::registry::fleet_catalog_registry_observations;
use super::shared::{normalize_module_hash, observation_gap, read_live_canister_status};
use crate::fleet_catalog::FleetCatalogEntryV1;
use crate::{icp::IcpCanisterStatusReport, release_set::ConfiguredPoolExpectation};
use std::path::Path;

pub(super) fn fleet_catalog_observations(
    installed_fleet: Option<&FleetCatalogEntryV1>,
    request: &LocalInventoryRequest,
    pool_expectations: &[ConfiguredPoolExpectation],
    unresolved_observations: &mut Vec<DeploymentObservationGapV1>,
) -> (Vec<ObservedCanisterV1>, Vec<ObservedPoolCanisterV1>) {
    let Some(fleet) = installed_fleet else {
        return (Vec::new(), Vec::new());
    };
    let mut observed_canisters = fleet_catalog_observed_canisters(
        fleet,
        &request.icp_root,
        &request.environment,
        unresolved_observations,
    );
    let observed_pool = fleet_catalog_registry_observations(
        fleet,
        request,
        pool_expectations,
        &mut observed_canisters,
        unresolved_observations,
    );
    (observed_canisters, observed_pool)
}
pub(super) fn observed_root_observation(
    installed_fleet: Option<&FleetCatalogEntryV1>,
    request: &LocalInventoryRequest,
    observed_canisters: &[ObservedCanisterV1],
) -> Option<DeploymentRootObservationV1> {
    let fleet = installed_fleet?;
    let observed = observed_canisters
        .iter()
        .find(|canister| canister.canister_id == fleet.root_principal)?;
    Some(DeploymentRootObservationV1 {
        canonical_network_id: fleet.canonical_network_id,
        fleet_id: fleet.fleet_id,
        fleet_name: fleet.fleet_name.to_string(),
        app: fleet.app.to_string(),
        environment: request.environment.clone(),
        root_principal: fleet.root_principal.clone(),
        observed_canister_id: observed.canister_id.clone(),
        observation_source: root_observation_source(observed),
        control_class: observed.control_class,
        controllers: observed.controllers.clone(),
        module_hash: observed.module_hash.clone(),
        status: observed.status.clone(),
        role_assignment_source: observed.role_assignment_source.clone(),
    })
}

fn root_observation_source(observed: &ObservedCanisterV1) -> DeploymentRootObservationSourceV1 {
    if observed
        .role_assignment_source
        .as_deref()
        .is_some_and(RoleAssignmentSourceV1::label_includes_live_status)
    {
        DeploymentRootObservationSourceV1::IcpCanisterStatus
    } else {
        DeploymentRootObservationSourceV1::FleetCatalog
    }
}

fn fleet_catalog_observed_canisters(
    fleet: &FleetCatalogEntryV1,
    icp_root: &Path,
    environment: &str,
    gaps: &mut Vec<DeploymentObservationGapV1>,
) -> Vec<ObservedCanisterV1> {
    match read_live_canister_status(icp_root, environment, &fleet.root_principal) {
        Ok(report) => vec![observed_root_from_status(fleet, &report)],
        Err(err) => {
            gaps.push(observation_gap(
                "live_canister_status.root",
                format!(
                    "could not observe live root canister status for {}: {err}",
                    fleet.root_principal
                ),
            ));
            vec![observed_root_from_fleet_catalog(fleet)]
        }
    }
}
pub(in crate::deployment_truth) fn observed_root_from_status(
    fleet: &FleetCatalogEntryV1,
    report: &IcpCanisterStatusReport,
) -> ObservedCanisterV1 {
    let controllers = report
        .settings
        .as_ref()
        .map(|settings| settings.controllers.clone())
        .unwrap_or_default();
    ObservedCanisterV1 {
        canister_id: if report.id.is_empty() {
            fleet.root_principal.clone()
        } else {
            report.id.clone()
        },
        role: Some("root".to_string()),
        control_class: classify_root_control(&controllers, &fleet.root_principal),
        controllers,
        module_hash: report.module_hash.as_deref().map(normalize_module_hash),
        status: Some(report.status.clone()),
        root_trust_anchor: Some(fleet.root_principal.clone()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some(
            RoleAssignmentSourceV1::IcpCanisterStatus
                .label()
                .to_string(),
        ),
    }
}

fn observed_root_from_fleet_catalog(fleet: &FleetCatalogEntryV1) -> ObservedCanisterV1 {
    ObservedCanisterV1 {
        canister_id: fleet.root_principal.clone(),
        role: Some("root".to_string()),
        control_class: CanisterControlClassV1::UnknownUnsafe,
        controllers: Vec::new(),
        module_hash: None,
        status: None,
        root_trust_anchor: Some(fleet.root_principal.clone()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some(RoleAssignmentSourceV1::FleetCatalog.label().to_string()),
    }
}

fn classify_root_control(controllers: &[String], root_canister_id: &str) -> CanisterControlClassV1 {
    if controllers
        .iter()
        .any(|controller| controller == root_canister_id)
    {
        CanisterControlClassV1::DeploymentControlled
    } else {
        CanisterControlClassV1::UnknownUnsafe
    }
}
