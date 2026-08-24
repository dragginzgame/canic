//! Module: domain::policy::pure::fleet_admission_root
//!
//! Responsibility: decide one Root journal's ordered prepare/activate/open transitions.
//! Does not own: hashing, storage, participant discovery, calls, timers, DTOs, or serialization.
//! Boundary: workflow supplies exact request and receipt hashes and commits returned state.

use crate::model::fleet_admission_root::{
    FLEET_ADMISSION_ROOT_SCHEMA_VERSION, FleetAdmissionRootParticipantModel,
    FleetAdmissionRootParticipantPhaseModel, FleetAdmissionRootPhaseModel,
    FleetAdmissionRootPrepareRequestModel, FleetAdmissionRootReleasedReservationModel,
    FleetAdmissionRootRetainedResultModel, FleetAdmissionRootState,
    FleetAdmissionRootTransitionModel, MAX_FLEET_ADMISSION_ROOT_PARTICIPANTS,
};
use thiserror::Error as ThisError;

/// Root journal invariant or ordered-transition rejection.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ThisError)]
pub enum FleetAdmissionRootTransitionError {
    #[error("Fleet admission Root state is invalid")]
    InvalidState,
    #[error("Fleet admission Root operation identity conflicts")]
    OperationConflict,
    #[error("Fleet admission Root transition phase conflicts")]
    PhaseConflict,
    #[error("Fleet admission Root participant capacity is exhausted")]
    ParticipantCapacity,
    #[error("Fleet admission Root participant receipt is invalid")]
    ReceiptConflict,
}

/// Result of beginning or replaying one Root prepare command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetAdmissionRootPrepareDecision {
    pub state: FleetAdmissionRootState,
    pub receipt_hash: [u8; 32],
    pub replayed: bool,
}

/// Begin, resume or exactly replay one Coordinator-authored Root transition.
pub fn prepare_fleet_admission_root(
    state: &FleetAdmissionRootState,
    request: FleetAdmissionRootPrepareRequestModel,
    participant_catalog_digest: [u8; 32],
    reservation_receipt_hash: [u8; 32],
    participants: Vec<FleetAdmissionRootParticipantModel>,
) -> Result<FleetAdmissionRootPrepareDecision, FleetAdmissionRootTransitionError> {
    validate_fleet_admission_root_state(state)?;
    if let Some(current) = &state.current_transition {
        if current.request.operation_id != request.operation_id
            || current.request.request_hash != request.request_hash
        {
            return Err(FleetAdmissionRootTransitionError::OperationConflict);
        }
        return Ok(FleetAdmissionRootPrepareDecision {
            state: state.clone(),
            receipt_hash: reservation_receipt_hash,
            replayed: true,
        });
    }
    if let Some(last) = &state.last_result
        && last.request.operation_id == request.operation_id
    {
        if last.request.request_hash != request.request_hash {
            return Err(FleetAdmissionRootTransitionError::OperationConflict);
        }
        return Ok(FleetAdmissionRootPrepareDecision {
            state: state.clone(),
            receipt_hash: reservation_receipt_hash,
            replayed: true,
        });
    }
    if let Some(last) = &state.last_release
        && last.request.operation_id == request.operation_id
    {
        if last.request.request_hash != request.request_hash {
            return Err(FleetAdmissionRootTransitionError::OperationConflict);
        }
        return Ok(FleetAdmissionRootPrepareDecision {
            state: state.clone(),
            receipt_hash: reservation_receipt_hash,
            replayed: true,
        });
    }
    if participants.len() > MAX_FLEET_ADMISSION_ROOT_PARTICIPANTS {
        return Err(FleetAdmissionRootTransitionError::ParticipantCapacity);
    }
    if request.operation_id == [0; 32]
        || request.request_hash == [0; 32]
        || participant_catalog_digest == [0; 32]
        || reservation_receipt_hash == [0; 32]
        || request.authority != request.root.authority.binding
        || request.authority.fleet != state.active_policy.fleet
        || request.expected_generation != state.active_policy.generation
        || request.expected_policy_digest != state.active_policy.policy_digest
        || request.successor.fleet != state.active_policy.fleet
        || request.successor.generation
            != state.active_policy.generation.checked_add(1).unwrap_or(0)
    {
        return Err(FleetAdmissionRootTransitionError::InvalidState);
    }
    validate_initial_participants(&participants)?;
    let mut next = state.clone();
    next.current_transition = Some(FleetAdmissionRootTransitionModel {
        request,
        phase: FleetAdmissionRootPhaseModel::Preparing,
        participant_catalog_digest,
        participants,
        fence_request_hash: None,
        prepare_receipt_hash: None,
        activate_request_hash: None,
        activate_receipt_hash: None,
        open_request_hash: None,
    });
    validate_fleet_admission_root_state(&next)?;
    Ok(FleetAdmissionRootPrepareDecision {
        state: next,
        receipt_hash: reservation_receipt_hash,
        replayed: false,
    })
}

