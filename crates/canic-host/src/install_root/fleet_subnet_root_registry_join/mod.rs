//! Module: install_root::fleet_subnet_root_registry_join
//!
//! Responsibility: register every Store-verified root as Fleet Registry `Joining`.
//! Does not own: root snapshot synchronization, acknowledgement, `Active`, or runtime activation.
//! Boundary: each compare-and-commit request and response is journalled, then the complete
//! Coordinator snapshot, manifest, and version are independently reproduced from the Fleet plan.

use super::fleet_subnet_root_install_journal::{
    FleetSubnetRootInstallPhase, PlanFleetSubnetRootInstallRequest, ResolvedFleetSubnetRootInstall,
    begin_registry_join, expected_registry_join_entry, plan_fleet_subnet_root_install,
    record_registry_join_verified, record_registry_joined,
};
use crate::{
    durable_io::write_bytes,
    fleet_install_plan::PersistedFleetInstallPlan,
    icp::{IcpCli, LocalReplicaTarget, decode_json_result_response},
    release_set::{AppConfigSnapshot, load_persisted_canic_infrastructure_artifact_manifest},
};
use candid::{CandidType, Principal};
use canic_core::{
    control_plane_support::{config::ComponentTopology, ops::fleet_registry::FleetRegistryOps},
    dto::fleet_registry::{
        FleetRegistry, FleetRegistryManifest, FleetRegistryVersion, FleetSubnetRootJoinRequest,
        FleetSubnetRootJoinResponse,
    },
    ids::{FleetCoordinatorBinding, FleetRegistryAuthority},
    protocol,
};
use std::{
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

const ICP_JSON_OUTPUT: &str = "json";
const MAX_REGISTRY_JOIN_TRANSITIONS: usize = 4;
const CALL_ARGS_FILE: &str = "registry-call-args.bin";

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

    #[error("root Registry join arguments are not a regular file: {0}")]
    UnsafeArgumentsFile(PathBuf),
}

struct LiveRegistryEvidence {
    registry: FleetRegistry,
    manifest: FleetRegistryManifest,
    version: FleetRegistryVersion,
}

pub(super) fn register_and_verify_fleet_subnet_roots_joining(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    config_path: &Path,
    fleet_install_plan: &PersistedFleetInstallPlan,
    coordinator: Principal,
    install_operation_id: [u8; 32],
) -> Result<FleetRegistryVersion, Box<dyn std::error::Error>> {
    let config = AppConfigSnapshot::load(config_path)?;
    let component_topology = config.model().compile_component_topology()?;
    let infrastructure_manifest = load_persisted_canic_infrastructure_artifact_manifest(
        icp_root,
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
    let mut expected_registry = FleetRegistryOps::compile_genesis(
        &fleet_install_plan.plan.fleet.app,
        authority.clone(),
        &component_topology,
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
            icp_root,
            environment,
            local_replica,
            &component_topology,
            current,
            &expected_registry,
            &next_registry,
        )?;
        expected_registry = next_registry;
    }

    let live = query_live_registry(
        &coordinator_icp(icp_root, environment, local_replica),
        coordinator,
    )?;
    require_exact_registry(
        &authority,
        &component_topology,
        &expected_registry,
        &live,
        "complete all-Joining",
    )?;
    Ok(live.version)
}

fn drive_registry_join(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    component_topology: &ComponentTopology,
    mut current: ResolvedFleetSubnetRootInstall,
    expected_before: &FleetRegistry,
    expected_after: &FleetRegistry,
) -> Result<(), Box<dyn std::error::Error>> {
    let coordinator = current.journal.authority.binding.coordinator;
    let icp = coordinator_icp(icp_root, environment, local_replica);
    let expected_after_version = FleetRegistryOps::version(
        &current.journal.authority,
        component_topology,
        expected_after,
    )?;

    for _ in 0..MAX_REGISTRY_JOIN_TRANSITIONS {
        current = match current.journal.phase {
            FleetSubnetRootInstallPhase::StoreVerified => {
                let live = query_live_registry(&icp, coordinator)?;
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
                let response = call_registry_join(&icp, coordinator, &current.path, &request)?;
                if response.entry != request.entry || response.version != expected_after_version {
                    return Err(RootRegistryJoinError::JoinResponseMismatch.into());
                }
                record_registry_joined(&current, response)?
            }
            FleetSubnetRootInstallPhase::RegistryJoined => {
                let live = query_live_registry(&icp, coordinator)?;
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
            | FleetSubnetRootInstallPhase::RegistrySyncVerified => {
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

fn call_registry_join(
    icp: &IcpCli,
    coordinator: Principal,
    journal_path: &Path,
    request: &FleetSubnetRootJoinRequest,
) -> Result<FleetSubnetRootJoinResponse, Box<dyn std::error::Error>> {
    let args_path = journal_path
        .parent()
        .expect("validated root journal has a parent")
        .join(CALL_ARGS_FILE);
    write_bytes(&args_path, &candid::encode_one(request)?)?;
    let result = icp.canister_call_binary_args_output_with_candid(
        &coordinator.to_text(),
        protocol::CANIC_FLEET_SUBNET_ROOT_JOIN,
        &args_path,
        Some(ICP_JSON_OUTPUT),
        None,
    );
    let cleanup = fs::remove_file(&args_path);
    let output = result?;
    cleanup.map_err(|_| RootRegistryJoinError::UnsafeArgumentsFile(args_path))?;
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

fn coordinator_icp(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
) -> IcpCli {
    IcpCli::new("icp", Some(environment.to_string()))
        .with_cwd(icp_root)
        .with_local_replica(local_replica.cloned())
}
