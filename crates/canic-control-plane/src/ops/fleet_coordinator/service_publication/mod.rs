//! Module: ops::fleet_coordinator::service_publication
//!
//! Responsibility: bind terminal Component provisioning evidence to canonical Fleet services.
//! Does not own: the Coordinator record, root provisioning, or Registry transition commits.
//! Boundary: returns one exact Registry/receipt pair for the parent operation to commit atomically.

use super::*;

#[derive(Clone, Copy, Eq, PartialEq)]
struct FleetComponentProvisioningAuthority {
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    configuration_digest: ComponentDeploymentConfigurationDigest,
}

const fn component_provisioning_authority(
    record: &FleetComponentProvisioningRecord,
) -> FleetComponentProvisioningAuthority {
    FleetComponentProvisioningAuthority {
        operation_id: record.operation_id,
        plan_hash: record.plan_hash,
        configuration_digest: record.plan.configuration_digest,
    }
}

const fn service_publication_authority(
    receipt: &FleetServicePublicationReceiptRecord,
) -> FleetComponentProvisioningAuthority {
    FleetComponentProvisioningAuthority {
        operation_id: receipt.operation_id,
        plan_hash: receipt.plan_hash,
        configuration_digest: receipt.configuration_digest,
    }
}

pub(super) fn validate_service_publication_authority(
    current: &FleetCoordinatorRegistryRecord,
    record: &FleetComponentProvisioningRecord,
) -> Result<(), InternalError> {
    let Some((publication, receipt)) = paired_service_publication_evidence(current, record)? else {
        return Ok(());
    };
    if component_provisioning_authority(record) != service_publication_authority(receipt) {
        return Err(receipt_invariant(
            "Fleet-service publication receipt differs from its provisioning plan",
        ));
    }
    if publication.published_at_ns < publication.components_provisioned_at_ns {
        return Err(receipt_invariant(
            "Fleet-service publication time precedes complete root provisioning",
        ));
    }
    let source_registry = component_operation_source_registry(current, record)?;
    let root_receipts = publication
        .provisions
        .iter()
        .map(|provision| provision.response.clone())
        .collect::<Vec<_>>();
    let receipt_hashes = root_receipts
        .iter()
        .map(|root_receipt| root_receipt.receipt_content_hash)
        .collect::<Vec<_>>();
    let services = compile_component_operation_services(
        &current.component_deployment_configuration,
        &source_registry,
        record,
        &root_receipts,
    )
    .map_err(|_| {
        receipt_invariant("published root provisioning receipts do not compile canonical services")
    })?;
    let receipt_is_exact = [
        receipt.previous_version == record.plan.fleet_registry,
        receipt.version == *publication.published_registry,
        receipt.root_receipt_content_hashes == receipt_hashes,
        receipt.services == services,
    ]
    .into_iter()
    .all(|fact| fact);
    if !receipt_is_exact {
        return Err(receipt_invariant(
            "Fleet-service publication receipt differs from its exact terminal evidence",
        ));
    }
    Ok(())
}

struct FleetServicePublicationState<'a> {
    provisions: &'a [FleetComponentProvisioningRootProvisionRecord],
    components_provisioned_at_ns: u64,
    published_registry: &'a FleetRegistryVersion,
    published_at_ns: u64,
}

fn service_publication_state(
    record: &FleetComponentProvisioningRecord,
) -> Option<FleetServicePublicationState<'_>> {
    match &record.state {
        FleetComponentProvisioningStateRecord::ServiceTopologyPublished {
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            ..
        }
        | FleetComponentProvisioningStateRecord::ConfirmingDirectories {
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            ..
        }
        | FleetComponentProvisioningStateRecord::DirectoriesConfirmed {
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            ..
        }
        | FleetComponentProvisioningStateRecord::ActivatingRuntimes {
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            ..
        }
        | FleetComponentProvisioningStateRecord::RuntimesActivated {
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            ..
        } => Some(FleetServicePublicationState {
            provisions,
            components_provisioned_at_ns: *components_provisioned_at_ns,
            published_registry: published_fleet_registry,
            published_at_ns: *service_topology_published_at_ns,
        }),
        FleetComponentProvisioningStateRecord::Planned { .. }
        | FleetComponentProvisioningStateRecord::AcceptingRoots { .. }
        | FleetComponentProvisioningStateRecord::RootsAccepted { .. }
        | FleetComponentProvisioningStateRecord::ProvisioningRoots { .. }
        | FleetComponentProvisioningStateRecord::ComponentsProvisioned { .. } => None,
    }
}

fn paired_service_publication_evidence<'a>(
    current: &'a FleetCoordinatorRegistryRecord,
    record: &'a FleetComponentProvisioningRecord,
) -> Result<
    Option<(
        FleetServicePublicationState<'a>,
        &'a FleetServicePublicationReceiptRecord,
    )>,
    InternalError,
> {
    let receipt = service_publication_receipt_for_operation(current, record.operation_id)?;
    match (service_publication_state(record), receipt) {
        (Some(publication), Some(receipt)) => Ok(Some((publication, receipt))),
        (None, None) => Ok(None),
        (Some(_), None) => Err(receipt_invariant(
            "Fleet-service publication state lacks its atomic receipt",
        )),
        (None, Some(_)) => Err(receipt_invariant(
            "Fleet-service publication receipt lacks its atomic state",
        )),
    }
}

