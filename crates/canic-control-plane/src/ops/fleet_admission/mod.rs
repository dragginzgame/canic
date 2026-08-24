//! Module: ops::fleet_admission
//!
//! Responsibility: convert, validate, persist and project the Coordinator admission authority.
//! Does not own: caller authorization, mutation policy, participant effects, or orchestration.
//! Boundary: workflow supplies the exact Registry and commits only pure-policy replacements.

use crate::storage::stable::fleet_admission::{
    FleetAdmissionAuthorityRecord, FleetAdmissionAuthorityStore,
    FleetAdmissionCoordinatorRootPhaseRecord, FleetAdmissionCoordinatorRootProgressRecord,
    FleetAdmissionCoordinatorTransitionPhaseRecord, FleetAdmissionMutationActionRecord,
    FleetAdmissionMutationOutcomeRecord, FleetAdmissionMutationRequestRecord,
    FleetAdmissionMutationResponseRecord, FleetAdmissionRetainedResultRecord,
    FleetAdmissionTransitionRecord,
};
use canic_core::{
    control_plane_support::error::InternalError,
    dto::{
        fleet_admission::{
            FleetAdmissionActivateRootRequest, FleetAdmissionMutationAction,
            FleetAdmissionMutationOutcome, FleetAdmissionMutationRequest,
            FleetAdmissionMutationResponse, FleetAdmissionOpenRootRequest,
            FleetAdmissionOperationPhase, FleetAdmissionOperationStatusResponse,
            FleetAdmissionPolicyStatus, FleetAdmissionPrepareRootRequest,
            FleetAdmissionPrepareRootStage, FleetAdmissionRootReceipt,
            FleetAdmissionRootTransitionPhase, FleetAdmissionStatusRequest,
            FleetAdmissionStatusResponse,
        },
        fleet_registry::FleetRegistry,
        page::Page,
    },
    ids::{
        FleetAdmissionPolicy, FleetAdmissionSelector, FleetCoordinatorBinding,
        FleetSubnetRootBinding,
    },
    shared_support::{
        fleet_admission_authority::{
            FLEET_ADMISSION_AUTHORITY_SCHEMA_VERSION, FleetAdmissionAuthorityPolicyError,
            FleetAdmissionAuthorityState, FleetAdmissionCoordinatorRootPhaseModel,
            FleetAdmissionCoordinatorRootProgressModel,
            FleetAdmissionCoordinatorTransitionPhaseModel, FleetAdmissionMutationActionModel,
            FleetAdmissionMutationOutcomeModel, FleetAdmissionMutationPolicyError,
            FleetAdmissionMutationRequestModel, FleetAdmissionMutationResponseModel,
            FleetAdmissionRetainedResultModel, FleetAdmissionRootCatalogAuthorityModel,
            FleetAdmissionTransitionModel, MAX_FLEET_ADMISSION_STATUS_PAGE,
            fleet_admission_mutation_request_digest, mutate_fleet_admission_membership,
            plan_fleet_admission_mutation,
        },
        fleet_admission_policy::{
            compile_installed_fleet_admission_policy, fleet_admission_participant_catalog_digest,
            fleet_admission_root_receipt_digest, fleet_admission_root_receipt_digest_from_binding,
            validate_installed_fleet_admission_policy,
        },
        fleet_admission_root::MAX_FLEET_ADMISSION_ROOT_PARTICIPANTS,
    },
};

/// Deterministic storage and DTO facade for the Coordinator admission authority.
pub struct FleetAdmissionOps;

/// One exact next Coordinator-owned distributed convergence action.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FleetAdmissionCoordinatorStep {
    PrepareRoot {
        fleet_subnet_root: candid::Principal,
        request: FleetAdmissionPrepareRootRequest,
    },
    PublishRegistry {
        request: FleetAdmissionMutationRequestModel,
        successor: FleetAdmissionPolicy,
    },
    ActivateRoot {
        fleet_subnet_root: candid::Principal,
        request: FleetAdmissionActivateRootRequest,
    },
    OpenRoot {
        fleet_subnet_root: candid::Principal,
        request: FleetAdmissionOpenRootRequest,
    },
    Complete,
    CompleteCatalogChanged,
}

impl FleetAdmissionOps {
    /// Compile the fresh generation-one policy into the sole mutable authority record.
    pub(crate) fn compile_genesis(
        policy: FleetAdmissionPolicy,
        authority: &FleetCoordinatorBinding,
    ) -> Result<FleetAdmissionAuthorityState, InternalError> {
        validate_installed_fleet_admission_policy(&policy)
            .map_err(|_error| InternalError::invalid_input())?;
        if policy.fleet != authority.fleet {
            return Err(InternalError::invalid_input());
        }
        Ok(FleetAdmissionAuthorityState {
            schema_version: FLEET_ADMISSION_AUTHORITY_SCHEMA_VERSION,
            active_policy: policy,
            current_transition: None,
            last_result: None,
        })
    }

    /// Commit fresh authority exactly once during Coordinator installation.
    pub(crate) fn commit_genesis(state: FleetAdmissionAuthorityState) -> Result<(), InternalError> {
        let record = model_to_record(state);
        if FleetAdmissionAuthorityStore::get().as_ref() == Some(&record)
            || FleetAdmissionAuthorityStore::initialize(record)
        {
            Ok(())
        } else {
            Err(InternalError::conflict())
        }
    }

    /// Plan or exactly replay one controller-authorized add/remove request.
    pub(crate) fn mutate(
        registry: &FleetRegistry,
        request: FleetAdmissionMutationRequest,
    ) -> Result<FleetAdmissionMutationResponse, InternalError> {
        let current = load_valid(registry)?;
        let request = request_to_model(request);
        let request_hash = fleet_admission_mutation_request_digest(&request);
        let retained_successor = current
            .current_transition
            .as_ref()
            .filter(|operation| operation.request.operation_id == request.operation_id)
            .map_or_else(
                || current.active_policy.clone(),
                |operation| operation.successor.clone(),
            );
        if current
            .current_transition
            .as_ref()
            .is_some_and(|operation| operation.request.operation_id == request.operation_id)
            || current
                .last_result
                .as_ref()
                .is_some_and(|result| result.request.operation_id == request.operation_id)
        {
            let decision = plan_fleet_admission_mutation(
                &current,
                &registry.authority.binding,
                request,
                request_hash,
                retained_successor,
                Vec::new(),
            )
            .map_err(map_authority_error)?;
            return Ok(response_to_dto(decision.response));
        }
        if !registry_accepts_admission_mutation(registry) {
            return Err(InternalError::conflict());
        }
        if !selector_exists(registry, &request.selector) {
            return Err(InternalError::invalid_input());
        }
        let roots = registry_roots(registry)?;
        let semantics = mutate_fleet_admission_membership(
            &current.active_policy,
            request.action,
            &request.selector,
            request.principal,
        )
        .map_err(map_membership_error)?;
        let generation = if semantics.changed {
            current
                .active_policy
                .generation
                .checked_add(1)
                .ok_or_else(InternalError::resource_exhausted)?
        } else {
            current.active_policy.generation
        };
        let successor = compile_installed_fleet_admission_policy(
            current.active_policy.fleet.clone(),
            generation,
            semantics.fleet_principals,
            semantics.rules,
        )
        .map_err(|_error| InternalError::invalid_input())?;
        let decision = plan_fleet_admission_mutation(
            &current,
            &registry.authority.binding,
            request,
            request_hash,
            successor,
            roots,
        )
        .map_err(map_authority_error)?;
        validate_state(registry, &decision.state)?;
        if !decision.replayed
            && !FleetAdmissionAuthorityStore::replace(model_to_record(decision.state))
        {
            return Err(InternalError::unavailable());
        }
        Ok(response_to_dto(decision.response))
    }

    /// Return one bounded controller-only active-policy inspection page.
    pub(crate) fn status(
        registry: &FleetRegistry,
        request: FleetAdmissionStatusRequest,
    ) -> Result<FleetAdmissionStatusResponse, InternalError> {
        let state = load_valid(registry)?;
        if !selector_exists(registry, &request.selector) {
            return Err(InternalError::invalid_input());
        }
        let principals = selector_principals(&state.active_policy, &request.selector);
        let total = principals.len() as u64;
        let entries = principals
            .into_iter()
            .skip(usize::try_from(request.page.offset).unwrap_or(usize::MAX))
            .take(
                usize::try_from(request.page.limit.min(MAX_FLEET_ADMISSION_STATUS_PAGE))
                    .unwrap_or(usize::MAX),
            )
            .collect();
        Ok(FleetAdmissionStatusResponse {
            fleet: state.active_policy.fleet.clone(),
            active: policy_status(&state.active_policy),
            selector: request.selector,
            principals: Page { entries, total },
            maximum_page_size: u16::try_from(MAX_FLEET_ADMISSION_STATUS_PAGE)
                .expect("Fleet admission page bound fits u16"),
            current_operation: state.current_transition.as_ref().map(current_status),
            last_result: state.last_result.as_ref().map(last_status),
        })
    }

    /// Resolve one exact current or retained operation without a secondary identity.
    pub(crate) fn operation_status(
        registry: &FleetRegistry,
        operation_id: [u8; 32],
    ) -> Result<Option<FleetAdmissionOperationStatusResponse>, InternalError> {
        if operation_id == [0; 32] {
            return Err(InternalError::invalid_input());
        }
        let state = load_valid(registry)?;
        if let Some(current) = &state.current_transition
            && current.request.operation_id == operation_id
        {
            return Ok(Some(current_status(current)));
        }
        if let Some(last) = &state.last_result
            && last.request.operation_id == operation_id
        {
            return Ok(Some(last_status(last)));
        }
        Ok(None)
    }

    /// Return whether the bounded admission owner currently retains this operation identity.
    pub(crate) fn retains_operation_id(
        registry: &FleetRegistry,
        operation_id: [u8; 32],
    ) -> Result<bool, InternalError> {
        if operation_id == [0; 32] {
            return Ok(false);
        }
        let state = load_valid(registry)?;
        Ok(state
            .current_transition
            .as_ref()
            .is_some_and(|current| current.request.operation_id == operation_id)
            || state
                .last_result
                .as_ref()
                .is_some_and(|last| last.request.operation_id == operation_id))
    }

    /// Reject a participant-catalog or Registry mutation while convergence is active.
    pub(crate) fn require_transition_idle(registry: &FleetRegistry) -> Result<(), InternalError> {
        if load_valid(registry)?.current_transition.is_some() {
            Err(InternalError::conflict())
        } else {
            Ok(())
        }
    }

