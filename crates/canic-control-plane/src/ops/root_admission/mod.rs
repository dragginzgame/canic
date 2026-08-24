//! Module: ops::root_admission
//!
//! Responsibility: convert, validate and atomically persist one Root admission journal.
//! Does not own: caller authorization, participant discovery, calls, timers, or policy choice.
//! Boundary: workflow supplies exact Root authority and compiled target projections.

use crate::storage::stable::root_admission::{
    RootAdmissionCommitError, RootAdmissionParticipantPhaseRecord, RootAdmissionParticipantRecord,
    RootAdmissionPhaseRecord, RootAdmissionPrepareRequestRecord, RootAdmissionRecord,
    RootAdmissionReleasedReservationRecord, RootAdmissionRetainedResultRecord, RootAdmissionStore,
    RootAdmissionTransitionRecord,
};
use canic_core::{
    control_plane_support::error::InternalError,
    dto::{
        fleet_admission::{
            FleetAdmissionActivateRootRequest, FleetAdmissionOpenRootRequest,
            FleetAdmissionPrepareRootRequest, FleetAdmissionPrepareRootStage,
            FleetAdmissionRootParticipantPhase, FleetAdmissionRootParticipantStatus,
            FleetAdmissionRootReceipt, FleetAdmissionRootStatusResponse,
            FleetAdmissionRootTransitionPhase, FleetAdmissionTargetReceipt,
            FleetAdmissionTargetTransitionPhase,
        },
        page::{Page, PageRequest},
    },
    ids::{
        FleetAdmissionPolicy, FleetAdmissionProjection, FleetSubnetRootBinding,
        ManagedCanisterBinding,
    },
    shared_support::{
        fleet_admission_policy::{
            expected_fleet_admission_target_receipt, fleet_admission_root_activate_request_digest,
            fleet_admission_root_open_request_digest,
            fleet_admission_root_participant_catalog_digest, fleet_admission_root_prepare_request,
            fleet_admission_root_prepare_request_digest, fleet_admission_root_receipt_digest,
            fleet_admission_target_for_binding, materialize_fleet_admission_projection,
            validate_installed_fleet_admission_policy,
        },
        fleet_admission_root::{
            FLEET_ADMISSION_ROOT_SCHEMA_VERSION, FleetAdmissionRootParticipantModel,
            FleetAdmissionRootParticipantPhaseModel, FleetAdmissionRootPhaseModel,
            FleetAdmissionRootPrepareRequestModel, FleetAdmissionRootReleasedReservationModel,
            FleetAdmissionRootRetainedResultModel, FleetAdmissionRootState,
            FleetAdmissionRootTransitionError, FleetAdmissionRootTransitionModel,
            MAX_FLEET_ADMISSION_ROOT_PARTICIPANTS, MAX_FLEET_ADMISSION_ROOT_STATUS_PAGE,
            activate_fleet_admission_root, complete_fleet_admission_root,
            fence_fleet_admission_root, open_fleet_admission_root, prepare_fleet_admission_root,
            record_fleet_admission_root_participant, release_fleet_admission_root,
            validate_fleet_admission_root_state,
        },
    },
};

/// One exact outbound target action selected from durable Root progress.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RootAdmissionStep {
    Prepare {
        projection: FleetAdmissionProjection,
    },
    Activate {
        projection: FleetAdmissionProjection,
    },
    Open {
        projection: FleetAdmissionProjection,
    },
    Complete,
    Waiting,
}

/// Deterministic storage and transition facade for memory ID 65.
pub struct RootAdmissionOps;

impl RootAdmissionOps {
    /// Return whether the bounded Root owner already retains this operation identity.
    pub(crate) fn retains_operation_id(
        root: &FleetSubnetRootBinding,
        operation_id: [u8; 32],
    ) -> Result<bool, InternalError> {
        if operation_id == [0; 32] {
            return Ok(false);
        }
        let Some(record) = RootAdmissionStore::export().current else {
            return Ok(false);
        };
        let state = record_to_model(record);
        validate_state_for_root(root, &state)?;
        Ok(state
            .current_transition
            .as_ref()
            .is_some_and(|current| current.request.operation_id == operation_id)
            || state
                .last_result
                .as_ref()
                .is_some_and(|last| last.request.operation_id == operation_id)
            || state
                .last_release
                .as_ref()
                .is_some_and(|last| last.request.operation_id == operation_id))
    }

    /// Begin, resume or exactly replay one Coordinator-authored prepare command.
    pub(crate) fn prepare(
        root: &FleetSubnetRootBinding,
        active_policy: FleetAdmissionPolicy,
        mut request: FleetAdmissionPrepareRootRequest,
        mut projections: Vec<FleetAdmissionProjection>,
    ) -> Result<Option<FleetAdmissionRootReceipt>, InternalError> {
        let current = load_or_initialize(root, active_policy)?;
        let stage = request.stage;
        let stage_request_hash = fleet_admission_root_prepare_request_digest(&request, root);
        request.stage = FleetAdmissionPrepareRootStage::Reserve;
        let request = fleet_admission_root_prepare_request(request, root.clone())
            .map_err(|_error| InternalError::invalid_input())?;
        match stage {
            FleetAdmissionPrepareRootStage::Reserve => {
                reserve(root, current, request, &mut projections)
            }
            FleetAdmissionPrepareRootStage::Fence => fence(current, request, stage_request_hash),
            FleetAdmissionPrepareRootStage::Release => {
                release(current, request, stage_request_hash)
            }
        }
    }

    /// Start or exactly replay the aggregate activation phase.
    pub(crate) fn activate(
        root: &FleetSubnetRootBinding,
        request: FleetAdmissionActivateRootRequest,
    ) -> Result<Option<FleetAdmissionRootReceipt>, InternalError> {
        let current = load_valid(root)?;
        validate_activate_request(&current, &request)?;
        let request_hash = fleet_admission_root_activate_request_digest(&request);
        let operation = current
            .current_transition
            .as_ref()
            .ok_or_else(InternalError::conflict)?;
        let aggregate_activate_receipt_hash = fleet_admission_root_receipt_digest(
            operation.request.operation_id,
            FleetAdmissionRootTransitionPhase::Opening,
            &operation.request.root,
            operation.request.successor.generation,
            operation.request.successor.policy_digest,
            operation.participant_catalog_digest,
            u32::try_from(operation.participants.len()).map_err(|_| InternalError::invariant())?,
        );
        let (next, receipt_hash) = activate_fleet_admission_root(
            &current,
            request.operation_id,
            request_hash,
            aggregate_activate_receipt_hash,
        )
        .map_err(map_transition_error)?;
        if next != current {
            commit(&current, next.clone())?;
        }
        Ok(receipt_hash.map(|receipt_hash| {
            root_receipt_from_state(
                &next,
                FleetAdmissionRootTransitionPhase::Opening,
                receipt_hash,
            )
            .expect("validated activated Root state reconstructs its receipt")
        }))
    }

