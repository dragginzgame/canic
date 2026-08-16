//! Module: ops::component_directory_synchronization
//!
//! Responsibility: validate and commit one root-local scale-out Directory cursor.
//! Does not own: endpoint authentication, Registry mirror mutation, or Component calls.
//! Boundary: workflow supplies deterministic targets and exact observed target evidence.

#[cfg(test)]
mod tests;

use crate::{
    storage::stable::component_provisioning::{
        RootComponentDirectorySynchronizationIntentRecord,
        RootComponentDirectorySynchronizationRecord,
        RootComponentDirectorySynchronizationStateRecord,
        RootComponentDirectorySynchronizationTargetRecord, RootComponentProvisioningCommitError,
        RootComponentProvisioningStore,
    },
    view::component_directory_synchronization::{
        RootComponentDirectorySynchronizationDisposition,
        RootComponentDirectorySynchronizationIntentView,
        RootComponentDirectorySynchronizationTargetView, RootComponentDirectorySynchronizationView,
    },
};
use candid::Principal;
use canic_core::{
    control_plane_support::{
        error::InternalError,
        ops::component_provisioning_plan::MAX_FLEET_COMPONENT_PROVISIONING_PLAN_ENTRIES,
    },
    dto::component_provisioning::{
        RootComponentDirectorySynchronizationRequest, RootComponentDirectorySynchronizationResponse,
    },
};

/// Root-local operations over the durable scale-out Directory journal.
pub struct RootComponentDirectorySynchronizationOps;

impl RootComponentDirectorySynchronizationOps {
    pub(crate) fn validate_command(
        request: &RootComponentDirectorySynchronizationRequest,
    ) -> Result<(), InternalError> {
        validate_request(request)
    }

    #[must_use]
    pub(crate) fn is_prepared(operation_id: [u8; 32]) -> bool {
        RootComponentProvisioningStore::directory_synchronization(operation_id).is_some()
    }

    pub(crate) fn accept(
        request: &RootComponentDirectorySynchronizationRequest,
        fleet_subnet_root: Principal,
        fleet_directory_content_hash: [u8; 32],
        targets: Vec<RootComponentDirectorySynchronizationTargetView>,
        planned_at_ns: u64,
    ) -> Result<RootComponentDirectorySynchronizationResponse, InternalError> {
        validate_request(request)?;
        validate_acceptance(
            fleet_subnet_root,
            fleet_directory_content_hash,
            &targets,
            planned_at_ns,
        )?;
        if let Some(existing) =
            RootComponentProvisioningStore::directory_synchronization(request.operation_id)
        {
            let view = validated_record(existing)?;
            require_exact_authority(
                &view,
                request,
                fleet_subnet_root,
                fleet_directory_content_hash,
                &targets,
            )?;
            return status_response(&view);
        }
        let record = RootComponentDirectorySynchronizationRecord {
            operation_id: request.operation_id,
            plan_hash: request.plan_hash,
            source_fleet_registry: request.source_fleet_registry.clone(),
            published_fleet_registry: request.published_fleet_registry.clone(),
            fleet_subnet_root,
            fleet_directory_content_hash,
            targets: targets.into_iter().map(target_view_to_record).collect(),
            state: RootComponentDirectorySynchronizationStateRecord::Planned { planned_at_ns },
        };
        RootComponentProvisioningStore::accept_directory_synchronization(record.clone())
            .map_err(map_commit_error)?;
        status_response(&validated_record(record)?)
    }

    pub(crate) fn status(
        request: &RootComponentDirectorySynchronizationRequest,
    ) -> Result<RootComponentDirectorySynchronizationView, InternalError> {
        validate_request(request)?;
        let record =
            RootComponentProvisioningStore::directory_synchronization(request.operation_id)
                .ok_or_else(InternalError::unavailable)?;
        let view = validated_record(record)?;
        require_request_authority(&view, request)?;
        if request.expected_synchronized_component_count > view.synchronized_component_count {
            return Err(InternalError::conflict());
        }
        Ok(view)
    }

