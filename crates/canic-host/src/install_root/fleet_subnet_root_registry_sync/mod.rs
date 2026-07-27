//! Module: install_root::fleet_subnet_root_registry_sync
//!
//! Responsibility: drive and independently verify every root's all-Joining snapshot acknowledgement.
//! Does not own: Registry `Active`, Directory activation, runtime activation, or Fleet publication.
//! Boundary: each root journal records intent before the root performs its inter-canister calls.

use super::{
    fleet_subnet_root_install_journal::{
        FleetSubnetRootInstallPhase, PlanFleetSubnetRootInstallRequest,
        ResolvedFleetSubnetRootInstall, begin_registry_sync, plan_fleet_subnet_root_install,
        record_registry_sync_verified, record_registry_synchronized,
    },
    fleet_subnet_root_store_bootstrap::canonical_manifest_bytes,
};
use crate::{
    fleet_install_plan::PersistedFleetInstallPlan,
    icp::{IcpCli, LocalReplicaTarget, decode_json_result_response},
    release_set::{AppConfigSnapshot, load_persisted_canic_infrastructure_artifact_manifest},
};
use candid::{CandidType, IDLValue, Principal};
use canic_core::{
    dto::fleet_registry::{
        FleetRegistryVersion, FleetSubnetRootRegistrySyncRequest,
        FleetSubnetRootSnapshotAcknowledgement,
    },
    dto::root_store::RootStoreBootstrapRequest,
    protocol,
};
use std::path::Path;
use thiserror::Error as ThisError;

const ICP_JSON_OUTPUT: &str = "json";
const MAX_SYNC_TRANSITIONS: usize = 4;

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
    pub icp_root: &'a Path,
    pub environment: &'a str,
    pub local_replica: Option<&'a LocalReplicaTarget>,
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
        icp_root,
        environment,
        local_replica,
        config_path,
        fleet_install_plan,
        coordinator,
        install_operation_id,
        joining_version,
    } = request;
    let config = AppConfigSnapshot::load(config_path)?;
    let component_topology = config.model().compile_component_topology()?;
    let infrastructure_manifest = load_persisted_canic_infrastructure_artifact_manifest(
        icp_root,
        fleet_install_plan.plan.release_build_id,
    )?;
    let mut expected = Vec::with_capacity(fleet_install_plan.plan.fleet_subnet_roots.len());

    for root_plan in &fleet_install_plan.plan.fleet_subnet_roots {
        let release_set = fleet_install_plan
            .root_release_sets
            .iter()
            .find(|release_set| release_set.placement_subnet == root_plan.placement_subnet)
            .ok_or(RootRegistrySyncError::MissingReleaseSet)?;
        let request = FleetSubnetRootRegistrySyncRequest {
            expected_registry: joining_version.clone(),
            store_bootstrap: RootStoreBootstrapRequest {
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
        drive_root_sync(icp_root, environment, local_replica, current, request)?;
    }

    let live: Vec<FleetSubnetRootSnapshotAcknowledgement> = query_no_arg(
        &coordinator_icp(icp_root, environment, local_replica),
        coordinator,
        protocol::CANIC_FLEET_REGISTRY_ROOT_ACKNOWLEDGEMENTS,
    )?;
    expected.sort_by(|left, right| left.as_slice().cmp(right.as_slice()));
    if live.len() != expected.len()
        || live
            .iter()
            .zip(expected)
            .any(|(ack, root)| ack.fleet_subnet_root != root || ack.version != joining_version)
    {
        return Err(RootRegistrySyncError::AcknowledgementSetMismatch.into());
    }
    Ok(())
}

fn drive_root_sync(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    mut current: ResolvedFleetSubnetRootInstall,
    request: FleetSubnetRootRegistrySyncRequest,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = current
        .journal
        .fleet_subnet_root
        .expect("Registry synchronization follows root verification");
    let icp = root_icp(icp_root, environment, local_replica);
    for _ in 0..MAX_SYNC_TRANSITIONS {
        current = match current.journal.phase {
            FleetSubnetRootInstallPhase::RegistryJoinVerified => {
                begin_registry_sync(&current, request.clone())?
            }
            FleetSubnetRootInstallPhase::RegistrySyncInFlight => {
                let response = call_with_arg(
                    &icp,
                    root,
                    protocol::CANIC_FLEET_REGISTRY_SYNCHRONIZE,
                    request.clone(),
                    false,
                )?;
                record_registry_synchronized(&current, response)?
            }
            FleetSubnetRootInstallPhase::RegistrySynchronized => {
                let response = call_with_arg(
                    &icp,
                    root,
                    protocol::CANIC_FLEET_REGISTRY_SYNC_STATUS,
                    request.clone(),
                    true,
                )?;
                record_registry_sync_verified(&current, response)?
            }
            FleetSubnetRootInstallPhase::RegistrySyncVerified => return Ok(()),
            phase => return Err(RootRegistrySyncError::UnexpectedPhase(phase).into()),
        };
    }
    Err(RootRegistrySyncError::TransitionBoundExceeded.into())
}

fn call_with_arg<I, O>(
    icp: &IcpCli,
    canister: Principal,
    method: &str,
    input: I,
    query: bool,
) -> Result<O, Box<dyn std::error::Error>>
where
    I: CandidType,
    O: CandidType + serde::de::DeserializeOwned,
{
    let value = IDLValue::try_from_candid_type(&input)?;
    let args = format!("({value})");
    let output = if query {
        icp.canister_query_arg_output_with_candid(
            &canister.to_text(),
            method,
            &args,
            Some(ICP_JSON_OUTPUT),
            None,
        )?
    } else {
        icp.canister_call_arg_output_with_candid(
            &canister.to_text(),
            method,
            &args,
            Some(ICP_JSON_OUTPUT),
            None,
        )?
    };
    decode_json_result_response(&output).map_err(Into::into)
}

fn query_no_arg<O>(
    icp: &IcpCli,
    canister: Principal,
    method: &str,
) -> Result<O, Box<dyn std::error::Error>>
where
    O: CandidType + serde::de::DeserializeOwned,
{
    let output = icp.canister_query_output_with_candid(
        &canister.to_text(),
        method,
        Some(ICP_JSON_OUTPUT),
        None,
    )?;
    decode_json_result_response(&output).map_err(Into::into)
}

fn root_icp(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
) -> IcpCli {
    IcpCli::new("icp", Some(environment.to_string()))
        .with_cwd(icp_root)
        .with_local_replica(local_replica.cloned())
}

fn coordinator_icp(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
) -> IcpCli {
    root_icp(icp_root, environment, local_replica)
}