    /// Start or exactly replay the aggregate open phase.
    pub(crate) fn open(
        root: &FleetSubnetRootBinding,
        request: FleetAdmissionOpenRootRequest,
    ) -> Result<Option<FleetAdmissionRootReceipt>, InternalError> {
        let current = load_valid(root)?;
        validate_open_request(&current, &request)?;
        let request_hash = fleet_admission_root_open_request_digest(&request);
        if let Some(last) = &current.last_result
            && last.request.operation_id == request.operation_id
        {
            if last.open_request_hash != request_hash {
                return Err(InternalError::conflict());
            }
            return Ok(Some(retained_root_receipt(last)));
        }
        let next = open_fleet_admission_root(&current, request.operation_id, request_hash)
            .map_err(map_transition_error)?
            .ok_or_else(InternalError::invariant)?;
        if next != current {
            commit(&current, next)?;
        }
        Ok(None)
    }

    /// Select the exact next target effect, completion, or Coordinator wait boundary.
    pub(crate) fn next_step(
        root: &FleetSubnetRootBinding,
    ) -> Result<(FleetAdmissionRootState, RootAdmissionStep), InternalError> {
        let state = load_valid(root)?;
        let Some(current) = &state.current_transition else {
            return Ok((state, RootAdmissionStep::Waiting));
        };
        let expected_phase = match current.phase {
            FleetAdmissionRootPhaseModel::Preparing if current.fence_request_hash.is_some() => {
                Some(FleetAdmissionRootParticipantPhaseModel::Pending)
            }
            FleetAdmissionRootPhaseModel::Activating => {
                Some(FleetAdmissionRootParticipantPhaseModel::Prepared)
            }
            FleetAdmissionRootPhaseModel::Opening if current.open_request_hash.is_some() => {
                Some(FleetAdmissionRootParticipantPhaseModel::Activated)
            }
            FleetAdmissionRootPhaseModel::Preparing
            | FleetAdmissionRootPhaseModel::PerimeterFenced
            | FleetAdmissionRootPhaseModel::Opening => None,
        };
        let Some(expected_phase) = expected_phase else {
            return Ok((state, RootAdmissionStep::Waiting));
        };
        if let Some(participant) = current
            .participants
            .iter()
            .find(|participant| participant.phase == expected_phase)
        {
            let target = fleet_admission_target_for_binding(&participant.target);
            let principals = canic_core::shared_support::fleet_admission_policy::effective_fleet_admission_principals(
                &current.request.successor,
                &target,
            );
            let projection = materialize_fleet_admission_projection(
                &current.request.successor,
                participant.target.clone(),
                principals,
            )
            .map_err(|_error| InternalError::invariant())?;
            if projection.projection_digest != participant.projection_digest {
                return Err(InternalError::invariant());
            }
            let step = match current.phase {
                FleetAdmissionRootPhaseModel::Preparing => {
                    RootAdmissionStep::Prepare { projection }
                }
                FleetAdmissionRootPhaseModel::Activating => {
                    RootAdmissionStep::Activate { projection }
                }
                FleetAdmissionRootPhaseModel::Opening => RootAdmissionStep::Open { projection },
                FleetAdmissionRootPhaseModel::PerimeterFenced => unreachable!(),
            };
            return Ok((state, step));
        }
        if current.phase == FleetAdmissionRootPhaseModel::Opening
            && current.open_request_hash.is_some()
            && current.participants.iter().all(|participant| {
                participant.phase == FleetAdmissionRootParticipantPhaseModel::Open
            })
        {
            Ok((state, RootAdmissionStep::Complete))
        } else {
            Err(InternalError::invariant())
        }
    }

    /// Validate and retain one exact target receipt.
    pub(crate) fn record_target_receipt(
        root: &FleetSubnetRootBinding,
        expected: &FleetAdmissionRootState,
        projection: FleetAdmissionProjection,
        phase: FleetAdmissionTargetTransitionPhase,
        receipt: FleetAdmissionTargetReceipt,
    ) -> Result<(), InternalError> {
        let current = load_valid(root)?;
        if &current != expected {
            return Err(InternalError::conflict());
        }
        let operation = current
            .current_transition
            .as_ref()
            .ok_or_else(InternalError::conflict)?;
        let (expected_generation, expected_policy_digest, participant_phase, aggregate_phase) =
            match phase {
                FleetAdmissionTargetTransitionPhase::Prepare => (
                    operation.request.expected_generation,
                    operation.request.expected_policy_digest,
                    FleetAdmissionRootParticipantPhaseModel::Prepared,
                    FleetAdmissionRootTransitionPhase::PerimeterFenced,
                ),
                FleetAdmissionTargetTransitionPhase::Activate => (
                    operation.request.expected_generation,
                    operation.request.expected_policy_digest,
                    FleetAdmissionRootParticipantPhaseModel::Activated,
                    FleetAdmissionRootTransitionPhase::Opening,
                ),
                FleetAdmissionTargetTransitionPhase::Open => (
                    projection.generation,
                    projection.policy_digest,
                    FleetAdmissionRootParticipantPhaseModel::Open,
                    FleetAdmissionRootTransitionPhase::Converged,
                ),
            };
        let exact = expected_fleet_admission_target_receipt(
            operation.request.operation_id,
            phase,
            expected_generation,
            expected_policy_digest,
            projection.clone(),
        )
        .map_err(|_error| InternalError::invariant())?;
        if receipt != exact {
            return Err(InternalError::conflict());
        }
        let aggregate_hash = fleet_admission_root_receipt_digest(
            operation.request.operation_id,
            aggregate_phase,
            &operation.request.root,
            operation.request.successor.generation,
            operation.request.successor.policy_digest,
            operation.participant_catalog_digest,
            u32::try_from(operation.participants.len()).map_err(|_| InternalError::invariant())?,
        );
        let next = record_fleet_admission_root_participant(
            &current,
            operation.request.operation_id,
            &projection.target,
            participant_phase,
            receipt.receipt_hash,
            aggregate_hash,
        )
        .map_err(map_transition_error)?;
        commit(&current, next)
    }