    /// Select one deterministic next Root call or local commit boundary.
    #[expect(
        clippy::too_many_lines,
        reason = "one closed phase dispatcher keeps the Coordinator transition order explicit"
    )]
    pub(crate) fn next_step(
        registry: &FleetRegistry,
    ) -> Result<FleetAdmissionCoordinatorStep, InternalError> {
        let mut state = load_valid(registry)?;
        if state.current_transition.as_ref().is_some_and(|current| {
            current.phase == FleetAdmissionCoordinatorTransitionPhaseModel::Planned
        }) {
            let expected = state.clone();
            state
                .current_transition
                .as_mut()
                .expect("planned transition exists")
                .phase = FleetAdmissionCoordinatorTransitionPhaseModel::Preparing;
            compare_and_commit(expected, state.clone())?;
        }
        let current = state
            .current_transition
            .as_ref()
            .ok_or_else(InternalError::conflict)?;
        match current.phase {
            FleetAdmissionCoordinatorTransitionPhaseModel::Planned => unreachable!(),
            FleetAdmissionCoordinatorTransitionPhaseModel::Preparing => {
                let (root, prepare_stage) =
                    if let Some(root) = current
                        .roots
                        .iter()
                        .find(|root| root.phase == FleetAdmissionCoordinatorRootPhaseModel::Pending)
                    {
                        (root, FleetAdmissionPrepareRootStage::Reserve)
                    } else {
                        let catalogs_match =
                            participant_catalog_authority_matches(&current.request, &current.roots);
                        if current.roots.iter().all(|root| {
                            root.phase == FleetAdmissionCoordinatorRootPhaseModel::Reserved
                        }) && !catalogs_match
                        {
                            let expected = state.clone();
                            state
                                .current_transition
                                .as_mut()
                                .expect("preparing transition exists")
                                .phase = FleetAdmissionCoordinatorTransitionPhaseModel::Releasing;
                            compare_and_commit(expected, state)?;
                            return Self::next_step(registry);
                        }
                        let root = current
                            .roots
                            .iter()
                            .find(|root| {
                                root.phase == FleetAdmissionCoordinatorRootPhaseModel::Reserved
                            })
                            .ok_or_else(InternalError::invariant)?;
                        if !catalogs_match {
                            return Err(InternalError::invariant());
                        }
                        (root, FleetAdmissionPrepareRootStage::Fence)
                    };
                Ok(FleetAdmissionCoordinatorStep::PrepareRoot {
                    fleet_subnet_root: root.fleet_subnet_root,
                    request: FleetAdmissionPrepareRootRequest {
                        authority: current.request.authority.clone(),
                        operation_id: current.request.operation_id,
                        expected_generation: current.request.expected_generation,
                        expected_policy_digest: current.request.expected_policy_digest,
                        successor: current.successor.clone(),
                        stage: prepare_stage,
                    },
                })
            }
            FleetAdmissionCoordinatorTransitionPhaseModel::Releasing => {
                if let Some(root) = current
                    .roots
                    .iter()
                    .find(|root| root.phase == FleetAdmissionCoordinatorRootPhaseModel::Reserved)
                {
                    return Ok(FleetAdmissionCoordinatorStep::PrepareRoot {
                        fleet_subnet_root: root.fleet_subnet_root,
                        request: FleetAdmissionPrepareRootRequest {
                            authority: current.request.authority.clone(),
                            operation_id: current.request.operation_id,
                            expected_generation: current.request.expected_generation,
                            expected_policy_digest: current.request.expected_policy_digest,
                            successor: current.successor.clone(),
                            stage: FleetAdmissionPrepareRootStage::Release,
                        },
                    });
                }
                if current
                    .roots
                    .iter()
                    .all(|root| root.phase == FleetAdmissionCoordinatorRootPhaseModel::Released)
                {
                    Ok(FleetAdmissionCoordinatorStep::CompleteCatalogChanged)
                } else {
                    Err(InternalError::invariant())
                }
            }
            FleetAdmissionCoordinatorTransitionPhaseModel::PerimeterFenced => {
                Ok(FleetAdmissionCoordinatorStep::PublishRegistry {
                    request: current.request.clone(),
                    successor: current.successor.clone(),
                })
            }
            FleetAdmissionCoordinatorTransitionPhaseModel::Activating => {
                let root = current
                    .roots
                    .iter()
                    .find(|root| root.phase == FleetAdmissionCoordinatorRootPhaseModel::Prepared)
                    .ok_or_else(InternalError::invariant)?;
                Ok(FleetAdmissionCoordinatorStep::ActivateRoot {
                    fleet_subnet_root: root.fleet_subnet_root,
                    request: FleetAdmissionActivateRootRequest {
                        authority: current.request.authority.clone(),
                        operation_id: current.request.operation_id,
                        expected_generation: current.request.expected_generation,
                        expected_policy_digest: current.request.expected_policy_digest,
                        successor_generation: current.successor.generation,
                        successor_policy_digest: current.successor.policy_digest,
                    },
                })
            }
            FleetAdmissionCoordinatorTransitionPhaseModel::Opening => {
                if let Some(root) = current
                    .roots
                    .iter()
                    .find(|root| root.phase == FleetAdmissionCoordinatorRootPhaseModel::Activated)
                {
                    return Ok(FleetAdmissionCoordinatorStep::OpenRoot {
                        fleet_subnet_root: root.fleet_subnet_root,
                        request: FleetAdmissionOpenRootRequest {
                            authority: current.request.authority.clone(),
                            operation_id: current.request.operation_id,
                            generation: current.successor.generation,
                            policy_digest: current.successor.policy_digest,
                        },
                    });
                }
                if current
                    .roots
                    .iter()
                    .all(|root| root.phase == FleetAdmissionCoordinatorRootPhaseModel::Open)
                {
                    Ok(FleetAdmissionCoordinatorStep::Complete)
                } else {
                    Err(InternalError::invariant())
                }
            }
        }
    }

    /// Retain one exact aggregate Root receipt and monotonically advance Fleet progress.
    pub(crate) fn record_root_receipt(
        registry: &FleetRegistry,
        receipt: FleetAdmissionRootReceipt,
    ) -> Result<(), InternalError> {
        let state = load_valid(registry)?;
        let current = state
            .current_transition
            .as_ref()
            .ok_or_else(InternalError::conflict)?;
        let expected_root = registry_root_binding(registry, receipt.root.fleet_subnet_root)?;
        let current_root_phase = current
            .roots
            .iter()
            .find(|root| root.fleet_subnet_root == receipt.root.fleet_subnet_root)
            .map(|root| root.phase)
            .ok_or_else(InternalError::invariant)?;
        let (expected_root_phase, next_root_phase, expected_receipt_phase) =
            root_receipt_phase_transition(current.phase, current_root_phase, receipt.phase)?;
        let receipt_hash = fleet_admission_root_receipt_digest(
            receipt.operation_id,
            receipt.phase,
            &receipt.root,
            receipt.generation,
            receipt.policy_digest,
            receipt.participant_catalog_digest,
            receipt.participant_count,
        );
        let receipt_matches = receipt.operation_id == current.request.operation_id
            && receipt.phase == expected_receipt_phase
            && receipt.root == expected_root
            && receipt.generation == current.successor.generation
            && receipt.policy_digest == current.successor.policy_digest
            && receipt.participant_catalog_digest != [0; 32]
            && usize::try_from(receipt.participant_count)
                .is_ok_and(|count| count <= MAX_FLEET_ADMISSION_ROOT_PARTICIPANTS)
            && receipt.receipt_hash == receipt_hash;
        if !receipt_matches {
            return Err(InternalError::conflict());
        }
        let mut next = state.clone();
        let transition = next
            .current_transition
            .as_mut()
            .ok_or_else(InternalError::invariant)?;
        let progress = transition
            .roots
            .iter_mut()
            .find(|root| root.fleet_subnet_root == receipt.root.fleet_subnet_root)
            .ok_or_else(InternalError::invariant)?;
        if progress.phase == next_root_phase {
            let replay_matches = progress.last_receipt_hash == Some(receipt.receipt_hash)
                && progress.participant_catalog_digest == Some(receipt.participant_catalog_digest)
                && progress.participant_count == Some(receipt.participant_count);
            return if replay_matches {
                Ok(())
            } else {
                Err(InternalError::conflict())
            };
        }
        if progress.phase != expected_root_phase
            || progress
                .participant_catalog_digest
                .is_some_and(|digest| digest != receipt.participant_catalog_digest)
            || progress
                .participant_count
                .is_some_and(|count| count != receipt.participant_count)
        {
            return Err(InternalError::conflict());
        }
        progress.phase = next_root_phase;
        progress.participant_catalog_digest = Some(receipt.participant_catalog_digest);
        progress.participant_count = Some(receipt.participant_count);
        progress.last_receipt_hash = Some(receipt.receipt_hash);
        if transition
            .roots
            .iter()
            .all(|root| root.phase == FleetAdmissionCoordinatorRootPhaseModel::Prepared)
        {
            transition.phase = FleetAdmissionCoordinatorTransitionPhaseModel::PerimeterFenced;
        } else if transition.roots.iter().all(|root| {
            matches!(
                root.phase,
                FleetAdmissionCoordinatorRootPhaseModel::Activated
                    | FleetAdmissionCoordinatorRootPhaseModel::Open
            )
        }) {
            transition.phase = FleetAdmissionCoordinatorTransitionPhaseModel::Opening;
        }
        validate_state(registry, &next)?;
        compare_and_commit(state, next)
    }

    /// Record successful Registry publication and release the activation phase.
    pub(crate) fn record_registry_published(registry: &FleetRegistry) -> Result<(), InternalError> {
        let state = load_valid(registry)?;
        let mut next = state.clone();
        let current = next
            .current_transition
            .as_mut()
            .ok_or_else(InternalError::conflict)?;
        if current.phase != FleetAdmissionCoordinatorTransitionPhaseModel::PerimeterFenced
            || registry.admission != current.successor
            || current
                .roots
                .iter()
                .any(|root| root.phase != FleetAdmissionCoordinatorRootPhaseModel::Prepared)
        {
            return Err(InternalError::conflict());
        }
        current.phase = FleetAdmissionCoordinatorTransitionPhaseModel::Activating;
        validate_state(registry, &next)?;
        compare_and_commit(state, next)
    }

    /// Commit the converged policy and retained complete Root history.
    pub(crate) fn complete(
        registry: &FleetRegistry,
    ) -> Result<FleetAdmissionMutationResponse, InternalError> {
        let state = load_valid(registry)?;
        let current = state
            .current_transition
            .as_ref()
            .ok_or_else(InternalError::conflict)?;
        if current.phase != FleetAdmissionCoordinatorTransitionPhaseModel::Opening
            || current
                .roots
                .iter()
                .any(|root| root.phase != FleetAdmissionCoordinatorRootPhaseModel::Open)
            || registry.admission != current.successor
        {
            return Err(InternalError::conflict());
        }
        let response = FleetAdmissionMutationResponseModel {
            outcome: FleetAdmissionMutationOutcomeModel::Converged,
            operation_id: current.request.operation_id,
            generation: current.successor.generation,
            policy_digest: current.successor.policy_digest,
        };
        let mut next = state.clone();
        next.active_policy = current.successor.clone();
        next.last_result = Some(FleetAdmissionRetainedResultModel {
            request: current.request.clone(),
            request_hash: current.request_hash,
            response: response.clone(),
            roots: current.roots.clone(),
        });
        next.current_transition = None;
        validate_state(registry, &next)?;
        compare_and_commit(state, next)?;
        Ok(response_to_dto(response))
    }

    /// Retain one stale-plan result after every pre-effect Root reservation is released.
    pub(crate) fn complete_catalog_changed(
        registry: &FleetRegistry,
    ) -> Result<FleetAdmissionMutationResponse, InternalError> {
        let state = load_valid(registry)?;
        let current = state
            .current_transition
            .as_ref()
            .ok_or_else(InternalError::conflict)?;
        if current.phase != FleetAdmissionCoordinatorTransitionPhaseModel::Releasing
            || current
                .roots
                .iter()
                .any(|root| root.phase != FleetAdmissionCoordinatorRootPhaseModel::Released)
            || registry.admission != state.active_policy
        {
            return Err(InternalError::conflict());
        }
        let response = FleetAdmissionMutationResponseModel {
            outcome: FleetAdmissionMutationOutcomeModel::CatalogChanged,
            operation_id: current.request.operation_id,
            generation: state.active_policy.generation,
            policy_digest: state.active_policy.policy_digest,
        };
        let mut next = state.clone();
        next.last_result = Some(FleetAdmissionRetainedResultModel {
            request: current.request.clone(),
            request_hash: current.request_hash,
            response: response.clone(),
            roots: current.roots.clone(),
        });
        next.current_transition = None;
        validate_state(registry, &next)?;
        compare_and_commit(state, next)?;
        Ok(response_to_dto(response))
    }
}

