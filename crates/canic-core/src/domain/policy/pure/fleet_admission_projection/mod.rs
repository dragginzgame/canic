//! Module: domain::policy::pure::fleet_admission_projection
//!
//! Responsibility: decide target-local projection phase transitions.
//! Does not own: hashing, storage, caller acquisition, or distributed orchestration.
//! Boundary: workflow commits only the complete state returned here.

use crate::model::fleet_admission_projection::{
    FleetAdmissionProjectionPhaseModel, FleetAdmissionProjectionReceiptModel,
    FleetAdmissionProjectionState, FleetAdmissionProjectionValidationError,
    FleetAdmissionTargetTransitionPhaseModel, FleetAdmissionTargetTransitionRequestModel,
    validate_fleet_admission_projection_state,
};
use thiserror::Error as ThisError;

/// Complete local decision for fresh activation or its exact replay.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenFreshFleetAdmissionProjectionDecision {
    pub state: FleetAdmissionProjectionState,
    pub transitioned: bool,
}

/// Target-local phase or replay invariant rejected before stable replacement.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ThisError)]
pub enum FleetAdmissionTargetTransitionError {
    #[error("Fleet admission target operation ID is all zero")]
    EmptyOperationId,
    #[error("Fleet admission target operation ID was reused for another request")]
    OperationConflict,
    #[error("Fleet admission target transition phase is out of order")]
    PhaseConflict,
    #[error("Fleet admission target successor does not match")]
    SuccessorConflict,
    #[error("Fleet admission target projection state is invalid")]
    InvalidState,
}

/// Complete target-local state and exact receipt produced by one phase decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetAdmissionTargetTransitionDecision {
    pub state: FleetAdmissionProjectionState,
    pub receipt: FleetAdmissionProjectionReceiptModel,
    pub replayed: bool,
}

/// Apply or exactly replay one ordered prepare/activate/open phase.
pub fn transition_fleet_admission_projection(
    state: &FleetAdmissionProjectionState,
    request: FleetAdmissionTargetTransitionRequestModel,
) -> Result<FleetAdmissionTargetTransitionDecision, FleetAdmissionTargetTransitionError> {
    validate_fleet_admission_projection_state(state)
        .map_err(|_error| FleetAdmissionTargetTransitionError::InvalidState)?;
    if request.operation_id == [0; 32] {
        return Err(FleetAdmissionTargetTransitionError::EmptyOperationId);
    }
    if request.request_hash == [0; 32] || request.receipt_hash == [0; 32] {
        return Err(FleetAdmissionTargetTransitionError::InvalidState);
    }
    if let Some(replay) = replay_target_transition(state, &request)? {
        return Ok(replay);
    }

    let predecessor_matches = state.active.generation == request.expected_generation
        && state.active.policy_digest == request.expected_policy_digest;
    let expected_successor_generation = match request.phase {
        FleetAdmissionTargetTransitionPhaseModel::Prepare
        | FleetAdmissionTargetTransitionPhaseModel::Activate => request
            .expected_generation
            .checked_add(1)
            .ok_or(FleetAdmissionTargetTransitionError::SuccessorConflict)?,
        FleetAdmissionTargetTransitionPhaseModel::Open => request.expected_generation,
    };
    let successor_matches = request.successor.authority == state.active.authority
        && request.successor.target == state.active.target
        && request.successor.generation == expected_successor_generation;
    if !successor_matches {
        return Err(FleetAdmissionTargetTransitionError::SuccessorConflict);
    }

    let receipt = FleetAdmissionProjectionReceiptModel {
        operation_id: request.operation_id,
        phase: request.phase,
        request_hash: request.request_hash,
        receipt_hash: request.receipt_hash,
    };
    let mut next = state.clone();
    match request.phase {
        FleetAdmissionTargetTransitionPhaseModel::Prepare => {
            if !predecessor_matches
                || state.phase != FleetAdmissionProjectionPhaseModel::Open
                || state.prepared.is_some()
            {
                return Err(FleetAdmissionTargetTransitionError::PhaseConflict);
            }
            next.prepared = Some(request.successor);
            next.phase = FleetAdmissionProjectionPhaseModel::Fenced;
        }
        FleetAdmissionTargetTransitionPhaseModel::Activate => {
            if !predecessor_matches
                || state.phase != FleetAdmissionProjectionPhaseModel::Fenced
                || state.prepared.as_ref() != Some(&request.successor)
                || state.last_receipt.as_ref().is_none_or(|last| {
                    last.operation_id != request.operation_id
                        || last.phase != FleetAdmissionTargetTransitionPhaseModel::Prepare
                })
            {
                return Err(FleetAdmissionTargetTransitionError::PhaseConflict);
            }
            next.active = request.successor;
            next.prepared = None;
        }
        FleetAdmissionTargetTransitionPhaseModel::Open => {
            if state.active != request.successor
                || state.phase != FleetAdmissionProjectionPhaseModel::Fenced
                || state.prepared.is_some()
                || state.last_receipt.as_ref().is_none_or(|last| {
                    last.operation_id != request.operation_id
                        || last.phase != FleetAdmissionTargetTransitionPhaseModel::Activate
                })
            {
                return Err(FleetAdmissionTargetTransitionError::PhaseConflict);
            }
            next.phase = FleetAdmissionProjectionPhaseModel::Open;
        }
    }
    next.last_receipt = Some(receipt.clone());
    validate_fleet_admission_projection_state(&next)
        .map_err(|_error| FleetAdmissionTargetTransitionError::InvalidState)?;
    Ok(FleetAdmissionTargetTransitionDecision {
        state: next,
        receipt,
        replayed: false,
    })
}

