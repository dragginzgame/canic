//! Module: canic_cli::info_subnets::transport
//!
//! Responsibility: collect exact Coordinator and Fleet Subnet Root inventory observations.
//! Does not own: report aggregation, authority policy, or rendering.
//! Boundary: validates Coordinator evidence before querying roots and returns no partial report.

use crate::info_subnets::{
    InfoSubnetsCommandError, InfoSubnetsOptions,
    model::{FleetSubnetInventoryReportV1, SubnetInventoryPlan},
};

use std::thread;

use candid::CandidType;
use canic_core::{
    dto::{
        fleet_registry::{FleetRegistry, FleetRegistryManifest, FleetRegistryVersion},
        fleet_subnet_root::FleetSubnetRootCanisterSummary,
    },
    protocol,
};
use canic_host::{
    icp::IcpCli,
    icp_config::resolve_current_canic_icp_root,
    installed_fleet::read_installed_fleet_from_root,
    protocol_binding::{ResolvedProtocolBinding, resolve_infrastructure_protocol_binding},
    query_canister_with_arg,
    release_set::{CanicInfrastructureRole, load_persisted_canic_infrastructure_artifact_manifest},
};

#[derive(CandidType)]
enum CoordinatorStatusRequestFragment {
    Registry,
    RegistryManifest,
    RegistryVersion,
}

#[derive(CandidType, serde::Deserialize)]
enum CoordinatorStatusResponseFragment {
    Registry(FleetRegistry),
    RegistryManifest(FleetRegistryManifest),
    RegistryVersion(FleetRegistryVersion),
}

#[derive(CandidType)]
enum RootStatusRequestFragment {
    Inventory,
}

#[derive(CandidType, serde::Deserialize)]
enum RootStatusResponseFragment {
    Inventory(FleetSubnetRootCanisterSummary),
}

pub(super) fn load_report(
    options: &InfoSubnetsOptions,
) -> Result<FleetSubnetInventoryReportV1, InfoSubnetsCommandError> {
    let icp_root = resolve_current_canic_icp_root().map_err(InfoSubnetsCommandError::IcpRoot)?;
    let catalog = read_installed_fleet_from_root(&options.environment, &options.fleet, &icp_root)?;
    let icp = IcpCli::new(&options.icp, Some(options.environment.clone())).with_cwd(&icp_root);
    let infrastructure_manifest =
        load_persisted_canic_infrastructure_artifact_manifest(&icp_root, catalog.release_build_id)
            .map_err(|error| InfoSubnetsCommandError::Usage(error.to_string()))?;
    let binding = |role| {
        let artifact = infrastructure_manifest
            .manifest
            .entries
            .iter()
            .find(|entry| entry.role == role)
            .ok_or_else(|| {
                InfoSubnetsCommandError::Usage(format!(
                    "installed release is missing {} protocol metadata",
                    role.as_str()
                ))
            })?;
        resolve_infrastructure_protocol_binding(&icp_root, &options.environment, artifact)
            .map_err(|error| InfoSubnetsCommandError::Usage(error.to_string()))
    };
    let coordinator_binding = binding(CanicInfrastructureRole::FleetCoordinator)?;
    let root_binding = binding(CanicInfrastructureRole::FleetSubnetRoot)?;
    let coordinator = &catalog.coordinator_principal;
    let coordinator_principal = candid::Principal::from_text(coordinator).map_err(|_| {
        InfoSubnetsCommandError::Usage("installed Coordinator Principal is invalid".to_string())
    })?;
    let CoordinatorStatusResponseFragment::Registry(registry) = query_canister_with_arg(
        &icp,
        &coordinator_binding,
        coordinator_principal,
        protocol::CANIC_STATUS,
        &CoordinatorStatusRequestFragment::Registry,
    )?
    else {
        return Err(correlation_error());
    };
    let CoordinatorStatusResponseFragment::RegistryManifest(manifest) = query_canister_with_arg(
        &icp,
        &coordinator_binding,
        coordinator_principal,
        protocol::CANIC_STATUS,
        &CoordinatorStatusRequestFragment::RegistryManifest,
    )?
    else {
        return Err(correlation_error());
    };
    let CoordinatorStatusResponseFragment::RegistryVersion(version) = query_canister_with_arg(
        &icp,
        &coordinator_binding,
        coordinator_principal,
        protocol::CANIC_STATUS,
        &CoordinatorStatusRequestFragment::RegistryVersion,
    )?
    else {
        return Err(correlation_error());
    };
    let plan = SubnetInventoryPlan::compile(catalog, registry, manifest, version)?;
    let summaries = query_root_summaries(&icp, &root_binding, plan.root_principals())?;
    plan.complete(summaries).map_err(Into::into)
}

fn query_root_summaries(
    icp: &IcpCli,
    binding: &ResolvedProtocolBinding,
    roots: Vec<candid::Principal>,
) -> Result<Vec<FleetSubnetRootCanisterSummary>, InfoSubnetsCommandError> {
    let mut handles = Vec::with_capacity(roots.len());
    for root in roots {
        let canister = root.to_text();
        let icp = icp.clone();
        let binding = binding.clone();
        let handle = thread::spawn(move || {
            let response: RootStatusResponseFragment = query_canister_with_arg(
                &icp,
                &binding,
                root,
                protocol::CANIC_STATUS,
                &RootStatusRequestFragment::Inventory,
            )?;
            match response {
                RootStatusResponseFragment::Inventory(summary) => Ok(summary),
            }
        });
        handles.push((canister, handle));
    }

    handles
        .into_iter()
        .map(|(root, handle)| {
            handle
                .join()
                .map_err(|_| InfoSubnetsCommandError::SummaryWorkerPanicked { root })?
        })
        .collect()
}

fn correlation_error() -> InfoSubnetsCommandError {
    InfoSubnetsCommandError::Usage(
        "Coordinator returned a differently correlated status response".to_string(),
    )
}
