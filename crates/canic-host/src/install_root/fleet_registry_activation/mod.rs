//! Module: install_root::fleet_registry_activation
//!
//! Responsibility: atomically activate and independently verify the complete acknowledged Registry.
//! Does not own: final root mirror/Directory publication, root runtime activation, or Fleet catalog.
//! Boundary: host recovery journals exact intent before the Coordinator mutation.

use super::{
    fleet_registry_activation_journal::{
        FleetRegistryActivationPhase, PlanFleetRegistryActivationRequest,
        ResolvedFleetRegistryActivation, begin_registry_activation, plan_fleet_registry_activation,
        record_registry_activated, record_registry_activation_verified,
    },
    fleet_subnet_root_install_journal::{
        FleetSubnetRootInstallPhase, PlanFleetSubnetRootInstallRequest,
        expected_registry_join_entry, plan_fleet_subnet_root_install,
    },
};
use crate::{
    fleet_install_plan::PersistedFleetInstallPlan,
    icp::{IcpCli, LocalReplicaTarget, decode_json_result_response},
    release_set::{AppConfigSnapshot, load_persisted_canic_infrastructure_artifact_manifest},
};
use candid::{CandidType, IDLValue, Principal};
use canic_core::{
    control_plane_support::{config::ComponentTopology, ops::fleet_registry::FleetRegistryOps},
    dto::fleet_registry::{
        FleetRegistry, FleetRegistryActivationResponse, FleetRegistryManifest,
        FleetRegistryVersion, FleetSubnetRootSnapshotAcknowledgement,
    },
    ids::{FleetCoordinatorBinding, FleetRegistryAuthority},
    protocol,
};
use std::path::Path;
use thiserror::Error as ThisError;

const ICP_JSON_OUTPUT: &str = "json";
const MAX_ACTIVATION_TRANSITIONS: usize = 4;

#[derive(Debug, ThisError)]
enum FleetRegistryActivationError {
    #[error("root Registry activation requires RegistrySyncVerified, observed {0:?}")]
    RootNotSynchronized(FleetSubnetRootInstallPhase),

    #[error("planned all-Joining Registry differs from the verified synchronization version")]
    JoiningVersionMismatch,

    #[error("live Fleet Registry differs from the exact planned {0} snapshot")]
    LiveRegistryMismatch(&'static str),

    #[error("Coordinator acknowledgement set differs from the complete planned root set")]
    AcknowledgementSetMismatch,

    #[error("Fleet Registry activation exceeded its bounded journal transitions")]
    TransitionBoundExceeded,
}

struct LiveRegistryEvidence {
    registry: FleetRegistry,
    manifest: FleetRegistryManifest,
    version: FleetRegistryVersion,
}

pub(super) struct VerifiedFleetRegistryActivation {
    pub registry: FleetRegistry,
    pub version: FleetRegistryVersion,
}

pub(super) struct ActivateFleetRegistryRequest<'a> {
    pub icp_root: &'a Path,
    pub environment: &'a str,
    pub local_replica: Option<&'a LocalReplicaTarget>,
    pub config_path: &'a Path,
    pub fleet_install_plan: &'a PersistedFleetInstallPlan,
    pub coordinator: Principal,
    pub install_operation_id: [u8; 32],
    pub joining_version: FleetRegistryVersion,
}