pub(super) fn service_publication_receipt_for_operation(
    current: &FleetCoordinatorRegistryRecord,
    operation_id: [u8; 32],
) -> Result<Option<&FleetServicePublicationReceiptRecord>, InternalError> {
    let mut matches = current
        .service_publication_receipts
        .iter()
        .filter(|receipt| receipt.operation_id == operation_id);
    let receipt = matches.next();
    if matches.next().is_some() {
        return Err(receipt_invariant(
            "Fleet-service publication operation has duplicate receipts",
        ));
    }
    Ok(receipt)
}

#[derive(Clone)]
pub(super) struct ComponentsProvisionedState {
    pub(super) planned_at_ns: u64,
    pub(super) acceptances: Vec<FleetComponentProvisioningRootAcceptanceRecord>,
    pub(super) roots_accepted_at_ns: u64,
    pub(super) provisions: Vec<FleetComponentProvisioningRootProvisionRecord>,
    pub(super) components_provisioned_at_ns: u64,
}

pub(super) struct ServicePublication {
    pub(super) registry: FleetRegistry,
    pub(super) receipt: FleetServicePublicationReceiptRecord,
}

pub(super) fn components_provisioned_state(
    record: &FleetComponentProvisioningRecord,
) -> Result<ComponentsProvisionedState, InternalError> {
    let FleetComponentProvisioningStateRecord::ComponentsProvisioned {
        planned_at_ns,
        acceptances,
        roots_accepted_at_ns,
        provisions,
        components_provisioned_at_ns,
    } = &record.state
    else {
        return Err(receipt_invariant(
            "Fleet-service publication disposition lacks ComponentsProvisioned state",
        ));
    };
    Ok(ComponentsProvisionedState {
        planned_at_ns: *planned_at_ns,
        acceptances: acceptances.clone(),
        roots_accepted_at_ns: *roots_accepted_at_ns,
        provisions: provisions.clone(),
        components_provisioned_at_ns: *components_provisioned_at_ns,
    })
}

pub(super) fn compile_service_publication(
    current: &FleetCoordinatorRegistryRecord,
    record: &FleetComponentProvisioningRecord,
    provisioned: &ComponentsProvisionedState,
) -> Result<ServicePublication, InternalError> {
    let source_registry = component_operation_source_registry(current, record)?;
    if current.registry != source_registry {
        return Err(InternalError::conflict());
    }
    if service_publication_receipt_for_operation(current, record.operation_id)?.is_some() {
        return Err(receipt_invariant(
            "ComponentsProvisioned state already contains Fleet-service publication evidence",
        ));
    }
    let root_receipts = provisioned
        .provisions
        .iter()
        .map(|provision| provision.response.clone())
        .collect::<Vec<_>>();
    let services = compile_component_operation_services(
        &current.component_deployment_configuration,
        &source_registry,
        record,
        &root_receipts,
    )?;
    let topology = &current
        .component_deployment_configuration
        .component_topology;
    let previous_version =
        FleetRegistryOps::version(&current.authority, topology, &current.registry)?;
    if previous_version != record.plan.fleet_registry {
        return Err(InternalError::conflict());
    }
    let registry = if services == current.registry.services {
        current.registry.clone()
    } else {
        match record.plan.operation {
            FleetComponentProvisioningOperation::FreshInstall => {
                FleetRegistryOps::compile_initial_services(
                    &current.authority,
                    topology,
                    &current.registry,
                    services.clone(),
                )?
            }
            FleetComponentProvisioningOperation::ScaleOut { .. } => {
                FleetRegistryOps::compile_service_additions(
                    &current.authority,
                    topology,
                    &current.registry,
                    services.clone(),
                )?
            }
        }
    };
    let version = FleetRegistryOps::version(&current.authority, topology, &registry)?;
    let root_receipt_content_hashes = root_receipts
        .iter()
        .map(|receipt| receipt.receipt_content_hash)
        .collect();
    Ok(ServicePublication {
        registry,
        receipt: FleetServicePublicationReceiptRecord {
            operation_id: record.operation_id,
            plan_hash: record.plan_hash,
            configuration_digest: record.plan.configuration_digest,
            root_receipt_content_hashes,
            services,
            previous_version,
            version,
        },
    })
}

pub(super) fn compile_component_operation_services(
    configuration: &canic_core::control_plane_support::config::ComponentDeploymentConfiguration,
    source_registry: &FleetRegistry,
    record: &FleetComponentProvisioningRecord,
    root_receipts: &[RootComponentProvisioningStatusResponse],
) -> Result<Vec<canic_core::dto::fleet_registry::FleetServiceBinding>, InternalError> {
    match record.plan.operation {
        FleetComponentProvisioningOperation::FreshInstall => {
            FleetServiceBindingOps::compile_initial_compiled(
                configuration,
                source_registry,
                &record.plan,
                record.operation_id,
                root_receipts,
            )
        }
        FleetComponentProvisioningOperation::ScaleOut { .. } => {
            FleetServiceBindingOps::compile_scale_out_compiled(
                configuration,
                source_registry,
                &record.plan,
                record.operation_id,
                record.plan_hash,
                root_receipts,
            )
        }
    }
}