fn replay_target_transition(
    state: &FleetAdmissionProjectionState,
    request: &FleetAdmissionTargetTransitionRequestModel,
) -> Result<Option<FleetAdmissionTargetTransitionDecision>, FleetAdmissionTargetTransitionError> {
    let Some(receipt) = state
        .last_receipt
        .as_ref()
        .filter(|receipt| receipt.operation_id == request.operation_id)
    else {
        return Ok(None);
    };
    if receipt.phase == request.phase {
        if receipt.request_hash == request.request_hash
            && receipt.receipt_hash == request.receipt_hash
        {
            return Ok(Some(FleetAdmissionTargetTransitionDecision {
                state: state.clone(),
                receipt: receipt.clone(),
                replayed: true,
            }));
        }
        return Err(FleetAdmissionTargetTransitionError::OperationConflict);
    }
    let next_phase = matches!(
        (receipt.phase, request.phase),
        (
            FleetAdmissionTargetTransitionPhaseModel::Prepare,
            FleetAdmissionTargetTransitionPhaseModel::Activate
        ) | (
            FleetAdmissionTargetTransitionPhaseModel::Activate,
            FleetAdmissionTargetTransitionPhaseModel::Open
        )
    );
    if !next_phase {
        return Err(FleetAdmissionTargetTransitionError::OperationConflict);
    }
    Ok(None)
}

