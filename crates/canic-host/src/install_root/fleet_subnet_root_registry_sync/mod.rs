//! Module: install_root::fleet_subnet_root_registry_sync
//!
//! Responsibility: drive and independently verify every root's all-Joining snapshot acknowledgement.
//! Does not own: Registry `Active`, Directory activation, runtime activation, or Fleet publication.
//! Boundary: each root journal records intent before the root performs its inter-canister calls.

use super::{
    fleet_subnet_root_install_journal::{
        FleetSubnetRootInstallPhase, PlanFleetSubnetRootInstallRequest,
        ResolvedFleetSubnetRootInstall, begin_registry_sync, expected_registry_join_entry,
        plan_fleet_subnet_root_install, record_registry_sync_verified,
        record_registry_synchronized,
    },
    fleet_subnet_root_store_bootstrap::canonical_manifest_bytes,
    icp_context::InstallIcpContext,
    operations::{
        call_with_arg, fleet_registry_authority, query_live_registry, query_with_arg,
        resolve_install_protocol_binding,
    },
};
use crate::{
    fleet_install_plan::PersistedFleetInstallPlan,
    protocol_binding::resolve_infrastructure_protocol_binding,
    release_set::{
        AppConfigSnapshot, CanicInfrastructureRole,
        load_persisted_canic_infrastructure_artifact_manifest,
    },
};
use candid::{CandidType, Principal};
use canic_control_plane::dto::fleet_coordinator::{
    CoordinatorStatusRequest, CoordinatorStatusResponse,
};
use canic_control_plane::dto::root::RootOperationStatusResponse;
use canic_core::{
    control_plane_support::ops::fleet_registry::FleetRegistryOps,
    dto::fleet_registry::{
        FleetRegistryVersion, FleetSubnetRootRegistrySyncRequest,
        FleetSubnetRootSnapshotAcknowledgement,
    },
    dto::role::{OperationReceipt, OperationStatusRequest},
    dto::root_store::RootStoreBootstrapRequest,
    protocol,
};
use serde::Deserialize;
use std::path::Path;
use thiserror::Error as ThisError;

const MAX_SYNC_TRANSITIONS: usize = 4;

#[derive(CandidType)]
enum RootCommandFragment {
    SynchronizeRegistry(FleetSubnetRootRegistrySyncRequest),
}

#[derive(CandidType, Deserialize)]
enum RootCommandResponseFragment {
    OperationAccepted(OperationReceipt),
}

#[derive(CandidType)]
enum RootStatusRequestFragment {
    Operation(OperationStatusRequest),
}

#[derive(CandidType, Deserialize)]
enum RootStatusResponseFragment {
    Operation(RootOperationStatusResponse),
}

#[derive(Debug, ThisError)]
enum RootRegistrySyncError {
    #[error("root Registry synchronization reached unexpected phase {0:?}")]
    UnexpectedPhase(FleetSubnetRootInstallPhase),

    #[error("root release-set manifest is missing for planned Subnet")]
    MissingReleaseSet,

    #[error("root Registry synchronization exceeded its bounded phase transitions")]
    TransitionBoundExceeded,

    #[error("Coordinator acknowledgement set differs from the complete planned root set")]
    AcknowledgementSetMismatch,
}

pub(super) struct SynchronizeFleetSubnetRootsRequest<'a> {
    pub icp: &'a InstallIcpContext,
    pub config_path: &'a Path,
    pub fleet_install_plan: &'a PersistedFleetInstallPlan,
    pub coordinator: Principal,
    pub install_operation_id: [u8; 32],
    pub joining_version: FleetRegistryVersion,
}

pub(super) fn synchronize_and_verify_fleet_subnet_roots(
    request: SynchronizeFleetSubnetRootsRequest<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let SynchronizeFleetSubnetRootsRequest {
        icp,
        config_path,
        fleet_install_plan,
        coordinator,
        install_operation_id,
        joining_version,
    } = request;
    let config = AppConfigSnapshot::load(config_path)?;
    let component_topology = config.model().compile_component_topology()?;
    let infrastructure_manifest = load_persisted_canic_infrastructure_artifact_manifest(
        icp.root(),
        fleet_install_plan.plan.release_build_id,
    )?;
    let authority = fleet_registry_authority(fleet_install_plan, coordinator);
    let coordinator_binding = resolve_install_protocol_binding(
        icp,
        &infrastructure_manifest,
        CanicInfrastructureRole::FleetCoordinator,
    )?;
    let mut joining_registry = FleetRegistryOps::compile_genesis(
        &fleet_install_plan.plan.fleet.app,
        authority.clone(),
        &component_topology,
    )?;
    let mut expected = Vec::with_capacity(fleet_install_plan.plan.fleet_subnet_roots.len());

    for root_plan in &fleet_install_plan.plan.fleet_subnet_roots {
        let release_set = fleet_install_plan
            .root_release_sets
            .iter()
            .find(|release_set| release_set.placement_subnet == root_plan.placement_subnet)
            .ok_or(RootRegistrySyncError::MissingReleaseSet)?;
        let request = FleetSubnetRootRegistrySyncRequest {
            operation_id: super::root_registry_synchronization_operation_id(install_operation_id),
            expected_registry: joining_version.clone(),
            store_bootstrap: RootStoreBootstrapRequest {
                operation_id: super::root_store_bootstrap_operation_id(install_operation_id),
                manifest_payload_size_bytes: canonical_manifest_bytes(release_set)?.len() as u64,
            },
        };
        let current = plan_fleet_subnet_root_install(PlanFleetSubnetRootInstallRequest {
            fleet_install_plan,
            infrastructure_manifest: &infrastructure_manifest,
            coordinator,
            install_operation_id,
            component_topology: component_topology.clone(),
            root_plan,
        })?;
        expected.push(
            current
                .journal
                .fleet_subnet_root
                .ok_or(RootRegistrySyncError::AcknowledgementSetMismatch)?,
        );
        joining_registry = FleetRegistryOps::compile_joining(
            &authority,
            &component_topology,
            &joining_registry,
            expected_registry_join_entry(&current.journal)?,
        )?;
        drive_root_sync(icp, current, request)?;
    }
    let expected_joining_version =
        FleetRegistryOps::version(&authority, &component_topology, &joining_registry)?;
    if expected_joining_version != joining_version {
        return Err(RootRegistrySyncError::AcknowledgementSetMismatch.into());
    }

    let coordinator_icp = icp.cli();
    let live = query_with_arg::<_, CoordinatorStatusResponse>(
        coordinator_icp,
        &coordinator_binding,
        coordinator,
        protocol::CANIC_STATUS,
        &CoordinatorStatusRequest::RootAcknowledgements,
    )?;
    let CoordinatorStatusResponse::RootAcknowledgements(live) = live else {
        return Err(RootRegistrySyncError::AcknowledgementSetMismatch.into());
    };
    if root_acknowledgements_match(&live, expected, &joining_version) {
        return Ok(());
    }
    let active_registry =
        FleetRegistryOps::compile_active(&authority, &component_topology, &joining_registry)?;
    let live_registry = query_live_registry(coordinator_icp, &coordinator_binding, coordinator)?;
    let expected_manifest =
        FleetRegistryOps::manifest(&authority, &component_topology, &active_registry)?;
    let expected_version =
        FleetRegistryOps::version(&authority, &component_topology, &active_registry)?;
    if live_registry.registry != active_registry
        || live_registry.manifest != expected_manifest
        || live_registry.version != expected_version
        || !live.is_empty()
    {
        return Err(RootRegistrySyncError::AcknowledgementSetMismatch.into());
    }
    Ok(())
}

