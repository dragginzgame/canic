use super::super::*;
use super::inventory::LocalInventoryRequest;
use super::registry::install_state_registry_observations;
use super::shared::{normalize_module_hash, observation_gap, read_live_canister_status};
use crate::{icp::IcpCanisterStatusReport, release_set::ConfiguredPoolExpectation};
use std::path::Path;

pub(super) fn install_state_observations(
    install_state: Option<&crate::install_root::InstallState>,
    request: &LocalInventoryRequest,
    pool_expectations: &[ConfiguredPoolExpectation],
    unresolved_observations: &mut Vec<DeploymentObservationGapV1>,
) -> (Vec<ObservedCanisterV1>, Vec<ObservedPoolCanisterV1>) {
    let Some(state) = install_state else {
        return (Vec::new(), Vec::new());
    };
    let mut observed_canisters = install_state_observed_canisters(
        state,
        &request.icp_root,
        &request.network,
        unresolved_observations,
    );
    let observed_pool = install_state_registry_observations(
        state,
        request,
        pool_expectations,
        &mut observed_canisters,
        unresolved_observations,
    );
    (observed_canisters, observed_pool)
}
pub(super) fn observed_root_observation(
    install_state: Option<&crate::install_root::InstallState>,
    request: &LocalInventoryRequest,
    fleet_name: &str,
    observed_canisters: &[ObservedCanisterV1],
) -> Option<DeploymentRootObservationV1> {
    let state = install_state?;
    let observed = observed_canisters
        .iter()
        .find(|canister| canister.canister_id == state.root_canister_id)?;
    Some(DeploymentRootObservationV1 {
        deployment_name: request.deployment_name.clone(),
        network: request.network.clone(),
        fleet_template: fleet_name.to_string(),
        root_principal: state.root_canister_id.clone(),
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
    if observed.role_assignment_source.as_deref() == Some("icp_canister_status") {
        DeploymentRootObservationSourceV1::IcpCanisterStatus
    } else {
        DeploymentRootObservationSourceV1::LocalDeploymentState
    }
}

fn install_state_observed_canisters(
    state: &crate::install_root::InstallState,
    icp_root: &Path,
    network: &str,
    gaps: &mut Vec<DeploymentObservationGapV1>,
) -> Vec<ObservedCanisterV1> {
    match read_live_canister_status(icp_root, network, &state.root_canister_id) {
        Ok(report) => vec![observed_root_from_status(state, &report)],
        Err(err) => {
            gaps.push(observation_gap(
                "live_canister_status.root",
                format!(
                    "could not observe live root canister status for {}: {err}",
                    state.root_canister_id
                ),
            ));
            vec![observed_root_from_install_state(state)]
        }
    }
}
pub(in crate::deployment_truth) fn observed_root_from_status(
    state: &crate::install_root::InstallState,
    report: &IcpCanisterStatusReport,
) -> ObservedCanisterV1 {
    let controllers = report
        .settings
        .as_ref()
        .map(|settings| settings.controllers.clone())
        .unwrap_or_default();
    ObservedCanisterV1 {
        canister_id: if report.id.is_empty() {
            state.root_canister_id.clone()
        } else {
            report.id.clone()
        },
        role: Some("root".to_string()),
        control_class: classify_root_control(&controllers, &state.root_canister_id),
        controllers,
        module_hash: report.module_hash.as_deref().map(normalize_module_hash),
        status: Some(report.status.clone()),
        root_trust_anchor: Some(state.root_canister_id.clone()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("icp_canister_status".to_string()),
    }
}

fn observed_root_from_install_state(
    state: &crate::install_root::InstallState,
) -> ObservedCanisterV1 {
    ObservedCanisterV1 {
        canister_id: state.root_canister_id.clone(),
        role: Some("root".to_string()),
        control_class: CanisterControlClassV1::UnknownUnsafe,
        controllers: Vec::new(),
        module_hash: None,
        status: None,
        root_trust_anchor: Some(state.root_canister_id.clone()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("local_install_state".to_string()),
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