/// Open the exact fresh projection without creating transition authority.
pub fn open_fresh_fleet_admission_projection(
    state: &FleetAdmissionProjectionState,
) -> Result<OpenFreshFleetAdmissionProjectionDecision, FleetAdmissionProjectionValidationError> {
    validate_fleet_admission_projection_state(state)?;
    if state.prepared.is_some() || state.last_receipt.is_some() {
        return Err(FleetAdmissionProjectionValidationError::OpenStateInvalid);
    }
    if state.phase == FleetAdmissionProjectionPhaseModel::Open {
        return Ok(OpenFreshFleetAdmissionProjectionDecision {
            state: state.clone(),
            transitioned: false,
        });
    }
    let mut next = state.clone();
    next.phase = FleetAdmissionProjectionPhaseModel::Open;
    Ok(OpenFreshFleetAdmissionProjectionDecision {
        state: next,
        transitioned: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_open_is_monotonic_and_exactly_replayable() {
        let projection = crate::test::support::fleet_admission_projection(
            crate::test::support::managed_component_binding(),
        );
        let state = FleetAdmissionProjectionState {
            schema_version: 1,
            active: projection,
            prepared: None,
            phase: FleetAdmissionProjectionPhaseModel::Fenced,
            last_receipt: None,
        };

        let opened = open_fresh_fleet_admission_projection(&state).expect("fresh open");
        assert!(opened.transitioned);
        assert_eq!(opened.state.phase, FleetAdmissionProjectionPhaseModel::Open);

        let replay =
            open_fresh_fleet_admission_projection(&opened.state).expect("exact open replay");
        assert!(!replay.transitioned);
        assert_eq!(replay.state, opened.state);
    }

    #[test]
    fn target_transition_fences_activates_opens_and_replays_exactly() {
        let active = crate::test::support::fleet_admission_projection(
            crate::test::support::managed_component_binding(),
        );
        let mut successor = active.clone();
        successor.generation += 1;
        successor.policy_digest = [0xa1; 32];
        successor.projection_digest = [0xa2; 32];
        let initial = FleetAdmissionProjectionState {
            schema_version: 1,
            active: active.clone(),
            prepared: None,
            phase: FleetAdmissionProjectionPhaseModel::Open,
            last_receipt: None,
        };

        let prepare = request(
            FleetAdmissionTargetTransitionPhaseModel::Prepare,
            &active,
            successor.clone(),
            1,
        );
        let prepared = transition_fleet_admission_projection(&initial, prepare.clone())
            .expect("prepare target");
        assert_eq!(prepared.state.prepared, Some(successor.clone()));
        assert_eq!(
            prepared.state.phase,
            FleetAdmissionProjectionPhaseModel::Fenced
        );
        assert!(
            transition_fleet_admission_projection(&prepared.state, prepare)
                .expect("replay prepare")
                .replayed
        );

        let activate = request(
            FleetAdmissionTargetTransitionPhaseModel::Activate,
            &active,
            successor.clone(),
            2,
        );
        let activated = transition_fleet_admission_projection(&prepared.state, activate)
            .expect("activate target");
        assert_eq!(activated.state.active, successor);
        assert!(activated.state.prepared.is_none());
        assert_eq!(
            activated.state.phase,
            FleetAdmissionProjectionPhaseModel::Fenced
        );

        let successor_active = successor.clone();
        let open = request(
            FleetAdmissionTargetTransitionPhaseModel::Open,
            &successor_active,
            successor,
            3,
        );
        let opened =
            transition_fleet_admission_projection(&activated.state, open).expect("open target");
        assert_eq!(opened.state.phase, FleetAdmissionProjectionPhaseModel::Open);
    }

    #[test]
    fn target_transition_rejects_reordering_and_operation_reuse() {
        let active = crate::test::support::fleet_admission_projection(
            crate::test::support::managed_component_binding(),
        );
        let mut successor = active.clone();
        successor.generation += 1;
        successor.policy_digest = [0xb1; 32];
        successor.projection_digest = [0xb2; 32];
        let initial = FleetAdmissionProjectionState {
            schema_version: 1,
            active: active.clone(),
            prepared: None,
            phase: FleetAdmissionProjectionPhaseModel::Open,
            last_receipt: None,
        };
        let activate = request(
            FleetAdmissionTargetTransitionPhaseModel::Activate,
            &active,
            successor.clone(),
            2,
        );
        assert_eq!(
            transition_fleet_admission_projection(&initial, activate),
            Err(FleetAdmissionTargetTransitionError::PhaseConflict)
        );

        let prepare = request(
            FleetAdmissionTargetTransitionPhaseModel::Prepare,
            &active,
            successor,
            1,
        );
        let prepared = transition_fleet_admission_projection(&initial, prepare.clone())
            .expect("prepare target");
        let mut conflicting = prepare;
        conflicting.request_hash = [0xcc; 32];
        assert_eq!(
            transition_fleet_admission_projection(&prepared.state, conflicting),
            Err(FleetAdmissionTargetTransitionError::OperationConflict)
        );
    }

    fn request(
        phase: FleetAdmissionTargetTransitionPhaseModel,
        active: &crate::ids::FleetAdmissionProjection,
        successor: crate::ids::FleetAdmissionProjection,
        hash_byte: u8,
    ) -> FleetAdmissionTargetTransitionRequestModel {
        FleetAdmissionTargetTransitionRequestModel {
            operation_id: [0x81; 32],
            phase,
            expected_generation: active.generation,
            expected_policy_digest: active.policy_digest,
            successor,
            request_hash: [hash_byte; 32],
            receipt_hash: [hash_byte.saturating_add(16); 32],
        }
    }
}
