//! Module: ops::fleet_coordinator::component_provisioning_projection
//!
//! Responsibility: project current and terminal Component provisioning authority into typed status and receipts.
//! Does not own: Coordinator storage, validation, commits, orchestration, or effects.
//! Boundary: derives deterministic read-only responses and content hashes from retained records.

use super::*;

pub(super) fn component_provisioning_status_response(
    record: &FleetComponentProvisioningRecord,
) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
    let counts = component_provisioning_plan_counts(&record.plan)?;
    let acceptance = component_provisioning_root_acceptance_progress(record)?;
    let provisioning = component_provisioning_root_provision_progress(record)?;
    let directory = if provisioning.published_fleet_registry.is_some() {
        Some(component_directory_confirmation_progress(record)?)
    } else {
        None
    };
    let activation = match &record.state {
        FleetComponentProvisioningStateRecord::ActivatingRuntimes { .. }
        | FleetComponentProvisioningStateRecord::RuntimesActivated { .. } => {
            Some(component_runtime_activation_progress(record)?)
        }
        _ => None,
    };
    let current_synchronization = match (
        &record.plan.operation,
        directory
            .as_ref()
            .and_then(|progress| progress.current.as_ref()),
    ) {
        (FleetComponentProvisioningOperation::ScaleOut { .. }, Some(current)) => {
            confirmation_synchronization_progress(current)
        }
        _ => None,
    };
    Ok(FleetComponentProvisioningStatusResponse {
        operation_id: record.operation_id,
        plan_hash: record.plan_hash,
        fleet_registry: record.plan.fleet_registry.clone(),
        configuration_digest: record.plan.configuration_digest,
        operation: record.plan.operation.clone(),
        phase: acceptance.phase,
        directory_confirmation_root_count: counts.directory_confirmation_roots,
        root_batch_count: counts.root_batches,
        accepted_root_count: acceptance.accepted_root_count,
        acceptance_in_flight_root: acceptance.in_flight.map(|intent| intent.fleet_subnet_root),
        provisioned_root_count: provisioning.provisioned_root_count,
        current_root: provisioning
            .current_response
            .as_ref()
            .map(root_provisioning_progress),
        provisioning_in_flight_root: provisioning
            .in_flight
            .as_ref()
            .map(|intent| intent.fleet_subnet_root),
        directory_confirmed_root_count: directory
            .as_ref()
            .map_or(0, |progress| progress.confirmed_root_count),
        current_synchronization,
        current_publication: directory
            .as_ref()
            .and_then(|progress| progress.current.as_ref())
            .and_then(confirmation_publication_response)
            .map(root_publication_progress),
        publication_in_flight_root: directory
            .as_ref()
            .and_then(|progress| progress.in_flight.as_ref())
            .map(confirmation_intent_root),
        runtime_activated_root_count: activation
            .as_ref()
            .map_or(0, |progress| progress.activated_root_count),
        current_activation: activation
            .as_ref()
            .and_then(|progress| progress.current.map(|record| record.progress)),
        activation_in_flight_root: activation
            .as_ref()
            .and_then(|progress| progress.in_flight)
            .map(|intent| intent.fleet_subnet_root),
        pending_root_failure: pending_component_provisioning_root_failure(record),
        group_placement_count: counts.group_placements,
        component_count: counts.components,
        planned_at_ns: acceptance.planned_at_ns,
        roots_accepted_at_ns: acceptance.roots_accepted_at_ns,
        components_provisioned_at_ns: provisioning.components_provisioned_at_ns,
        published_fleet_registry: provisioning.published_fleet_registry,
        service_topology_published_at_ns: provisioning.service_topology_published_at_ns,
        directories_confirmed_at_ns: match &record.state {
            FleetComponentProvisioningStateRecord::DirectoriesConfirmed {
                directories_confirmed_at_ns,
                ..
            }
            | FleetComponentProvisioningStateRecord::ActivatingRuntimes {
                directories_confirmed_at_ns,
                ..
            }
            | FleetComponentProvisioningStateRecord::RuntimesActivated {
                directories_confirmed_at_ns,
                ..
            } => Some(*directories_confirmed_at_ns),
            _ => None,
        },
        runtimes_activated_at_ns: activation
            .as_ref()
            .and_then(|progress| progress.runtimes_activated_at_ns),
    })
}

