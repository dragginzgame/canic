//! Directory-confirmation call authority, retained evidence projections and response validation.
//!
//! Boundary: this owner prepares and validates immutable Root-facing Directory evidence without
//! mutating the Coordinator journal or issuing an inter-canister call.

use super::*;

pub(super) fn scale_out_synchronization_call(
    record: &FleetComponentProvisioningRecord,
    progress: &FleetComponentDirectoryConfirmationProgress,
    root_index: u32,
    root: Principal,
    expected_synchronized_component_count: u32,
    started_at_ns: u64,
) -> (
    FleetComponentDirectoryConfirmationCallView,
    FleetComponentDirectoryConfirmationIntentRecord,
) {
    let request = RootComponentDirectorySynchronizationRequest {
        operation_id: record.operation_id,
        plan_hash: record.plan_hash,
        source_fleet_registry: record.plan.fleet_registry.clone(),
        published_fleet_registry: progress.published_fleet_registry.clone(),
        expected_synchronized_component_count,
    };
    (
        FleetComponentDirectoryConfirmationCallView::ScaleOutSynchronization {
            fleet_subnet_root: root,
            request: request.clone(),
        },
        FleetComponentDirectoryConfirmationIntentRecord::ScaleOutSynchronization {
            root_index,
            fleet_subnet_root: root,
            request,
            started_at_ns,
        },
    )
}

pub(super) fn scale_out_publication_call(
    record: &FleetComponentProvisioningRecord,
    progress: &FleetComponentDirectoryConfirmationProgress,
    root_index: u32,
    root: Principal,
    expected_published_component_count: u32,
    started_at_ns: u64,
) -> Result<
    (
        FleetComponentDirectoryConfirmationCallView,
        FleetComponentDirectoryConfirmationIntentRecord,
    ),
    InternalError,
> {
    selected_root_batch(record, root)?;
    let request = RootComponentPublicationRequest {
        operation_id: record.operation_id,
        plan_hash: record.plan_hash,
        published_fleet_registry: progress.published_fleet_registry.clone(),
        expected_published_component_count,
    };
    Ok((
        FleetComponentDirectoryConfirmationCallView::ScaleOutPublication {
            fleet_subnet_root: root,
            request: request.clone(),
        },
        FleetComponentDirectoryConfirmationIntentRecord::ScaleOutPublication {
            root_index,
            fleet_subnet_root: root,
            request,
            started_at_ns,
        },
    ))
}

pub(super) fn confirmation_root(
    record: &FleetComponentProvisioningRecord,
    root_index: u32,
) -> Result<Principal, InternalError> {
    let index = usize::try_from(root_index)
        .map_err(|_| receipt_invariant("Directory confirmation root index exceeds usize"))?;
    let root = *record
        .plan
        .directory_confirmation_roots
        .get(index)
        .ok_or_else(|| receipt_invariant("Directory confirmation root index is out of bounds"))?;
    if matches!(
        record.plan.operation,
        FleetComponentProvisioningOperation::ScaleOut { .. }
    ) {
        return Ok(root);
    }
    let batch =
        record.plan.batches.get(index).ok_or_else(|| {
            receipt_invariant("Directory confirmation root has no selected batch")
        })?;
    if batch.root.fleet_subnet_root != root {
        return Err(receipt_invariant(
            "fresh Directory confirmation roots differ from selected batch order",
        ));
    }
    Ok(root)
}

pub(super) fn directory_confirmation_call_from_intent(
    intent: &FleetComponentDirectoryConfirmationIntentRecord,
) -> FleetComponentDirectoryConfirmationCallView {
    match intent {
        FleetComponentDirectoryConfirmationIntentRecord::FreshPublication {
            fleet_subnet_root,
            request,
            ..
        } => FleetComponentDirectoryConfirmationCallView::FreshPublication {
            fleet_subnet_root: *fleet_subnet_root,
            request: request.clone(),
        },
        FleetComponentDirectoryConfirmationIntentRecord::ScaleOutSynchronization {
            fleet_subnet_root,
            request,
            ..
        } => FleetComponentDirectoryConfirmationCallView::ScaleOutSynchronization {
            fleet_subnet_root: *fleet_subnet_root,
            request: request.clone(),
        },
        FleetComponentDirectoryConfirmationIntentRecord::ScaleOutPublication {
            fleet_subnet_root,
            request,
            ..
        } => FleetComponentDirectoryConfirmationCallView::ScaleOutPublication {
            fleet_subnet_root: *fleet_subnet_root,
            request: request.clone(),
        },
    }
}

