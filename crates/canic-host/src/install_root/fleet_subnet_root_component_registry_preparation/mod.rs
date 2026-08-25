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
use super::icp_context::InstallIcpContext;
use super::operations::call_with_arg;
use crate::{
    fleet_install_plan::PersistedFleetInstallPlan,
    protocol_binding::resolve_infrastructure_protocol_binding,
    release_set::{AppConfigSnapshot, load_persisted_canic_infrastructure_artifact_manifest},
};
use candid::{CandidType, Principal};
use canic_core::{
    dto::component_registry::{
        RootComponentRegistryPreparationRequest, RootComponentRegistryStatusResponse,
    },
    protocol,
};
use serde::Deserialize;
use std::path::Path;
use thiserror::Error as ThisError;

const MAX_COMPONENT_REGISTRY_PREPARATION_TRANSITIONS: usize = 4;

#[derive(CandidType)]
enum RootCommandFragment {
    PrepareComponentRegistry(RootComponentRegistryPreparationRequest),
}

#[derive(CandidType, Deserialize)]
enum RootCommandResponseFragment {
    PrepareComponentRegistry(RootComponentRegistryStatusResponse),
}

#[derive(Debug, ThisError)]
enum RootComponentRegistryPreparationError {
    #[error("root Component Registry preparation reached unexpected phase {0:?}")]
    UnexpectedPhase(FleetSubnetRootInstallPhase),

    #[error("root Component Registry preparation exceeded its bounded phase transitions")]
    TransitionBoundExceeded,

    #[error("live root Component Registry differs from durable preparation evidence")]
    LiveEvidenceMismatch,
}

/// Re-run the exact retained preparation command as an idempotent live-state proof.
///
/// The command's Root-owned workflow verifies the durable Store bootstrap and active Registry
/// mirror before it can return the Component Registry status. Comparing that status with the
/// retained host receipt therefore proves the upgraded Root preserved the installation state;
/// matching controllers, authority and module bytes alone cannot establish that property.
pub(super) fn verify_retained_component_registry_preparation(
    icp_context: &InstallIcpContext,
    journal: &super::fleet_subnet_root_install_journal::FleetSubnetRootInstallJournal,
) -> Result<(), Box<dyn std::error::Error>> {
    if journal.phase != FleetSubnetRootInstallPhase::ComponentRegistryPreparationVerified {
        return Err(RootComponentRegistryPreparationError::UnexpectedPhase(journal.phase).into());
    }
    let root = journal
        .fleet_subnet_root
        .ok_or(RootComponentRegistryPreparationError::LiveEvidenceMismatch)?;
    let request = journal
        .component_registry_preparation_request
        .clone()
        .ok_or(RootComponentRegistryPreparationError::LiveEvidenceMismatch)?;
    let expected = journal
        .component_registry_preparation_response
        .as_ref()
        .ok_or(RootComponentRegistryPreparationError::LiveEvidenceMismatch)?;
    let binding = resolve_infrastructure_protocol_binding(
        icp_context.root(),
        icp_context.environment(),
        &journal.root_artifact,
    )?;
    let response: RootCommandResponseFragment = call_with_arg(
        icp_context.cli(),
        &binding,
        root,
        protocol::CANIC_COMMAND,
        &RootCommandFragment::PrepareComponentRegistry(request),
    )?;
    let RootCommandResponseFragment::PrepareComponentRegistry(observed) = response;
    if &observed != expected {
        return Err(RootComponentRegistryPreparationError::LiveEvidenceMismatch.into());
    }
    Ok(())
}

pub(super) struct PrepareFleetSubnetRootComponentRegistriesRequest<'a> {
    pub icp: &'a InstallIcpContext,
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
        request.icp.root(),
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
        drive_component_registry_preparation(request.icp, current, preparation_request)?;
    }
    Ok(())
}

fn drive_component_registry_preparation(
    icp_context: &InstallIcpContext,
    mut current: ResolvedFleetSubnetRootInstall,
    request: RootComponentRegistryPreparationRequest,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = current
        .journal
        .fleet_subnet_root
        .ok_or(RootComponentRegistryPreparationError::LiveEvidenceMismatch)?;
    let binding = resolve_infrastructure_protocol_binding(
        icp_context.root(),
        icp_context.environment(),
        &current.journal.root_artifact,
    )?;
    let icp = icp_context.cli();
    for _ in 0..MAX_COMPONENT_REGISTRY_PREPARATION_TRANSITIONS {
        current = match current.journal.phase {
            FleetSubnetRootInstallPhase::RegistryMirrorActivationVerified => {
                begin_component_registry_preparation(&current, request.clone())?
            }
            FleetSubnetRootInstallPhase::ComponentRegistryPreparationInFlight => {
                let response: RootCommandResponseFragment = call_with_arg(
                    icp,
                    &binding,
                    root,
                    protocol::CANIC_COMMAND,
                    &RootCommandFragment::PrepareComponentRegistry(request.clone()),
                )?;
                let RootCommandResponseFragment::PrepareComponentRegistry(response) = response;
                record_component_registry_prepared(&current, response)?
            }
            FleetSubnetRootInstallPhase::ComponentRegistryPrepared => {
                let response: RootCommandResponseFragment = call_with_arg(
                    icp,
                    &binding,
                    root,
                    protocol::CANIC_COMMAND,
                    &RootCommandFragment::PrepareComponentRegistry(request.clone()),
                )?;
                let RootCommandResponseFragment::PrepareComponentRegistry(response) = response;
                record_component_registry_preparation_verified(&current, response)?
            }
            FleetSubnetRootInstallPhase::ComponentRegistryPreparationVerified => return Ok(()),
            phase => {
                return Err(RootComponentRegistryPreparationError::UnexpectedPhase(phase).into());
            }
        };
    }
    Err(RootComponentRegistryPreparationError::TransitionBoundExceeded.into())
}