/// Authorize target fencing only after every Root catalog reservation is accepted.
pub fn fence_fleet_admission_root(
    state: &FleetAdmissionRootState,
    operation_id: [u8; 32],
    request_hash: [u8; 32],
    aggregate_prepare_receipt_hash: [u8; 32],
) -> Result<(FleetAdmissionRootState, Option<[u8; 32]>), FleetAdmissionRootTransitionError> {
    validate_fleet_admission_root_state(state)?;
    let mut next = state.clone();
    let current = next
        .current_transition
        .as_mut()
        .ok_or(FleetAdmissionRootTransitionError::PhaseConflict)?;
    if current.request.operation_id != operation_id
        || request_hash == [0; 32]
        || aggregate_prepare_receipt_hash == [0; 32]
    {
        return Err(FleetAdmissionRootTransitionError::OperationConflict);
    }
    match current.fence_request_hash {
        Some(existing) if existing != request_hash => {
            return Err(FleetAdmissionRootTransitionError::OperationConflict);
        }
        Some(_) => return Ok((state.clone(), current.prepare_receipt_hash)),
        None if current.phase == FleetAdmissionRootPhaseModel::Preparing => {
            current.fence_request_hash = Some(request_hash);
            if current.participants.is_empty() {
                current.phase = FleetAdmissionRootPhaseModel::PerimeterFenced;
                current.prepare_receipt_hash = Some(aggregate_prepare_receipt_hash);
            }
        }
        None => return Err(FleetAdmissionRootTransitionError::PhaseConflict),
    }
    let receipt_hash = current.prepare_receipt_hash;
    validate_fleet_admission_root_state(&next)?;
    Ok((next, receipt_hash))
}

/// Release one catalog reservation before any target preparation effect begins.
pub fn release_fleet_admission_root(
    state: &FleetAdmissionRootState,
    operation_id: [u8; 32],
    request_hash: [u8; 32],
    receipt_hash: [u8; 32],
) -> Result<(FleetAdmissionRootState, [u8; 32]), FleetAdmissionRootTransitionError> {
    validate_fleet_admission_root_state(state)?;
    if let Some(last) = &state.last_release
        && last.request.operation_id == operation_id
    {
        return if last.release_request_hash == request_hash && last.receipt_hash == receipt_hash {
            Ok((state.clone(), last.receipt_hash))
        } else {
            Err(FleetAdmissionRootTransitionError::OperationConflict)
        };
    }
    let current = state
        .current_transition
        .as_ref()
        .ok_or(FleetAdmissionRootTransitionError::PhaseConflict)?;
    if current.request.operation_id != operation_id
        || request_hash == [0; 32]
        || receipt_hash == [0; 32]
        || current.phase != FleetAdmissionRootPhaseModel::Preparing
        || current.fence_request_hash.is_some()
        || current.prepare_receipt_hash.is_some()
        || current.participants.iter().any(|participant| {
            participant.phase != FleetAdmissionRootParticipantPhaseModel::Pending
        })
    {
        return Err(FleetAdmissionRootTransitionError::OperationConflict);
    }
    let mut next = state.clone();
    next.current_transition = None;
    next.last_result = None;
    next.last_release = Some(FleetAdmissionRootReleasedReservationModel {
        request: current.request.clone(),
        participant_catalog_digest: current.participant_catalog_digest,
        participant_count: u32::try_from(current.participants.len())
            .map_err(|_| FleetAdmissionRootTransitionError::ParticipantCapacity)?,
        release_request_hash: request_hash,
        receipt_hash,
    });
    validate_fleet_admission_root_state(&next)?;
    Ok((next, receipt_hash))
}