pub(super) fn activate_and_verify_fleet_registry(
    request: ActivateFleetRegistryRequest<'_>,
) -> Result<VerifiedFleetRegistryActivation, Box<dyn std::error::Error>> {
    let config = AppConfigSnapshot::load(request.config_path)?;
    let component_topology = config.model().compile_component_topology()?;
    let infrastructure_manifest = load_persisted_canic_infrastructure_artifact_manifest(
        request.icp_root,
        request.fleet_install_plan.plan.release_build_id,
    )?;
    let authority = FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: request.fleet_install_plan.plan.fleet.clone(),
            coordinator_subnet: request
                .fleet_install_plan
                .plan
                .coordinator
                .coordinator_subnet,
            coordinator: request.coordinator,
        },
        epoch: 1,
    };
    let mut joining_registry = FleetRegistryOps::compile_genesis(
        &request.fleet_install_plan.plan.fleet.app,
        authority.clone(),
        &component_topology,
    )?;
    let mut expected_roots =
        Vec::with_capacity(request.fleet_install_plan.plan.fleet_subnet_roots.len());
    for root_plan in &request.fleet_install_plan.plan.fleet_subnet_roots {
        let current = plan_fleet_subnet_root_install(PlanFleetSubnetRootInstallRequest {
            fleet_install_plan: request.fleet_install_plan,
            infrastructure_manifest: &infrastructure_manifest,
            coordinator: request.coordinator,
            install_operation_id: request.install_operation_id,
            component_topology: component_topology.clone(),
            root_plan,
        })?;
        if !matches!(
            current.journal.phase,
            FleetSubnetRootInstallPhase::RegistrySyncVerified
                | FleetSubnetRootInstallPhase::RegistryMirrorActivationInFlight
                | FleetSubnetRootInstallPhase::RegistryMirrorActivated
                | FleetSubnetRootInstallPhase::RegistryMirrorActivationVerified
                | FleetSubnetRootInstallPhase::ComponentRegistryPreparationInFlight
                | FleetSubnetRootInstallPhase::ComponentRegistryPrepared
                | FleetSubnetRootInstallPhase::ComponentRegistryPreparationVerified
        ) {
            return Err(
                FleetRegistryActivationError::RootNotSynchronized(current.journal.phase).into(),
            );
        }
        if current
            .journal
            .registry_sync_request
            .as_ref()
            .is_none_or(|sync| sync.expected_registry != request.joining_version)
        {
            return Err(FleetRegistryActivationError::JoiningVersionMismatch.into());
        }
        let entry = expected_registry_join_entry(&current.journal)?;
        expected_roots.push(entry.fleet_subnet_root);
        joining_registry = FleetRegistryOps::compile_joining(
            &authority,
            &component_topology,
            &joining_registry,
            entry,
        )?;
    }
    let planned_joining_version =
        FleetRegistryOps::version(&authority, &component_topology, &joining_registry)?;
    if planned_joining_version != request.joining_version {
        return Err(FleetRegistryActivationError::JoiningVersionMismatch.into());
    }

    let planned = plan_fleet_registry_activation(PlanFleetRegistryActivationRequest {
        fleet_install_plan: request.fleet_install_plan,
        component_topology: component_topology.clone(),
        joining_registry,
    })?;
    let icp = coordinator_icp(request.icp_root, request.environment, request.local_replica);
    let current = drive_activation(
        &icp,
        request.coordinator,
        &component_topology,
        expected_roots,
        planned,
    )?;
    let live = query_live_registry(&icp, request.coordinator)?;
    require_exact_registry(
        &component_topology,
        &current.journal.active_registry,
        &live,
        "verified all-Active",
    )?;
    Ok(VerifiedFleetRegistryActivation {
        registry: live.registry,
        version: live.version,
    })
}

fn drive_activation(
    icp: &IcpCli,
    coordinator: Principal,
    component_topology: &ComponentTopology,
    mut expected_roots: Vec<Principal>,
    mut current: ResolvedFleetRegistryActivation,
) -> Result<ResolvedFleetRegistryActivation, Box<dyn std::error::Error>> {
    for _ in 0..MAX_ACTIVATION_TRANSITIONS {
        current = match current.journal.phase {
            FleetRegistryActivationPhase::Planned => {
                let live = query_live_registry(icp, coordinator)?;
                require_exact_registry(
                    component_topology,
                    &current.journal.joining_registry,
                    &live,
                    "pre-activation all-Joining",
                )?;
                require_exact_acknowledgements(
                    icp,
                    coordinator,
                    &mut expected_roots,
                    &current.journal.request.expected_registry,
                )?;
                begin_registry_activation(&current)?
            }
            FleetRegistryActivationPhase::ActivationInFlight => {
                let response = call_activation(icp, coordinator, &current.journal.request)?;
                record_registry_activated(&current, response)?
            }
            FleetRegistryActivationPhase::Activated => {
                let live = query_live_registry(icp, coordinator)?;
                require_exact_registry(
                    component_topology,
                    &current.journal.active_registry,
                    &live,
                    "post-activation all-Active",
                )?;
                record_registry_activation_verified(&current, live.manifest, live.version)?
            }
            FleetRegistryActivationPhase::Verified => return Ok(current),
        };
    }
    Err(FleetRegistryActivationError::TransitionBoundExceeded.into())
}

