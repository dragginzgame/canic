//! Module: install_root::fleet_component_provisioning_install
//!
//! Responsibility: drive the host-owned fresh-install Component transaction to terminal catalog publication.
//! Does not own: placement selection, Coordinator/root transitions, or catalog truth.
//! Boundary: passive status reconciles uncertain updates and the catalog is unreachable before exact `RuntimesActivated` evidence.

use super::{
    clock::current_unix_timestamp_secs,
    fleet_catalog_closeout::{
        PublishInstalledFleetCatalogRequest, publish_installed_fleet_catalog,
    },
    fleet_component_provisioning_journal::{
        FleetComponentProvisioningInstallPhase, PlanFleetComponentProvisioningInstallRequest,
        ResolvedFleetComponentProvisioningInstall, begin_component_provisioning_advance,
        begin_component_provisioning_preparation, begin_fleet_catalog_publication,
        complete_fleet_component_provisioning_install, plan_fleet_component_provisioning_install,
        record_component_provisioning_advanced, record_component_provisioning_prepared,
        record_fleet_catalog_published,
    },
    fleet_component_provisioning_plan::{
        CompileFleetComponentProvisioningPlanRequest, compile_fleet_component_provisioning_plan,
    },
    icp_context::InstallIcpContext,
    operations::{call_with_arg, query_with_arg},
};
use crate::{
    canister_protocol::CanisterProtocolError, fleet_catalog::FleetCatalogEntryV1,
    fleet_install_plan::PersistedFleetInstallPlan, icp::IcpCli, release_set::AppConfigSnapshot,
};
use std::path::Path;

use candid::Principal;
use canic_core::{
    diagnostics::codes,
    dto::{
        component_provisioning::{
            FleetComponentProvisioningPhase, FleetComponentProvisioningStatusRequest,
            FleetComponentProvisioningStatusResponse,
        },
        fleet_registry::FleetRegistry,
    },
    ids::FleetName,
    protocol,
};
use thiserror::Error as ThisError;

const BASE_ADVANCE_LIMIT: usize = 32;
const ADVANCES_PER_COMPONENT: usize = 8;
const ADVANCES_PER_PLACEMENT: usize = 4;
const ADVANCES_PER_ROOT: usize = 8;

pub(super) struct InstallFleetComponentsRequest<'a> {
    pub icp: &'a InstallIcpContext,
    pub config_path: &'a Path,
    pub fleet_name: FleetName,
    pub fleet_install_plan: &'a PersistedFleetInstallPlan,
    pub coordinator: Principal,
    pub install_operation_id: [u8; 32],
    pub initial_active_registry: &'a FleetRegistry,
}

#[derive(Debug, ThisError)]
enum FleetComponentProvisioningInstallError {
    #[error("Fleet Component provisioning exceeded its bounded advance count")]
    AdvanceBoundExceeded,

    #[error("terminal Fleet Component provisioning has no published Fleet Registry version")]
    MissingPublishedRegistry,

    #[error("Fleet catalog publication has no exact durable row intent")]
    MissingCatalogIntent,

    #[error("durable Fleet Component provisioning advance intent has no exact request")]
    MissingAdvanceRequest,