/// Start or exactly replay the aggregate activation phase.
pub fn activate_fleet_admission_root(
    state: &FleetAdmissionRootState,
    operation_id: [u8; 32],
    request_hash: [u8; 32],
    aggregate_activate_receipt_hash: [u8; 32],
) -> Result<(FleetAdmissionRootState, Option<[u8; 32]>), FleetAdmissionRootTransitionError> {
    validate_fleet_admission_root_state(state)?;
    let mut next = state.clone();
    let current = next
        .current_transition
        .as_mut()
        .ok_or(FleetAdmissionRootTransitionError::PhaseConflict)?;
    if current.request.operation_id != operation_id
        || request_hash == [0; 32]
        || aggregate_activate_receipt_hash == [0; 32]
    {
        return Err(FleetAdmissionRootTransitionError::OperationConflict);
    }
    match current.activate_request_hash {
        Some(existing) if existing != request_hash => {
            return Err(FleetAdmissionRootTransitionError::OperationConflict);
        }
        Some(_) => return Ok((state.clone(), current.activate_receipt_hash)),
        None if current.phase == FleetAdmissionRootPhaseModel::PerimeterFenced => {
            current.activate_request_hash = Some(request_hash);
            if current.participants.is_empty() {
                current.phase = FleetAdmissionRootPhaseModel::Opening;
                current.activate_receipt_hash = Some(aggregate_activate_receipt_hash);
            } else {
                current.phase = FleetAdmissionRootPhaseModel::Activating;
            }
        }
        None => return Err(FleetAdmissionRootTransitionError::PhaseConflict),
    }
    let receipt_hash = current.activate_receipt_hash;
    validate_fleet_admission_root_state(&next)?;
    Ok((next, receipt_hash))
}

/// Start or exactly replay the aggregate open phase.
pub fn open_fleet_admission_root(
    state: &FleetAdmissionRootState,
    operation_id: [u8; 32],
    request_hash: [u8; 32],
) -> Result<Option<FleetAdmissionRootState>, FleetAdmissionRootTransitionError> {
    validate_fleet_admission_root_state(state)?;
    if let Some(last) = &state.last_result
        && last.request.operation_id == operation_id
    {
        return if last.open_request_hash == request_hash {
            Ok(None)
        } else {
            Err(FleetAdmissionRootTransitionError::OperationConflict)
        };
    }
    let mut next = state.clone();
    let current = next
        .current_transition
        .as_mut()
        .ok_or(FleetAdmissionRootTransitionError::PhaseConflict)?;
    if current.request.operation_id != operation_id || request_hash == [0; 32] {
        return Err(FleetAdmissionRootTransitionError::OperationConflict);
    }
    match current.open_request_hash {
        Some(existing) if existing != request_hash => {
            Err(FleetAdmissionRootTransitionError::OperationConflict)
        }
        Some(_) => Ok(Some(state.clone())),
        None if current.phase == FleetAdmissionRootPhaseModel::Opening => {
            current.open_request_hash = Some(request_hash);
            Ok(Some(next))
        }
        None => Err(FleetAdmissionRootTransitionError::PhaseConflict),
    }
}

