//! Module: install_root::fleet_subnet_root_registry_mirror_activation
//!
//! Responsibility: atomically activate and independently verify every root mirror/Directory pair.
//! Does not own: Coordinator Registry mutation, Component runtime activation, or Fleet publication.
//! Boundary: each root journal records exact active authority before the root performs any call.

use super::{
    fleet_subnet_root_install_journal::{
        FleetSubnetRootInstallPhase, PlanFleetSubnetRootInstallRequest,
        ResolvedFleetSubnetRootInstall, begin_registry_mirror_activation,
        plan_fleet_subnet_root_install, record_registry_mirror_activated,
        record_registry_mirror_activation_verified,
    },
    fleet_subnet_root_store_bootstrap::canonical_manifest_bytes,
    icp_context::InstallIcpContext,
    operations::query_with_arg,
};
use crate::{
    fleet_install_plan::PersistedFleetInstallPlan,
    protocol_binding::resolve_infrastructure_protocol_binding,
    release_set::{AppConfigSnapshot, load_persisted_canic_infrastructure_artifact_manifest},
};
use candid::{CandidType, Principal};
use canic_control_plane::dto::root::RootOperationStatusResponse;
use canic_core::{
    control_plane_support::ops::fleet_registry::FleetRegistryOps,
    dto::{
        fleet_registry::{
            FleetRegistry, FleetRegistryVersion, FleetSubnetRootRegistryMirrorActivationRequest,
        },
        role::OperationStatusRequest,
        root_store::RootStoreBootstrapRequest,
    },
    protocol,
};
use serde::Deserialize;
use std::path::Path;
use thiserror::Error as ThisError;

const MAX_MIRROR_ACTIVATION_STALLED_POLLS: usize = 4;

#[derive(CandidType)]
enum RootStatusRequestFragment {
    Operation(OperationStatusRequest),
}

#[derive(CandidType, Deserialize)]
enum RootStatusResponseFragment {
    Operation(RootOperationStatusResponse),
}

#[derive(Debug, ThisError)]
enum RootRegistryMirrorActivationError {
    #[error("root Registry mirror activation reached unexpected phase {0:?}")]
    UnexpectedPhase(FleetSubnetRootInstallPhase),

    #[error("root release-set manifest is missing for planned Subnet")]
    MissingReleaseSet,

    #[error("active Registry differs from its verified Coordinator version")]
    ActiveRegistryVersionMismatch,

    #[error("root Registry mirror activation exceeded four consecutive no-progress polls")]
    StalledPollBoundExceeded,

    #[error("live root Registry mirror/Directory differs from durable activation evidence")]
    LiveEvidenceMismatch,
}

pub(super) struct ActivateFleetSubnetRootRegistryMirrorsRequest<'a> {
    pub icp: &'a InstallIcpContext,
    pub config_path: &'a Path,
    pub fleet_install_plan: &'a PersistedFleetInstallPlan,
    pub coordinator: Principal,
    pub install_operation_id: [u8; 32],
    pub joining_version: FleetRegistryVersion,
    pub active_registry: &'a FleetRegistry,
    pub active_version: FleetRegistryVersion,
}

pub(super) fn activate_and_verify_fleet_subnet_root_registry_mirrors(
    request: ActivateFleetSubnetRootRegistryMirrorsRequest<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfigSnapshot::load(request.config_path)?;
    let component_topology = config.model().compile_component_topology()?;
    let infrastructure_manifest = load_persisted_canic_infrastructure_artifact_manifest(
        request.icp.root(),
        request.fleet_install_plan.plan.release_build_id,
    )?;
    let observed_version = FleetRegistryOps::version(
        &request.active_registry.authority,
        &component_topology,
        request.active_registry,
    )?;
    if observed_version != request.active_version {
        return Err(RootRegistryMirrorActivationError::ActiveRegistryVersionMismatch.into());
    }

    for root_plan in &request.fleet_install_plan.plan.fleet_subnet_roots {
        let release_set = request
            .fleet_install_plan
            .root_release_sets
            .iter()
            .find(|release_set| release_set.placement_subnet == root_plan.placement_subnet)
            .ok_or(RootRegistryMirrorActivationError::MissingReleaseSet)?;
        let current = plan_fleet_subnet_root_install(PlanFleetSubnetRootInstallRequest {
            fleet_install_plan: request.fleet_install_plan,
            infrastructure_manifest: &infrastructure_manifest,
            coordinator: request.coordinator,
            install_operation_id: request.install_operation_id,
            component_topology: component_topology.clone(),
            root_plan,
        })?;
        let root = current
            .journal
            .fleet_subnet_root
            .ok_or(RootRegistryMirrorActivationError::LiveEvidenceMismatch)?;
        let directory = FleetRegistryOps::directory_for_root(
            &current.journal.authority,
            &component_topology,
            request.active_registry,
            root,
        )?;
        let activation_request = FleetSubnetRootRegistryMirrorActivationRequest {
            previous_registry: request.joining_version.clone(),
            expected_registry: request.active_version.clone(),
            expected_directory: directory,
            store_bootstrap: RootStoreBootstrapRequest {
                operation_id: super::root_store_bootstrap_operation_id(
                    request.install_operation_id,
                ),
                manifest_payload_size_bytes: canonical_manifest_bytes(release_set)?.len() as u64,
            },
        };
        drive_root_mirror_activation(request.icp, current, activation_request)?;
    }
    Ok(())
}