fn compare_and_commit(
    expected: FleetAdmissionAuthorityState,
    next: FleetAdmissionAuthorityState,
) -> Result<(), InternalError> {
    if FleetAdmissionAuthorityStore::compare_and_replace(
        &model_to_record(expected),
        model_to_record(next),
    ) {
        Ok(())
    } else {
        Err(InternalError::conflict())
    }
}

const fn root_receipt_phase_transition(
    phase: FleetAdmissionCoordinatorTransitionPhaseModel,
    root_phase: FleetAdmissionCoordinatorRootPhaseModel,
    receipt_phase: FleetAdmissionRootTransitionPhase,
) -> Result<
    (
        FleetAdmissionCoordinatorRootPhaseModel,
        FleetAdmissionCoordinatorRootPhaseModel,
        FleetAdmissionRootTransitionPhase,
    ),
    InternalError,
> {
    match (phase, root_phase, receipt_phase) {
        (
            FleetAdmissionCoordinatorTransitionPhaseModel::Preparing,
            FleetAdmissionCoordinatorRootPhaseModel::Pending
            | FleetAdmissionCoordinatorRootPhaseModel::Reserved,
            FleetAdmissionRootTransitionPhase::Preparing,
        ) => Ok((
            FleetAdmissionCoordinatorRootPhaseModel::Pending,
            FleetAdmissionCoordinatorRootPhaseModel::Reserved,
            FleetAdmissionRootTransitionPhase::Preparing,
        )),
        (
            FleetAdmissionCoordinatorTransitionPhaseModel::Preparing,
            FleetAdmissionCoordinatorRootPhaseModel::Reserved
            | FleetAdmissionCoordinatorRootPhaseModel::Prepared,
            FleetAdmissionRootTransitionPhase::PerimeterFenced,
        ) => Ok((
            FleetAdmissionCoordinatorRootPhaseModel::Reserved,
            FleetAdmissionCoordinatorRootPhaseModel::Prepared,
            FleetAdmissionRootTransitionPhase::PerimeterFenced,
        )),
        (
            FleetAdmissionCoordinatorTransitionPhaseModel::Releasing,
            FleetAdmissionCoordinatorRootPhaseModel::Reserved
            | FleetAdmissionCoordinatorRootPhaseModel::Released,
            FleetAdmissionRootTransitionPhase::Released,
        ) => Ok((
            FleetAdmissionCoordinatorRootPhaseModel::Reserved,
            FleetAdmissionCoordinatorRootPhaseModel::Released,
            FleetAdmissionRootTransitionPhase::Released,
        )),
        (
            FleetAdmissionCoordinatorTransitionPhaseModel::Activating,
            FleetAdmissionCoordinatorRootPhaseModel::Prepared
            | FleetAdmissionCoordinatorRootPhaseModel::Activated,
            FleetAdmissionRootTransitionPhase::Opening,
        ) => Ok((
            FleetAdmissionCoordinatorRootPhaseModel::Prepared,
            FleetAdmissionCoordinatorRootPhaseModel::Activated,
            FleetAdmissionRootTransitionPhase::Opening,
        )),
        (
            FleetAdmissionCoordinatorTransitionPhaseModel::Opening,
            FleetAdmissionCoordinatorRootPhaseModel::Activated
            | FleetAdmissionCoordinatorRootPhaseModel::Open,
            FleetAdmissionRootTransitionPhase::Converged,
        ) => Ok((
            FleetAdmissionCoordinatorRootPhaseModel::Activated,
            FleetAdmissionCoordinatorRootPhaseModel::Open,
            FleetAdmissionRootTransitionPhase::Converged,
        )),
        _ => Err(InternalError::conflict()),
    }
}

fn load_valid(registry: &FleetRegistry) -> Result<FleetAdmissionAuthorityState, InternalError> {
    let record = FleetAdmissionAuthorityStore::get().ok_or_else(InternalError::unavailable)?;
    let state = record_to_model(record);
    validate_state(registry, &state)?;
    Ok(state)
}