    /// Complete a fully opened operation and return its retained receipt.
    pub(crate) fn complete(
        root: &FleetSubnetRootBinding,
        expected: &FleetAdmissionRootState,
    ) -> Result<FleetAdmissionRootReceipt, InternalError> {
        let current = load_valid(root)?;
        if &current != expected {
            return Err(InternalError::conflict());
        }
        let operation = current
            .current_transition
            .as_ref()
            .ok_or_else(InternalError::conflict)?;
        let receipt_hash = fleet_admission_root_receipt_digest(
            operation.request.operation_id,
            FleetAdmissionRootTransitionPhase::Converged,
            &operation.request.root,
            operation.request.successor.generation,
            operation.request.successor.policy_digest,
            operation.participant_catalog_digest,
            u32::try_from(operation.participants.len()).map_err(|_| InternalError::invariant())?,
        );
        let next =
            complete_fleet_admission_root(&current, receipt_hash).map_err(map_transition_error)?;
        let receipt = retained_root_receipt(
            next.last_result
                .as_ref()
                .ok_or_else(InternalError::invariant)?,
        );
        commit(&current, next)?;
        Ok(receipt)
    }

    /// Return the current converged policy for new Component projection compilation.
    pub(crate) fn active_policy(
        root: &FleetSubnetRootBinding,
        fallback: FleetAdmissionPolicy,
    ) -> Result<FleetAdmissionPolicy, InternalError> {
        match RootAdmissionStore::export().current {
            None => {
                validate_policy_for_root(root, &fallback)?;
                Ok(fallback)
            }
            Some(record) => {
                let state = record_to_model(record);
                validate_state_for_root(root, &state)?;
                if state.current_transition.is_some() {
                    return Err(InternalError::conflict());
                }
                Ok(state.active_policy)
            }
        }
    }

    /// Reject creation or retirement while this Root is distributing a successor.
    pub(crate) fn require_catalog_mutation_allowed(
        root: &FleetSubnetRootBinding,
    ) -> Result<(), InternalError> {
        let Some(record) = RootAdmissionStore::export().current else {
            return Ok(());
        };
        let state = record_to_model(record);
        validate_state_for_root(root, &state)?;
        if state.current_transition.is_some() {
            Err(InternalError::conflict())
        } else {
            Ok(())
        }
    }

    /// Return whether an idle status view must compile the current live catalog.
    pub(crate) fn status_requires_live_catalog(
        root: &FleetSubnetRootBinding,
        fallback: FleetAdmissionPolicy,
    ) -> Result<bool, InternalError> {
        Ok(load_or_initialize(root, fallback)?
            .current_transition
            .is_none())
    }

    /// Return one bounded controller-only current/retained participant page.
    pub(crate) fn status(
        root: &FleetSubnetRootBinding,
        active_policy: FleetAdmissionPolicy,
        mut live_projections: Vec<FleetAdmissionProjection>,
        page: PageRequest,
    ) -> Result<FleetAdmissionRootStatusResponse, InternalError> {
        let state = load_or_initialize(root, active_policy)?;
        let (live_catalog_digest, live_participants) = if state.current_transition.is_some() {
            if !live_projections.is_empty() {
                return Err(InternalError::invariant());
            }
            (None, Vec::new())
        } else {
            live_projections.sort_by(|left, right| {
                target_principal(&left.target)
                    .as_slice()
                    .cmp(target_principal(&right.target).as_slice())
            });
            if live_projections.len() > MAX_FLEET_ADMISSION_ROOT_PARTICIPANTS
                || live_projections.windows(2).any(|pair| {
                    target_principal(&pair[0].target) == target_principal(&pair[1].target)
                })
            {
                return Err(InternalError::resource_exhausted());
            }
            validate_projections(root, &state.active_policy, &live_projections)?;
            let digest = fleet_admission_root_participant_catalog_digest(&live_projections);
            let participants = live_projections
                .into_iter()
                .map(|projection| FleetAdmissionRootParticipantModel {
                    target: projection.target,
                    projection_digest: projection.projection_digest,
                    phase: FleetAdmissionRootParticipantPhaseModel::Open,
                    last_receipt_hash: None,
                })
                .collect::<Vec<_>>();
            (Some(digest), participants)
        };
        let (operation_id, phase, successor, catalog, participants) =
            if let Some(current) = &state.current_transition {
                (
                    Some(current.request.operation_id),
                    Some(root_phase_to_dto(current.phase)),
                    Some(&current.request.successor),
                    Some(current.participant_catalog_digest),
                    current.participants.as_slice(),
                )
            } else {
                (
                    None,
                    None,
                    None,
                    live_catalog_digest,
                    live_participants.as_slice(),
                )
            };
        let total = u64::try_from(participants.len()).map_err(|_| InternalError::invariant())?;
        let entries = participants
            .iter()
            .skip(usize::try_from(page.offset).unwrap_or(usize::MAX))
            .take(
                usize::try_from(page.limit.min(MAX_FLEET_ADMISSION_ROOT_STATUS_PAGE))
                    .expect("Root admission page bound fits usize"),
            )
            .map(participant_status)
            .collect();
        Ok(FleetAdmissionRootStatusResponse {
            operation_id,
            phase,
            active_generation: state.active_policy.generation,
            active_policy_digest: state.active_policy.policy_digest,
            successor_generation: successor.map(|policy| policy.generation),
            successor_policy_digest: successor.map(|policy| policy.policy_digest),
            participant_catalog_digest: catalog,
            participants: Page { entries, total },
            maximum_page_size: u16::try_from(MAX_FLEET_ADMISSION_ROOT_STATUS_PAGE)
                .expect("Root admission page bound fits u16"),
            last_result: state.last_result.as_ref().map(retained_root_receipt),
        })
    }
}

