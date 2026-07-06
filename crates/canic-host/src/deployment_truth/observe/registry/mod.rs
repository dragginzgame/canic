use super::super::*;
use super::inventory::LocalInventoryRequest;
use super::shared::{normalize_module_hash, observation_gap, read_live_canister_status};
use crate::{
    icp::IcpCanisterStatusReport,
    installed_deployment::{InstalledDeploymentRequest, resolve_installed_deployment_from_root},
    registry::RegistryEntry,
    release_set::ConfiguredPoolExpectation,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

pub(super) fn install_state_registry_observations(
    state: &crate::install_root::InstallState,
    request: &LocalInventoryRequest,
    pool_expectations: &[ConfiguredPoolExpectation],
    observed_canisters: &mut Vec<ObservedCanisterV1>,
    gaps: &mut Vec<DeploymentObservationGapV1>,
) -> Vec<ObservedPoolCanisterV1> {
    match resolve_installed_deployment_from_root(
        &InstalledDeploymentRequest {
            deployment: request.deployment_name.clone(),
            network: request.network.clone(),
            icp: "icp".to_string(),
            detect_lost_local_root: false,
        },
        &request.icp_root,
    ) {
        Ok(resolution) => {
            let mut registry_canisters = registry_entries_to_observed_canisters(
                &state.root_canister_id,
                &resolution.registry.entries,
            );
            enrich_registry_observed_canisters(
                &mut registry_canisters,
                &request.icp_root,
                &request.network,
                gaps,
            );
            let mut observed_pool = registry_entries_to_observed_pool(
                &state.root_canister_id,
                &resolution.registry.entries,
                pool_expectations,
                gaps,
            );
            apply_canister_control_to_observed_pool(&mut observed_pool, &registry_canisters);
            observed_canisters.extend(registry_canisters);
            observed_pool
        }
        Err(err) => {
            gaps.push(observation_gap(
                "live_subnet_registry",
                format!(
                    "could not observe live subnet registry for root {}: {err}",
                    state.root_canister_id
                ),
            ));
            Vec::new()
        }
    }
}

pub(in crate::deployment_truth) fn registry_entries_to_observed_canisters(
    root_canister_id: &str,
    entries: &[RegistryEntry],
) -> Vec<ObservedCanisterV1> {
    entries
        .iter()
        .filter(|entry| entry.pid != root_canister_id)
        .filter_map(registry_entry_to_observed_canister)
        .collect()
}

fn registry_entry_to_observed_canister(entry: &RegistryEntry) -> Option<ObservedCanisterV1> {
    let role = entry.role.clone()?;
    Some(ObservedCanisterV1 {
        canister_id: entry.pid.clone(),
        role: Some(role),
        control_class: registry_entry_control_class(entry),
        controllers: Vec::new(),
        module_hash: entry.module_hash.as_deref().map(normalize_module_hash),
        status: None,
        root_trust_anchor: entry.parent_pid.clone(),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("subnet_registry".to_string()),
    })
}

pub(in crate::deployment_truth) fn apply_canister_control_to_observed_pool(
    observed_pool: &mut [ObservedPoolCanisterV1],
    observed_canisters: &[ObservedCanisterV1],
) {
    let control_by_canister = observed_canisters
        .iter()
        .map(|canister| (canister.canister_id.as_str(), canister.control_class))
        .collect::<BTreeMap<_, _>>();
    for pool in observed_pool {
        if let Some(control_class) = control_by_canister.get(pool.canister_id.as_str()) {
            pool.control_class = *control_class;
        }
    }
}

fn enrich_registry_observed_canisters(
    observed_canisters: &mut [ObservedCanisterV1],
    icp_root: &Path,
    network: &str,
    gaps: &mut Vec<DeploymentObservationGapV1>,
) {
    for observed in observed_canisters {
        match read_live_canister_status(icp_root, network, &observed.canister_id) {
            Ok(report) => apply_live_status_to_registry_observation(observed, &report),
            Err(err) => gaps.push(observation_gap(
                live_status_gap_key(observed),
                format!(
                    "could not observe live canister status for role {} at {}: {err}",
                    observed.role.as_deref().unwrap_or("unknown"),
                    observed.canister_id
                ),
            )),
        }
    }
}

pub(in crate::deployment_truth) fn apply_live_status_to_registry_observation(
    observed: &mut ObservedCanisterV1,
    report: &IcpCanisterStatusReport,
) {
    let controllers = report
        .settings
        .as_ref()
        .map(|settings| settings.controllers.clone())
        .unwrap_or_default();
    observed.canister_id = if report.id.is_empty() {
        observed.canister_id.clone()
    } else {
        report.id.clone()
    };
    observed.control_class = classify_registry_observed_control(
        observed.control_class,
        &controllers,
        observed.root_trust_anchor.as_deref(),
    );
    observed.controllers = controllers;
    observed.module_hash = report.module_hash.as_deref().map(normalize_module_hash);
    observed.status = Some(report.status.clone());
    observed.role_assignment_source = Some("subnet_registry+icp_canister_status".to_string());
}

fn live_status_gap_key(observed: &ObservedCanisterV1) -> String {
    observed.role.as_ref().map_or_else(
        || format!("live_canister_status.{}", observed.canister_id),
        |role| format!("live_canister_status.{role}"),
    )
}

fn classify_registry_observed_control(
    registry_control_class: CanisterControlClassV1,
    controllers: &[String],
    root_trust_anchor: Option<&str>,
) -> CanisterControlClassV1 {
    let Some(anchor) = root_trust_anchor else {
        return registry_control_class;
    };
    if controllers.iter().any(|controller| controller == anchor) {
        registry_control_class
    } else {
        CanisterControlClassV1::UnknownUnsafe
    }
}

const fn registry_entry_control_class(entry: &RegistryEntry) -> CanisterControlClassV1 {
    if entry.parent_pid.is_some() {
        CanisterControlClassV1::CanicManagedPool
    } else {
        CanisterControlClassV1::UnknownUnsafe
    }
}

pub(in crate::deployment_truth) fn registry_entries_to_observed_pool(
    root_canister_id: &str,
    entries: &[RegistryEntry],
    pool_expectations: &[ConfiguredPoolExpectation],
    gaps: &mut Vec<DeploymentObservationGapV1>,
) -> Vec<ObservedPoolCanisterV1> {
    let expectations_by_role = pool_expectations_by_role(pool_expectations);
    let mut seen = BTreeSet::new();
    let mut observed = Vec::new();

    for entry in entries {
        if entry.pid == root_canister_id {
            continue;
        }
        let Some(role) = entry.role.as_ref() else {
            continue;
        };
        let Some(expectations) = expectations_by_role.get(role.as_str()) else {
            continue;
        };
        let [expectation] = expectations.as_slice() else {
            gaps.push(observation_gap(
                format!("live_subnet_registry.pool.{role}"),
                format!(
                    "could not assign observed role {role} to one configured pool without ambiguity"
                ),
            ));
            continue;
        };
        if !seen.insert(entry.pid.as_str()) {
            continue;
        }
        observed.push(ObservedPoolCanisterV1 {
            pool: expectation.pool.clone(),
            canister_id: entry.pid.clone(),
            role: Some(role.clone()),
            control_class: pool_control_class(entry),
        });
    }

    observed
}

fn pool_expectations_by_role(
    pool_expectations: &[ConfiguredPoolExpectation],
) -> BTreeMap<&str, Vec<&ConfiguredPoolExpectation>> {
    let mut by_role = BTreeMap::<&str, Vec<&ConfiguredPoolExpectation>>::new();
    for expectation in pool_expectations {
        by_role
            .entry(expectation.canister_role.as_str())
            .or_default()
            .push(expectation);
    }
    by_role
}

const fn pool_control_class(entry: &RegistryEntry) -> CanisterControlClassV1 {
    if entry.parent_pid.is_some() {
        CanisterControlClassV1::CanicManagedPool
    } else {
        CanisterControlClassV1::UnknownUnsafe
    }
}
