//! Module: canic_cli::info_subnets::transport
//!
//! Responsibility: collect exact Coordinator and Fleet Subnet Root inventory observations.
//! Does not own: report aggregation, terminal ensure authority, or rendering.
//! Boundary: validates terminal current authority before querying Roots and returns no partial
//! report.

use crate::info_subnets::{
    InfoSubnetsCommandError, InfoSubnetsOptions,
    model::{FleetSubnetInventoryReportV1, SubnetInventoryPlan},
};
use canic_contracts::dto::fleet_subnet_root::FleetSubnetRootCanisterSummary;
use canic_host::{
    fleet_ensure::resolve_current_fleet, icp::IcpCli, icp_config::resolve_current_canic_icp_root,
    registry::RegistryEntry,
};
use ic_platform::principal::ExternalPrincipal as Principal;

pub(super) fn load_report(
    options: &InfoSubnetsOptions,
) -> Result<FleetSubnetInventoryReportV1, InfoSubnetsCommandError> {
    let icp_root = resolve_current_canic_icp_root().map_err(InfoSubnetsCommandError::IcpRoot)?;
    let current = resolve_current_fleet(&icp_root, &options.environment, &options.fleet)?;
    let expected = current.initial_active_registry(&options.fleet)?.clone();
    let icp = IcpCli::new(&options.icp, Some(options.environment.clone())).with_cwd(&icp_root);
    let coordinator = parse_principal(&current.topology.coordinator_canister_id)?;
    let coordinator_entry = exact_entry(&current.registry.entries, coordinator)?;
    let schema = canic_host::protocol_binding::resolve_current_registry_wire_schema(
        &icp_root,
        &current,
        coordinator_entry,
    )?;
    let registry = canic_host::registry_discovery::snapshot(&icp, coordinator)?;
    let manifest = canic_host::registry_discovery::manifest(&icp, &schema, coordinator)?;
    let version = canic_host::registry_discovery::version(&icp, &schema, coordinator)?;
    let plan = SubnetInventoryPlan::compile(
        options.fleet.clone(),
        &expected,
        registry,
        manifest,
        version,
    )?;
    let summaries = query_root_summaries(&icp, &icp_root, &current, plan.root_principals())?;
    plan.complete(summaries).map_err(Into::into)
}

fn query_root_summaries(
    icp: &IcpCli,
    icp_root: &std::path::Path,
    current: &canic_host::fleet_ensure::CurrentFleetResolution,
    roots: Vec<Principal>,
) -> Result<Vec<FleetSubnetRootCanisterSummary>, InfoSubnetsCommandError> {
    roots
        .into_iter()
        .map(|root| {
            let entry = exact_entry(&current.registry.entries, root)?;
            canic_host::protocol_binding::resolve_current_registry_wire_schema(
                icp_root, current, entry,
            )?;
            Ok(canic_host::registry_discovery::root_inventory(icp, root)?)
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