pub(super) const fn confirmation_intent_root(
    intent: &FleetComponentDirectoryConfirmationIntentRecord,
) -> Principal {
    match intent {
        FleetComponentDirectoryConfirmationIntentRecord::FreshPublication {
            fleet_subnet_root,
            ..
        }
        | FleetComponentDirectoryConfirmationIntentRecord::ScaleOutSynchronization {
            fleet_subnet_root,
            ..
        }
        | FleetComponentDirectoryConfirmationIntentRecord::ScaleOutPublication {
            fleet_subnet_root,
            ..
        } => *fleet_subnet_root,
    }
}

pub(super) const fn confirmation_intent_started_at_ns(
    intent: &FleetComponentDirectoryConfirmationIntentRecord,
) -> u64 {
    match intent {
        FleetComponentDirectoryConfirmationIntentRecord::FreshPublication {
            started_at_ns, ..
        }
        | FleetComponentDirectoryConfirmationIntentRecord::ScaleOutSynchronization {
            started_at_ns,
            ..
        }
        | FleetComponentDirectoryConfirmationIntentRecord::ScaleOutPublication {
            started_at_ns,
            ..
        } => *started_at_ns,
    }
}

pub(super) const fn confirmation_call_publication_request(
    call: &FleetComponentDirectoryConfirmationCallView,
) -> Result<&RootComponentPublicationRequest, InternalError> {
    match call {
        FleetComponentDirectoryConfirmationCallView::FreshPublication { request, .. }
        | FleetComponentDirectoryConfirmationCallView::ScaleOutPublication { request, .. } => {
            Ok(request)
        }
        FleetComponentDirectoryConfirmationCallView::ScaleOutSynchronization { .. } => Err(
            receipt_invariant("Directory publication call contains synchronization authority"),
        ),
    }
}

pub(super) const fn fresh_confirmation_intent(
    intent: &FleetComponentDirectoryConfirmationIntentRecord,
) -> Result<(u32, Principal, &RootComponentPublicationRequest, u64), InternalError> {
    let FleetComponentDirectoryConfirmationIntentRecord::FreshPublication {
        root_index,
        fleet_subnet_root,
        request,
        started_at_ns,
    } = intent
    else {
        return Err(receipt_invariant(
            "fresh Directory confirmation contains scale-out intent",
        ));
    };
    Ok((*root_index, *fleet_subnet_root, request, *started_at_ns))
}

pub(super) const fn scale_out_synchronization_intent(
    intent: &FleetComponentDirectoryConfirmationIntentRecord,
) -> Result<
    (
        u32,
        Principal,
        &RootComponentDirectorySynchronizationRequest,
        u64,
    ),
    InternalError,
> {
    let FleetComponentDirectoryConfirmationIntentRecord::ScaleOutSynchronization {
        root_index,
        fleet_subnet_root,
        request,
        started_at_ns,
    } = intent
    else {
        return Err(receipt_invariant(
            "scale-out Directory synchronization contains different intent",
        ));
    };
    Ok((*root_index, *fleet_subnet_root, request, *started_at_ns))
}

pub(super) const fn scale_out_publication_intent(
    intent: &FleetComponentDirectoryConfirmationIntentRecord,
) -> Result<(u32, Principal, &RootComponentPublicationRequest, u64), InternalError> {
    let FleetComponentDirectoryConfirmationIntentRecord::ScaleOutPublication {
        root_index,
        fleet_subnet_root,
        request,
        started_at_ns,
    } = intent
    else {
        return Err(receipt_invariant(
            "scale-out Directory publication contains different intent",
        ));
    };
    Ok((*root_index, *fleet_subnet_root, request, *started_at_ns))
}