pub(super) fn component_scale_out_terminal_receipt(
    record: &FleetComponentProvisioningRecord,
    deployments: &[FleetComponentGroupDeploymentRecord],
) -> Result<FleetComponentScaleOutReceiptRecord, InternalError> {
    let FleetComponentProvisioningStateRecord::RuntimesActivated {
        planned_at_ns,
        roots_accepted_at_ns,
        components_provisioned_at_ns,
        published_fleet_registry,
        service_topology_published_at_ns,
        directories_confirmed_at_ns,
        runtimes_activated_at_ns,
        ..
    } = &record.state
    else {
        return Err(receipt_invariant(
            "only terminal scale-out authority may be retired",
        ));
    };
    if !matches!(
        record.plan.operation,
        FleetComponentProvisioningOperation::ScaleOut { .. }
    ) {
        return Err(receipt_invariant(
            "retired Component operation is not scale-out",
        ));
    }
    let counts = component_provisioning_plan_counts(&record.plan)?;
    let mut placements = deployments
        .iter()
        .flat_map(|deployment| &deployment.placements)
        .filter(|placement| placement.operation_id == record.operation_id)
        .cloned()
        .collect::<Vec<_>>();
    placements.sort_unstable_by(|left, right| left.placement.cmp(&right.placement));
    let mut receipt = FleetComponentScaleOutReceiptRecord {
        operation_id: record.operation_id,
        plan_hash: record.plan_hash,
        fleet_registry: record.plan.fleet_registry.clone(),
        configuration_digest: record.plan.configuration_digest,
        operation: record.plan.operation.clone(),
        directory_confirmation_root_count: counts.directory_confirmation_roots,
        root_batch_count: counts.root_batches,
        component_count: counts.components,
        planned_at_ns: *planned_at_ns,
        roots_accepted_at_ns: *roots_accepted_at_ns,
        components_provisioned_at_ns: *components_provisioned_at_ns,
        published_fleet_registry: published_fleet_registry.clone(),
        service_topology_published_at_ns: *service_topology_published_at_ns,
        directories_confirmed_at_ns: *directories_confirmed_at_ns,
        runtimes_activated_at_ns: *runtimes_activated_at_ns,
        placements,
        receipt_content_hash: [0; 32],
    };
    receipt.receipt_content_hash = component_scale_out_receipt_content_hash(&receipt)?;
    Ok(receipt)
}

pub(super) fn component_scale_out_receipt_content_hash(
    receipt: &FleetComponentScaleOutReceiptRecord,
) -> Result<[u8; 32], InternalError> {
    let mut authority = receipt.clone();
    authority.receipt_content_hash = [0; 32];
    let payload = candid::encode_one(authority).map_err(|_error| InternalError::invariant())?;
    let mut hasher = Sha256::new();
    hasher.update(COMPONENT_SCALE_OUT_RECEIPT_HASH_DOMAIN);
    hasher.update(payload);
    Ok(hasher.finalize().into())
}

pub(super) fn component_scale_out_receipt_response(
    receipt: &FleetComponentScaleOutReceiptRecord,
) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
    let FleetComponentProvisioningOperation::ScaleOut {
        previous_placements,
        requested_placements,
        ..
    } = receipt.operation
    else {
        return Err(receipt_invariant(
            "retired Component operation is not scale-out",
        ));
    };
    let group_placement_count = requested_placements
        .checked_sub(previous_placements)
        .filter(|count| *count > 0)
        .ok_or_else(|| receipt_invariant("retired scale-out count is not monotonic"))?;
    Ok(FleetComponentProvisioningStatusResponse {
        operation_id: receipt.operation_id,
        plan_hash: receipt.plan_hash,
        fleet_registry: receipt.fleet_registry.clone(),
        configuration_digest: receipt.configuration_digest,
        operation: receipt.operation.clone(),
        phase: FleetComponentProvisioningPhase::RuntimesActivated,
        directory_confirmation_root_count: receipt.directory_confirmation_root_count,
        root_batch_count: receipt.root_batch_count,
        accepted_root_count: receipt.root_batch_count,
        acceptance_in_flight_root: None,
        provisioned_root_count: receipt.root_batch_count,
        current_root: None,
        provisioning_in_flight_root: None,
        directory_confirmed_root_count: receipt.directory_confirmation_root_count,
        current_synchronization: None,
        current_publication: None,
        publication_in_flight_root: None,
        runtime_activated_root_count: receipt.root_batch_count,
        current_activation: None,
        activation_in_flight_root: None,
        pending_root_failure: None,
        group_placement_count,
        component_count: receipt.component_count,
        planned_at_ns: receipt.planned_at_ns,
        roots_accepted_at_ns: Some(receipt.roots_accepted_at_ns),
        components_provisioned_at_ns: Some(receipt.components_provisioned_at_ns),
        published_fleet_registry: Some(receipt.published_fleet_registry.clone()),
        service_topology_published_at_ns: Some(receipt.service_topology_published_at_ns),
        directories_confirmed_at_ns: Some(receipt.directories_confirmed_at_ns),
        runtimes_activated_at_ns: Some(receipt.runtimes_activated_at_ns),
    })
}

pub(super) fn component_scale_out_receipt_for_operation(
    receipts: &[FleetComponentScaleOutReceiptRecord],
    operation_id: [u8; 32],
) -> Result<Option<&FleetComponentScaleOutReceiptRecord>, InternalError> {
    let mut matches = receipts
        .iter()
        .filter(|receipt| receipt.operation_id == operation_id);
    let receipt = matches.next();
    if matches.next().is_some() {
        return Err(receipt_invariant(
            "retired scale-out operation has duplicate receipts",
        ));
    }
    Ok(receipt)
}
