//! Module: install_root::fleet_subnet_root_runtime_activation
//!
//! Responsibility: seal, activate, and independently verify every prepared root runtime.
//! Does not own: initial Component provisioning, Fleet Registry mutation, or catalog publication.
//! Boundary: 0.100 seals the empty initial inventory; exact nonempty provisioning starts in 0.101.

use super::fleet_subnet_root_install_journal::{
    FleetSubnetRootInstallPhase, PlanFleetSubnetRootInstallRequest, ResolvedFleetSubnetRootInstall,
    begin_root_activation, begin_root_activation_preparation, plan_fleet_subnet_root_install,
    record_root_activated, record_root_activation_prepared, record_root_activation_verified,
    validate_live_root_activation_status,
};
use super::operations::{call_no_arg, call_with_arg, query_no_arg, query_with_arg};
use crate::{
    fleet_install_plan::PersistedFleetInstallPlan,
    icp::LocalReplicaTarget,
    release_set::{AppConfigSnapshot, load_persisted_canic_infrastructure_artifact_manifest},
};
use candid::Principal;
use canic_core::{
    dto::{
        component_registry::{
            RootComponentRegistryPreparationRequest, RootComponentRegistryStatusResponse,
        },
        fleet_activation::{FleetActivationResumeRequest, FleetActivationStatusResponse},
    },
    protocol,
};
use std::path::Path;
use thiserror::Error as ThisError;

const MAX_ROOT_ACTIVATION_TRANSITIONS: usize = 7;

#[derive(Debug, ThisError)]
enum RootRuntimeActivationError {
    #[error("root runtime activation reached unexpected phase {0:?}")]
    UnexpectedPhase(FleetSubnetRootInstallPhase),

    #[error("root runtime activation exceeded its bounded phase transitions")]
    TransitionBoundExceeded,

    #[error("live root runtime or Component Registry differs from durable activation evidence")]
    LiveEvidenceMismatch,
}

pub(super) struct ActivateFleetSubnetRootRuntimesRequest<'a> {
    pub icp_root: &'a Path,
    pub environment: &'a str,
    pub local_replica: Option<&'a LocalReplicaTarget>,
    pub config_path: &'a Path,
    pub fleet_install_plan: &'a PersistedFleetInstallPlan,
    pub coordinator: Principal,
    pub install_operation_id: [u8; 32],
}

pub(super) fn activate_and_verify_fleet_subnet_root_runtimes(
    request: ActivateFleetSubnetRootRuntimesRequest<'_>,
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
        let component_registry_request = current
            .journal
            .component_registry_preparation_request
            .clone()
            .ok_or(RootRuntimeActivationError::LiveEvidenceMismatch)?;
        drive_root_runtime_activation(
            request.icp_root,
            request.environment,
            request.local_replica,
            current,
            component_registry_request,
        )?;
    }
    Ok(())
}

fn drive_root_runtime_activation(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    mut current: ResolvedFleetSubnetRootInstall,
    component_registry_request: RootComponentRegistryPreparationRequest,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = current
        .journal
        .fleet_subnet_root
        .ok_or(RootRuntimeActivationError::LiveEvidenceMismatch)?;
    let icp = super::install_icp(icp_root, environment, local_replica);

    for _ in 0..MAX_ROOT_ACTIVATION_TRANSITIONS {
        current = match current.journal.phase {
            FleetSubnetRootInstallPhase::ComponentRegistryPreparationVerified => {
                begin_root_activation_preparation(&current)?
            }
            FleetSubnetRootInstallPhase::RootActivationPreparationInFlight => {
                let observed: FleetActivationStatusResponse =
                    query_no_arg(&icp, root, protocol::CANIC_FLEET_ACTIVATION_STATUS)?;
                validate_live_root_activation_status(&current.path, &current.journal, &observed)?;
                let response = call_no_arg(&icp, root, protocol::CANIC_PREPARE_FLEET_ACTIVATION)?;
                record_root_activation_prepared(&current, response)?
            }
            FleetSubnetRootInstallPhase::RootActivationPrepared => begin_root_activation(&current)?,
            FleetSubnetRootInstallPhase::RootActivationInFlight => {
                let prepared = current
                    .journal
                    .root_activation_preparation_response
                    .as_ref()
                    .ok_or(RootRuntimeActivationError::LiveEvidenceMismatch)?;
                let request = FleetActivationResumeRequest {
                    operation_id: current.journal.install_operation_id,
                    credential: prepared
                        .credential
                        .ok_or(RootRuntimeActivationError::LiveEvidenceMismatch)?,
                };
                let observed: FleetActivationStatusResponse =
                    query_no_arg(&icp, root, protocol::CANIC_FLEET_ACTIVATION_STATUS)?;
                validate_live_root_activation_status(&current.path, &current.journal, &observed)?;
                let response = call_with_arg(
                    &icp,
                    root,
                    protocol::CANIC_RESUME_FLEET_ACTIVATION,
                    &request,
                )?;
                record_root_activated(&current, response)?
            }
            FleetSubnetRootInstallPhase::RootActivated => {
                let response = query_no_arg(&icp, root, protocol::CANIC_FLEET_ACTIVATION_STATUS)?;
                let component_registry = query_with_arg(
                    &icp,
                    root,
                    protocol::CANIC_ROOT_COMPONENT_REGISTRY_STATUS,
                    &component_registry_request,
                )?;
                record_root_activation_verified(&current, response, component_registry)?
            }
            FleetSubnetRootInstallPhase::RootActivationVerified => {
                let response: FleetActivationStatusResponse =
                    query_no_arg(&icp, root, protocol::CANIC_FLEET_ACTIVATION_STATUS)?;
                let component_registry: RootComponentRegistryStatusResponse = query_with_arg(
                    &icp,
                    root,
                    protocol::CANIC_ROOT_COMPONENT_REGISTRY_STATUS,
                    &component_registry_request,
                )?;
                if current.journal.root_activation_response.as_ref() != Some(&response)
                    || current
                        .journal
                        .component_registry_activation_response
                        .as_ref()
                        != Some(&component_registry)
                {
                    return Err(RootRuntimeActivationError::LiveEvidenceMismatch.into());
                }
                return Ok(());
            }
            phase => return Err(RootRuntimeActivationError::UnexpectedPhase(phase).into()),
        };
    }

    Err(RootRuntimeActivationError::TransitionBoundExceeded.into())
}