fn root_acknowledgements_match(
    live: &[FleetSubnetRootSnapshotAcknowledgement],
    mut expected: Vec<Principal>,
    joining_version: &FleetRegistryVersion,
) -> bool {
    expected.sort_by(|left, right| left.as_slice().cmp(right.as_slice()));
    live.len() == expected.len()
        && live
            .iter()
            .zip(expected)
            .all(|(ack, root)| ack.fleet_subnet_root == root && &ack.version == joining_version)
}

fn drive_root_sync(
    icp_context: &InstallIcpContext,
    mut current: ResolvedFleetSubnetRootInstall,
    request: FleetSubnetRootRegistrySyncRequest,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = current
        .journal
        .fleet_subnet_root
        .expect("Registry synchronization follows root verification");
    let binding = resolve_infrastructure_protocol_binding(
        icp_context.root(),
        icp_context.environment(),
        &current.journal.root_artifact,
    )?;
    let icp = icp_context.cli();
    for _ in 0..MAX_SYNC_TRANSITIONS {
        current = match current.journal.phase {
            FleetSubnetRootInstallPhase::RegistryJoinVerified => {
                begin_registry_sync(&current, request.clone())?
            }
            FleetSubnetRootInstallPhase::RegistrySyncInFlight => {
                let response: RootCommandResponseFragment = call_with_arg(
                    icp,
                    &binding,
                    root,
                    protocol::CANIC_COMMAND,
                    &RootCommandFragment::SynchronizeRegistry(request.clone()),
                )?;
                let RootCommandResponseFragment::OperationAccepted(receipt) = response;
                if receipt.operation_id != request.operation_id {
                    return Err(RootRegistrySyncError::AcknowledgementSetMismatch.into());
                }
                let response =
                    query_root_registry_synchronization(icp, &binding, root, request.operation_id)?;
                record_registry_synchronized(&current, response.synchronization)?
            }
            FleetSubnetRootInstallPhase::RegistrySynchronized => {
                let response =
                    query_root_registry_synchronization(icp, &binding, root, request.operation_id)?;
                record_registry_sync_verified(&current, response.synchronization)?
            }
            FleetSubnetRootInstallPhase::RegistrySyncVerified
            | FleetSubnetRootInstallPhase::RegistryMirrorActivationInFlight
            | FleetSubnetRootInstallPhase::RegistryMirrorActivated
            | FleetSubnetRootInstallPhase::RegistryMirrorActivationVerified
            | FleetSubnetRootInstallPhase::ComponentRegistryPreparationInFlight
            | FleetSubnetRootInstallPhase::ComponentRegistryPrepared
            | FleetSubnetRootInstallPhase::ComponentRegistryPreparationVerified => return Ok(()),
            phase => return Err(RootRegistrySyncError::UnexpectedPhase(phase).into()),
        };
    }
    Err(RootRegistrySyncError::TransitionBoundExceeded.into())
}

fn query_root_registry_synchronization(
    icp: &crate::icp::IcpCli,
    binding: &crate::protocol_binding::ResolvedProtocolBinding,
    root: Principal,
    operation_id: [u8; 32],
) -> Result<
    canic_control_plane::dto::root::RootRegistrySynchronizationOperationStatus,
    Box<dyn std::error::Error>,
> {
    let response: RootStatusResponseFragment = query_with_arg(
        icp,
        binding,
        root,
        protocol::CANIC_STATUS,
        &RootStatusRequestFragment::Operation(OperationStatusRequest { operation_id }),
    )?;
    match response {
        RootStatusResponseFragment::Operation(
            RootOperationStatusResponse::SynchronizeRegistry(response),
        ) => Ok(response),
        RootStatusResponseFragment::Operation(_) => {
            Err(RootRegistrySyncError::AcknowledgementSetMismatch.into())
        }
    }
}