fn reserve(
    root: &FleetSubnetRootBinding,
    current: FleetAdmissionRootState,
    request: FleetAdmissionRootPrepareRequestModel,
    projections: &mut Vec<FleetAdmissionProjection>,
) -> Result<Option<FleetAdmissionRootReceipt>, InternalError> {
    let retained_catalog = retained_catalog(&current, request.operation_id);
    let (participant_catalog_digest, participant_count, participants) =
        if let Some((digest, count)) = retained_catalog {
            (digest, count, Vec::new())
        } else {
            projections.sort_by(|left, right| {
                target_principal(&left.target)
                    .as_slice()
                    .cmp(target_principal(&right.target).as_slice())
            });
            if projections.len() > MAX_FLEET_ADMISSION_ROOT_PARTICIPANTS
                || projections.windows(2).any(|pair| {
                    target_principal(&pair[0].target) == target_principal(&pair[1].target)
                })
            {
                return Err(InternalError::resource_exhausted());
            }
            validate_projections(root, &request.successor, projections)?;
            let digest = fleet_admission_root_participant_catalog_digest(projections);
            let count = u32::try_from(projections.len()).map_err(|_| InternalError::invariant())?;
            let participants = projections
                .drain(..)
                .map(|projection| FleetAdmissionRootParticipantModel {
                    target: projection.target,
                    projection_digest: projection.projection_digest,
                    phase: FleetAdmissionRootParticipantPhaseModel::Pending,
                    last_receipt_hash: None,
                })
                .collect();
            (digest, count, participants)
        };
    let receipt_hash = fleet_admission_root_receipt_digest(
        request.operation_id,
        FleetAdmissionRootTransitionPhase::Preparing,
        &request.root,
        request.successor.generation,
        request.successor.policy_digest,
        participant_catalog_digest,
        participant_count,
    );
    let decision = prepare_fleet_admission_root(
        &current,
        request.clone(),
        participant_catalog_digest,
        receipt_hash,
        participants,
    )
    .map_err(map_transition_error)?;
    if decision.state != current {
        commit(&current, decision.state)?;
    }
    Ok(Some(root_receipt(
        &request,
        FleetAdmissionRootTransitionPhase::Preparing,
        participant_catalog_digest,
        participant_count,
        decision.receipt_hash,
    )))
}

fn fence(
    current: FleetAdmissionRootState,
    request: FleetAdmissionRootPrepareRequestModel,
    request_hash: [u8; 32],
) -> Result<Option<FleetAdmissionRootReceipt>, InternalError> {
    let operation = current
        .current_transition
        .as_ref()
        .ok_or_else(InternalError::conflict)?;
    if operation.request != request {
        return Err(InternalError::conflict());
    }
    let participant_count =
        u32::try_from(operation.participants.len()).map_err(|_| InternalError::invariant())?;
    let participant_catalog_digest = operation.participant_catalog_digest;
    let receipt_hash = fleet_admission_root_receipt_digest(
        request.operation_id,
        FleetAdmissionRootTransitionPhase::PerimeterFenced,
        &request.root,
        request.successor.generation,
        request.successor.policy_digest,
        participant_catalog_digest,
        participant_count,
    );
    let (next, retained_receipt) =
        fence_fleet_admission_root(&current, request.operation_id, request_hash, receipt_hash)
            .map_err(map_transition_error)?;
    if next != current {
        commit(&current, next)?;
    }
    Ok(retained_receipt.map(|retained_receipt| {
        root_receipt(
            &request,
            FleetAdmissionRootTransitionPhase::PerimeterFenced,
            participant_catalog_digest,
            participant_count,
            retained_receipt,
        )
    }))
}

fn release(
    current: FleetAdmissionRootState,
    request: FleetAdmissionRootPrepareRequestModel,
    request_hash: [u8; 32],
) -> Result<Option<FleetAdmissionRootReceipt>, InternalError> {
    let (participant_catalog_digest, participant_count) =
        retained_catalog(&current, request.operation_id).ok_or_else(InternalError::conflict)?;
    let receipt_hash = fleet_admission_root_receipt_digest(
        request.operation_id,
        FleetAdmissionRootTransitionPhase::Released,
        &request.root,
        request.successor.generation,
        request.successor.policy_digest,
        participant_catalog_digest,
        participant_count,
    );
    let (next, retained_receipt) =
        release_fleet_admission_root(&current, request.operation_id, request_hash, receipt_hash)
            .map_err(map_transition_error)?;
    if next != current {
        commit(&current, next)?;
    }
    Ok(Some(root_receipt(
        &request,
        FleetAdmissionRootTransitionPhase::Released,
        participant_catalog_digest,
        participant_count,
        retained_receipt,
    )))
}

fn retained_catalog(
    state: &FleetAdmissionRootState,
    operation_id: [u8; 32],
) -> Option<([u8; 32], u32)> {
    state
        .current_transition
        .as_ref()
        .filter(|current| current.request.operation_id == operation_id)
        .and_then(|current| {
            u32::try_from(current.participants.len())
                .ok()
                .map(|count| (current.participant_catalog_digest, count))
        })
        .or_else(|| {
            state
                .last_result
                .as_ref()
                .filter(|last| last.request.operation_id == operation_id)
                .and_then(|last| {
                    u32::try_from(last.participants.len())
                        .ok()
                        .map(|count| (last.participant_catalog_digest, count))
                })
        })
        .or_else(|| {
            state
                .last_release
                .as_ref()
                .filter(|last| last.request.operation_id == operation_id)
                .map(|last| (last.participant_catalog_digest, last.participant_count))
        })
}

fn root_receipt(
    request: &FleetAdmissionRootPrepareRequestModel,
    phase: FleetAdmissionRootTransitionPhase,
    participant_catalog_digest: [u8; 32],
    participant_count: u32,
    receipt_hash: [u8; 32],
) -> FleetAdmissionRootReceipt {
    FleetAdmissionRootReceipt {
        operation_id: request.operation_id,
        phase,
        root: request.root.clone(),
        generation: request.successor.generation,
        policy_digest: request.successor.policy_digest,
        participant_catalog_digest,
        participant_count,
        receipt_hash,
    }
}

fn load_or_initialize(
    root: &FleetSubnetRootBinding,
    active_policy: FleetAdmissionPolicy,
) -> Result<FleetAdmissionRootState, InternalError> {
    if let Some(record) = RootAdmissionStore::export().current {
        let state = record_to_model(record);
        validate_state_for_root(root, &state)?;
        return Ok(state);
    }
    validate_policy_for_root(root, &active_policy)?;
    let state = FleetAdmissionRootState {
        schema_version: FLEET_ADMISSION_ROOT_SCHEMA_VERSION,
        active_policy,
        current_transition: None,
        last_result: None,
        last_release: None,
    };
    RootAdmissionStore::commit_genesis(model_to_record(state.clone())).map_err(map_commit_error)?;
    Ok(state)
}

fn load_valid(root: &FleetSubnetRootBinding) -> Result<FleetAdmissionRootState, InternalError> {
    let record = RootAdmissionStore::export()
        .current
        .ok_or_else(InternalError::unavailable)?;
    let state = record_to_model(record);
    validate_state_for_root(root, &state)?;
    Ok(state)
}

