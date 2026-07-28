//! Module: install_root::fleet_subnet_root_component_registry_preparation
//!
//! Responsibility: prepare and independently verify every root's empty Component Registry.
//! Does not own: Component allocation, paid Canister effects, or runtime activation.
//! Boundary: each root journal freezes exact Store and active Registry authority before mutation.

use super::fleet_subnet_root_install_journal::{
    FleetSubnetRootInstallPhase, PlanFleetSubnetRootInstallRequest, ResolvedFleetSubnetRootInstall,
    begin_component_registry_preparation, plan_fleet_subnet_root_install,
    record_component_registry_preparation_verified, record_component_registry_prepared,
};
use crate::{
    fleet_install_plan::PersistedFleetInstallPlan,
    icp::{IcpCli, LocalReplicaTarget, decode_json_result_response},
    release_set::{AppConfigSnapshot, load_persisted_canic_infrastructure_artifact_manifest},
};
use candid::{CandidType, IDLValue, Principal};
use canic_core::{dto::component_registry::RootComponentRegistryPreparationRequest, protocol};
use std::path::Path;
use thiserror::Error as ThisError;

const ICP_JSON_OUTPUT: &str = "json";
const MAX_COMPONENT_REGISTRY_PREPARATION_TRANSITIONS: usize = 4;

#[derive(Debug, ThisError)]
enum RootComponentRegistryPreparationError {
    #[error("root Component Registry preparation reached unexpected phase {0:?}")]
    UnexpectedPhase(FleetSubnetRootInstallPhase),

    #[error("root Component Registry preparation exceeded its bounded phase transitions")]
    TransitionBoundExceeded,

    #[error("live root Component Registry differs from durable preparation evidence")]
    LiveEvidenceMismatch,
}

pub(super) struct PrepareFleetSubnetRootComponentRegistriesRequest<'a> {
    pub icp_root: &'a Path,
    pub environment: &'a str,
    pub local_replica: Option<&'a LocalReplicaTarget>,
    pub config_path: &'a Path,
    pub fleet_install_plan: &'a PersistedFleetInstallPlan,
    pub coordinator: Principal,
    pub install_operation_id: [u8; 32],
}

pub(super) fn prepare_and_verify_fleet_subnet_root_component_registries(
    request: PrepareFleetSubnetRootComponentRegistriesRequest<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfigSnapshot::load(request.config_path)?;
    let component_topology = config.model().compile_component_topology()?;
    let infrastructure_manifest = load_persisted_canic_infrastructure_artifact_manifest(
        request.icp_root,
        request.fleet_install_plan.plan.release_build_id,
    )?;

    for root_plan in &request.fleet_install_plan.plan.fleet_subnet_roots {
        let current = plan_fleet_subnet_root_install(PlanFleetSubnetRootInstallRequest {
            fleet_install_plan: request.fleet_install_plan,
            infrastructure_manifest: &infrastructure_manifest,
            coordinator: request.coordinator,
            install_operation_id: request.install_operation_id,
            component_topology: component_topology.clone(),
            root_plan,
        })?;
        let mirror_request = current
            .journal
            .registry_mirror_activation_request
            .clone()
            .ok_or(RootComponentRegistryPreparationError::LiveEvidenceMismatch)?;
        let preparation_request = RootComponentRegistryPreparationRequest {
            store_bootstrap: mirror_request.store_bootstrap,
            expected_fleet_registry: mirror_request.expected_registry,
        };
        drive_component_registry_preparation(
            request.icp_root,
            request.environment,
            request.local_replica,
            current,
            preparation_request,
        )?;
    }
    Ok(())
}

fn drive_component_registry_preparation(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    mut current: ResolvedFleetSubnetRootInstall,
    request: RootComponentRegistryPreparationRequest,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = current
        .journal
        .fleet_subnet_root
        .ok_or(RootComponentRegistryPreparationError::LiveEvidenceMismatch)?;
    let icp = root_icp(icp_root, environment, local_replica);
    for _ in 0..MAX_COMPONENT_REGISTRY_PREPARATION_TRANSITIONS {
        current = match current.journal.phase {
            FleetSubnetRootInstallPhase::RegistryMirrorActivationVerified => {
                begin_component_registry_preparation(&current, request.clone())?
            }
            FleetSubnetRootInstallPhase::ComponentRegistryPreparationInFlight => {
                let response = call_with_arg(
                    &icp,
                    root,
                    protocol::CANIC_ROOT_COMPONENT_REGISTRY_PREPARE,
                    request.clone(),
                    false,
                )?;
                record_component_registry_prepared(&current, response)?
            }
            FleetSubnetRootInstallPhase::ComponentRegistryPrepared => {
                let response = call_with_arg(
                    &icp,
                    root,
                    protocol::CANIC_ROOT_COMPONENT_REGISTRY_STATUS,
                    request.clone(),
                    true,
                )?;
                record_component_registry_preparation_verified(&current, response)?
            }
            FleetSubnetRootInstallPhase::ComponentRegistryPreparationVerified => {
                let response = call_with_arg(
                    &icp,
                    root,
                    protocol::CANIC_ROOT_COMPONENT_REGISTRY_STATUS,
                    request,
                    true,
                )?;
                if current
                    .journal
                    .component_registry_preparation_response
                    .as_ref()
                    != Some(&response)
                {
                    return Err(RootComponentRegistryPreparationError::LiveEvidenceMismatch.into());
                }
                return Ok(());
            }
            FleetSubnetRootInstallPhase::RootActivationPreparationInFlight
            | FleetSubnetRootInstallPhase::RootActivationPrepared
            | FleetSubnetRootInstallPhase::RootActivationInFlight
            | FleetSubnetRootInstallPhase::RootActivated
            | FleetSubnetRootInstallPhase::RootActivationVerified => return Ok(()),
            phase => {
                return Err(RootComponentRegistryPreparationError::UnexpectedPhase(phase).into());
            }
        };
    }
    Err(RootComponentRegistryPreparationError::TransitionBoundExceeded.into())
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

fn root_icp(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
) -> IcpCli {
    IcpCli::new("icp", Some(environment.to_string()))
        .with_cwd(icp_root)
        .with_local_replica(local_replica.cloned())
}