/// Retain one exact target phase receipt and monotonically advance aggregate state.
pub fn record_fleet_admission_root_participant(
    state: &FleetAdmissionRootState,
    operation_id: [u8; 32],
    target: &crate::ids::ManagedCanisterBinding,
    expected_phase: FleetAdmissionRootParticipantPhaseModel,
    receipt_hash: [u8; 32],
    aggregate_receipt_hash: [u8; 32],
) -> Result<FleetAdmissionRootState, FleetAdmissionRootTransitionError> {
    validate_fleet_admission_root_state(state)?;
    if receipt_hash == [0; 32] || aggregate_receipt_hash == [0; 32] {
        return Err(FleetAdmissionRootTransitionError::ReceiptConflict);
    }
    let mut next = state.clone();
    let current = next
        .current_transition
        .as_mut()
        .ok_or(FleetAdmissionRootTransitionError::PhaseConflict)?;
    if current.request.operation_id != operation_id {
        return Err(FleetAdmissionRootTransitionError::OperationConflict);
    }
    let participant = current
        .participants
        .iter_mut()
        .find(|participant| &participant.target == target)
        .ok_or(FleetAdmissionRootTransitionError::ReceiptConflict)?;
    let predecessor = match expected_phase {
        FleetAdmissionRootParticipantPhaseModel::Pending => {
            return Err(FleetAdmissionRootTransitionError::ReceiptConflict);
        }
        FleetAdmissionRootParticipantPhaseModel::Prepared => {
            FleetAdmissionRootParticipantPhaseModel::Pending
        }
        FleetAdmissionRootParticipantPhaseModel::Activated => {
            FleetAdmissionRootParticipantPhaseModel::Prepared
        }
        FleetAdmissionRootParticipantPhaseModel::Open => {
            FleetAdmissionRootParticipantPhaseModel::Activated
        }
    };
    if participant.phase == expected_phase {
        return if participant.last_receipt_hash == Some(receipt_hash) {
            Ok(state.clone())
        } else {
            Err(FleetAdmissionRootTransitionError::ReceiptConflict)
        };
    }
    if participant.phase != predecessor {
        return Err(FleetAdmissionRootTransitionError::PhaseConflict);
    }
    participant.phase = expected_phase;
    participant.last_receipt_hash = Some(receipt_hash);
    advance_aggregate(current, aggregate_receipt_hash);
    validate_fleet_admission_root_state(&next)?;
    Ok(next)
}

/// Move a fully opened operation into the retained terminal slot.
pub fn complete_fleet_admission_root(
    state: &FleetAdmissionRootState,
    receipt_hash: [u8; 32],
) -> Result<FleetAdmissionRootState, FleetAdmissionRootTransitionError> {
    validate_fleet_admission_root_state(state)?;
    let current = state
        .current_transition
        .as_ref()
        .ok_or(FleetAdmissionRootTransitionError::PhaseConflict)?;
    if receipt_hash == [0; 32]
        || current.phase != FleetAdmissionRootPhaseModel::Opening
        || current.open_request_hash.is_none()
        || current
            .participants
            .iter()
            .any(|participant| participant.phase != FleetAdmissionRootParticipantPhaseModel::Open)
    {
        return Err(FleetAdmissionRootTransitionError::PhaseConflict);
    }
    let result = FleetAdmissionRootRetainedResultModel {
        request: current.request.clone(),
        participant_catalog_digest: current.participant_catalog_digest,
        participants: current.participants.clone(),
        fence_request_hash: current
            .fence_request_hash
            .ok_or(FleetAdmissionRootTransitionError::InvalidState)?,
        prepare_receipt_hash: current
            .prepare_receipt_hash
            .ok_or(FleetAdmissionRootTransitionError::InvalidState)?,
        activate_request_hash: current
            .activate_request_hash
            .ok_or(FleetAdmissionRootTransitionError::InvalidState)?,
        activate_receipt_hash: current
            .activate_receipt_hash
            .ok_or(FleetAdmissionRootTransitionError::InvalidState)?,
        open_request_hash: current
            .open_request_hash
            .ok_or(FleetAdmissionRootTransitionError::InvalidState)?,
        receipt_hash,
    };
    let mut next = state.clone();
    next.active_policy = current.request.successor.clone();
    next.current_transition = None;
    next.last_result = Some(result);
    next.last_release = None;
    validate_fleet_admission_root_state(&next)?;
    Ok(next)
}

