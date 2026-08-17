//! Module: install_root::fleet_catalog_closeout
//!
//! Responsibility: collect complete live terminal Fleet evidence and invoke catalog publication.
//! Does not own: root activation, Registry mutation, or catalog persistence mechanics.
//! Boundary: Coordinator authority is validated before its roots become query targets, and no
//! catalog state is published until every root summary has been collected.

use super::fleet_catalog_publication::{
    TerminalFleetCatalogPublicationRequest, publish_terminal_fleet_catalog,
    validate_terminal_fleet_registry,
};
use super::icp_context::InstallIcpContext;
use crate::{
    canister_protocol::{CanisterProtocolError, query_with_arg},
    fleet_catalog::CommittedFleetCatalog,
    fleet_install_plan::PersistedFleetInstallPlan,
    icp::IcpCli,
    release_set::AppConfigSnapshot,
};
use std::{path::Path, thread};

use candid::{CandidType, Principal};
use canic_control_plane::dto::fleet_coordinator::{
    CoordinatorStatusRequest, CoordinatorStatusResponse,
};
use canic_core::{
    dto::{
        fleet_registry::{FleetRegistrySnapshotResponse, FleetRegistryVersion},
        fleet_subnet_root::FleetSubnetRootCanisterSummary,
    },
    ids::FleetName,
    protocol,
};
use serde::Deserialize;
use thiserror::Error as ThisError;

#[derive(CandidType)]
enum RootStatusRequestFragment {
    Inventory,
}

#[derive(CandidType, Deserialize)]
enum RootStatusResponseFragment {
    Inventory(FleetSubnetRootCanisterSummary),
}

///
/// PublishInstalledFleetCatalogRequest
///
/// Exact local and live authority needed to close one fresh Fleet installation.
///

pub(super) struct PublishInstalledFleetCatalogRequest<'a> {
    pub icp: &'a InstallIcpContext,
    pub config_path: &'a Path,
    pub fleet_name: FleetName,
    pub fleet_install_plan: &'a PersistedFleetInstallPlan,
    pub coordinator: Principal,
    pub deployed_at_unix_secs: u64,
    pub terminal_fleet_registry: &'a FleetRegistryVersion,
}

///
/// FleetCatalogCloseoutError
///
/// Typed live-evidence or clock failure before terminal catalog publication.
///

#[derive(Debug, ThisError)]
enum FleetCatalogCloseoutError {
    #[error(transparent)]
    Protocol(#[from] CanisterProtocolError),

    #[error("terminal summary worker for Fleet Subnet Root {root} panicked")]
    SummaryWorkerPanicked { root: Principal },

    #[error("live Fleet Registry differs from terminal Component provisioning evidence")]
    TerminalRegistryMismatch,

    #[error("Coordinator returned an unrelated {expected} status response")]
    UnexpectedCoordinatorStatus { expected: &'static str },
}

/// Re-read terminal Coordinator/root authority and publish one durable Fleet discovery row.
pub(super) fn publish_installed_fleet_catalog(
    request: PublishInstalledFleetCatalogRequest<'_>,
) -> Result<CommittedFleetCatalog, Box<dyn std::error::Error>> {
    let config = AppConfigSnapshot::load(request.config_path)?;
    let component_topology = config.model().compile_component_topology()?;
    let icp = request.icp.cli();
    let registry = query_registry(icp, request.coordinator)?;

    validate_terminal_fleet_registry(
        &request.fleet_install_plan.plan,
        &component_topology,
        request.coordinator,
        &registry,
    )?;
    if &registry.version != request.terminal_fleet_registry {
        return Err(FleetCatalogCloseoutError::TerminalRegistryMismatch.into());
    }
    let root_summaries = query_root_summaries(icp, &registry)?;

    publish_terminal_fleet_catalog(TerminalFleetCatalogPublicationRequest {
        workspace_root: request.icp.root(),
        fleet_name: request.fleet_name,
        environment: request.icp.environment(),
        deployed_at_unix_secs: request.deployed_at_unix_secs,
        fleet_install_plan: &request.fleet_install_plan.plan,
        component_topology: &component_topology,
        coordinator: request.coordinator,
        registry: &registry,
        root_summaries: &root_summaries,
    })
    .map_err(Into::into)
}

fn query_registry(
    icp: &IcpCli,
    coordinator: Principal,
) -> Result<FleetRegistrySnapshotResponse, FleetCatalogCloseoutError> {
    let registry = query_with_arg::<_, CoordinatorStatusResponse>(
        icp,
        coordinator,
        protocol::CANIC_STATUS,
        &CoordinatorStatusRequest::Registry,
    )?;
    let manifest = query_with_arg::<_, CoordinatorStatusResponse>(
        icp,
        coordinator,
        protocol::CANIC_STATUS,
        &CoordinatorStatusRequest::RegistryManifest,
    )?;
    let version = query_with_arg::<_, CoordinatorStatusResponse>(
        icp,
        coordinator,
        protocol::CANIC_STATUS,
        &CoordinatorStatusRequest::RegistryVersion,
    )?;
    Ok(FleetRegistrySnapshotResponse {
        registry: match registry {
            CoordinatorStatusResponse::Registry(registry) => registry,
            _ => return Err(protocol_mismatch("Registry")),
        },
        manifest: match manifest {
            CoordinatorStatusResponse::RegistryManifest(manifest) => manifest,
            _ => return Err(protocol_mismatch("RegistryManifest")),
        },
        version: match version {
            CoordinatorStatusResponse::RegistryVersion(version) => version,
            _ => return Err(protocol_mismatch("RegistryVersion")),
        },
    })
}

fn query_root_summaries(
    icp: &IcpCli,
    registry: &FleetRegistrySnapshotResponse,
) -> Result<Vec<FleetSubnetRootCanisterSummary>, FleetCatalogCloseoutError> {
    let mut workers = Vec::with_capacity(registry.registry.fleet_subnet_roots.len());
    for registered in &registry.registry.fleet_subnet_roots {
        let root = registered.fleet_subnet_root;
        let worker_icp = icp.clone();
        let worker = thread::spawn(move || {
            query_with_arg::<_, RootStatusResponseFragment>(
                &worker_icp,
                root,
                protocol::CANIC_STATUS,
                &RootStatusRequestFragment::Inventory,
            )
            .map(|response| match response {
                RootStatusResponseFragment::Inventory(summary) => summary,
            })
        });
        workers.push((root, worker));
    }

    workers
        .into_iter()
        .map(|(root, worker)| {
            worker
                .join()
                .map_err(|_| FleetCatalogCloseoutError::SummaryWorkerPanicked { root })?
                .map_err(Into::into)
        })
        .collect()
}

const fn protocol_mismatch(expected: &'static str) -> FleetCatalogCloseoutError {
    FleetCatalogCloseoutError::UnexpectedCoordinatorStatus { expected }
}