    pub(crate) fn advance(
        request: &RootComponentDirectorySynchronizationRequest,
        intent: Option<RootComponentDirectorySynchronizationIntentView>,
        started_at_ns: u64,
    ) -> Result<RootComponentDirectorySynchronizationDisposition, InternalError> {
        let view = Self::status(request)?;
        if request.expected_synchronized_component_count < view.synchronized_component_count
            || view.complete
        {
            return status_response(&view)
                .map(Box::new)
                .map(RootComponentDirectorySynchronizationDisposition::Current);
        }
        if let Some(in_flight) = view.in_flight {
            return Ok(RootComponentDirectorySynchronizationDisposition::Reconcile(
                in_flight,
            ));
        }
        if view.synchronized_component_count == target_count(&view)? {
            let terminal = terminal_record(&view, started_at_ns)?;
            RootComponentProvisioningStore::replace_directory_synchronization(
                &view_to_record(&view)?,
                terminal.clone(),
                true,
            )
            .map_err(map_commit_error)?;
            return status_response(&validated_record(terminal)?)
                .map(Box::new)
                .map(RootComponentDirectorySynchronizationDisposition::Current);
        }
        let intent = intent.ok_or_else(InternalError::invariant)?;
        validate_next_intent(&view, &intent, started_at_ns)?;
        let mut next = view_to_record(&view)?;
        next.state = RootComponentDirectorySynchronizationStateRecord::Synchronizing {
            planned_at_ns: view.planned_at_ns,
            synchronized_component_count: view.synchronized_component_count,
            in_flight: Some(Box::new(intent_view_to_record(&intent))),
        };
        RootComponentProvisioningStore::replace_directory_synchronization(
            &view_to_record(&view)?,
            next,
            false,
        )
        .map_err(map_commit_error)?;
        Ok(RootComponentDirectorySynchronizationDisposition::Invoke(
            intent,
        ))
    }

    pub(crate) fn record_synchronized(
        request: &RootComponentDirectorySynchronizationRequest,
        observed: &RootComponentDirectorySynchronizationIntentView,
        recorded_at_ns: u64,
    ) -> Result<RootComponentDirectorySynchronizationResponse, InternalError> {
        let view = Self::status(request)?;
        let in_flight = view
            .in_flight
            .as_ref()
            .ok_or_else(InternalError::conflict)?;
        if in_flight != observed || recorded_at_ns < in_flight.started_at_ns {
            return Err(InternalError::conflict());
        }
        let synchronized_component_count = view
            .synchronized_component_count
            .checked_add(1)
            .ok_or_else(InternalError::resource_exhausted)?;
        let complete = synchronized_component_count == target_count(&view)?;
        let mut next = view_to_record(&view)?;
        next.state = if complete {
            let terminal_view = RootComponentDirectorySynchronizationView {
                synchronized_component_count,
                in_flight: None,
                synchronized_at_ns: Some(recorded_at_ns),
                complete: true,
                ..view.clone()
            };
            terminal_record(&terminal_view, recorded_at_ns)?.state
        } else {
            RootComponentDirectorySynchronizationStateRecord::Synchronizing {
                planned_at_ns: view.planned_at_ns,
                synchronized_component_count,
                in_flight: None,
            }
        };
        RootComponentProvisioningStore::replace_directory_synchronization(
            &view_to_record(&view)?,
            next.clone(),
            complete,
        )
        .map_err(map_commit_error)?;
        status_response(&validated_record(next)?)
    }
}

fn validate_request(
    request: &RootComponentDirectorySynchronizationRequest,
) -> Result<(), InternalError> {
    let authority_is_present = [request.operation_id, request.plan_hash]
        .into_iter()
        .all(|value| value != [0; 32]);
    let registry_transition_is_valid = request.source_fleet_registry.authority
        == request.published_fleet_registry.authority
        && request.source_fleet_registry.revision <= request.published_fleet_registry.revision
        && request.source_fleet_registry.content_hash != [0; 32]
        && request.published_fleet_registry.content_hash != [0; 32];
    if !authority_is_present || !registry_transition_is_valid {
        return Err(InternalError::invalid_input());
    }
    Ok(())
}