fn require_exact_acknowledgements(
    icp: &IcpCli,
    coordinator: Principal,
    expected_roots: &mut [Principal],
    version: &FleetRegistryVersion,
) -> Result<(), Box<dyn std::error::Error>> {
    let live: Vec<FleetSubnetRootSnapshotAcknowledgement> = query_coordinator(
        icp,
        coordinator,
        protocol::CANIC_FLEET_REGISTRY_ROOT_ACKNOWLEDGEMENTS,
    )?;
    expected_roots.sort_by(|left, right| left.as_slice().cmp(right.as_slice()));
    if live.len() != expected_roots.len()
        || live
            .iter()
            .zip(expected_roots)
            .any(|(ack, root)| ack.fleet_subnet_root != *root || &ack.version != version)
    {
        return Err(FleetRegistryActivationError::AcknowledgementSetMismatch.into());
    }
    Ok(())
}

fn call_activation(
    icp: &IcpCli,
    coordinator: Principal,
    request: &canic_core::dto::fleet_registry::FleetRegistryActivationRequest,
) -> Result<FleetRegistryActivationResponse, Box<dyn std::error::Error>> {
    let value = IDLValue::try_from_candid_type(request)?;
    let args = format!("({value})");
    let output = icp.canister_call_arg_output_with_candid(
        &coordinator.to_text(),
        protocol::CANIC_FLEET_REGISTRY_ACTIVATE,
        &args,
        Some(ICP_JSON_OUTPUT),
        None,
    )?;
    decode_json_result_response(&output).map_err(Into::into)
}

fn query_live_registry(
    icp: &IcpCli,
    coordinator: Principal,
) -> Result<LiveRegistryEvidence, Box<dyn std::error::Error>> {
    Ok(LiveRegistryEvidence {
        registry: query_coordinator(icp, coordinator, protocol::CANIC_FLEET_REGISTRY)?,
        manifest: query_coordinator(icp, coordinator, protocol::CANIC_FLEET_REGISTRY_MANIFEST)?,
        version: query_coordinator(icp, coordinator, protocol::CANIC_FLEET_REGISTRY_VERSION)?,
    })
}

fn query_coordinator<T>(
    icp: &IcpCli,
    coordinator: Principal,
    method: &str,
) -> Result<T, Box<dyn std::error::Error>>
where
    T: CandidType + serde::de::DeserializeOwned,
{
    let output = icp.canister_query_output_with_candid(
        &coordinator.to_text(),
        method,
        Some(ICP_JSON_OUTPUT),
        None,
    )?;
    decode_json_result_response(&output).map_err(Into::into)
}

fn require_exact_registry(
    component_topology: &ComponentTopology,
    expected: &FleetRegistry,
    live: &LiveRegistryEvidence,
    stage: &'static str,
) -> Result<(), Box<dyn std::error::Error>> {
    let manifest = FleetRegistryOps::manifest(&expected.authority, component_topology, expected)?;
    let version = FleetRegistryOps::version(&expected.authority, component_topology, expected)?;
    if live.registry != *expected || live.manifest != manifest || live.version != version {
        return Err(FleetRegistryActivationError::LiveRegistryMismatch(stage).into());
    }
    Ok(())
}

fn coordinator_icp(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
) -> IcpCli {
    IcpCli::new("icp", Some(environment.to_string()))
        .with_cwd(icp_root)
        .with_local_replica(local_replica.cloned())
}