pub(super) fn confirmation_publication_response(
    record: &FleetComponentDirectoryConfirmationRecord,
) -> Option<&RootComponentProvisioningStatusResponse> {
    match record {
        FleetComponentDirectoryConfirmationRecord::FreshPublication { response, .. } => {
            Some(response.as_ref())
        }
        FleetComponentDirectoryConfirmationRecord::ScaleOut { publication, .. } => {
            publication.as_deref()
        }
    }
}

pub(super) fn fresh_confirmation_response(
    record: &FleetComponentDirectoryConfirmationRecord,
) -> Result<&RootComponentProvisioningStatusResponse, InternalError> {
    let FleetComponentDirectoryConfirmationRecord::FreshPublication { response, .. } = record
    else {
        return Err(receipt_invariant(
            "fresh Directory confirmation contains scale-out evidence",
        ));
    };
    Ok(response.as_ref())
}

pub(super) const fn confirmation_started_at_ns(
    record: &FleetComponentDirectoryConfirmationRecord,
) -> u64 {
    match record {
        FleetComponentDirectoryConfirmationRecord::FreshPublication { started_at_ns, .. }
        | FleetComponentDirectoryConfirmationRecord::ScaleOut { started_at_ns, .. } => {
            *started_at_ns
        }
    }
}

pub(super) const fn confirmation_recorded_at_ns(
    record: &FleetComponentDirectoryConfirmationRecord,
) -> u64 {
    match record {
        FleetComponentDirectoryConfirmationRecord::FreshPublication { recorded_at_ns, .. }
        | FleetComponentDirectoryConfirmationRecord::ScaleOut { recorded_at_ns, .. } => {
            *recorded_at_ns
        }
    }
}

pub(super) fn scale_out_confirmation_progress(
    record: &FleetComponentDirectoryConfirmationRecord,
) -> Result<
    (
        &RootComponentDirectorySynchronizationResponse,
        Option<&RootComponentProvisioningStatusResponse>,
    ),
    InternalError,
> {
    let FleetComponentDirectoryConfirmationRecord::ScaleOut {
        synchronization,
        publication,
        ..
    } = record
    else {
        return Err(receipt_invariant(
            "scale-out Directory confirmation contains fresh evidence",
        ));
    };
    Ok((synchronization.as_ref(), publication.as_deref()))
}

pub(super) const fn require_scale_out_operation(
    record: &FleetComponentProvisioningRecord,
) -> Result<(), InternalError> {
    if matches!(
        record.plan.operation,
        FleetComponentProvisioningOperation::ScaleOut { .. }
    ) {
        return Ok(());
    }
    Err(InternalError::conflict())
}

fn selected_root_batch(
    record: &FleetComponentProvisioningRecord,
    root: Principal,
) -> Result<&FleetSubnetRootProvisioningBatch, InternalError> {
    record
        .plan
        .batches
        .iter()
        .find(|batch| batch.root.fleet_subnet_root == root)
        .ok_or_else(|| receipt_invariant("Directory publication root has no selected batch"))
}

pub(super) fn selected_root_provisioned_response<'a>(
    record: &FleetComponentProvisioningRecord,
    progress: &'a FleetComponentDirectoryConfirmationProgress,
    root: Principal,
) -> Result<&'a RootComponentProvisioningStatusResponse, InternalError> {
    let index = record
        .plan
        .batches
        .iter()
        .position(|batch| batch.root.fleet_subnet_root == root)
        .ok_or_else(|| receipt_invariant("Directory publication root has no selected batch"))?;
    let response = progress
        .provisions
        .get(index)
        .map(|record| &record.response)
        .ok_or_else(|| receipt_invariant("selected Directory root lacks provisioning evidence"))?;
    if response.fleet_subnet_root != root {
        return Err(receipt_invariant(
            "selected Directory root provisioning evidence changed root",
        ));
    }
    Ok(response)
}

