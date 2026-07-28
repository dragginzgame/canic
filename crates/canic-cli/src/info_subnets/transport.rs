//! Module: canic_cli::info_subnets::transport
//!
//! Responsibility: collect exact Coordinator and Fleet Subnet Root inventory observations.
//! Does not own: report aggregation, authority policy, or rendering.
//! Boundary: validates Coordinator evidence before querying roots and returns no partial report.

use crate::info_subnets::{
    InfoSubnetsCommandError, InfoSubnetsOptions,
    model::{FleetSubnetInventoryReportV1, SubnetInventoryPlan},
};

use std::{path::Path, thread};

use candid::CandidType;
use canic_core::{
    dto::{
        fleet_registry::{FleetRegistry, FleetRegistryManifest, FleetRegistryVersion},
        fleet_subnet_root::FleetSubnetRootCanisterSummary,
    },
    protocol,
};
use canic_host::{
    icp::{IcpCli, decode_json_result_response},
    icp_config::resolve_current_canic_icp_root,
    installed_fleet::read_installed_fleet_from_root,
};
use serde::de::DeserializeOwned;

pub(super) fn load_report(
    options: &InfoSubnetsOptions,
) -> Result<FleetSubnetInventoryReportV1, InfoSubnetsCommandError> {
    let icp_root = resolve_current_canic_icp_root().map_err(InfoSubnetsCommandError::IcpRoot)?;
    let catalog = read_installed_fleet_from_root(&options.environment, &options.fleet, &icp_root)?;
    let icp = IcpCli::new(&options.icp, Some(options.environment.clone())).with_cwd(&icp_root);
    let coordinator = &catalog.coordinator_principal;
    let registry =
        query_result::<FleetRegistry>(&icp, coordinator, protocol::CANIC_FLEET_REGISTRY)?;
    let manifest = query_result::<FleetRegistryManifest>(
        &icp,
        coordinator,
        protocol::CANIC_FLEET_REGISTRY_MANIFEST,
    )?;
    let version = query_result::<FleetRegistryVersion>(
        &icp,
        coordinator,
        protocol::CANIC_FLEET_REGISTRY_VERSION,
    )?;
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
        let worker_canister = canister.clone();
        let icp = icp.clone();
        let handle = thread::spawn(move || {
            query_result::<FleetSubnetRootCanisterSummary>(
                &icp,
                &worker_canister,
                protocol::CANIC_FLEET_SUBNET_ROOT_CANISTER_SUMMARY,
            )
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

fn query_result<T>(
    icp: &IcpCli,
    canister: &str,
    method: &'static str,
) -> Result<T, InfoSubnetsCommandError>
where
    T: CandidType + DeserializeOwned,
{
    let output = icp
        .canister_query_output_with_candid(canister, method, Some("json"), None::<&Path>)
        .map_err(|source| InfoSubnetsCommandError::Query {
            canister: canister.to_string(),
            method,
            source,
        })?;
    decode_json_result_response(&output).map_err(|source| InfoSubnetsCommandError::Response {
        canister: canister.to_string(),
        method,
        source,
    })
}
