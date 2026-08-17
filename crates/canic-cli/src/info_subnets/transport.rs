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
    icp::IcpCli, icp_config::resolve_current_canic_icp_root,
    installed_fleet::read_installed_fleet_from_root, query_canister_with_arg,
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
    let coordinator = &catalog.coordinator_principal;
    let coordinator_principal = candid::Principal::from_text(coordinator).map_err(|_| {
        InfoSubnetsCommandError::Usage("installed Coordinator Principal is invalid".to_string())
    })?;
    let registry = match query_canister_with_arg(
        &icp,
        coordinator_principal,
        protocol::CANIC_STATUS,
        &CoordinatorStatusRequestFragment::Registry,
    )? {
        CoordinatorStatusResponseFragment::Registry(registry) => registry,
        _ => return Err(correlation_error()),
    };
    let manifest = match query_canister_with_arg(
        &icp,
        coordinator_principal,
        protocol::CANIC_STATUS,
        &CoordinatorStatusRequestFragment::RegistryManifest,
    )? {
        CoordinatorStatusResponseFragment::RegistryManifest(manifest) => manifest,
        _ => return Err(correlation_error()),
    };
    let version = match query_canister_with_arg(
        &icp,
        coordinator_principal,
        protocol::CANIC_STATUS,
        &CoordinatorStatusRequestFragment::RegistryVersion,
    )? {
        CoordinatorStatusResponseFragment::RegistryVersion(version) => version,
        _ => return Err(correlation_error()),
    };
    let plan = SubnetInventoryPlan::compile(catalog, registry, manifest, version)?;
    let summaries = query_root_summaries(&icp, plan.root_principals())?;
    plan.complete(summaries).map_err(Into::into)
}

fn query_root_summaries(
    icp: &IcpCli,
    roots: Vec<candid::Principal>,
) -> Result<Vec<FleetSubnetRootCanisterSummary>, InfoSubnetsCommandError> {
    let mut handles = Vec::with_capacity(roots.len());
    for root in roots {
        let canister = root.to_text();
        let icp = icp.clone();
        let handle = thread::spawn(move || {
            let response: RootStatusResponseFragment = query_canister_with_arg(
                &icp,
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