fn validate_acceptance(
    root: Principal,
    fleet_directory_content_hash: [u8; 32],
    targets: &[RootComponentDirectorySynchronizationTargetView],
    planned_at_ns: u64,
) -> Result<(), InternalError> {
    let authority_is_valid = [
        root != Principal::anonymous(),
        fleet_directory_content_hash != [0; 32],
        planned_at_ns > 0,
    ]
    .into_iter()
    .all(|matches| matches);
    if !authority_is_valid {
        return Err(InternalError::invalid_input());
    }
    if targets.len() > MAX_FLEET_COMPONENT_PROVISIONING_PLAN_ENTRIES {
        return Err(InternalError::resource_exhausted());
    }
    let canonical = targets.windows(2).all(|pair| {
        pair[0].component < pair[1].component
            && pair[0].canister_id != pair[1].canister_id
            && pair[0].allocation_operation_id != pair[1].allocation_operation_id
    });
    let targets_are_valid = targets.iter().all(|target| {
        target.canister_id != Principal::anonymous()
            && target.allocation_operation_id != [0; 32]
            && target.source_registry.component == target.component
            && target.source_registry.revision > 0
            && target.source_registry.content_hash != [0; 32]
    });
    if !canonical || !targets_are_valid {
        return Err(InternalError::invalid_input());
    }
    Ok(())
}