pub(super) fn drive_root_mirror_activation(
    icp_context: &InstallIcpContext,
    current: ResolvedFleetSubnetRootInstall,
    request: FleetSubnetRootRegistryMirrorActivationRequest,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = current
        .journal
        .fleet_subnet_root
        .ok_or(RootRegistryMirrorActivationError::LiveEvidenceMismatch)?;
    let binding = resolve_infrastructure_protocol_binding(
        icp_context.root(),
        icp_context.environment(),
        &current.journal.root_artifact,
    )?;
    let icp = icp_context.cli();
    let operation_id =
        super::root_registry_synchronization_operation_id(current.journal.install_operation_id);
    drive_root_mirror_activation_with_query(current, request, || {
        query_registry_synchronization(icp, &binding, root, operation_id)
            .map(|response| response.activation)
    })
}

fn drive_root_mirror_activation_with_query(
    mut current: ResolvedFleetSubnetRootInstall,
    request: FleetSubnetRootRegistryMirrorActivationRequest,
    mut query_activation: impl FnMut() -> Result<
        Option<canic_core::dto::fleet_registry::FleetSubnetRootRegistryMirrorActivationResponse>,
        Box<dyn std::error::Error>,
    >,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stalled_polls = 0_usize;
    loop {
        current = match current.journal.phase {
            FleetSubnetRootInstallPhase::RegistrySyncVerified => {
                begin_registry_mirror_activation(&current, request.clone())?
            }
            FleetSubnetRootInstallPhase::RegistryMirrorActivationInFlight => {
                let Some(activation) = query_activation()? else {
                    stalled_polls = stalled_polls
                        .checked_add(1)
                        .ok_or(RootRegistryMirrorActivationError::StalledPollBoundExceeded)?;
                    if stalled_polls > MAX_MIRROR_ACTIVATION_STALLED_POLLS {
                        return Err(
                            RootRegistryMirrorActivationError::StalledPollBoundExceeded.into()
                        );
                    }
                    continue;
                };
                stalled_polls = 0;
                record_registry_mirror_activated(&current, activation)?
            }
            FleetSubnetRootInstallPhase::RegistryMirrorActivated => {
                let activation = query_activation()?
                    .ok_or(RootRegistryMirrorActivationError::LiveEvidenceMismatch)?;
                stalled_polls = 0;
                record_registry_mirror_activation_verified(&current, activation)?
            }
            FleetSubnetRootInstallPhase::RegistryMirrorActivationVerified
            | FleetSubnetRootInstallPhase::ComponentRegistryPreparationInFlight
            | FleetSubnetRootInstallPhase::ComponentRegistryPrepared
            | FleetSubnetRootInstallPhase::ComponentRegistryPreparationVerified => return Ok(()),
            phase => {
                return Err(RootRegistryMirrorActivationError::UnexpectedPhase(phase).into());
            }
        };
    }
}

#[cfg(test)]
pub(super) fn drive_root_mirror_activation_with_observations(
    current: ResolvedFleetSubnetRootInstall,
    request: FleetSubnetRootRegistryMirrorActivationRequest,
    observations: impl IntoIterator<
        Item = Option<
            canic_core::dto::fleet_registry::FleetSubnetRootRegistryMirrorActivationResponse,
        >,
    >,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut observations = observations.into_iter();
    drive_root_mirror_activation_with_query(current, request, || {
        observations
            .next()
            .ok_or_else(|| RootRegistryMirrorActivationError::StalledPollBoundExceeded.into())
    })
}

fn query_registry_synchronization(
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
            Err(RootRegistryMirrorActivationError::LiveEvidenceMismatch.into())
        }
    }
}
