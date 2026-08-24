//! Module: install_root::fleet_subnet_root_registry_join
//!
//! Responsibility: register every Store-verified root as Fleet Registry `Joining`.
//! Does not own: root snapshot synchronization, acknowledgement, `Active`, or runtime activation.
//! Boundary: each compare-and-commit request and response is journalled, then the complete
//! Coordinator snapshot, manifest, and version are independently reproduced from the Fleet plan.

use super::fleet_component_provisioning_plan::{
    CompileFleetComponentProvisioningPlanRequest, compile_fleet_component_provisioning_plan,
};
use super::fleet_registry_recovery::{
    ActiveRegistryRecoveryRequest, JoiningRegistryRecoveryRequest,
    require_joining_or_recovered_registry,
};
use super::fleet_subnet_root_install_journal::{
    FleetSubnetRootInstallPhase, PlanFleetSubnetRootInstallRequest, ResolvedFleetSubnetRootInstall,
    begin_registry_join, expected_registry_join_entry, plan_fleet_subnet_root_install,
    record_registry_join_verified, record_registry_joined,
};
use super::icp_context::InstallIcpContext;
use super::operations::{
    LiveRegistryEvidence, call_with_arg, query_live_registry, resolve_install_protocol_binding,
};
use crate::{
    fleet_install_plan::PersistedFleetInstallPlan,
    release_set::{
        AppConfigSnapshot, CanicInfrastructureRole,
        load_persisted_canic_infrastructure_artifact_manifest,
    },
};
use candid::Principal;
use canic_control_plane::dto::fleet_coordinator::{CoordinatorCommand, CoordinatorCommandResponse};
use canic_core::{
    control_plane_support::{config::ComponentTopology, ops::fleet_registry::FleetRegistryOps},
    dto::fleet_registry::{FleetRegistry, FleetRegistryVersion},
    ids::{FleetCoordinatorBinding, FleetRegistryAuthority},
    protocol,
};
use std::path::Path;
use thiserror::Error as ThisError;

const MAX_REGISTRY_JOIN_TRANSITIONS: usize = 4;

#[derive(Debug, ThisError)]
enum RootRegistryJoinError {
    #[error("root Registry join reached phase {0:?} before Store verification")]
    StoreNotVerified(FleetSubnetRootInstallPhase),

    #[error("live Fleet Registry differs from the exact planned {0} snapshot")]
    LiveRegistryMismatch(&'static str),

    #[error("root Registry join journal is missing its durable request")]
    MissingJoinRequest,

    #[error("root Registry join response differs from the exact planned snapshot")]
    JoinResponseMismatch,

    #[error("root Registry join workflow exceeded its bounded phase transitions")]
    TransitionBoundExceeded,
}

pub(super) fn register_and_verify_fleet_subnet_roots_joining(
    icp_context: &InstallIcpContext,
    config_path: &Path,
    fleet_install_plan: &PersistedFleetInstallPlan,
    coordinator: Principal,
    install_operation_id: [u8; 32],
) -> Result<FleetRegistryVersion, Box<dyn std::error::Error>> {
    let config = AppConfigSnapshot::load(config_path)?;
    let component_topology = config.model().compile_component_topology()?;
    let infrastructure_manifest = load_persisted_canic_infrastructure_artifact_manifest(
        icp_context.root(),
        fleet_install_plan.plan.release_build_id,
    )?;
    let authority = FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: fleet_install_plan.plan.fleet.clone(),
            coordinator_subnet: fleet_install_plan.plan.coordinator.coordinator_subnet,
            coordinator,
        },
        epoch: 1,
    };
    let protocol_binding = resolve_install_protocol_binding(
        icp_context,
        &infrastructure_manifest,
        CanicInfrastructureRole::FleetCoordinator,
    )?;
    let mut expected_registry = FleetRegistryOps::compile_genesis(
        &fleet_install_plan.plan.fleet.app,
        authority.clone(),
        &component_topology,
        fleet_install_plan.plan.admission.clone(),
    )?;

    for root_plan in &fleet_install_plan.plan.fleet_subnet_roots {
        let current = plan_fleet_subnet_root_install(PlanFleetSubnetRootInstallRequest {
            fleet_install_plan,
            infrastructure_manifest: &infrastructure_manifest,
            coordinator,
            install_operation_id,
            component_topology: component_topology.clone(),
            root_plan,
        })?;
        let entry = expected_registry_join_entry(&current.journal)?;
        let next_registry = FleetRegistryOps::compile_joining(
            &authority,
            &component_topology,
            &expected_registry,
            entry,
        )?;
        drive_registry_join(
            icp_context,
            &protocol_binding,
            &component_topology,
            current,
            &expected_registry,
            &next_registry,
        )?;
        expected_registry = next_registry;
    }

    let joining_version =
        FleetRegistryOps::version(&authority, &component_topology, &expected_registry)?;
    let live = query_live_registry(icp_context.cli(), &protocol_binding, coordinator)?;
    let active_registry =
        FleetRegistryOps::compile_active(&authority, &component_topology, &expected_registry)?;
    let compiled =
        compile_fleet_component_provisioning_plan(CompileFleetComponentProvisioningPlanRequest {
            config: config.model(),
            fleet_install_plan: &fleet_install_plan.plan,
            registry: &active_registry,
            operation_id: super::root_component_provisioning_operation_id(install_operation_id),
        })?;
    require_joining_or_recovered_registry(JoiningRegistryRecoveryRequest {
        active: ActiveRegistryRecoveryRequest {
            icp: icp_context.cli(),
            binding: &protocol_binding,
            coordinator,
            component_topology: &component_topology,
            active: &active_registry,
            live: &live,
            expected_operation_id: compiled.prepare_request.operation_id,
            expected_plan_hash: compiled.plan_hash,
        },
        joining: &expected_registry,
    })?;
    Ok(joining_version)
}