pub(super) fn scale_out_confirmation_is_terminal(
    record: &FleetComponentProvisioningRecord,
    root: Principal,
    confirmation: &FleetComponentDirectoryConfirmationRecord,
) -> Result<bool, InternalError> {
    let (synchronization, publication) = scale_out_confirmation_progress(confirmation)?;
    if !synchronization.complete {
        return Ok(false);
    }
    let selected = record
        .plan
        .batches
        .iter()
        .any(|batch| batch.root.fleet_subnet_root == root);
    Ok(if selected {
        publication
            .is_some_and(|response| response.phase == RootComponentProvisioningPhase::Published)
    } else {
        publication.is_none()
    })
}

pub(super) struct ScaleOutSynchronizationValidationContext<'a> {
    pub(super) coordinator: &'a FleetCoordinatorRegistryRecord,
    pub(super) operation: &'a FleetComponentProvisioningRecord,
    pub(super) progress: &'a FleetComponentDirectoryConfirmationProgress,
    pub(super) root_index: u32,
    pub(super) root: Principal,
    pub(super) request: &'a RootComponentDirectorySynchronizationRequest,
    pub(super) started_at_ns: u64,
    pub(super) recorded_at_ns: u64,
}

pub(super) fn validate_scale_out_synchronization_response(
    context: &ScaleOutSynchronizationValidationContext<'_>,
    response: &RootComponentDirectorySynchronizationResponse,
) -> Result<(), InternalError> {
    if context.root_index != context.progress.confirmed_root_count
        || confirmation_root(context.operation, context.root_index)? != context.root
    {
        return Err(receipt_invariant(
            "scale-out Directory synchronization cursor changed canonical root",
        ));
    }
    let previous = context
        .progress
        .current
        .as_ref()
        .map(scale_out_confirmation_progress)
        .transpose()?
        .map_or(0, |(response, _)| response.synchronized_component_count);
    let count_advances = response.synchronized_component_count == previous
        || previous.checked_add(1) == Some(response.synchronized_component_count);
    let authority_is_exact = [
        context.request.operation_id == context.operation.operation_id,
        context.request.plan_hash == context.operation.plan_hash,
        context.request.source_fleet_registry == context.operation.plan.fleet_registry,
        context.request.published_fleet_registry == context.progress.published_fleet_registry,
        context.request.expected_synchronized_component_count == previous,
        response.operation_id == context.operation.operation_id,
        response.plan_hash == context.operation.plan_hash,
        response.source_fleet_registry == context.operation.plan.fleet_registry,
        response.published_fleet_registry == context.progress.published_fleet_registry,
        response.fleet_subnet_root == context.root,
        response.synchronized_component_count <= response.affected_component_count,
        count_advances,
        context.recorded_at_ns >= context.started_at_ns,
    ]
    .into_iter()
    .all(|matches| matches);
    if !authority_is_exact {
        return Err(InternalError::conflict());
    }
    if let Some(current) = &context.progress.current {
        let (previous_response, publication) = scale_out_confirmation_progress(current)?;
        let retained_authority_changed = [
            previous_response.affected_component_count != response.affected_component_count,
            previous_response.fleet_directory_content_hash != response.fleet_directory_content_hash,
            publication.is_some(),
        ]
        .into_iter()
        .any(|changed| changed);
        if retained_authority_changed {
            return Err(InternalError::conflict());
        }
    }
    let expected_directory_hash = expected_fleet_directory_content_hash(
        context.coordinator,
        &context.progress.published_fleet_registry,
        context.root,
    )?;
    if response.fleet_directory_content_hash != expected_directory_hash {
        return Err(InternalError::conflict());
    }
    let terminal_evidence_is_exact = if response.complete {
        [
            response.synchronized_component_count == response.affected_component_count,
            response
                .synchronized_at_ns
                .is_some_and(|time| time >= context.started_at_ns),
            response.receipt_content_hash
                == RootComponentProvisioningReceiptOps::directory_synchronization_content_hash(
                    response,
                )?,
        ]
        .into_iter()
        .all(|matches| matches)
    } else {
        [
            response.synchronized_component_count < response.affected_component_count,
            response.synchronized_at_ns.is_none(),
            response.receipt_content_hash == [0; 32],
        ]
        .into_iter()
        .all(|matches| matches)
    };
    if !terminal_evidence_is_exact {
        return Err(InternalError::conflict());
    }
    Ok(())
}