fn validate_state(
    registry: &FleetRegistry,
    state: &FleetAdmissionAuthorityState,
) -> Result<(), InternalError> {
    if state.schema_version != FLEET_ADMISSION_AUTHORITY_SCHEMA_VERSION {
        return Err(InternalError::invariant());
    }
    validate_installed_fleet_admission_policy(&state.active_policy)
        .map_err(|_error| InternalError::invariant())?;
    if let Some(current) = &state.current_transition {
        validate_current(registry, state, current)?;
    } else if state.active_policy != registry.admission {
        return Err(InternalError::invariant());
    }
    if let Some(last) = &state.last_result {
        validate_last(registry, state, last)?;
    }
    if state.current_transition.as_ref().is_some_and(|current| {
        state
            .last_result
            .as_ref()
            .is_some_and(|last| current.request.operation_id == last.request.operation_id)
    }) {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn validate_current(
    registry: &FleetRegistry,
    state: &FleetAdmissionAuthorityState,
    current: &FleetAdmissionTransitionModel,
) -> Result<(), InternalError> {
    let request = &current.request;
    let semantics = mutate_fleet_admission_membership(
        &state.active_policy,
        request.action,
        &request.selector,
        request.principal,
    )
    .map_err(|_error| InternalError::invariant())?;
    let successor_generation = state
        .active_policy
        .generation
        .checked_add(1)
        .ok_or_else(InternalError::invariant)?;
    let registry_admission_matches = match current.phase {
        FleetAdmissionCoordinatorTransitionPhaseModel::Planned
        | FleetAdmissionCoordinatorTransitionPhaseModel::Preparing
        | FleetAdmissionCoordinatorTransitionPhaseModel::Releasing => {
            registry.admission == state.active_policy
        }
        FleetAdmissionCoordinatorTransitionPhaseModel::PerimeterFenced => {
            registry.admission == state.active_policy || registry.admission == current.successor
        }
        FleetAdmissionCoordinatorTransitionPhaseModel::Activating
        | FleetAdmissionCoordinatorTransitionPhaseModel::Opening => {
            registry.admission == current.successor
        }
    };
    let expected_roots = registry_roots(registry)?;
    let root_set_matches = current.roots.len() == expected_roots.len()
        && current
            .roots
            .iter()
            .zip(&expected_roots)
            .all(|(actual, expected)| {
                actual.fleet_subnet_root == expected.fleet_subnet_root
                    && actual.placement_subnet == expected.placement_subnet
            });
    let root_progress_matches = current
        .roots
        .iter()
        .all(|root| valid_root_progress(current.phase, root));
    let root_receipts_match =
        root_progress_receipts_match(request, &current.successor, &current.roots);
    if request.operation_id == [0; 32]
        || request.authority != registry.authority.binding
        || request.expected_generation != state.active_policy.generation
        || request.expected_policy_digest != state.active_policy.policy_digest
        || request.successor_policy_digest != current.successor.policy_digest
        || current.request_hash != fleet_admission_mutation_request_digest(request)
        || !semantics.changed
        || current.successor.fleet != state.active_policy.fleet
        || current.successor.generation != successor_generation
        || current.successor.fleet_principals != semantics.fleet_principals
        || current.successor.rules != semantics.rules
        || !selector_exists(registry, &request.selector)
        || !registry_admission_matches
        || !root_set_matches
        || !root_progress_matches
        || !root_receipts_match
        || (matches!(
            current.phase,
            FleetAdmissionCoordinatorTransitionPhaseModel::PerimeterFenced
                | FleetAdmissionCoordinatorTransitionPhaseModel::Activating
                | FleetAdmissionCoordinatorTransitionPhaseModel::Opening
        ) && !participant_catalog_authority_matches(request, &current.roots))
        || current.phase == FleetAdmissionCoordinatorTransitionPhaseModel::Releasing
            && participant_catalog_authority_matches(request, &current.roots)
    {
        return Err(InternalError::invariant());
    }
    validate_installed_fleet_admission_policy(&current.successor)
        .map_err(|_error| InternalError::invariant())
}

fn validate_last(
    registry: &FleetRegistry,
    state: &FleetAdmissionAuthorityState,
    last: &FleetAdmissionRetainedResultModel,
) -> Result<(), InternalError> {
    let request = &last.request;
    let response = &last.response;
    let request_hash_matches =
        last.request_hash == fleet_admission_mutation_request_digest(request);
    let retained_successor = match response.outcome {
        FleetAdmissionMutationOutcomeModel::CatalogChanged => {
            let semantics = mutate_fleet_admission_membership(
                &state.active_policy,
                request.action,
                &request.selector,
                request.principal,
            )
            .map_err(|_error| InternalError::invariant())?;
            if !semantics.changed {
                return Err(InternalError::invariant());
            }
            let generation = state
                .active_policy
                .generation
                .checked_add(1)
                .ok_or_else(InternalError::invariant)?;
            compile_installed_fleet_admission_policy(
                state.active_policy.fleet.clone(),
                generation,
                semantics.fleet_principals,
                semantics.rules,
            )
            .map_err(|_error| InternalError::invariant())?
        }
        FleetAdmissionMutationOutcomeModel::Converged
        | FleetAdmissionMutationOutcomeModel::AlreadyPresent
        | FleetAdmissionMutationOutcomeModel::AlreadyAbsent => state.active_policy.clone(),
        FleetAdmissionMutationOutcomeModel::Planned => return Err(InternalError::invariant()),
    };
    let root_receipts_match =
        root_progress_receipts_match(request, &retained_successor, &last.roots);
    let terminal_matches = match response.outcome {
        FleetAdmissionMutationOutcomeModel::Planned => false,
        FleetAdmissionMutationOutcomeModel::Converged => request
            .expected_generation
            .checked_add(1)
            .is_some_and(|generation| generation == state.active_policy.generation),
        FleetAdmissionMutationOutcomeModel::CatalogChanged
        | FleetAdmissionMutationOutcomeModel::AlreadyPresent
        | FleetAdmissionMutationOutcomeModel::AlreadyAbsent => {
            request.expected_generation == state.active_policy.generation
                && request.expected_policy_digest == state.active_policy.policy_digest
        }
    };
    let root_history_matches = match response.outcome {
        FleetAdmissionMutationOutcomeModel::Converged => {
            !last.roots.is_empty()
                && participant_catalog_authority_matches(request, &last.roots)
                && last.roots.windows(2).all(|pair| {
                    pair[0].fleet_subnet_root.as_slice() < pair[1].fleet_subnet_root.as_slice()
                })
                && last.roots.iter().all(|root| {
                    root.phase == FleetAdmissionCoordinatorRootPhaseModel::Open
                        && valid_retained_root_progress(root)
                })
        }
        FleetAdmissionMutationOutcomeModel::CatalogChanged => {
            !last.roots.is_empty()
                && !participant_catalog_authority_matches(request, &last.roots)
                && last.roots.windows(2).all(|pair| {
                    pair[0].fleet_subnet_root.as_slice() < pair[1].fleet_subnet_root.as_slice()
                })
                && last.roots.iter().all(|root| {
                    root.phase == FleetAdmissionCoordinatorRootPhaseModel::Released
                        && valid_retained_root_progress(root)
                })
        }
        FleetAdmissionMutationOutcomeModel::AlreadyPresent
        | FleetAdmissionMutationOutcomeModel::AlreadyAbsent => last.roots.is_empty(),
        FleetAdmissionMutationOutcomeModel::Planned => false,
    };
    if request.operation_id == [0; 32]
        || request.authority != registry.authority.binding
        || response.operation_id != request.operation_id
        || response.generation != state.active_policy.generation
        || response.policy_digest != state.active_policy.policy_digest
        || request.successor_policy_digest != retained_successor.policy_digest
        || !request_hash_matches
        || !root_receipts_match
        || !terminal_matches
        || !root_history_matches
    {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn participant_catalog_authority_matches(
    request: &FleetAdmissionMutationRequestModel,
    roots: &[FleetAdmissionCoordinatorRootProgressModel],
) -> bool {
    retained_participant_catalog_authority(roots).is_some_and(|(digest, count)| {
        digest == request.participant_catalog_digest && count == request.participant_count
    })
}

fn retained_participant_catalog_authority(
    roots: &[FleetAdmissionCoordinatorRootProgressModel],
) -> Option<([u8; 32], u32)> {
    if roots.is_empty()
        || !roots
            .windows(2)
            .all(|pair| pair[0].fleet_subnet_root.as_slice() < pair[1].fleet_subnet_root.as_slice())
    {
        return None;
    }
    let mut total = 0_u32;
    let mut catalogs = Vec::with_capacity(roots.len());
    for root in roots {
        let participant_catalog_digest = root.participant_catalog_digest?;
        let participant_count = root.participant_count?;
        if participant_catalog_digest == [0; 32]
            || usize::try_from(participant_count)
                .map_or(true, |count| count > MAX_FLEET_ADMISSION_ROOT_PARTICIPANTS)
        {
            return None;
        }
        total = total.checked_add(participant_count)?;
        catalogs.push(FleetAdmissionRootCatalogAuthorityModel {
            fleet_subnet_root: root.fleet_subnet_root,
            participant_catalog_digest,
            participant_count,
        });
    }
    if usize::try_from(total).map_or(true, |count| count > MAX_FLEET_ADMISSION_ROOT_PARTICIPANTS) {
        return None;
    }
    Some((fleet_admission_participant_catalog_digest(&catalogs), total))
}

fn valid_root_progress(
    transition_phase: FleetAdmissionCoordinatorTransitionPhaseModel,
    root: &FleetAdmissionCoordinatorRootProgressModel,
) -> bool {
    let phase_allowed = match transition_phase {
        FleetAdmissionCoordinatorTransitionPhaseModel::Planned => {
            root.phase == FleetAdmissionCoordinatorRootPhaseModel::Pending
        }
        FleetAdmissionCoordinatorTransitionPhaseModel::Preparing => matches!(
            root.phase,
            FleetAdmissionCoordinatorRootPhaseModel::Pending
                | FleetAdmissionCoordinatorRootPhaseModel::Reserved
                | FleetAdmissionCoordinatorRootPhaseModel::Prepared
        ),
        FleetAdmissionCoordinatorTransitionPhaseModel::Releasing => matches!(
            root.phase,
            FleetAdmissionCoordinatorRootPhaseModel::Reserved
                | FleetAdmissionCoordinatorRootPhaseModel::Released
        ),
        FleetAdmissionCoordinatorTransitionPhaseModel::PerimeterFenced => {
            root.phase == FleetAdmissionCoordinatorRootPhaseModel::Prepared
        }
        FleetAdmissionCoordinatorTransitionPhaseModel::Activating => matches!(
            root.phase,
            FleetAdmissionCoordinatorRootPhaseModel::Prepared
                | FleetAdmissionCoordinatorRootPhaseModel::Activated
        ),
        FleetAdmissionCoordinatorTransitionPhaseModel::Opening => matches!(
            root.phase,
            FleetAdmissionCoordinatorRootPhaseModel::Activated
                | FleetAdmissionCoordinatorRootPhaseModel::Open
        ),
    };
    let progress_matches = if root.phase == FleetAdmissionCoordinatorRootPhaseModel::Pending {
        root.participant_catalog_digest.is_none()
            && root.participant_count.is_none()
            && root.last_receipt_hash.is_none()
    } else {
        valid_retained_root_progress(root)
    };
    phase_allowed && progress_matches
}

fn valid_retained_root_progress(root: &FleetAdmissionCoordinatorRootProgressModel) -> bool {
    valid_expected_root_catalog(root)
        && root
            .last_receipt_hash
            .is_some_and(|digest| digest != [0; 32])
}

fn valid_expected_root_catalog(root: &FleetAdmissionCoordinatorRootProgressModel) -> bool {
    root.participant_catalog_digest
        .is_some_and(|digest| digest != [0; 32])
        && root.participant_count.is_some_and(|count| {
            usize::try_from(count).is_ok_and(|count| count <= MAX_FLEET_ADMISSION_ROOT_PARTICIPANTS)
        })
}

fn root_progress_receipts_match(
    request: &FleetAdmissionMutationRequestModel,
    successor: &FleetAdmissionPolicy,
    roots: &[FleetAdmissionCoordinatorRootProgressModel],
) -> bool {
    for root in roots {
        let Some(phase) = (match root.phase {
            FleetAdmissionCoordinatorRootPhaseModel::Pending => None,
            FleetAdmissionCoordinatorRootPhaseModel::Reserved => {
                Some(FleetAdmissionRootTransitionPhase::Preparing)
            }
            FleetAdmissionCoordinatorRootPhaseModel::Prepared => {
                Some(FleetAdmissionRootTransitionPhase::PerimeterFenced)
            }
            FleetAdmissionCoordinatorRootPhaseModel::Activated => {
                Some(FleetAdmissionRootTransitionPhase::Opening)
            }
            FleetAdmissionCoordinatorRootPhaseModel::Open => {
                Some(FleetAdmissionRootTransitionPhase::Converged)
            }
            FleetAdmissionCoordinatorRootPhaseModel::Released => {
                Some(FleetAdmissionRootTransitionPhase::Released)
            }
        }) else {
            if root.participant_catalog_digest.is_some()
                || root.participant_count.is_some()
                || root.last_receipt_hash.is_some()
            {
                return false;
            }
            continue;
        };
        let Some(participant_catalog_digest) = root.participant_catalog_digest else {
            return false;
        };
        let Some(participant_count) = root.participant_count else {
            return false;
        };
        let expected = fleet_admission_root_receipt_digest_from_binding(
            request.operation_id,
            phase,
            &request.authority,
            root.placement_subnet,
            root.fleet_subnet_root,
            successor.generation,
            successor.policy_digest,
            participant_catalog_digest,
            participant_count,
        );
        if root.last_receipt_hash != Some(expected) {
            return false;
        }
    }
    true
}

fn registry_root_binding(
    registry: &FleetRegistry,
    fleet_subnet_root: candid::Principal,
) -> Result<FleetSubnetRootBinding, InternalError> {
    let mut matches = registry
        .fleet_subnet_roots
        .iter()
        .filter(|root| root.fleet_subnet_root == fleet_subnet_root);
    let root = matches.next().ok_or_else(InternalError::conflict)?;
    if matches.next().is_some()
        || root.status != canic_core::dto::fleet_registry::FleetSubnetRootStatus::Active
    {
        return Err(InternalError::conflict());
    }
    Ok(FleetSubnetRootBinding {
        authority: registry.authority.clone(),
        placement_subnet: root.placement_subnet,
        fleet_subnet_root: root.fleet_subnet_root,
        component_admissions: root.component_admissions.clone(),
        component_topology_digest: root.component_topology_digest,
        limits: root.limits.clone(),
        funding: root.funding.clone(),
    })
}

fn selector_exists(registry: &FleetRegistry, selector: &FleetAdmissionSelector) -> bool {
    match selector {
        FleetAdmissionSelector::Fleet => true,
        FleetAdmissionSelector::ComponentSpec(component_spec) => registry
            .component_specs
            .iter()
            .any(|entry| &entry.component_spec == component_spec),
        FleetAdmissionSelector::ComponentInstance(component) => {
            registry.services.iter().any(|service| {
                service.members.iter().any(|member| {
                    &member.component == component
                        && registry.fleet_subnet_roots.iter().any(|root| {
                            root.fleet_subnet_root == member.fleet_subnet_root
                                && root.status
                                    == canic_core::dto::fleet_registry::FleetSubnetRootStatus::Active
                        })
                })
            })
        }
        FleetAdmissionSelector::FleetSubnetRoot(placement_subnet) => registry
            .fleet_subnet_roots
            .iter()
            .any(|root| {
                &root.placement_subnet == placement_subnet
                    && root.status
                        == canic_core::dto::fleet_registry::FleetSubnetRootStatus::Active
            }),
    }
}

fn registry_accepts_admission_mutation(registry: &FleetRegistry) -> bool {
    registry
        .fleet_subnet_roots
        .iter()
        .any(|root| root.status == canic_core::dto::fleet_registry::FleetSubnetRootStatus::Active)
        && registry.fleet_subnet_roots.iter().all(|root| {
            matches!(
                root.status,
                canic_core::dto::fleet_registry::FleetSubnetRootStatus::Active
                    | canic_core::dto::fleet_registry::FleetSubnetRootStatus::Removed
            )
        })
}

fn selector_principals(
    policy: &FleetAdmissionPolicy,
    selector: &FleetAdmissionSelector,
) -> Vec<candid::Principal> {
    if selector == &FleetAdmissionSelector::Fleet {
        return policy.fleet_principals.clone();
    }
    policy
        .rules
        .binary_search_by(|rule| rule.selector.cmp(selector))
        .ok()
        .map_or_else(
            || policy.fleet_principals.clone(),
            |index| policy.rules[index].principals.clone(),
        )
}

fn policy_status(policy: &FleetAdmissionPolicy) -> FleetAdmissionPolicyStatus {
    let narrower_principal_reference_count = policy
        .rules
        .iter()
        .map(|rule| rule.principals.len())
        .sum::<usize>();
    FleetAdmissionPolicyStatus {
        generation: policy.generation,
        policy_digest: policy.policy_digest,
        fleet_principal_count: u16::try_from(policy.fleet_principals.len())
            .expect("validated Fleet admission Principal count fits u16"),
        narrower_rule_count: u16::try_from(policy.rules.len())
            .expect("validated Fleet admission rule count fits u16"),
        narrower_principal_reference_count: u16::try_from(narrower_principal_reference_count)
            .expect("validated Fleet admission rule-reference count fits u16"),
    }
}

fn current_status(
    current: &FleetAdmissionTransitionModel,
) -> FleetAdmissionOperationStatusResponse {
    let successor = policy_status(&current.successor);
    FleetAdmissionOperationStatusResponse {
        operation_id: current.request.operation_id,
        action: action_to_dto(current.request.action),
        selector: current.request.selector.clone(),
        principal: current.request.principal,
        phase: match current.phase {
            FleetAdmissionCoordinatorTransitionPhaseModel::Planned => {
                FleetAdmissionOperationPhase::Planned { successor }
            }
            FleetAdmissionCoordinatorTransitionPhaseModel::Preparing => {
                FleetAdmissionOperationPhase::Preparing { successor }
            }
            FleetAdmissionCoordinatorTransitionPhaseModel::Releasing => {
                FleetAdmissionOperationPhase::Releasing { successor }
            }
            FleetAdmissionCoordinatorTransitionPhaseModel::PerimeterFenced => {
                FleetAdmissionOperationPhase::PerimeterFenced { successor }
            }
            FleetAdmissionCoordinatorTransitionPhaseModel::Activating => {
                FleetAdmissionOperationPhase::Activating { successor }
            }
            FleetAdmissionCoordinatorTransitionPhaseModel::Opening => {
                FleetAdmissionOperationPhase::Opening { successor }
            }
        },
    }
}

fn last_status(last: &FleetAdmissionRetainedResultModel) -> FleetAdmissionOperationStatusResponse {
    FleetAdmissionOperationStatusResponse {
        operation_id: last.request.operation_id,
        action: action_to_dto(last.request.action),
        selector: last.request.selector.clone(),
        principal: last.request.principal,
        phase: FleetAdmissionOperationPhase::Completed(response_to_dto(last.response.clone())),
    }
}

fn request_to_model(request: FleetAdmissionMutationRequest) -> FleetAdmissionMutationRequestModel {
    FleetAdmissionMutationRequestModel {
        authority: request.authority,
        expected_generation: request.expected_generation,
        expected_policy_digest: request.expected_policy_digest,
        action: match request.action {
            FleetAdmissionMutationAction::Add => FleetAdmissionMutationActionModel::Add,
            FleetAdmissionMutationAction::Remove => FleetAdmissionMutationActionModel::Remove,
        },
        selector: request.selector,
        principal: request.principal,
        operation_id: request.operation_id,
        successor_policy_digest: request.successor_policy_digest,
        participant_catalog_digest: request.participant_catalog_digest,
        participant_count: request.participant_count,
    }
}

const fn response_to_dto(
    response: FleetAdmissionMutationResponseModel,
) -> FleetAdmissionMutationResponse {
    FleetAdmissionMutationResponse {
        outcome: match response.outcome {
            FleetAdmissionMutationOutcomeModel::Planned => FleetAdmissionMutationOutcome::Planned,
            FleetAdmissionMutationOutcomeModel::Converged => {
                FleetAdmissionMutationOutcome::Converged
            }
            FleetAdmissionMutationOutcomeModel::CatalogChanged => {
                FleetAdmissionMutationOutcome::CatalogChanged
            }
            FleetAdmissionMutationOutcomeModel::AlreadyPresent => {
                FleetAdmissionMutationOutcome::AlreadyPresent
            }
            FleetAdmissionMutationOutcomeModel::AlreadyAbsent => {
                FleetAdmissionMutationOutcome::AlreadyAbsent
            }
        },
        operation_id: response.operation_id,
        generation: response.generation,
        policy_digest: response.policy_digest,
    }
}

const fn action_to_dto(action: FleetAdmissionMutationActionModel) -> FleetAdmissionMutationAction {
    match action {
        FleetAdmissionMutationActionModel::Add => FleetAdmissionMutationAction::Add,
        FleetAdmissionMutationActionModel::Remove => FleetAdmissionMutationAction::Remove,
    }
}

const fn map_membership_error(error: FleetAdmissionMutationPolicyError) -> InternalError {
    match error {
        FleetAdmissionMutationPolicyError::PrincipalCapacityExhausted
        | FleetAdmissionMutationPolicyError::RuleCapacityExhausted
        | FleetAdmissionMutationPolicyError::RulePrincipalReferenceCapacityExhausted => {
            InternalError::resource_exhausted()
        }
        FleetAdmissionMutationPolicyError::AnonymousPrincipal
        | FleetAdmissionMutationPolicyError::EmptyFleet
        | FleetAdmissionMutationPolicyError::RuleWidensFleet => InternalError::invalid_input(),
    }
}

const fn map_authority_error(error: FleetAdmissionAuthorityPolicyError) -> InternalError {
    match error {
        FleetAdmissionAuthorityPolicyError::EmptyOperationId
        | FleetAdmissionAuthorityPolicyError::AuthorityMismatch
        | FleetAdmissionAuthorityPolicyError::InvalidSuccessor => InternalError::invalid_input(),
        FleetAdmissionAuthorityPolicyError::GenerationExhausted => {
            InternalError::resource_exhausted()
        }
        FleetAdmissionAuthorityPolicyError::OperationConflict
        | FleetAdmissionAuthorityPolicyError::OperationInProgress
        | FleetAdmissionAuthorityPolicyError::GenerationConflict
        | FleetAdmissionAuthorityPolicyError::PolicyDigestConflict => InternalError::conflict(),
        FleetAdmissionAuthorityPolicyError::UnsupportedSchema
        | FleetAdmissionAuthorityPolicyError::InvalidCurrentTransition
        | FleetAdmissionAuthorityPolicyError::InvalidRetainedResult => InternalError::invariant(),
    }
}

fn record_to_model(record: FleetAdmissionAuthorityRecord) -> FleetAdmissionAuthorityState {
    FleetAdmissionAuthorityState {
        schema_version: record.schema_version,
        active_policy: record.active_policy,
        current_transition: record.current_transition.map(|current| {
            FleetAdmissionTransitionModel {
                request: request_record_to_model(current.request),
                request_hash: current.request_hash,
                successor: current.successor,
                phase: coordinator_phase_record_to_model(current.phase),
                roots: current
                    .roots
                    .into_iter()
                    .map(root_progress_record_to_model)
                    .collect(),
            }
        }),
        last_result: record
            .last_result
            .map(|last| FleetAdmissionRetainedResultModel {
                request: request_record_to_model(last.request),
                request_hash: last.request_hash,
                response: response_record_to_model(last.response),
                roots: last
                    .roots
                    .into_iter()
                    .map(root_progress_record_to_model)
                    .collect(),
            }),
    }
}

fn model_to_record(state: FleetAdmissionAuthorityState) -> FleetAdmissionAuthorityRecord {
    FleetAdmissionAuthorityRecord {
        schema_version: state.schema_version,
        active_policy: state.active_policy,
        current_transition: state.current_transition.map(|current| {
            FleetAdmissionTransitionRecord {
                request: request_model_to_record(current.request),
                request_hash: current.request_hash,
                successor: current.successor,
                phase: coordinator_phase_model_to_record(current.phase),
                roots: current
                    .roots
                    .into_iter()
                    .map(root_progress_model_to_record)
                    .collect(),
            }
        }),
        last_result: state
            .last_result
            .map(|last| FleetAdmissionRetainedResultRecord {
                request: request_model_to_record(last.request),
                request_hash: last.request_hash,
                response: response_model_to_record(last.response),
                roots: last
                    .roots
                    .into_iter()
                    .map(root_progress_model_to_record)
                    .collect(),
            }),
    }
}

fn registry_roots(
    registry: &FleetRegistry,
) -> Result<Vec<FleetAdmissionCoordinatorRootProgressModel>, InternalError> {
    let mut active_roots = registry
        .fleet_subnet_roots
        .iter()
        .filter(|root| {
            root.status == canic_core::dto::fleet_registry::FleetSubnetRootStatus::Active
        })
        .collect::<Vec<_>>();
    active_roots.sort_by(|left, right| {
        left.fleet_subnet_root
            .as_slice()
            .cmp(right.fleet_subnet_root.as_slice())
    });
    if active_roots.is_empty()
        || active_roots
            .windows(2)
            .any(|pair| pair[0].fleet_subnet_root == pair[1].fleet_subnet_root)
    {
        return Err(InternalError::conflict());
    }
    Ok(active_roots
        .into_iter()
        .map(|root| FleetAdmissionCoordinatorRootProgressModel {
            fleet_subnet_root: root.fleet_subnet_root,
            placement_subnet: root.placement_subnet,
            phase: FleetAdmissionCoordinatorRootPhaseModel::Pending,
            participant_catalog_digest: None,
            participant_count: None,
            last_receipt_hash: None,
        })
        .collect())
}

const fn coordinator_phase_record_to_model(
    phase: FleetAdmissionCoordinatorTransitionPhaseRecord,
) -> FleetAdmissionCoordinatorTransitionPhaseModel {
    match phase {
        FleetAdmissionCoordinatorTransitionPhaseRecord::Planned => {
            FleetAdmissionCoordinatorTransitionPhaseModel::Planned
        }
        FleetAdmissionCoordinatorTransitionPhaseRecord::Preparing => {
            FleetAdmissionCoordinatorTransitionPhaseModel::Preparing
        }
        FleetAdmissionCoordinatorTransitionPhaseRecord::Releasing => {
            FleetAdmissionCoordinatorTransitionPhaseModel::Releasing
        }
        FleetAdmissionCoordinatorTransitionPhaseRecord::PerimeterFenced => {
            FleetAdmissionCoordinatorTransitionPhaseModel::PerimeterFenced
        }
        FleetAdmissionCoordinatorTransitionPhaseRecord::Activating => {
            FleetAdmissionCoordinatorTransitionPhaseModel::Activating
        }
        FleetAdmissionCoordinatorTransitionPhaseRecord::Opening => {
            FleetAdmissionCoordinatorTransitionPhaseModel::Opening
        }
    }
}

const fn coordinator_phase_model_to_record(
    phase: FleetAdmissionCoordinatorTransitionPhaseModel,
) -> FleetAdmissionCoordinatorTransitionPhaseRecord {
    match phase {
        FleetAdmissionCoordinatorTransitionPhaseModel::Planned => {
            FleetAdmissionCoordinatorTransitionPhaseRecord::Planned
        }
        FleetAdmissionCoordinatorTransitionPhaseModel::Preparing => {
            FleetAdmissionCoordinatorTransitionPhaseRecord::Preparing
        }
        FleetAdmissionCoordinatorTransitionPhaseModel::Releasing => {
            FleetAdmissionCoordinatorTransitionPhaseRecord::Releasing
        }
        FleetAdmissionCoordinatorTransitionPhaseModel::PerimeterFenced => {
            FleetAdmissionCoordinatorTransitionPhaseRecord::PerimeterFenced
        }
        FleetAdmissionCoordinatorTransitionPhaseModel::Activating => {
            FleetAdmissionCoordinatorTransitionPhaseRecord::Activating
        }
        FleetAdmissionCoordinatorTransitionPhaseModel::Opening => {
            FleetAdmissionCoordinatorTransitionPhaseRecord::Opening
        }
    }
}

const fn root_progress_record_to_model(
    root: FleetAdmissionCoordinatorRootProgressRecord,
) -> FleetAdmissionCoordinatorRootProgressModel {
    FleetAdmissionCoordinatorRootProgressModel {
        fleet_subnet_root: root.fleet_subnet_root,
        placement_subnet: root.placement_subnet,
        phase: match root.phase {
            FleetAdmissionCoordinatorRootPhaseRecord::Pending => {
                FleetAdmissionCoordinatorRootPhaseModel::Pending
            }
            FleetAdmissionCoordinatorRootPhaseRecord::Reserved => {
                FleetAdmissionCoordinatorRootPhaseModel::Reserved
            }
            FleetAdmissionCoordinatorRootPhaseRecord::Prepared => {
                FleetAdmissionCoordinatorRootPhaseModel::Prepared
            }
            FleetAdmissionCoordinatorRootPhaseRecord::Activated => {
                FleetAdmissionCoordinatorRootPhaseModel::Activated
            }
            FleetAdmissionCoordinatorRootPhaseRecord::Open => {
                FleetAdmissionCoordinatorRootPhaseModel::Open
            }
            FleetAdmissionCoordinatorRootPhaseRecord::Released => {
                FleetAdmissionCoordinatorRootPhaseModel::Released
            }
        },
        participant_catalog_digest: root.participant_catalog_digest,
        participant_count: root.participant_count,
        last_receipt_hash: root.last_receipt_hash,
    }
}

const fn root_progress_model_to_record(
    root: FleetAdmissionCoordinatorRootProgressModel,
) -> FleetAdmissionCoordinatorRootProgressRecord {
    FleetAdmissionCoordinatorRootProgressRecord {
        fleet_subnet_root: root.fleet_subnet_root,
        placement_subnet: root.placement_subnet,
        phase: match root.phase {
            FleetAdmissionCoordinatorRootPhaseModel::Pending => {
                FleetAdmissionCoordinatorRootPhaseRecord::Pending
            }
            FleetAdmissionCoordinatorRootPhaseModel::Reserved => {
                FleetAdmissionCoordinatorRootPhaseRecord::Reserved
            }
            FleetAdmissionCoordinatorRootPhaseModel::Prepared => {
                FleetAdmissionCoordinatorRootPhaseRecord::Prepared
            }
            FleetAdmissionCoordinatorRootPhaseModel::Activated => {
                FleetAdmissionCoordinatorRootPhaseRecord::Activated
            }
            FleetAdmissionCoordinatorRootPhaseModel::Open => {
                FleetAdmissionCoordinatorRootPhaseRecord::Open
            }
            FleetAdmissionCoordinatorRootPhaseModel::Released => {
                FleetAdmissionCoordinatorRootPhaseRecord::Released
            }
        },
        participant_catalog_digest: root.participant_catalog_digest,
        participant_count: root.participant_count,
        last_receipt_hash: root.last_receipt_hash,
    }
}

fn request_record_to_model(
    request: FleetAdmissionMutationRequestRecord,
) -> FleetAdmissionMutationRequestModel {
    FleetAdmissionMutationRequestModel {
        authority: request.authority,
        expected_generation: request.expected_generation,
        expected_policy_digest: request.expected_policy_digest,
        action: match request.action {
            FleetAdmissionMutationActionRecord::Add => FleetAdmissionMutationActionModel::Add,
            FleetAdmissionMutationActionRecord::Remove => FleetAdmissionMutationActionModel::Remove,
        },
        selector: request.selector,
        principal: request.principal,
        operation_id: request.operation_id,
        successor_policy_digest: request.successor_policy_digest,
        participant_catalog_digest: request.participant_catalog_digest,
        participant_count: request.participant_count,
    }
}

fn request_model_to_record(
    request: FleetAdmissionMutationRequestModel,
) -> FleetAdmissionMutationRequestRecord {
    FleetAdmissionMutationRequestRecord {
        authority: request.authority,
        expected_generation: request.expected_generation,
        expected_policy_digest: request.expected_policy_digest,
        action: match request.action {
            FleetAdmissionMutationActionModel::Add => FleetAdmissionMutationActionRecord::Add,
            FleetAdmissionMutationActionModel::Remove => FleetAdmissionMutationActionRecord::Remove,
        },
        selector: request.selector,
        principal: request.principal,
        operation_id: request.operation_id,
        successor_policy_digest: request.successor_policy_digest,
        participant_catalog_digest: request.participant_catalog_digest,
        participant_count: request.participant_count,
    }
}

const fn response_record_to_model(
    response: FleetAdmissionMutationResponseRecord,
) -> FleetAdmissionMutationResponseModel {
    FleetAdmissionMutationResponseModel {
        outcome: match response.outcome {
            FleetAdmissionMutationOutcomeRecord::Planned => {
                FleetAdmissionMutationOutcomeModel::Planned
            }
            FleetAdmissionMutationOutcomeRecord::Converged => {
                FleetAdmissionMutationOutcomeModel::Converged
            }
            FleetAdmissionMutationOutcomeRecord::CatalogChanged => {
                FleetAdmissionMutationOutcomeModel::CatalogChanged
            }
            FleetAdmissionMutationOutcomeRecord::AlreadyPresent => {
                FleetAdmissionMutationOutcomeModel::AlreadyPresent
            }
            FleetAdmissionMutationOutcomeRecord::AlreadyAbsent => {
                FleetAdmissionMutationOutcomeModel::AlreadyAbsent
            }
        },
        operation_id: response.operation_id,
        generation: response.generation,
        policy_digest: response.policy_digest,
    }
}

const fn response_model_to_record(
    response: FleetAdmissionMutationResponseModel,
) -> FleetAdmissionMutationResponseRecord {
    FleetAdmissionMutationResponseRecord {
        outcome: match response.outcome {
            FleetAdmissionMutationOutcomeModel::Planned => {
                FleetAdmissionMutationOutcomeRecord::Planned
            }
            FleetAdmissionMutationOutcomeModel::Converged => {
                FleetAdmissionMutationOutcomeRecord::Converged
            }
            FleetAdmissionMutationOutcomeModel::CatalogChanged => {
                FleetAdmissionMutationOutcomeRecord::CatalogChanged
            }
            FleetAdmissionMutationOutcomeModel::AlreadyPresent => {
                FleetAdmissionMutationOutcomeRecord::AlreadyPresent
            }
            FleetAdmissionMutationOutcomeModel::AlreadyAbsent => {
                FleetAdmissionMutationOutcomeRecord::AlreadyAbsent
            }
        },
        operation_id: response.operation_id,
        generation: response.generation,
        policy_digest: response.policy_digest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::{
        cdk::structures::storable::Storable,
        cdk::types::Cycles,
        dto::fleet_registry::{FleetRegistry, FleetSubnetRootEntry, FleetSubnetRootStatus},
        ids::{
            AppId, CanonicalNetworkId, ComponentTopologyDigest, CyclesFundingBudget, FleetBinding,
            FleetCoordinatorBinding, FleetId, FleetKey, FleetRegistryAuthority,
            FleetSubnetCanisterPoolConfig, FleetSubnetRootLimits, FleetSubnetRootReleaseSet,
            ReleaseBuildId, ReleaseBuildNonce, ReleaseSetDigest, SubnetId,
        },
    };

    #[test]
    fn genesis_and_stable_conversion_preserve_the_exact_compiled_authority() {
        let (registry, state) = fixture();
        validate_state(&registry, &state).expect("valid state");
        assert_eq!(record_to_model(model_to_record(state.clone())), state);
        assert_eq!(
            selector_principals(&state.active_policy, &FleetAdmissionSelector::Fleet),
            state.active_policy.fleet_principals
        );
    }

    #[test]
    fn public_response_projection_preserves_every_terminal_identity() {
        let response = FleetAdmissionMutationResponseModel {
            outcome: FleetAdmissionMutationOutcomeModel::Converged,
            operation_id: [6; 32],
            generation: 9,
            policy_digest: [7; 32],
        };
        assert_eq!(
            response_to_dto(response),
            FleetAdmissionMutationResponse {
                outcome: FleetAdmissionMutationOutcome::Converged,
                operation_id: [6; 32],
                generation: 9,
                policy_digest: [7; 32],
            }
        );
    }

    #[test]
    fn fleet_catalog_authority_retains_roots_with_zero_enrolled_targets() {
        let roots = vec![
            FleetAdmissionCoordinatorRootProgressModel {
                fleet_subnet_root: principal(1),
                placement_subnet: SubnetId::from_principal(principal(11)),
                phase: FleetAdmissionCoordinatorRootPhaseModel::Prepared,
                participant_catalog_digest: Some([21; 32]),
                participant_count: Some(0),
                last_receipt_hash: Some([31; 32]),
            },
            FleetAdmissionCoordinatorRootProgressModel {
                fleet_subnet_root: principal(2),
                placement_subnet: SubnetId::from_principal(principal(12)),
                phase: FleetAdmissionCoordinatorRootPhaseModel::Prepared,
                participant_catalog_digest: Some([22; 32]),
                participant_count: Some(1),
                last_receipt_hash: Some([32; 32]),
            },
        ];

        let (digest, count) = retained_participant_catalog_authority(&roots)
            .expect("zero-target Root remains in the exact Fleet catalog");
        assert_ne!(digest, [0; 32]);
        assert_eq!(count, 1);
        assert!(roots.iter().all(valid_expected_root_catalog));
    }

    #[test]
    fn effective_request_is_planned_once_and_exactly_replayed() {
        let (registry, state) = fixture();
        FleetAdmissionOps::commit_genesis(state.clone()).expect("commit genesis");
        let added = principal(8);
        let successor = compile_installed_fleet_admission_policy(
            state.active_policy.fleet.clone(),
            2,
            vec![state.active_policy.fleet_principals[0], added],
            Vec::new(),
        )
        .expect("successor");
        let request = FleetAdmissionMutationRequest {
            authority: registry.authority.binding.clone(),
            expected_generation: 1,
            expected_policy_digest: state.active_policy.policy_digest,
            action: FleetAdmissionMutationAction::Add,
            selector: FleetAdmissionSelector::Fleet,
            principal: added,
            operation_id: [9; 32],
            successor_policy_digest: successor.policy_digest,
            participant_catalog_digest: participant_catalog_authority(&registry).0,
            participant_count: participant_catalog_authority(&registry).1,
        };

        let accepted = FleetAdmissionOps::mutate(&registry, request.clone()).expect("plan");
        assert_eq!(accepted.outcome, FleetAdmissionMutationOutcome::Planned);
        assert_eq!(accepted.generation, 2);
        assert_eq!(
            FleetAdmissionOps::mutate(&registry, request.clone()).expect("exact replay"),
            accepted
        );
        assert!(matches!(
            FleetAdmissionOps::operation_status(&registry, [9; 32]).expect("operation status"),
            Some(FleetAdmissionOperationStatusResponse {
                phase: FleetAdmissionOperationPhase::Planned { .. },
                ..
            })
        ));

        let mut conflicting = request;
        conflicting.principal = principal(10);
        let error = FleetAdmissionOps::mutate(&registry, conflicting)
            .expect_err("operation identity conflict");
        assert_eq!(
            error.public_error().code(),
            canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
        );
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one complete Coordinator phase replay journey"
    )]
    fn effective_request_converges_all_root_phases_and_replays_terminally() {
        let (mut registry, state) = fixture();
        FleetAdmissionOps::commit_genesis(state.clone()).expect("commit genesis");
        let added = principal(8);
        let successor = compile_installed_fleet_admission_policy(
            state.active_policy.fleet.clone(),
            2,
            vec![state.active_policy.fleet_principals[0], added],
            Vec::new(),
        )
        .expect("successor");
        let request = FleetAdmissionMutationRequest {
            authority: registry.authority.binding.clone(),
            expected_generation: 1,
            expected_policy_digest: state.active_policy.policy_digest,
            action: FleetAdmissionMutationAction::Add,
            selector: FleetAdmissionSelector::Fleet,
            principal: added,
            operation_id: [31; 32],
            successor_policy_digest: successor.policy_digest,
            participant_catalog_digest: participant_catalog_authority(&registry).0,
            participant_count: participant_catalog_authority(&registry).1,
        };
        FleetAdmissionOps::mutate(&registry, request.clone()).expect("plan");
        let prepare_step = FleetAdmissionOps::next_step(&registry).expect("prepare step");
        assert!(matches!(
            &prepare_step,
            FleetAdmissionCoordinatorStep::PrepareRoot {
                request: FleetAdmissionPrepareRootRequest {
                    stage: FleetAdmissionPrepareRootStage::Reserve,
                    ..
                },
                ..
            }
        ));
        reload_admission_authority();
        assert_eq!(
            FleetAdmissionOps::next_step(&registry).expect("restored prepare step"),
            prepare_step
        );

        let root =
            registry_root_binding(&registry, registry.fleet_subnet_roots[0].fleet_subnet_root)
                .expect("root binding");
        let reserved = root_receipt(
            &request,
            &successor,
            root.clone(),
            FleetAdmissionRootTransitionPhase::Preparing,
            [32; 32],
        );
        FleetAdmissionOps::record_root_receipt(&registry, reserved).expect("reserved");
        assert!(matches!(
            FleetAdmissionOps::next_step(&registry).expect("fence step"),
            FleetAdmissionCoordinatorStep::PrepareRoot {
                request: FleetAdmissionPrepareRootRequest {
                    stage: FleetAdmissionPrepareRootStage::Fence,
                    ..
                },
                ..
            }
        ));
        let prepared = root_receipt(
            &request,
            &successor,
            root.clone(),
            FleetAdmissionRootTransitionPhase::PerimeterFenced,
            [32; 32],
        );
        FleetAdmissionOps::record_root_receipt(&registry, prepared).expect("prepared");
        let publish_step = FleetAdmissionOps::next_step(&registry).expect("publish step");
        assert!(matches!(
            &publish_step,
            FleetAdmissionCoordinatorStep::PublishRegistry { .. }
        ));
        reload_admission_authority();
        assert_eq!(
            FleetAdmissionOps::next_step(&registry).expect("restored publish step"),
            publish_step
        );

        registry.admission = successor.clone();
        registry.revision += 1;
        FleetAdmissionOps::record_registry_published(&registry).expect("published");
        let activate_step = FleetAdmissionOps::next_step(&registry).expect("activate step");
        assert!(matches!(
            &activate_step,
            FleetAdmissionCoordinatorStep::ActivateRoot { .. }
        ));
        reload_admission_authority();
        assert_eq!(
            FleetAdmissionOps::next_step(&registry).expect("restored activate step"),
            activate_step
        );
        let activated = root_receipt(
            &request,
            &successor,
            root.clone(),
            FleetAdmissionRootTransitionPhase::Opening,
            [32; 32],
        );
        FleetAdmissionOps::record_root_receipt(&registry, activated).expect("activated");
        let open_step = FleetAdmissionOps::next_step(&registry).expect("open step");
        assert!(matches!(
            &open_step,
            FleetAdmissionCoordinatorStep::OpenRoot { .. }
        ));
        reload_admission_authority();
        assert_eq!(
            FleetAdmissionOps::next_step(&registry).expect("restored open step"),
            open_step
        );
        let opened = root_receipt(
            &request,
            &successor,
            root,
            FleetAdmissionRootTransitionPhase::Converged,
            [32; 32],
        );
        FleetAdmissionOps::record_root_receipt(&registry, opened).expect("opened");
        reload_admission_authority();
        assert_eq!(
            FleetAdmissionOps::next_step(&registry).expect("complete step"),
            FleetAdmissionCoordinatorStep::Complete
        );
        let completed = FleetAdmissionOps::complete(&registry).expect("complete");
        assert_eq!(completed.outcome, FleetAdmissionMutationOutcome::Converged);
        assert_eq!(
            FleetAdmissionOps::mutate(&registry, request).expect("terminal replay"),
            completed
        );
    }

    fn reload_admission_authority() {
        let record = FleetAdmissionAuthorityStore::get().expect("admission authority");
        let restored = FleetAdmissionAuthorityRecord::from_bytes(record.to_bytes());
        assert!(FleetAdmissionAuthorityStore::replace(restored));
    }

    #[test]
    fn idempotent_request_is_terminal_and_status_pages_only_the_active_policy() {
        let (registry, state) = fixture();
        FleetAdmissionOps::commit_genesis(state.clone()).expect("commit genesis");
        let request = FleetAdmissionMutationRequest {
            authority: registry.authority.binding.clone(),
            expected_generation: 1,
            expected_policy_digest: state.active_policy.policy_digest,
            action: FleetAdmissionMutationAction::Add,
            selector: FleetAdmissionSelector::Fleet,
            principal: state.active_policy.fleet_principals[0],
            operation_id: [11; 32],
            successor_policy_digest: state.active_policy.policy_digest,
            participant_catalog_digest: participant_catalog_authority(&registry).0,
            participant_count: participant_catalog_authority(&registry).1,
        };

        let response = FleetAdmissionOps::mutate(&registry, request).expect("terminal no-op");
        assert_eq!(
            response.outcome,
            FleetAdmissionMutationOutcome::AlreadyPresent
        );
        let status = FleetAdmissionOps::status(
            &registry,
            FleetAdmissionStatusRequest {
                selector: FleetAdmissionSelector::Fleet,
                page: canic_core::dto::page::PageRequest {
                    limit: u64::MAX,
                    offset: 0,
                },
            },
        )
        .expect("status");
        assert_eq!(status.principals.total, 1);
        assert_eq!(
            status.principals.entries,
            state.active_policy.fleet_principals
        );
        assert!(status.current_operation.is_none());
        assert!(matches!(
            status.last_result,
            Some(FleetAdmissionOperationStatusResponse {
                phase: FleetAdmissionOperationPhase::Completed(_),
                ..
            })
        ));
    }

    #[test]
    fn preactivation_registry_rejects_a_new_mutation_before_state_change() {
        let (mut registry, state) = fixture();
        registry.fleet_subnet_roots[0].status = FleetSubnetRootStatus::Joining;
        FleetAdmissionOps::commit_genesis(state.clone()).expect("commit genesis");
        let request = FleetAdmissionMutationRequest {
            authority: registry.authority.binding.clone(),
            expected_generation: 1,
            expected_policy_digest: state.active_policy.policy_digest,
            action: FleetAdmissionMutationAction::Add,
            selector: FleetAdmissionSelector::Fleet,
            principal: principal(12),
            operation_id: [13; 32],
            successor_policy_digest: [14; 32],
            participant_catalog_digest: participant_catalog_authority(&registry).0,
            participant_count: participant_catalog_authority(&registry).1,
        };

        let error = FleetAdmissionOps::mutate(&registry, request)
            .expect_err("Joining Registry must reject");
        assert_eq!(
            error.public_error().code(),
            canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
        );
        assert!(
            FleetAdmissionAuthorityStore::get()
                .expect("retained state")
                .current_transition
                .is_none()
        );
    }

    #[test]
    fn restore_rejects_a_corrupt_retained_root_receipt_hash() {
        let (registry, state) = fixture();
        FleetAdmissionOps::commit_genesis(state.clone()).expect("commit genesis");
        let added = principal(45);
        let successor = compile_installed_fleet_admission_policy(
            state.active_policy.fleet.clone(),
            2,
            vec![state.active_policy.fleet_principals[0], added],
            Vec::new(),
        )
        .expect("successor");
        let request = FleetAdmissionMutationRequest {
            authority: registry.authority.binding.clone(),
            expected_generation: state.active_policy.generation,
            expected_policy_digest: state.active_policy.policy_digest,
            action: FleetAdmissionMutationAction::Add,
            selector: FleetAdmissionSelector::Fleet,
            principal: added,
            operation_id: [46; 32],
            successor_policy_digest: successor.policy_digest,
            participant_catalog_digest: participant_catalog_authority(&registry).0,
            participant_count: participant_catalog_authority(&registry).1,
        };
        FleetAdmissionOps::mutate(&registry, request.clone()).expect("plan mutation");
        let FleetAdmissionCoordinatorStep::PrepareRoot {
            fleet_subnet_root, ..
        } = FleetAdmissionOps::next_step(&registry).expect("reserve Root catalog")
        else {
            panic!("expected Root reservation")
        };
        let root = registry_root_binding(&registry, fleet_subnet_root).expect("registered Root");
        let reserved = root_receipt(
            &request,
            &successor,
            root,
            FleetAdmissionRootTransitionPhase::Preparing,
            [47; 32],
        );
        FleetAdmissionOps::record_root_receipt(&registry, reserved).expect("retain reservation");

        let mut corrupt = FleetAdmissionAuthorityStore::get().expect("retained authority");
        corrupt
            .current_transition
            .as_mut()
            .expect("current transition")
            .roots[0]
            .last_receipt_hash = Some([48; 32]);
        assert!(FleetAdmissionAuthorityStore::replace(corrupt));
        assert!(FleetAdmissionOps::next_step(&registry).is_err());
    }

    fn root_receipt(
        request: &FleetAdmissionMutationRequest,
        successor: &FleetAdmissionPolicy,
        root: FleetSubnetRootBinding,
        phase: FleetAdmissionRootTransitionPhase,
        participant_catalog_digest: [u8; 32],
    ) -> FleetAdmissionRootReceipt {
        let participant_count = 1;
        let receipt_hash = fleet_admission_root_receipt_digest(
            request.operation_id,
            phase,
            &root,
            successor.generation,
            successor.policy_digest,
            participant_catalog_digest,
            participant_count,
        );
        FleetAdmissionRootReceipt {
            operation_id: request.operation_id,
            phase,
            root,
            generation: successor.generation,
            policy_digest: successor.policy_digest,
            participant_catalog_digest,
            participant_count,
            receipt_hash,
        }
    }

    #[test]
    fn authority_successor_and_selector_substitutions_fail_before_commit() {
        let (registry, state) = fixture();
        FleetAdmissionOps::commit_genesis(state.clone()).expect("commit genesis");
        let added = principal(14);
        let successor = compile_installed_fleet_admission_policy(
            state.active_policy.fleet.clone(),
            2,
            vec![state.active_policy.fleet_principals[0], added],
            Vec::new(),
        )
        .expect("successor");
        let request = FleetAdmissionMutationRequest {
            authority: registry.authority.binding.clone(),
            expected_generation: 1,
            expected_policy_digest: state.active_policy.policy_digest,
            action: FleetAdmissionMutationAction::Add,
            selector: FleetAdmissionSelector::Fleet,
            principal: added,
            operation_id: [15; 32],
            successor_policy_digest: successor.policy_digest,
            participant_catalog_digest: participant_catalog_authority(&registry).0,
            participant_count: participant_catalog_authority(&registry).1,
        };

        let mut wrong_authority = request.clone();
        wrong_authority.authority.coordinator = principal(16);
        assert!(FleetAdmissionOps::mutate(&registry, wrong_authority).is_err());

        let mut wrong_successor = request.clone();
        wrong_successor.successor_policy_digest[0] ^= 1;
        assert!(FleetAdmissionOps::mutate(&registry, wrong_successor).is_err());

        let mut unknown_selector = request;
        unknown_selector.selector =
            FleetAdmissionSelector::ComponentSpec("unknown".parse().expect("Component Spec ID"));
        assert!(FleetAdmissionOps::mutate(&registry, unknown_selector).is_err());

        assert!(
            FleetAdmissionAuthorityStore::get()
                .expect("retained state")
                .current_transition
                .is_none()
        );
    }

    #[test]
    fn mismatched_reserved_catalog_releases_before_any_participant_effect() {
        let (registry, state) = fixture();
        FleetAdmissionOps::commit_genesis(state.clone()).expect("commit genesis");
        let added = principal(18);
        let successor = compile_installed_fleet_admission_policy(
            state.active_policy.fleet.clone(),
            2,
            vec![state.active_policy.fleet_principals[0], added],
            Vec::new(),
        )
        .expect("successor");
        let mut participant_catalog_digest = participant_catalog_authority(&registry).0;
        participant_catalog_digest[0] ^= 1;
        let request = FleetAdmissionMutationRequest {
            authority: registry.authority.binding.clone(),
            expected_generation: 1,
            expected_policy_digest: state.active_policy.policy_digest,
            action: FleetAdmissionMutationAction::Add,
            selector: FleetAdmissionSelector::Fleet,
            principal: added,
            operation_id: [19; 32],
            successor_policy_digest: successor.policy_digest,
            participant_catalog_digest,
            participant_count: 1,
        };

        assert_eq!(
            FleetAdmissionOps::mutate(&registry, request.clone())
                .expect("retain no-effect plan")
                .outcome,
            FleetAdmissionMutationOutcome::Planned
        );
        assert!(matches!(
            FleetAdmissionOps::next_step(&registry).expect("prepare step"),
            FleetAdmissionCoordinatorStep::PrepareRoot { .. }
        ));
        let root =
            registry_root_binding(&registry, registry.fleet_subnet_roots[0].fleet_subnet_root)
                .expect("root binding");
        let reserved = root_receipt(
            &request,
            &successor,
            root.clone(),
            FleetAdmissionRootTransitionPhase::Preparing,
            [32; 32],
        );
        FleetAdmissionOps::record_root_receipt(&registry, reserved).expect("reserve stale catalog");
        assert!(matches!(
            FleetAdmissionOps::next_step(&registry).expect("release stale reservation"),
            FleetAdmissionCoordinatorStep::PrepareRoot {
                request: FleetAdmissionPrepareRootRequest {
                    stage: FleetAdmissionPrepareRootStage::Release,
                    ..
                },
                ..
            }
        ));
        let released = root_receipt(
            &request,
            &successor,
            root,
            FleetAdmissionRootTransitionPhase::Released,
            [32; 32],
        );
        FleetAdmissionOps::record_root_receipt(&registry, released).expect("release Root catalog");
        assert_eq!(
            FleetAdmissionOps::next_step(&registry).expect("catalog-changed terminal step"),
            FleetAdmissionCoordinatorStep::CompleteCatalogChanged
        );
        let completed =
            FleetAdmissionOps::complete_catalog_changed(&registry).expect("retain stale plan");
        assert_eq!(
            completed.outcome,
            FleetAdmissionMutationOutcome::CatalogChanged
        );
        assert_eq!(completed.generation, state.active_policy.generation);
        assert_eq!(
            FleetAdmissionOps::mutate(&registry, request).expect("terminal stale-plan replay"),
            completed
        );
        assert_eq!(
            FleetAdmissionAuthorityStore::get()
                .expect("retained stale-plan result")
                .last_result
                .expect("last result")
                .response
                .outcome,
            FleetAdmissionMutationOutcomeRecord::CatalogChanged
        );
        assert_eq!(registry.admission, state.active_policy);
    }

    fn fixture() -> (FleetRegistry, FleetAdmissionAuthorityState) {
        let fleet = FleetBinding {
            fleet: FleetKey {
                canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                fleet_id: FleetId::from_generated_bytes([3; 32]),
            },
            app: AppId::from("demo"),
        };
        let authority = FleetCoordinatorBinding {
            fleet: fleet.clone(),
            coordinator_subnet: SubnetId::from_principal(principal(4)),
            coordinator: principal(5),
        };
        let policy = crate::test_support::fleet_admission_policy(fleet);
        let registry = FleetRegistry {
            authority: FleetRegistryAuthority {
                binding: authority.clone(),
                epoch: 1,
            },
            revision: 1,
            admission: policy.clone(),
            component_specs: Vec::new(),
            fleet_subnet_roots: vec![FleetSubnetRootEntry {
                placement_subnet: SubnetId::from_principal(principal(6)),
                fleet_subnet_root: principal(7),
                component_admissions: Vec::new(),
                component_topology_digest: ComponentTopologyDigest::from_bytes([8; 32]),
                active_release_set: FleetSubnetRootReleaseSet {
                    release_build_id: ReleaseBuildId::from_nonce(
                        ReleaseBuildNonce::from_random_bytes([9; 32]),
                    ),
                    manifest_digest: ReleaseSetDigest::from_bytes([10; 32]),
                },
                limits: FleetSubnetRootLimits {
                    maximum_component_instances: 1,
                    maximum_registry_bytes: 1,
                    maximum_wasm_store_bytes: 1,
                    canister_pool: FleetSubnetCanisterPoolConfig {
                        minimum_size: 0,
                        maximum_size: 0,
                        canister_cycles: Cycles::new(1),
                    },
                    cycles_funding: CyclesFundingBudget {
                        window_secs: 1,
                        maximum_cycles: Cycles::new(1),
                    },
                    maximum_group_placements: 1,
                },
                funding: crate::test_support::fleet_subnet_root_funding_authority(),
                status: FleetSubnetRootStatus::Active,
            }],
            services: Vec::new(),
        };
        let state = FleetAdmissionOps::compile_genesis(policy, &authority).expect("genesis");
        (registry, state)
    }

    fn participant_catalog_authority(registry: &FleetRegistry) -> ([u8; 32], u32) {
        let catalogs = [FleetAdmissionRootCatalogAuthorityModel {
            fleet_subnet_root: registry.fleet_subnet_roots[0].fleet_subnet_root,
            participant_catalog_digest: [32; 32],
            participant_count: 1,
        }];
        (fleet_admission_participant_catalog_digest(&catalogs), 1)
    }

    fn principal(byte: u8) -> candid::Principal {
        candid::Principal::from_slice(&[byte; 29])
    }
}