fn require_exact_authority(
    view: &RootComponentDirectorySynchronizationView,
    request: &RootComponentDirectorySynchronizationRequest,
    root: Principal,
    fleet_directory_content_hash: [u8; 32],
    targets: &[RootComponentDirectorySynchronizationTargetView],
) -> Result<(), InternalError> {
    require_request_authority(view, request)?;
    if view.fleet_subnet_root != root
        || view.fleet_directory_content_hash != fleet_directory_content_hash
        || view.targets != targets
    {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn require_request_authority(
    view: &RootComponentDirectorySynchronizationView,
    request: &RootComponentDirectorySynchronizationRequest,
) -> Result<(), InternalError> {
    let exact = [
        view.operation_id == request.operation_id,
        view.plan_hash == request.plan_hash,
        view.source_fleet_registry == request.source_fleet_registry,
        view.published_fleet_registry == request.published_fleet_registry,
    ]
    .into_iter()
    .all(|matches| matches);
    if !exact {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn validate_next_intent(
    view: &RootComponentDirectorySynchronizationView,
    intent: &RootComponentDirectorySynchronizationIntentView,
    started_at_ns: u64,
) -> Result<(), InternalError> {
    let index = usize::try_from(view.synchronized_component_count)
        .map_err(|_| InternalError::resource_exhausted())?;
    let target = view
        .targets
        .get(index)
        .ok_or_else(InternalError::invariant)?;
    let target_is_exact = [
        intent.component_index == view.synchronized_component_count,
        intent.component == target.component,
        intent.canister_id == target.canister_id,
        intent.allocation_operation_id == target.allocation_operation_id,
        intent.previous_registry.component == target.source_registry.component,
        intent.previous_registry.revision >= target.source_registry.revision,
        intent.registry.component == target.component,
        intent.registry.revision > intent.previous_registry.revision,
        intent.registry.content_hash != intent.previous_registry.content_hash,
        intent.directory_synchronized_at_ns >= view.planned_at_ns,
        intent.directory_authority_hash != [0; 32],
        intent.started_at_ns == started_at_ns,
        started_at_ns >= view.planned_at_ns,
    ]
    .into_iter()
    .all(|matches| matches);
    if !target_is_exact {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn target_count(view: &RootComponentDirectorySynchronizationView) -> Result<u32, InternalError> {
    u32::try_from(view.targets.len()).map_err(|_| InternalError::resource_exhausted())
}

fn terminal_record(
    view: &RootComponentDirectorySynchronizationView,
    synchronized_at_ns: u64,
) -> Result<RootComponentDirectorySynchronizationRecord, InternalError> {
    if synchronized_at_ns < view.planned_at_ns
        || view.synchronized_component_count != target_count(view)?
    {
        return Err(InternalError::conflict());
    }
    let mut response = response_from_view(view)?;
    response.complete = true;
    response.synchronized_at_ns = Some(synchronized_at_ns);
    response.receipt_content_hash =
        canic_core::control_plane_support::ops::component_provisioning_receipt::RootComponentProvisioningReceiptOps::directory_synchronization_content_hash(&response)?;
    let mut record = view_to_record(view)?;
    record.state = RootComponentDirectorySynchronizationStateRecord::Synchronized {
        planned_at_ns: view.planned_at_ns,
        synchronized_at_ns,
        receipt_content_hash: response.receipt_content_hash,
    };
    Ok(record)
}

fn status_response(
    view: &RootComponentDirectorySynchronizationView,
) -> Result<RootComponentDirectorySynchronizationResponse, InternalError> {
    let response = response_from_view(view)?;
    if view.complete
        && canic_core::control_plane_support::ops::component_provisioning_receipt::RootComponentProvisioningReceiptOps::directory_synchronization_content_hash(&response)?
            != view.receipt_content_hash
    {
        return Err(InternalError::invariant());
    }
    Ok(response)
}

fn response_from_view(
    view: &RootComponentDirectorySynchronizationView,
) -> Result<RootComponentDirectorySynchronizationResponse, InternalError> {
    Ok(RootComponentDirectorySynchronizationResponse {
        operation_id: view.operation_id,
        plan_hash: view.plan_hash,
        source_fleet_registry: view.source_fleet_registry.clone(),
        published_fleet_registry: view.published_fleet_registry.clone(),
        fleet_subnet_root: view.fleet_subnet_root,
        affected_component_count: target_count(view)?,
        synchronized_component_count: view.synchronized_component_count,
        fleet_directory_content_hash: view.fleet_directory_content_hash,
        complete: view.complete,
        synchronized_at_ns: view.synchronized_at_ns,
        receipt_content_hash: view.receipt_content_hash,
    })
}

fn validated_record(
    record: RootComponentDirectorySynchronizationRecord,
) -> Result<RootComponentDirectorySynchronizationView, InternalError> {
    let (
        planned_at_ns,
        synchronized_component_count,
        in_flight,
        synchronized_at_ns,
        hash,
        complete,
    ) = match &record.state {
        RootComponentDirectorySynchronizationStateRecord::Planned { planned_at_ns } => {
            (*planned_at_ns, 0, None, None, [0; 32], false)
        }
        RootComponentDirectorySynchronizationStateRecord::Synchronizing {
            planned_at_ns,
            synchronized_component_count,
            in_flight,
        } => (
            *planned_at_ns,
            *synchronized_component_count,
            in_flight.as_deref().map(intent_record_to_view),
            None,
            [0; 32],
            false,
        ),
        RootComponentDirectorySynchronizationStateRecord::Synchronized {
            planned_at_ns,
            synchronized_at_ns,
            receipt_content_hash,
        } => (
            *planned_at_ns,
            u32::try_from(record.targets.len()).map_err(|_| InternalError::resource_exhausted())?,
            None,
            Some(*synchronized_at_ns),
            *receipt_content_hash,
            true,
        ),
    };
    let view = RootComponentDirectorySynchronizationView {
        operation_id: record.operation_id,
        plan_hash: record.plan_hash,
        source_fleet_registry: record.source_fleet_registry,
        published_fleet_registry: record.published_fleet_registry,
        fleet_subnet_root: record.fleet_subnet_root,
        fleet_directory_content_hash: record.fleet_directory_content_hash,
        targets: record
            .targets
            .into_iter()
            .map(target_record_to_view)
            .collect(),
        synchronized_component_count,
        in_flight,
        planned_at_ns,
        synchronized_at_ns,
        receipt_content_hash: hash,
        complete,
    };
    validate_acceptance(
        view.fleet_subnet_root,
        view.fleet_directory_content_hash,
        &view.targets,
        view.planned_at_ns,
    )?;
    if view.synchronized_component_count > target_count(&view)? {
        return Err(InternalError::invariant());
    }
    Ok(view)
}

fn view_to_record(
    view: &RootComponentDirectorySynchronizationView,
) -> Result<RootComponentDirectorySynchronizationRecord, InternalError> {
    let state = if view.complete {
        RootComponentDirectorySynchronizationStateRecord::Synchronized {
            planned_at_ns: view.planned_at_ns,
            synchronized_at_ns: view
                .synchronized_at_ns
                .ok_or_else(InternalError::invariant)?,
            receipt_content_hash: view.receipt_content_hash,
        }
    } else if view.synchronized_component_count == 0 && view.in_flight.is_none() {
        RootComponentDirectorySynchronizationStateRecord::Planned {
            planned_at_ns: view.planned_at_ns,
        }
    } else {
        RootComponentDirectorySynchronizationStateRecord::Synchronizing {
            planned_at_ns: view.planned_at_ns,
            synchronized_component_count: view.synchronized_component_count,
            in_flight: view
                .in_flight
                .as_ref()
                .map(intent_view_to_record)
                .map(Box::new),
        }
    };
    Ok(RootComponentDirectorySynchronizationRecord {
        operation_id: view.operation_id,
        plan_hash: view.plan_hash,
        source_fleet_registry: view.source_fleet_registry.clone(),
        published_fleet_registry: view.published_fleet_registry.clone(),
        fleet_subnet_root: view.fleet_subnet_root,
        fleet_directory_content_hash: view.fleet_directory_content_hash,
        targets: view
            .targets
            .iter()
            .cloned()
            .map(target_view_to_record)
            .collect(),
        state,
    })
}

const fn target_view_to_record(
    target: RootComponentDirectorySynchronizationTargetView,
) -> RootComponentDirectorySynchronizationTargetRecord {
    RootComponentDirectorySynchronizationTargetRecord {
        component: target.component,
        canister_id: target.canister_id,
        allocation_operation_id: target.allocation_operation_id,
        source_registry: target.source_registry,
    }
}

const fn target_record_to_view(
    target: RootComponentDirectorySynchronizationTargetRecord,
) -> RootComponentDirectorySynchronizationTargetView {
    RootComponentDirectorySynchronizationTargetView {
        component: target.component,
        canister_id: target.canister_id,
        allocation_operation_id: target.allocation_operation_id,
        source_registry: target.source_registry,
    }
}

fn intent_view_to_record(
    intent: &RootComponentDirectorySynchronizationIntentView,
) -> RootComponentDirectorySynchronizationIntentRecord {
    RootComponentDirectorySynchronizationIntentRecord {
        component_index: intent.component_index,
        component: intent.component,
        canister_id: intent.canister_id,
        allocation_operation_id: intent.allocation_operation_id,
        previous_registry: intent.previous_registry.clone(),
        registry: intent.registry.clone(),
        directory_synchronized_at_ns: intent.directory_synchronized_at_ns,
        directory_authority_hash: intent.directory_authority_hash,
        started_at_ns: intent.started_at_ns,
    }
}

fn intent_record_to_view(
    intent: &RootComponentDirectorySynchronizationIntentRecord,
) -> RootComponentDirectorySynchronizationIntentView {
    RootComponentDirectorySynchronizationIntentView {
        component_index: intent.component_index,
        component: intent.component,
        canister_id: intent.canister_id,
        allocation_operation_id: intent.allocation_operation_id,
        previous_registry: intent.previous_registry.clone(),
        registry: intent.registry.clone(),
        directory_synchronized_at_ns: intent.directory_synchronized_at_ns,
        directory_authority_hash: intent.directory_authority_hash,
        started_at_ns: intent.started_at_ns,
    }
}

const fn map_commit_error(error: RootComponentProvisioningCommitError) -> InternalError {
    match error {
        RootComponentProvisioningCommitError::ActiveOperationConflict => {
            InternalError::public(canic_core::diagnostics::codes::REQUEST_UNEXPECTED_STATE)
        }
        RootComponentProvisioningCommitError::ConflictingOperation => {
            InternalError::public(canic_core::diagnostics::codes::REQUEST_CONFLICT)
        }
        RootComponentProvisioningCommitError::OperationChanged => {
            InternalError::public(canic_core::diagnostics::codes::AUTHORITY_CONFLICT)
        }
        RootComponentProvisioningCommitError::PlacementConflict => {
            InternalError::public(canic_core::diagnostics::codes::POSITION_CONFLICT)
        }
        RootComponentProvisioningCommitError::PlacementCountOverflow => {
            InternalError::public(canic_core::diagnostics::codes::CAPACITY_LIMIT)
        }
    }
}
