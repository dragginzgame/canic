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
    operations::{call_with_arg, query_with_arg},
};
use crate::{
    fleet_install_plan::PersistedFleetInstallPlan,
    icp::LocalReplicaTarget,
    release_set::{AppConfigSnapshot, load_persisted_canic_infrastructure_artifact_manifest},
};
use candid::Principal;
use canic_core::{
    control_plane_support::ops::fleet_registry::FleetRegistryOps,
    dto::{
        fleet_registry::{
            FleetRegistry, FleetRegistryVersion, FleetSubnetRootRegistryMirrorActivationRequest,
        },
        root_store::RootStoreBootstrapRequest,
    },
    protocol,
};
use std::path::Path;
use thiserror::Error as ThisError;

const MAX_MIRROR_ACTIVATION_TRANSITIONS: usize = 4;

#[derive(Debug, ThisError)]
enum RootRegistryMirrorActivationError {
    #[error("root Registry mirror activation reached unexpected phase {0:?}")]
    UnexpectedPhase(FleetSubnetRootInstallPhase),

    #[error("root release-set manifest is missing for planned Subnet")]
    MissingReleaseSet,

    #[error("active Registry differs from its verified Coordinator version")]
    ActiveRegistryVersionMismatch,

    #[error("root Registry mirror activation exceeded its bounded phase transitions")]
    TransitionBoundExceeded,

    #[error("live root Registry mirror/Directory differs from durable activation evidence")]
    LiveEvidenceMismatch,
}

pub(super) struct ActivateFleetSubnetRootRegistryMirrorsRequest<'a> {
    pub icp_root: &'a Path,
    pub environment: &'a str,
    pub local_replica: Option<&'a LocalReplicaTarget>,
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
        request.icp_root,
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
                manifest_payload_size_bytes: canonical_manifest_bytes(release_set)?.len() as u64,
            },
        };
        drive_root_mirror_activation(
            request.icp_root,
            request.environment,
            request.local_replica,
            current,
            activation_request,
        )?;
    }
    Ok(())
}

fn drive_root_mirror_activation(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    mut current: ResolvedFleetSubnetRootInstall,
    request: FleetSubnetRootRegistryMirrorActivationRequest,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = current
        .journal
        .fleet_subnet_root
        .ok_or(RootRegistryMirrorActivationError::LiveEvidenceMismatch)?;
    let icp = super::install_icp(icp_root, environment, local_replica);
    for _ in 0..MAX_MIRROR_ACTIVATION_TRANSITIONS {
        current = match current.journal.phase {
            FleetSubnetRootInstallPhase::RegistrySyncVerified => {
                begin_registry_mirror_activation(&current, request.clone())?
            }
            FleetSubnetRootInstallPhase::RegistryMirrorActivationInFlight => {
                let response = call_with_arg(
                    &icp,
                    root,
                    protocol::CANIC_FLEET_REGISTRY_ACTIVATE_MIRROR,
                    &request,
                )?;
                record_registry_mirror_activated(&current, response)?
            }
            FleetSubnetRootInstallPhase::RegistryMirrorActivated => {
                let response = query_with_arg(
                    &icp,
                    root,
                    protocol::CANIC_FLEET_REGISTRY_MIRROR_STATUS,
                    &request,
                )?;
                record_registry_mirror_activation_verified(&current, response)?
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
    Err(RootRegistryMirrorActivationError::TransitionBoundExceeded.into())
}