/// Validate all bounded relationships without reading ambient authority.
pub fn validate_fleet_admission_root_state(
    state: &FleetAdmissionRootState,
) -> Result<(), FleetAdmissionRootTransitionError> {
    if state.schema_version != FLEET_ADMISSION_ROOT_SCHEMA_VERSION {
        return Err(FleetAdmissionRootTransitionError::InvalidState);
    }
    if let Some(current) = &state.current_transition {
        if current.request.operation_id == [0; 32]
            || current.request.request_hash == [0; 32]
            || current.participant_catalog_digest == [0; 32]
            || current.request.authority != current.request.root.authority.binding
            || current.request.authority.fleet != state.active_policy.fleet
            || current.request.expected_generation != state.active_policy.generation
            || current.request.expected_policy_digest != state.active_policy.policy_digest
            || current.request.successor.fleet != state.active_policy.fleet
            || current.request.successor.generation
                != state.active_policy.generation.checked_add(1).unwrap_or(0)
        {
            return Err(FleetAdmissionRootTransitionError::InvalidState);
        }
        validate_participants(current)?;
    }
    if let Some(last) = &state.last_release
        && (last.request.operation_id == [0; 32]
            || last.request.request_hash == [0; 32]
            || last.participant_catalog_digest == [0; 32]
            || usize::try_from(last.participant_count)
                .map_or(true, |count| count > MAX_FLEET_ADMISSION_ROOT_PARTICIPANTS)
            || last.release_request_hash == [0; 32]
            || last.receipt_hash == [0; 32]
            || last.request.authority != last.request.root.authority.binding
            || last.request.authority.fleet != state.active_policy.fleet
            || last.request.expected_generation != state.active_policy.generation
            || last.request.expected_policy_digest != state.active_policy.policy_digest
            || last.request.successor.fleet != state.active_policy.fleet
            || last.request.successor.generation
                != state.active_policy.generation.checked_add(1).unwrap_or(0))
    {
        return Err(FleetAdmissionRootTransitionError::InvalidState);
    }
    if let Some(last) = &state.last_result
        && (last.request.operation_id == [0; 32]
            || last.request.successor != state.active_policy
            || last.participant_catalog_digest == [0; 32]
            || last.fence_request_hash == [0; 32]
            || last.prepare_receipt_hash == [0; 32]
            || last.activate_request_hash == [0; 32]
            || last.activate_receipt_hash == [0; 32]
            || last.open_request_hash == [0; 32]
            || last.receipt_hash == [0; 32]
            || last.participants.len() > MAX_FLEET_ADMISSION_ROOT_PARTICIPANTS
            || last.participants.iter().any(|participant| {
                participant.phase != FleetAdmissionRootParticipantPhaseModel::Open
                    || participant.last_receipt_hash.is_none()
            }))
    {
        return Err(FleetAdmissionRootTransitionError::InvalidState);
    }
    if state.current_transition.as_ref().is_some_and(|current| {
        state
            .last_result
            .as_ref()
            .is_some_and(|last| current.request.operation_id == last.request.operation_id)
    }) {
        return Err(FleetAdmissionRootTransitionError::InvalidState);
    }
    if state.last_result.is_some() && state.last_release.is_some() {
        return Err(FleetAdmissionRootTransitionError::InvalidState);
    }
    if state.current_transition.as_ref().is_some_and(|current| {
        state
            .last_release
            .as_ref()
            .is_some_and(|last| current.request.operation_id == last.request.operation_id)
    }) || state.last_result.as_ref().is_some_and(|result| {
        state
            .last_release
            .as_ref()
            .is_some_and(|last| result.request.operation_id == last.request.operation_id)
    }) {
        return Err(FleetAdmissionRootTransitionError::InvalidState);
    }
    Ok(())
}

fn advance_aggregate(
    current: &mut FleetAdmissionRootTransitionModel,
    aggregate_receipt_hash: [u8; 32],
) {
    let all = |phase| {
        current
            .participants
            .iter()
            .all(|participant| participant.phase == phase)
    };
    match current.phase {
        FleetAdmissionRootPhaseModel::Preparing
            if all(FleetAdmissionRootParticipantPhaseModel::Prepared) =>
        {
            current.phase = FleetAdmissionRootPhaseModel::PerimeterFenced;
            current.prepare_receipt_hash = Some(aggregate_receipt_hash);
        }
        FleetAdmissionRootPhaseModel::Activating
            if all(FleetAdmissionRootParticipantPhaseModel::Activated) =>
        {
            current.phase = FleetAdmissionRootPhaseModel::Opening;
            current.activate_receipt_hash = Some(aggregate_receipt_hash);
        }
        _ => {}
    }
}