fn validate_state_for_root(
    root: &FleetSubnetRootBinding,
    state: &FleetAdmissionRootState,
) -> Result<(), InternalError> {
    validate_fleet_admission_root_state(state).map_err(|_error| InternalError::invariant())?;
    validate_policy_for_root(root, &state.active_policy)?;
    if let Some(current) = &state.current_transition {
        let projections = validate_retained_operation(
            root,
            &current.request,
            current.participant_catalog_digest,
            &current.participants,
        )?;
        validate_current_receipts(current, &projections)?;
    }
    if let Some(last) = &state.last_result {
        let projections = validate_retained_operation(
            root,
            &last.request,
            last.participant_catalog_digest,
            &last.participants,
        )?;
        validate_terminal_receipts(last, &projections)?;
    }
    if let Some(last) = &state.last_release {
        validate_released_reservation(root, last)?;
    }
    Ok(())
}

fn validate_retained_operation(
    root: &FleetSubnetRootBinding,
    request: &FleetAdmissionRootPrepareRequestModel,
    participant_catalog_digest: [u8; 32],
    participants: &[FleetAdmissionRootParticipantModel],
) -> Result<Vec<FleetAdmissionProjection>, InternalError> {
    if &request.root != root || request.authority != root.authority.binding {
        return Err(InternalError::invariant());
    }
    validate_policy_for_root(root, &request.successor)?;
    let reconstructed = fleet_admission_root_prepare_request(
        FleetAdmissionPrepareRootRequest {
            authority: request.authority.clone(),
            operation_id: request.operation_id,
            expected_generation: request.expected_generation,
            expected_policy_digest: request.expected_policy_digest,
            successor: request.successor.clone(),
            stage: FleetAdmissionPrepareRootStage::Reserve,
        },
        root.clone(),
    )
    .map_err(|_error| InternalError::invariant())?;
    if &reconstructed != request {
        return Err(InternalError::invariant());
    }
    let projections = participants
        .iter()
        .map(|participant| {
            if target_root(&participant.target) != root.fleet_subnet_root {
                return Err(InternalError::invariant());
            }
            let selector = fleet_admission_target_for_binding(&participant.target);
            let principals = canic_core::shared_support::fleet_admission_policy::effective_fleet_admission_principals(
                &request.successor,
                &selector,
            );
            let projection = materialize_fleet_admission_projection(
                &request.successor,
                participant.target.clone(),
                principals,
            )
            .map_err(|_error| InternalError::invariant())?;
            if projection.projection_digest != participant.projection_digest {
                return Err(InternalError::invariant());
            }
            Ok(projection)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if fleet_admission_root_participant_catalog_digest(&projections) != participant_catalog_digest {
        return Err(InternalError::invariant());
    }
    Ok(projections)
}

fn validate_released_reservation(
    root: &FleetSubnetRootBinding,
    last: &FleetAdmissionRootReleasedReservationModel,
) -> Result<(), InternalError> {
    if &last.request.root != root
        || last.request.authority != root.authority.binding
        || usize::try_from(last.participant_count)
            .map_or(true, |count| count > MAX_FLEET_ADMISSION_ROOT_PARTICIPANTS)
    {
        return Err(InternalError::invariant());
    }
    validate_policy_for_root(root, &last.request.successor)?;
    let reserve_request = FleetAdmissionPrepareRootRequest {
        authority: last.request.authority.clone(),
        operation_id: last.request.operation_id,
        expected_generation: last.request.expected_generation,
        expected_policy_digest: last.request.expected_policy_digest,
        successor: last.request.successor.clone(),
        stage: FleetAdmissionPrepareRootStage::Reserve,
    };
    let reconstructed = fleet_admission_root_prepare_request(reserve_request, root.clone())
        .map_err(|_error| InternalError::invariant())?;
    let release_request = FleetAdmissionPrepareRootRequest {
        authority: last.request.authority.clone(),
        operation_id: last.request.operation_id,
        expected_generation: last.request.expected_generation,
        expected_policy_digest: last.request.expected_policy_digest,
        successor: last.request.successor.clone(),
        stage: FleetAdmissionPrepareRootStage::Release,
    };
    let release_request_hash = fleet_admission_root_prepare_request_digest(&release_request, root);
    let expected_receipt = fleet_admission_root_receipt_digest(
        last.request.operation_id,
        FleetAdmissionRootTransitionPhase::Released,
        root,
        last.request.successor.generation,
        last.request.successor.policy_digest,
        last.participant_catalog_digest,
        last.participant_count,
    );
    if reconstructed != last.request
        || release_request_hash != last.release_request_hash
        || expected_receipt != last.receipt_hash
    {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn validate_current_receipts(
    current: &FleetAdmissionRootTransitionModel,
    projections: &[FleetAdmissionProjection],
) -> Result<(), InternalError> {
    validate_participant_receipts(&current.request, &current.participants, projections)?;
    let participant_count =
        u32::try_from(projections.len()).map_err(|_| InternalError::invariant())?;
    let prepare_receipt = fleet_admission_root_receipt_digest(
        current.request.operation_id,
        FleetAdmissionRootTransitionPhase::PerimeterFenced,
        &current.request.root,
        current.request.successor.generation,
        current.request.successor.policy_digest,
        current.participant_catalog_digest,
        participant_count,
    );
    let activate_request = FleetAdmissionActivateRootRequest {
        authority: current.request.authority.clone(),
        operation_id: current.request.operation_id,
        expected_generation: current.request.expected_generation,
        expected_policy_digest: current.request.expected_policy_digest,
        successor_generation: current.request.successor.generation,
        successor_policy_digest: current.request.successor.policy_digest,
    };
    let activate_receipt = fleet_admission_root_receipt_digest(
        current.request.operation_id,
        FleetAdmissionRootTransitionPhase::Opening,
        &current.request.root,
        current.request.successor.generation,
        current.request.successor.policy_digest,
        current.participant_catalog_digest,
        participant_count,
    );
    let open_request = FleetAdmissionOpenRootRequest {
        authority: current.request.authority.clone(),
        operation_id: current.request.operation_id,
        generation: current.request.successor.generation,
        policy_digest: current.request.successor.policy_digest,
    };
    let fence_request = FleetAdmissionPrepareRootRequest {
        authority: current.request.authority.clone(),
        operation_id: current.request.operation_id,
        expected_generation: current.request.expected_generation,
        expected_policy_digest: current.request.expected_policy_digest,
        successor: current.request.successor.clone(),
        stage: FleetAdmissionPrepareRootStage::Fence,
    };
    if current.fence_request_hash.is_some_and(|hash| {
        hash != fleet_admission_root_prepare_request_digest(&fence_request, &current.request.root)
    }) || current
        .prepare_receipt_hash
        .is_some_and(|hash| hash != prepare_receipt)
        || current.activate_request_hash.is_some_and(|hash| {
            hash != fleet_admission_root_activate_request_digest(&activate_request)
        })
        || current
            .activate_receipt_hash
            .is_some_and(|hash| hash != activate_receipt)
        || current
            .open_request_hash
            .is_some_and(|hash| hash != fleet_admission_root_open_request_digest(&open_request))
    {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn validate_terminal_receipts(
    last: &FleetAdmissionRootRetainedResultModel,
    projections: &[FleetAdmissionProjection],
) -> Result<(), InternalError> {
    validate_participant_receipts(&last.request, &last.participants, projections)?;
    let participant_count =
        u32::try_from(projections.len()).map_err(|_| InternalError::invariant())?;
    let prepare_receipt = fleet_admission_root_receipt_digest(
        last.request.operation_id,
        FleetAdmissionRootTransitionPhase::PerimeterFenced,
        &last.request.root,
        last.request.successor.generation,
        last.request.successor.policy_digest,
        last.participant_catalog_digest,
        participant_count,
    );
    let activate_request = FleetAdmissionActivateRootRequest {
        authority: last.request.authority.clone(),
        operation_id: last.request.operation_id,
        expected_generation: last.request.expected_generation,
        expected_policy_digest: last.request.expected_policy_digest,
        successor_generation: last.request.successor.generation,
        successor_policy_digest: last.request.successor.policy_digest,
    };
    let activate_receipt = fleet_admission_root_receipt_digest(
        last.request.operation_id,
        FleetAdmissionRootTransitionPhase::Opening,
        &last.request.root,
        last.request.successor.generation,
        last.request.successor.policy_digest,
        last.participant_catalog_digest,
        participant_count,
    );
    let open_request = FleetAdmissionOpenRootRequest {
        authority: last.request.authority.clone(),
        operation_id: last.request.operation_id,
        generation: last.request.successor.generation,
        policy_digest: last.request.successor.policy_digest,
    };
    let fence_request = FleetAdmissionPrepareRootRequest {
        authority: last.request.authority.clone(),
        operation_id: last.request.operation_id,
        expected_generation: last.request.expected_generation,
        expected_policy_digest: last.request.expected_policy_digest,
        successor: last.request.successor.clone(),
        stage: FleetAdmissionPrepareRootStage::Fence,
    };
    let terminal_receipt = fleet_admission_root_receipt_digest(
        last.request.operation_id,
        FleetAdmissionRootTransitionPhase::Converged,
        &last.request.root,
        last.request.successor.generation,
        last.request.successor.policy_digest,
        last.participant_catalog_digest,
        participant_count,
    );
    if last.fence_request_hash
        != fleet_admission_root_prepare_request_digest(&fence_request, &last.request.root)
        || last.prepare_receipt_hash != prepare_receipt
        || last.activate_request_hash
            != fleet_admission_root_activate_request_digest(&activate_request)
        || last.activate_receipt_hash != activate_receipt
        || last.open_request_hash != fleet_admission_root_open_request_digest(&open_request)
        || last.receipt_hash != terminal_receipt
    {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn validate_participant_receipts(
    request: &FleetAdmissionRootPrepareRequestModel,
    participants: &[FleetAdmissionRootParticipantModel],
    projections: &[FleetAdmissionProjection],
) -> Result<(), InternalError> {
    for (participant, projection) in participants.iter().zip(projections) {
        let expected = match participant.phase {
            FleetAdmissionRootParticipantPhaseModel::Pending => None,
            FleetAdmissionRootParticipantPhaseModel::Prepared => Some((
                FleetAdmissionTargetTransitionPhase::Prepare,
                request.expected_generation,
                request.expected_policy_digest,
            )),
            FleetAdmissionRootParticipantPhaseModel::Activated => Some((
                FleetAdmissionTargetTransitionPhase::Activate,
                request.expected_generation,
                request.expected_policy_digest,
            )),
            FleetAdmissionRootParticipantPhaseModel::Open => Some((
                FleetAdmissionTargetTransitionPhase::Open,
                request.successor.generation,
                request.successor.policy_digest,
            )),
        };
        let expected_hash = expected
            .map(|(phase, generation, policy_digest)| {
                expected_fleet_admission_target_receipt(
                    request.operation_id,
                    phase,
                    generation,
                    policy_digest,
                    projection.clone(),
                )
                .map(|receipt| receipt.receipt_hash)
                .map_err(|_error| InternalError::invariant())
            })
            .transpose()?;
        if participant.last_receipt_hash != expected_hash {
            return Err(InternalError::invariant());
        }
    }
    Ok(())
}

fn validate_policy_for_root(
    root: &FleetSubnetRootBinding,
    policy: &FleetAdmissionPolicy,
) -> Result<(), InternalError> {
    validate_installed_fleet_admission_policy(policy)
        .map_err(|_error| InternalError::invariant())?;
    if policy.fleet != root.authority.binding.fleet {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn validate_projections(
    root: &FleetSubnetRootBinding,
    successor: &FleetAdmissionPolicy,
    projections: &[FleetAdmissionProjection],
) -> Result<(), InternalError> {
    for projection in projections {
        if projection.authority != root.authority.binding
            || projection.generation != successor.generation
            || projection.policy_digest != successor.policy_digest
            || target_root(&projection.target) != root.fleet_subnet_root
        {
            return Err(InternalError::invariant());
        }
    }
    Ok(())
}

fn validate_activate_request(
    state: &FleetAdmissionRootState,
    request: &FleetAdmissionActivateRootRequest,
) -> Result<(), InternalError> {
    let current = state
        .current_transition
        .as_ref()
        .ok_or_else(InternalError::conflict)?;
    if request.authority != current.request.authority
        || request.operation_id != current.request.operation_id
        || request.expected_generation != current.request.expected_generation
        || request.expected_policy_digest != current.request.expected_policy_digest
        || request.successor_generation != current.request.successor.generation
        || request.successor_policy_digest != current.request.successor.policy_digest
    {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn validate_open_request(
    state: &FleetAdmissionRootState,
    request: &FleetAdmissionOpenRootRequest,
) -> Result<(), InternalError> {
    let operation = state
        .current_transition
        .as_ref()
        .map(|current| &current.request)
        .or_else(|| state.last_result.as_ref().map(|last| &last.request))
        .ok_or_else(InternalError::conflict)?;
    if request.authority != operation.authority
        || request.operation_id != operation.operation_id
        || request.generation != operation.successor.generation
        || request.policy_digest != operation.successor.policy_digest
    {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn root_receipt_from_state(
    state: &FleetAdmissionRootState,
    phase: FleetAdmissionRootTransitionPhase,
    receipt_hash: [u8; 32],
) -> Option<FleetAdmissionRootReceipt> {
    let operation = state
        .current_transition
        .as_ref()
        .map(|current| {
            (
                &current.request,
                current.participant_catalog_digest,
                current.participants.len(),
            )
        })
        .or_else(|| {
            state.last_result.as_ref().map(|last| {
                (
                    &last.request,
                    last.participant_catalog_digest,
                    last.participants.len(),
                )
            })
        })?;
    Some(FleetAdmissionRootReceipt {
        operation_id: operation.0.operation_id,
        phase,
        root: operation.0.root.clone(),
        generation: operation.0.successor.generation,
        policy_digest: operation.0.successor.policy_digest,
        participant_catalog_digest: operation.1,
        participant_count: u32::try_from(operation.2).ok()?,
        receipt_hash,
    })
}

fn retained_root_receipt(
    last: &FleetAdmissionRootRetainedResultModel,
) -> FleetAdmissionRootReceipt {
    FleetAdmissionRootReceipt {
        operation_id: last.request.operation_id,
        phase: FleetAdmissionRootTransitionPhase::Converged,
        root: last.request.root.clone(),
        generation: last.request.successor.generation,
        policy_digest: last.request.successor.policy_digest,
        participant_catalog_digest: last.participant_catalog_digest,
        participant_count: u32::try_from(last.participants.len())
            .expect("validated Root participant count fits u32"),
        receipt_hash: last.receipt_hash,
    }
}

fn participant_status(
    participant: &FleetAdmissionRootParticipantModel,
) -> FleetAdmissionRootParticipantStatus {
    FleetAdmissionRootParticipantStatus {
        target: participant.target.clone(),
        projection_digest: participant.projection_digest,
        phase: match participant.phase {
            FleetAdmissionRootParticipantPhaseModel::Pending => {
                FleetAdmissionRootParticipantPhase::Pending
            }
            FleetAdmissionRootParticipantPhaseModel::Prepared => {
                FleetAdmissionRootParticipantPhase::Prepared
            }
            FleetAdmissionRootParticipantPhaseModel::Activated => {
                FleetAdmissionRootParticipantPhase::Activated
            }
            FleetAdmissionRootParticipantPhaseModel::Open => {
                FleetAdmissionRootParticipantPhase::Open
            }
        },
        last_receipt_hash: participant.last_receipt_hash,
    }
}

const fn root_phase_to_dto(
    phase: FleetAdmissionRootPhaseModel,
) -> FleetAdmissionRootTransitionPhase {
    match phase {
        FleetAdmissionRootPhaseModel::Preparing => FleetAdmissionRootTransitionPhase::Preparing,
        FleetAdmissionRootPhaseModel::PerimeterFenced => {
            FleetAdmissionRootTransitionPhase::PerimeterFenced
        }
        FleetAdmissionRootPhaseModel::Activating => FleetAdmissionRootTransitionPhase::Activating,
        FleetAdmissionRootPhaseModel::Opening => FleetAdmissionRootTransitionPhase::Opening,
    }
}

fn commit(
    expected: &FleetAdmissionRootState,
    next: FleetAdmissionRootState,
) -> Result<(), InternalError> {
    RootAdmissionStore::commit_transition(&model_to_record(expected.clone()), model_to_record(next))
        .map_err(map_commit_error)
        .map(|_outcome| ())
}

const fn map_commit_error(error: RootAdmissionCommitError) -> InternalError {
    match error {
        RootAdmissionCommitError::ConflictingState => InternalError::conflict(),
        RootAdmissionCommitError::Uninitialized => InternalError::unavailable(),
    }
}

const fn map_transition_error(error: FleetAdmissionRootTransitionError) -> InternalError {
    match error {
        FleetAdmissionRootTransitionError::ParticipantCapacity => {
            InternalError::resource_exhausted()
        }
        FleetAdmissionRootTransitionError::OperationConflict
        | FleetAdmissionRootTransitionError::PhaseConflict
        | FleetAdmissionRootTransitionError::ReceiptConflict => InternalError::conflict(),
        FleetAdmissionRootTransitionError::InvalidState => InternalError::invariant(),
    }
}

const fn target_principal(target: &ManagedCanisterBinding) -> candid::Principal {
    match target {
        ManagedCanisterBinding::Component(component) => component.canister_id,
        ManagedCanisterBinding::ComponentChild(child) => child.canister_id,
    }
}

const fn target_root(target: &ManagedCanisterBinding) -> candid::Principal {
    match target {
        ManagedCanisterBinding::Component(component) => component.fleet_subnet_root,
        ManagedCanisterBinding::ComponentChild(child) => child.component.fleet_subnet_root,
    }
}

fn record_to_model(record: RootAdmissionRecord) -> FleetAdmissionRootState {
    FleetAdmissionRootState {
        schema_version: record.schema_version,
        active_policy: record.active_policy,
        current_transition: record.current_transition.map(transition_record_to_model),
        last_result: record.last_result.map(retained_record_to_model),
        last_release: record.last_release.map(released_record_to_model),
    }
}

fn model_to_record(state: FleetAdmissionRootState) -> RootAdmissionRecord {
    RootAdmissionRecord {
        schema_version: state.schema_version,
        active_policy: state.active_policy,
        current_transition: state.current_transition.map(transition_model_to_record),
        last_result: state.last_result.map(retained_model_to_record),
        last_release: state.last_release.map(released_model_to_record),
    }
}

fn transition_record_to_model(
    current: RootAdmissionTransitionRecord,
) -> FleetAdmissionRootTransitionModel {
    FleetAdmissionRootTransitionModel {
        request: prepare_request_record_to_model(current.request),
        phase: phase_record_to_model(current.phase),
        participant_catalog_digest: current.participant_catalog_digest,
        participants: current
            .participants
            .into_iter()
            .map(participant_record_to_model)
            .collect(),
        fence_request_hash: current.fence_request_hash,
        prepare_receipt_hash: current.prepare_receipt_hash,
        activate_request_hash: current.activate_request_hash,
        activate_receipt_hash: current.activate_receipt_hash,
        open_request_hash: current.open_request_hash,
    }
}

fn transition_model_to_record(
    current: FleetAdmissionRootTransitionModel,
) -> RootAdmissionTransitionRecord {
    RootAdmissionTransitionRecord {
        request: prepare_request_model_to_record(current.request),
        phase: phase_model_to_record(current.phase),
        participant_catalog_digest: current.participant_catalog_digest,
        participants: current
            .participants
            .into_iter()
            .map(participant_model_to_record)
            .collect(),
        fence_request_hash: current.fence_request_hash,
        prepare_receipt_hash: current.prepare_receipt_hash,
        activate_request_hash: current.activate_request_hash,
        activate_receipt_hash: current.activate_receipt_hash,
        open_request_hash: current.open_request_hash,
    }
}

fn retained_record_to_model(
    last: RootAdmissionRetainedResultRecord,
) -> FleetAdmissionRootRetainedResultModel {
    FleetAdmissionRootRetainedResultModel {
        request: prepare_request_record_to_model(last.request),
        participant_catalog_digest: last.participant_catalog_digest,
        participants: last
            .participants
            .into_iter()
            .map(participant_record_to_model)
            .collect(),
        fence_request_hash: last.fence_request_hash,
        prepare_receipt_hash: last.prepare_receipt_hash,
        activate_request_hash: last.activate_request_hash,
        activate_receipt_hash: last.activate_receipt_hash,
        open_request_hash: last.open_request_hash,
        receipt_hash: last.receipt_hash,
    }
}

fn retained_model_to_record(
    last: FleetAdmissionRootRetainedResultModel,
) -> RootAdmissionRetainedResultRecord {
    RootAdmissionRetainedResultRecord {
        request: prepare_request_model_to_record(last.request),
        participant_catalog_digest: last.participant_catalog_digest,
        participants: last
            .participants
            .into_iter()
            .map(participant_model_to_record)
            .collect(),
        fence_request_hash: last.fence_request_hash,
        prepare_receipt_hash: last.prepare_receipt_hash,
        activate_request_hash: last.activate_request_hash,
        activate_receipt_hash: last.activate_receipt_hash,
        open_request_hash: last.open_request_hash,
        receipt_hash: last.receipt_hash,
    }
}

fn released_record_to_model(
    last: RootAdmissionReleasedReservationRecord,
) -> FleetAdmissionRootReleasedReservationModel {
    FleetAdmissionRootReleasedReservationModel {
        request: prepare_request_record_to_model(last.request),
        participant_catalog_digest: last.participant_catalog_digest,
        participant_count: last.participant_count,
        release_request_hash: last.release_request_hash,
        receipt_hash: last.receipt_hash,
    }
}

fn released_model_to_record(
    last: FleetAdmissionRootReleasedReservationModel,
) -> RootAdmissionReleasedReservationRecord {
    RootAdmissionReleasedReservationRecord {
        request: prepare_request_model_to_record(last.request),
        participant_catalog_digest: last.participant_catalog_digest,
        participant_count: last.participant_count,
        release_request_hash: last.release_request_hash,
        receipt_hash: last.receipt_hash,
    }
}

fn prepare_request_record_to_model(
    request: RootAdmissionPrepareRequestRecord,
) -> FleetAdmissionRootPrepareRequestModel {
    FleetAdmissionRootPrepareRequestModel {
        authority: request.authority,
        root: request.root,
        operation_id: request.operation_id,
        expected_generation: request.expected_generation,
        expected_policy_digest: request.expected_policy_digest,
        successor: request.successor,
        request_hash: request.request_hash,
    }
}

fn prepare_request_model_to_record(
    request: FleetAdmissionRootPrepareRequestModel,
) -> RootAdmissionPrepareRequestRecord {
    RootAdmissionPrepareRequestRecord {
        authority: request.authority,
        root: request.root,
        operation_id: request.operation_id,
        expected_generation: request.expected_generation,
        expected_policy_digest: request.expected_policy_digest,
        successor: request.successor,
        request_hash: request.request_hash,
    }
}

const fn phase_record_to_model(phase: RootAdmissionPhaseRecord) -> FleetAdmissionRootPhaseModel {
    match phase {
        RootAdmissionPhaseRecord::Preparing => FleetAdmissionRootPhaseModel::Preparing,
        RootAdmissionPhaseRecord::PerimeterFenced => FleetAdmissionRootPhaseModel::PerimeterFenced,
        RootAdmissionPhaseRecord::Activating => FleetAdmissionRootPhaseModel::Activating,
        RootAdmissionPhaseRecord::Opening => FleetAdmissionRootPhaseModel::Opening,
    }
}

const fn phase_model_to_record(phase: FleetAdmissionRootPhaseModel) -> RootAdmissionPhaseRecord {
    match phase {
        FleetAdmissionRootPhaseModel::Preparing => RootAdmissionPhaseRecord::Preparing,
        FleetAdmissionRootPhaseModel::PerimeterFenced => RootAdmissionPhaseRecord::PerimeterFenced,
        FleetAdmissionRootPhaseModel::Activating => RootAdmissionPhaseRecord::Activating,
        FleetAdmissionRootPhaseModel::Opening => RootAdmissionPhaseRecord::Opening,
    }
}

fn participant_record_to_model(
    participant: RootAdmissionParticipantRecord,
) -> FleetAdmissionRootParticipantModel {
    FleetAdmissionRootParticipantModel {
        target: participant.target,
        projection_digest: participant.projection_digest,
        phase: match participant.phase {
            RootAdmissionParticipantPhaseRecord::Pending => {
                FleetAdmissionRootParticipantPhaseModel::Pending
            }
            RootAdmissionParticipantPhaseRecord::Prepared => {
                FleetAdmissionRootParticipantPhaseModel::Prepared
            }
            RootAdmissionParticipantPhaseRecord::Activated => {
                FleetAdmissionRootParticipantPhaseModel::Activated
            }
            RootAdmissionParticipantPhaseRecord::Open => {
                FleetAdmissionRootParticipantPhaseModel::Open
            }
        },
        last_receipt_hash: participant.last_receipt_hash,
    }
}

fn participant_model_to_record(
    participant: FleetAdmissionRootParticipantModel,
) -> RootAdmissionParticipantRecord {
    RootAdmissionParticipantRecord {
        target: participant.target,
        projection_digest: participant.projection_digest,
        phase: match participant.phase {
            FleetAdmissionRootParticipantPhaseModel::Pending => {
                RootAdmissionParticipantPhaseRecord::Pending
            }
            FleetAdmissionRootParticipantPhaseModel::Prepared => {
                RootAdmissionParticipantPhaseRecord::Prepared
            }
            FleetAdmissionRootParticipantPhaseModel::Activated => {
                RootAdmissionParticipantPhaseRecord::Activated
            }
            FleetAdmissionRootParticipantPhaseModel::Open => {
                RootAdmissionParticipantPhaseRecord::Open
            }
        },
        last_receipt_hash: participant.last_receipt_hash,
    }
}

#[cfg(test)]
mod tests;