    #[error("Fleet Component provisioning status query failed after preparation: {0}")]
    StatusQuery(#[source] CanisterProtocolError),
}

/// Provision every explicitly placed initial Component, activate all roots, and publish discovery.
pub(super) fn install_fleet_components_and_publish_catalog(
    request: InstallFleetComponentsRequest<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfigSnapshot::load(request.config_path)?;
    let compiled =
        compile_fleet_component_provisioning_plan(CompileFleetComponentProvisioningPlanRequest {
            config: config.model(),
            fleet_install_plan: &request.fleet_install_plan.plan,
            registry: request.initial_active_registry,
            operation_id: request.install_operation_id,
        })?;
    let advance_limit = provisioning_advance_limit(&compiled.prepare_request.plan)?;
    let mut current =
        plan_fleet_component_provisioning_install(PlanFleetComponentProvisioningInstallRequest {
            fleet_install_plan: request.fleet_install_plan,
            coordinator: request.coordinator,
            fleet_name: request.fleet_name.clone(),
            environment: request.icp.environment().to_string(),
            compiled,
        })?;
    let icp = request.icp.cli();
    let mut remote_advances = 0_usize;

    loop {
        current = match current.journal.phase {
            FleetComponentProvisioningInstallPhase::Planned => {
                begin_component_provisioning_preparation(&current)?
            }
            FleetComponentProvisioningInstallPhase::PreparationInFlight => {
                let status = query_or_prepare(icp, request.coordinator, &current)?;
                record_component_provisioning_prepared(&current, status)?
            }
            FleetComponentProvisioningInstallPhase::Prepared => {
                begin_component_provisioning_advance(&current)?
            }
            FleetComponentProvisioningInstallPhase::AdvanceInFlight => {
                remote_advances = remote_advances
                    .checked_add(1)
                    .ok_or(FleetComponentProvisioningInstallError::AdvanceBoundExceeded)?;
                if remote_advances > advance_limit {
                    return Err(FleetComponentProvisioningInstallError::AdvanceBoundExceeded.into());
                }
                let status = reconcile_or_advance(icp, request.coordinator, &current)?;
                record_component_provisioning_advanced(&current, status)?
            }
            FleetComponentProvisioningInstallPhase::RuntimesActivated => {
                begin_fleet_catalog_publication(
                    &current,
                    catalog_entry(&request, current_unix_timestamp_secs()?),
                )?
            }
            FleetComponentProvisioningInstallPhase::CatalogPublicationInFlight => {
                let committed = publish_catalog(&request, &current)?;
                record_fleet_catalog_published(&current, committed)?
            }
            FleetComponentProvisioningInstallPhase::CatalogPublished => {
                complete_fleet_component_provisioning_install(&current)?
            }
            FleetComponentProvisioningInstallPhase::Complete => return Ok(()),
        };
    }
}

fn query_or_prepare(
    icp: &IcpCli,
    coordinator: Principal,
    current: &ResolvedFleetComponentProvisioningInstall,
) -> Result<FleetComponentProvisioningStatusResponse, Box<dyn std::error::Error>> {
    let status_request = status_request(current);
    match query_with_arg(
        icp,
        coordinator,
        protocol::CANIC_FLEET_COMPONENT_PROVISIONING_STATUS,
        &status_request,
    ) {
        Ok(status) => Ok(status),
        Err(error) if error.is_rejected_with(codes::STATE_UNAVAILABLE) => call_with_arg(
            icp,
            coordinator,
            protocol::CANIC_FLEET_COMPONENT_PROVISIONING_PREPARE,
            &current.journal.prepare_request,
        )
        .map_err(Into::into),
        Err(error) => Err(FleetComponentProvisioningInstallError::StatusQuery(error).into()),
    }
}

fn reconcile_or_advance(
    icp: &IcpCli,
    coordinator: Principal,
    current: &ResolvedFleetComponentProvisioningInstall,
) -> Result<FleetComponentProvisioningStatusResponse, Box<dyn std::error::Error>> {
    let observed: FleetComponentProvisioningStatusResponse = query_with_arg(
        icp,
        coordinator,
        protocol::CANIC_FLEET_COMPONENT_PROVISIONING_STATUS,
        &status_request(current),
    )?;
    if current.journal.last_status.as_ref() != Some(&observed) {
        return Ok(observed);
    }
    let advance = current
        .journal
        .advance_request
        .as_ref()
        .ok_or(FleetComponentProvisioningInstallError::MissingAdvanceRequest)?;
    call_with_arg(
        icp,
        coordinator,
        protocol::CANIC_FLEET_COMPONENT_PROVISIONING_ADVANCE,
        advance,
    )
    .map_err(Into::into)
}

const fn status_request(
    current: &ResolvedFleetComponentProvisioningInstall,
) -> FleetComponentProvisioningStatusRequest {
    FleetComponentProvisioningStatusRequest {
        operation_id: current.journal.prepare_request.operation_id,
        plan_hash: current.journal.plan_hash,
    }
}

fn publish_catalog(
    request: &InstallFleetComponentsRequest<'_>,
    current: &ResolvedFleetComponentProvisioningInstall,
) -> Result<crate::fleet_catalog::CommittedFleetCatalog, Box<dyn std::error::Error>> {
    let status = current
        .journal
        .last_status
        .as_ref()
        .ok_or(FleetComponentProvisioningInstallError::MissingPublishedRegistry)?;
    if status.phase != FleetComponentProvisioningPhase::RuntimesActivated {
        return Err(FleetComponentProvisioningInstallError::MissingPublishedRegistry.into());
    }
    let terminal_fleet_registry = status
        .published_fleet_registry
        .as_ref()
        .ok_or(FleetComponentProvisioningInstallError::MissingPublishedRegistry)?;
    let catalog_entry = current
        .journal
        .catalog_entry
        .as_ref()
        .ok_or(FleetComponentProvisioningInstallError::MissingCatalogIntent)?;
    publish_installed_fleet_catalog(PublishInstalledFleetCatalogRequest {
        icp: request.icp,
        config_path: request.config_path,
        fleet_name: catalog_entry.fleet_name.clone(),
        fleet_install_plan: request.fleet_install_plan,
        coordinator: request.coordinator,
        deployed_at_unix_secs: catalog_entry.deployed_at_unix_secs,
        terminal_fleet_registry,
    })
}

fn catalog_entry(
    request: &InstallFleetComponentsRequest<'_>,
    deployed_at_unix_secs: u64,
) -> FleetCatalogEntryV1 {
    let fleet = &request.fleet_install_plan.plan.fleet;
    FleetCatalogEntryV1 {
        canonical_network_id: fleet.fleet.canonical_network_id,
        fleet_id: fleet.fleet.fleet_id,
        fleet_name: request.fleet_name.clone(),
        app: fleet.app.clone(),
        environment: request.icp.environment().to_string(),
        deployed_at_unix_secs,
        coordinator_principal: request.coordinator.to_text(),
    }
}

fn provisioning_advance_limit(
    plan: &canic_core::dto::component_provisioning::FleetComponentProvisioningPlan,
) -> Result<usize, FleetComponentProvisioningInstallError> {
    let placements = plan
        .batches
        .iter()
        .try_fold(0_usize, |total, batch| {
            total.checked_add(batch.placements.len())
        })
        .ok_or(FleetComponentProvisioningInstallError::AdvanceBoundExceeded)?;
    let components = plan
        .batches
        .iter()
        .flat_map(|batch| &batch.placements)
        .try_fold(0_usize, |total, placement| {
            total.checked_add(placement.entries.len())
        })
        .ok_or(FleetComponentProvisioningInstallError::AdvanceBoundExceeded)?;
    BASE_ADVANCE_LIMIT
        .checked_add(
            components
                .checked_mul(ADVANCES_PER_COMPONENT)
                .ok_or(FleetComponentProvisioningInstallError::AdvanceBoundExceeded)?,
        )
        .and_then(|limit| {
            placements
                .checked_mul(ADVANCES_PER_PLACEMENT)
                .and_then(|placement_work| limit.checked_add(placement_work))
        })
        .and_then(|limit| {
            plan.directory_confirmation_roots
                .len()
                .checked_mul(ADVANCES_PER_ROOT)
                .and_then(|root_work| limit.checked_add(root_work))
        })
        .ok_or(FleetComponentProvisioningInstallError::AdvanceBoundExceeded)
}