fn validate_initial_participants(
    participants: &[FleetAdmissionRootParticipantModel],
) -> Result<(), FleetAdmissionRootTransitionError> {
    if participants.iter().any(|participant| {
        participant.phase != FleetAdmissionRootParticipantPhaseModel::Pending
            || participant.last_receipt_hash.is_some()
            || participant.projection_digest == [0; 32]
    }) || participants.windows(2).any(|pair| {
        target_principal(&pair[0].target).as_slice() >= target_principal(&pair[1].target).as_slice()
    }) {
        return Err(FleetAdmissionRootTransitionError::InvalidState);
    }
    Ok(())
}

fn validate_participants(
    current: &FleetAdmissionRootTransitionModel,
) -> Result<(), FleetAdmissionRootTransitionError> {
    if current.participants.len() > MAX_FLEET_ADMISSION_ROOT_PARTICIPANTS
        || current.participants.windows(2).any(|pair| {
            target_principal(&pair[0].target).as_slice()
                >= target_principal(&pair[1].target).as_slice()
        })
    {
        return Err(FleetAdmissionRootTransitionError::InvalidState);
    }
    let phases_valid = current.participants.iter().all(|participant| {
        participant.projection_digest != [0; 32]
            && (participant.phase == FleetAdmissionRootParticipantPhaseModel::Pending
                && participant.last_receipt_hash.is_none()
                || participant.phase != FleetAdmissionRootParticipantPhaseModel::Pending
                    && participant.last_receipt_hash.is_some())
            && match current.phase {
                FleetAdmissionRootPhaseModel::Preparing => matches!(
                    participant.phase,
                    FleetAdmissionRootParticipantPhaseModel::Pending
                        | FleetAdmissionRootParticipantPhaseModel::Prepared
                ),
                FleetAdmissionRootPhaseModel::PerimeterFenced => {
                    participant.phase == FleetAdmissionRootParticipantPhaseModel::Prepared
                }
                FleetAdmissionRootPhaseModel::Activating => matches!(
                    participant.phase,
                    FleetAdmissionRootParticipantPhaseModel::Prepared
                        | FleetAdmissionRootParticipantPhaseModel::Activated
                ),
                FleetAdmissionRootPhaseModel::Opening => matches!(
                    participant.phase,
                    FleetAdmissionRootParticipantPhaseModel::Activated
                        | FleetAdmissionRootParticipantPhaseModel::Open
                ),
            }
    });
    if !phases_valid
        || current.fence_request_hash.is_none()
            && (current.phase != FleetAdmissionRootPhaseModel::Preparing
                || current.prepare_receipt_hash.is_some()
                || current.participants.iter().any(|participant| {
                    participant.phase != FleetAdmissionRootParticipantPhaseModel::Pending
                }))
        || current.phase == FleetAdmissionRootPhaseModel::PerimeterFenced
            && current.prepare_receipt_hash.is_none()
        || matches!(
            current.phase,
            FleetAdmissionRootPhaseModel::Activating | FleetAdmissionRootPhaseModel::Opening
        ) && (current.prepare_receipt_hash.is_none() || current.activate_request_hash.is_none())
        || current.phase == FleetAdmissionRootPhaseModel::Opening
            && current.activate_receipt_hash.is_none()
    {
        return Err(FleetAdmissionRootTransitionError::InvalidState);
    }
    Ok(())
}

const fn target_principal(target: &crate::ids::ManagedCanisterBinding) -> candid::Principal {
    match target {
        crate::ids::ManagedCanisterBinding::Component(component) => component.canister_id,
        crate::ids::ManagedCanisterBinding::ComponentChild(child) => child.canister_id,
    }
}

#[cfg(test)]
mod tests;
