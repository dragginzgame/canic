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
use crate::{
    canister_protocol::{CanisterProtocolError, query_no_arg},
    fleet_catalog::CommittedFleetCatalog,
    fleet_install_plan::PersistedFleetInstallPlan,
    icp::{IcpCli, LocalReplicaTarget},
    release_set::AppConfigSnapshot,
};
use candid::Principal;
use canic_core::{
    dto::{
        fleet_registry::{
            FleetRegistry, FleetRegistryManifest, FleetRegistrySnapshotResponse,
            FleetRegistryVersion,
        },
        fleet_subnet_root::FleetSubnetRootCanisterSummary,
    },
    ids::FleetName,
    protocol,
};
use std::{
    path::Path,
    thread,
    time::{SystemTime, SystemTimeError, UNIX_EPOCH},
};
use thiserror::Error as ThisError;

///
/// PublishInstalledFleetCatalogRequest
///
/// Exact local and live authority needed to close one fresh Fleet installation.
///

pub(super) struct PublishInstalledFleetCatalogRequest<'a> {
    pub icp_root: &'a Path,
    pub environment: &'a str,
    pub local_replica: Option<&'a LocalReplicaTarget>,
    pub config_path: &'a Path,
    pub fleet_name: FleetName,
    pub fleet_install_plan: &'a PersistedFleetInstallPlan,
    pub coordinator: Principal,
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

    #[error("system clock is earlier than the Unix epoch: {0}")]
    SystemClock(#[source] SystemTimeError),
}

/// Re-read terminal Coordinator/root authority and publish one durable Fleet discovery row.
pub(super) fn publish_installed_fleet_catalog(
    request: PublishInstalledFleetCatalogRequest<'_>,
) -> Result<CommittedFleetCatalog, Box<dyn std::error::Error>> {
    let config = AppConfigSnapshot::load(request.config_path)?;
    let component_topology = config.model().compile_component_topology()?;
    let icp = IcpCli::new("icp", Some(request.environment.to_string()))
        .with_cwd(request.icp_root)
        .with_local_replica(request.local_replica.cloned());
    let registry = query_registry(&icp, request.coordinator)?;

    validate_terminal_fleet_registry(
        &request.fleet_install_plan.plan,
        &component_topology,
        request.coordinator,
        &registry,
    )?;
    let root_summaries = query_root_summaries(&icp, &registry)?;
    let deployed_at_unix_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(FleetCatalogCloseoutError::SystemClock)?
        .as_secs();

    publish_terminal_fleet_catalog(TerminalFleetCatalogPublicationRequest {
        workspace_root: request.icp_root,
        fleet_name: request.fleet_name,
        environment: request.environment,
        deployed_at_unix_secs,
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
    Ok(FleetRegistrySnapshotResponse {
        registry: query_no_arg::<FleetRegistry>(icp, coordinator, protocol::CANIC_FLEET_REGISTRY)?,
        manifest: query_no_arg::<FleetRegistryManifest>(
            icp,
            coordinator,
            protocol::CANIC_FLEET_REGISTRY_MANIFEST,
        )?,
        version: query_no_arg::<FleetRegistryVersion>(
            icp,
            coordinator,
            protocol::CANIC_FLEET_REGISTRY_VERSION,
        )?,
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
            query_no_arg::<FleetSubnetRootCanisterSummary>(
                &worker_icp,
                root,
                protocol::CANIC_FLEET_SUBNET_ROOT_CANISTER_SUMMARY,
            )
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
