//! Module: canic_cli::info_subnets::transport
//!
//! Responsibility: collect exact Coordinator and Fleet Subnet Root inventory observations.
//! Does not own: report aggregation, terminal ensure authority, or rendering.
//! Boundary: validates terminal current authority before querying Roots and returns no partial
//! report.

use crate::{
    info_subnets::{
        InfoSubnetsCommandError, InfoSubnetsOptions,
        model::{FleetSubnetInventoryReportV1, SubnetInventoryPlan},
    },
    support::candid::registry_entry_candid_path,
};

use candid::{CandidType, Principal};
use canic_core::{
    dto::{
        fleet_registry::{FleetRegistry, FleetRegistryManifest, FleetRegistryVersion},
        fleet_subnet_root::FleetSubnetRootCanisterSummary,
    },
    protocol,
};
use canic_host::{
    fleet_ensure::resolve_current_fleet, icp::IcpCli, icp_config::resolve_current_canic_icp_root,
    query_canister_with_arg, registry::RegistryEntry,
};

#[derive(CandidType)]
enum CoordinatorStatusRequestFragment {
    Registry,
    RegistryManifest,
    RegistryVersion,
}

#[derive(CandidType, serde::Deserialize)]
enum CoordinatorStatusResponseFragment {
    Registry(Box<FleetRegistry>),
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
    let current = resolve_current_fleet(&icp_root, &options.environment, &options.fleet)?;
    let expected = current.initial_active_registry(&options.fleet)?.clone();
    let icp = IcpCli::new(&options.icp, Some(options.environment.clone())).with_cwd(&icp_root);
    let coordinator = parse_principal(&current.topology.coordinator_canister_id)?;
    let coordinator_entry = exact_entry(&current.registry.entries, coordinator)?;
    let coordinator_binding =
        registry_entry_candid_path(Some(&icp_root), &options.environment, coordinator_entry)?;

    let CoordinatorStatusResponseFragment::Registry(registry) = query_canister_with_arg(
        &icp,
        &coordinator_binding,
        coordinator,
        protocol::CANIC_STATUS,
        &CoordinatorStatusRequestFragment::Registry,
    )?
    else {
        return Err(correlation_error());
    };
    let CoordinatorStatusResponseFragment::RegistryManifest(manifest) = query_canister_with_arg(
        &icp,
        &coordinator_binding,
        coordinator,
        protocol::CANIC_STATUS,
        &CoordinatorStatusRequestFragment::RegistryManifest,
    )?
    else {
        return Err(correlation_error());
    };
    let CoordinatorStatusResponseFragment::RegistryVersion(version) = query_canister_with_arg(
        &icp,
        &coordinator_binding,
        coordinator,
        protocol::CANIC_STATUS,
        &CoordinatorStatusRequestFragment::RegistryVersion,
    )?
    else {
        return Err(correlation_error());
    };
    let plan = SubnetInventoryPlan::compile(
        options.fleet.clone(),
        &expected,
        *registry,
        manifest,
        version,
    )?;
    let summaries = query_root_summaries(
        &icp,
        &icp_root,
        &options.environment,
        &current.registry.entries,
        plan.root_principals(),
    )?;
    plan.complete(summaries).map_err(Into::into)
}

fn query_root_summaries(
    icp: &IcpCli,
    icp_root: &std::path::Path,
    environment: &str,
    entries: &[RegistryEntry],
    roots: Vec<Principal>,
) -> Result<Vec<FleetSubnetRootCanisterSummary>, InfoSubnetsCommandError> {
    roots
        .into_iter()
        .map(|root| {
            let entry = exact_entry(entries, root)?;
            let binding = registry_entry_candid_path(Some(icp_root), environment, entry)?;
            let response: RootStatusResponseFragment = query_canister_with_arg(
                icp,
                &binding,
                root,
                protocol::CANIC_STATUS,
                &RootStatusRequestFragment::Inventory,
            )?;
            match response {
                RootStatusResponseFragment::Inventory(summary) => Ok(summary),
            }
        })
        .collect()
}

fn exact_entry(
    entries: &[RegistryEntry],
    principal: Principal,
) -> Result<&RegistryEntry, InfoSubnetsCommandError> {
    let principal = principal.to_text();
    let mut matches = entries.iter().filter(|entry| entry.pid == principal);
    let entry = matches.next().ok_or_else(|| {
        InfoSubnetsCommandError::Usage(format!(
            "terminal current inventory is missing canister {principal}"
        ))
    })?;
    if matches.next().is_some() {
        return Err(InfoSubnetsCommandError::Usage(format!(
            "terminal current inventory duplicates canister {principal}"
        )));
    }
    Ok(entry)
}

fn parse_principal(value: &str) -> Result<Principal, InfoSubnetsCommandError> {
    Principal::from_text(value).map_err(|_| {
        InfoSubnetsCommandError::Usage(format!(
            "terminal current inventory contains invalid Principal {value}"
        ))
    })
}

fn correlation_error() -> InfoSubnetsCommandError {
    InfoSubnetsCommandError::Usage(
        "Canister returned a differently correlated status response".to_string(),
    )
}