fn drive_registry_join(
    icp_context: &InstallIcpContext,
    binding: &crate::protocol_binding::ResolvedProtocolBinding,
    component_topology: &ComponentTopology,
    mut current: ResolvedFleetSubnetRootInstall,
    expected_before: &FleetRegistry,
    expected_after: &FleetRegistry,
) -> Result<(), Box<dyn std::error::Error>> {
    let coordinator = current.journal.authority.binding.coordinator;
    let icp = icp_context.cli();
    let expected_after_version = FleetRegistryOps::version(
        &current.journal.authority,
        component_topology,
        expected_after,
    )?;

    for _ in 0..MAX_REGISTRY_JOIN_TRANSITIONS {
        current = match current.journal.phase {
            FleetSubnetRootInstallPhase::StoreVerified => {
                let live = query_live_registry(icp, binding, coordinator)?;
                require_exact_registry(
                    &current.journal.authority,
                    component_topology,
                    expected_before,
                    &live,
                    "pre-join",
                )?;
                begin_registry_join(&current, live.version)?
            }
            FleetSubnetRootInstallPhase::RegistryJoinInFlight => {
                let request = current
                    .journal
                    .registry_join_request
                    .clone()
                    .ok_or(RootRegistryJoinError::MissingJoinRequest)?;
                let response: CoordinatorCommandResponse = call_with_arg(
                    icp,
                    binding,
                    coordinator,
                    protocol::CANIC_COMMAND,
                    &CoordinatorCommand::JoinRoot(request.clone()),
                )?;
                let CoordinatorCommandResponse::JoinRoot(response) = response else {
                    return Err(RootRegistryJoinError::JoinResponseMismatch.into());
                };
                if response.entry != request.entry || response.version != expected_after_version {
                    return Err(RootRegistryJoinError::JoinResponseMismatch.into());
                }
                record_registry_joined(&current, response)?
            }
            FleetSubnetRootInstallPhase::RegistryJoined => {
                let live = query_live_registry(icp, binding, coordinator)?;
                require_exact_registry(
                    &current.journal.authority,
                    component_topology,
                    expected_after,
                    &live,
                    "post-join",
                )?;
                record_registry_join_verified(
                    &current,
                    &live.registry,
                    &live.manifest,
                    &live.version,
                )?
            }
            FleetSubnetRootInstallPhase::RegistryJoinVerified
            | FleetSubnetRootInstallPhase::RegistrySyncInFlight
            | FleetSubnetRootInstallPhase::RegistrySynchronized
            | FleetSubnetRootInstallPhase::RegistrySyncVerified
            | FleetSubnetRootInstallPhase::RegistryMirrorActivationInFlight
            | FleetSubnetRootInstallPhase::RegistryMirrorActivated
            | FleetSubnetRootInstallPhase::RegistryMirrorActivationVerified
            | FleetSubnetRootInstallPhase::ComponentRegistryPreparationInFlight
            | FleetSubnetRootInstallPhase::ComponentRegistryPrepared
            | FleetSubnetRootInstallPhase::ComponentRegistryPreparationVerified => {
                let response = current
                    .journal
                    .registry_join_response
                    .as_ref()
                    .ok_or(RootRegistryJoinError::JoinResponseMismatch)?;
                if response.version != expected_after_version {
                    return Err(RootRegistryJoinError::JoinResponseMismatch.into());
                }
                return Ok(());
            }
            phase => return Err(RootRegistryJoinError::StoreNotVerified(phase).into()),
        };
    }
    Err(RootRegistryJoinError::TransitionBoundExceeded.into())
}

fn require_exact_registry(
    authority: &FleetRegistryAuthority,
    component_topology: &ComponentTopology,
    expected_registry: &FleetRegistry,
    live: &LiveRegistryEvidence,
    stage: &'static str,
) -> Result<(), Box<dyn std::error::Error>> {
    let expected_manifest =
        FleetRegistryOps::manifest(authority, component_topology, expected_registry)?;
    let expected_version =
        FleetRegistryOps::version(authority, component_topology, expected_registry)?;
    if live.registry != *expected_registry
        || live.manifest != expected_manifest
        || live.version != expected_version
    {
        return Err(RootRegistryJoinError::LiveRegistryMismatch(stage).into());
    }
    Ok(())
}
