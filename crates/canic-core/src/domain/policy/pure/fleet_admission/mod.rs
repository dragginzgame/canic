//! Module: domain::policy::pure::fleet_admission
//!
//! Responsibility: derive effective target membership and decide one canonical policy mutation.
//! Does not own: policy validation, hashing, storage, caller acquisition, or orchestration.
//! Boundary: workflow supplies validated policy plus exact Registry-derived target or mutation.

use crate::{
    ids::{
        FleetAdmissionPolicy, FleetAdmissionPolicyTemplate, FleetAdmissionRule,
        FleetAdmissionSelector, FleetAdmissionTarget, MAX_FLEET_ADMISSION_PRINCIPALS,
        MAX_FLEET_ADMISSION_RULE_PRINCIPAL_REFERENCES, MAX_FLEET_ADMISSION_RULES,
    },
    model::fleet_admission_authority::FleetAdmissionMutationActionModel,
    model::fleet_admission_authority::{
        FLEET_ADMISSION_AUTHORITY_SCHEMA_VERSION, FleetAdmissionAuthorityState,
        FleetAdmissionCoordinatorRootPhaseModel, FleetAdmissionCoordinatorRootProgressModel,
        FleetAdmissionCoordinatorTransitionPhaseModel, FleetAdmissionMutationOutcomeModel,
        FleetAdmissionMutationRequestModel, FleetAdmissionMutationResponseModel,
        FleetAdmissionRetainedResultModel, FleetAdmissionTransitionModel,
    },
};
use candid::Principal;
use thiserror::Error as ThisError;

/// Pure mutation rejection before a successor policy is compiled.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ThisError)]
pub enum FleetAdmissionMutationPolicyError {
    #[error("Fleet admission mutation must not use the anonymous Principal")]
    AnonymousPrincipal,
    #[error("Fleet admission mutation would remove the final Fleet Principal")]
    EmptyFleet,
    #[error("Fleet admission Principal capacity is exhausted")]
    PrincipalCapacityExhausted,
    #[error("Fleet admission narrower-rule capacity is exhausted")]
    RuleCapacityExhausted,
    #[error("Fleet admission narrower-rule Principal-reference capacity is exhausted")]
    RulePrincipalReferenceCapacityExhausted,
    #[error("Fleet admission narrower mutation attempts to widen the Fleet set")]
    RuleWidensFleet,
}

/// Complete canonical membership semantics produced by one add/remove action.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetAdmissionMembershipMutation {
    pub fleet_principals: Vec<Principal>,
    pub rules: Vec<FleetAdmissionRule>,
    pub changed: bool,
}

/// Coordinator authority or replay invariant rejected by one mutation request.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ThisError)]
pub enum FleetAdmissionAuthorityPolicyError {
    #[error("Fleet admission authority schema is unsupported")]
    UnsupportedSchema,
    #[error("Fleet admission operation ID is all zero")]
    EmptyOperationId,
    #[error("Fleet admission operation ID was reused for another request")]
    OperationConflict,
    #[error("another Fleet admission transition is already active")]
    OperationInProgress,
    #[error("Fleet admission mutation authority does not match the installed Coordinator")]
    AuthorityMismatch,
    #[error("Fleet admission expected generation does not match")]
    GenerationConflict,
    #[error("Fleet admission expected policy digest does not match")]
    PolicyDigestConflict,
    #[error("Fleet admission generation is exhausted")]
    GenerationExhausted,
    #[error("Fleet admission compiled successor is inconsistent")]
    InvalidSuccessor,
    #[error("Fleet admission current transition is inconsistent")]
    InvalidCurrentTransition,
    #[error("Fleet admission retained result is inconsistent")]
    InvalidRetainedResult,
}

/// Complete pure decision for one new request or exact replay.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetAdmissionMutationDecision {
    pub state: FleetAdmissionAuthorityState,
    pub response: FleetAdmissionMutationResponseModel,
    pub replayed: bool,
}

/// Derive the deterministic intersection of the Fleet set and every matching narrower rule.
#[must_use]
pub fn effective_fleet_admission_principals(
    policy: &FleetAdmissionPolicy,
    target: &FleetAdmissionTarget,
) -> Vec<Principal> {
    effective_principals(&policy.fleet_principals, &policy.rules, target)
}

/// Derive one generation-one target set before the exact Fleet identity is allocated.
#[must_use]
pub fn effective_fleet_admission_template_principals(
    template: &FleetAdmissionPolicyTemplate,
    target: &FleetAdmissionTarget,
) -> Vec<Principal> {
    effective_principals(&template.fleet_principals, &template.rules, target)
}

/// Decide one selector-local add/remove operation without hashing or mutation.
pub fn mutate_fleet_admission_membership(
    policy: &FleetAdmissionPolicy,
    action: FleetAdmissionMutationActionModel,
    selector: &FleetAdmissionSelector,
    principal: Principal,
) -> Result<FleetAdmissionMembershipMutation, FleetAdmissionMutationPolicyError> {
    if principal == Principal::anonymous() {
        return Err(FleetAdmissionMutationPolicyError::AnonymousPrincipal);
    }
    let mut fleet_principals = policy.fleet_principals.clone();
    let mut rules = policy.rules.clone();
    let changed = match selector {
        FleetAdmissionSelector::Fleet => {
            mutate_fleet_membership(&mut fleet_principals, &mut rules, action, principal)?
        }
        narrower => {
            mutate_narrower_membership(&fleet_principals, &mut rules, action, narrower, principal)?
        }
    };
    if rules.len() > MAX_FLEET_ADMISSION_RULES {
        return Err(FleetAdmissionMutationPolicyError::RuleCapacityExhausted);
    }
    let reference_count = rules.iter().try_fold(0_usize, |count, rule| {
        count
            .checked_add(rule.principals.len())
            .ok_or(FleetAdmissionMutationPolicyError::RulePrincipalReferenceCapacityExhausted)
    })?;
    if reference_count > MAX_FLEET_ADMISSION_RULE_PRINCIPAL_REFERENCES {
        return Err(FleetAdmissionMutationPolicyError::RulePrincipalReferenceCapacityExhausted);
    }
    Ok(FleetAdmissionMembershipMutation {
        fleet_principals,
        rules,
        changed,
    })
}

/// Decide one exact mutation after ops has compiled and validated its successor.
pub fn plan_fleet_admission_mutation(
    state: &FleetAdmissionAuthorityState,
    installed_authority: &crate::ids::FleetCoordinatorBinding,
    request: FleetAdmissionMutationRequestModel,
    request_hash: [u8; 32],
    successor: FleetAdmissionPolicy,
    roots: Vec<FleetAdmissionCoordinatorRootProgressModel>,
) -> Result<FleetAdmissionMutationDecision, FleetAdmissionAuthorityPolicyError> {
    if state.schema_version != FLEET_ADMISSION_AUTHORITY_SCHEMA_VERSION {
        return Err(FleetAdmissionAuthorityPolicyError::UnsupportedSchema);
    }
    if request.operation_id == [0; 32] {
        return Err(FleetAdmissionAuthorityPolicyError::EmptyOperationId);
    }
    if let Some(replay) = replay_mutation(state, &request, request_hash) {
        return replay;
    }
    if state.current_transition.is_some() {
        return Err(FleetAdmissionAuthorityPolicyError::OperationInProgress);
    }
    if &request.authority != installed_authority
        || request.authority.fleet != state.active_policy.fleet
    {
        return Err(FleetAdmissionAuthorityPolicyError::AuthorityMismatch);
    }
    if request.expected_generation != state.active_policy.generation {
        return Err(FleetAdmissionAuthorityPolicyError::GenerationConflict);
    }
    if request.expected_policy_digest != state.active_policy.policy_digest {
        return Err(FleetAdmissionAuthorityPolicyError::PolicyDigestConflict);
    }
    let semantics = mutate_fleet_admission_membership(
        &state.active_policy,
        request.action,
        &request.selector,
        request.principal,
    )
    .map_err(|_error| FleetAdmissionAuthorityPolicyError::InvalidSuccessor)?;
    let expected_generation = if semantics.changed {
        state
            .active_policy
            .generation
            .checked_add(1)
            .ok_or(FleetAdmissionAuthorityPolicyError::GenerationExhausted)?
    } else {
        state.active_policy.generation
    };
    let successor_matches = successor.fleet == state.active_policy.fleet
        && successor.generation == expected_generation
        && successor.fleet_principals == semantics.fleet_principals
        && successor.rules == semantics.rules
        && successor.policy_digest == request.successor_policy_digest;
    if !successor_matches {
        return Err(FleetAdmissionAuthorityPolicyError::InvalidSuccessor);
    }
    validate_initial_transition_authority(&request, &roots, semantics.changed)?;

    let response = FleetAdmissionMutationResponseModel {
        outcome: if semantics.changed {
            FleetAdmissionMutationOutcomeModel::Planned
        } else {
            match request.action {
                FleetAdmissionMutationActionModel::Add => {
                    FleetAdmissionMutationOutcomeModel::AlreadyPresent
                }
                FleetAdmissionMutationActionModel::Remove => {
                    FleetAdmissionMutationOutcomeModel::AlreadyAbsent
                }
            }
        },
        operation_id: request.operation_id,
        generation: successor.generation,
        policy_digest: successor.policy_digest,
    };
    let mut next = state.clone();
    if semantics.changed {
        if roots.is_empty() {
            return Err(FleetAdmissionAuthorityPolicyError::InvalidCurrentTransition);
        }
        next.current_transition = Some(FleetAdmissionTransitionModel {
            request,
            request_hash,
            successor,
            phase: FleetAdmissionCoordinatorTransitionPhaseModel::Planned,
            roots,
        });
    } else {
        next.last_result = Some(FleetAdmissionRetainedResultModel {
            request,
            request_hash,
            response: response.clone(),
            roots: Vec::new(),
        });
    }
    Ok(FleetAdmissionMutationDecision {
        state: next,
        response,
        replayed: false,
    })
}

fn validate_initial_transition_authority(
    request: &FleetAdmissionMutationRequestModel,
    roots: &[FleetAdmissionCoordinatorRootProgressModel],
    changed: bool,
) -> Result<(), FleetAdmissionAuthorityPolicyError> {
    if !changed {
        return Ok(());
    }
    let roots_have_no_progress = roots.iter().all(|root| {
        root.phase == FleetAdmissionCoordinatorRootPhaseModel::Pending
            && root.participant_catalog_digest.is_none()
            && root.participant_count.is_none()
            && root.last_receipt_hash.is_none()
    });
    let participant_count_is_bounded =
        usize::try_from(request.participant_count).is_ok_and(|count| {
            count <= crate::model::fleet_admission_root::MAX_FLEET_ADMISSION_ROOT_PARTICIPANTS
        });
    if request.participant_catalog_digest == [0; 32]
        || !participant_count_is_bounded
        || !roots_have_no_progress
    {
        return Err(FleetAdmissionAuthorityPolicyError::InvalidCurrentTransition);
    }
    Ok(())
}

fn replay_mutation(
    state: &FleetAdmissionAuthorityState,
    request: &FleetAdmissionMutationRequestModel,
    request_hash: [u8; 32],
) -> Option<Result<FleetAdmissionMutationDecision, FleetAdmissionAuthorityPolicyError>> {
    let response = if let Some(current) = &state.current_transition
        && current.request.operation_id == request.operation_id
    {
        if current.request_hash != request_hash {
            return Some(Err(FleetAdmissionAuthorityPolicyError::OperationConflict));
        }
        planned_response(current)
    } else if let Some(last) = &state.last_result
        && last.request.operation_id == request.operation_id
    {
        if last.request_hash != request_hash {
            return Some(Err(FleetAdmissionAuthorityPolicyError::OperationConflict));
        }
        last.response.clone()
    } else {
        return None;
    };
    Some(Ok(FleetAdmissionMutationDecision {
        state: state.clone(),
        response,
        replayed: true,
    }))
}

const fn planned_response(
    current: &FleetAdmissionTransitionModel,
) -> FleetAdmissionMutationResponseModel {
    FleetAdmissionMutationResponseModel {
        outcome: FleetAdmissionMutationOutcomeModel::Planned,
        operation_id: current.request.operation_id,
        generation: current.successor.generation,
        policy_digest: current.successor.policy_digest,
    }
}

fn mutate_fleet_membership(
    fleet_principals: &mut Vec<Principal>,
    rules: &mut [FleetAdmissionRule],
    action: FleetAdmissionMutationActionModel,
    principal: Principal,
) -> Result<bool, FleetAdmissionMutationPolicyError> {
    let position = fleet_principals.binary_search(&principal);
    match (action, position) {
        (FleetAdmissionMutationActionModel::Add, Ok(_))
        | (FleetAdmissionMutationActionModel::Remove, Err(_)) => Ok(false),
        (FleetAdmissionMutationActionModel::Add, Err(index)) => {
            if fleet_principals.len() == MAX_FLEET_ADMISSION_PRINCIPALS {
                return Err(FleetAdmissionMutationPolicyError::PrincipalCapacityExhausted);
            }
            fleet_principals.insert(index, principal);
            Ok(true)
        }
        (FleetAdmissionMutationActionModel::Remove, Ok(index)) => {
            if fleet_principals.len() == 1 {
                return Err(FleetAdmissionMutationPolicyError::EmptyFleet);
            }
            fleet_principals.remove(index);
            for rule in rules {
                if let Ok(index) = rule.principals.binary_search(&principal) {
                    rule.principals.remove(index);
                }
            }
            Ok(true)
        }
    }
}

fn mutate_narrower_membership(
    fleet_principals: &[Principal],
    rules: &mut Vec<FleetAdmissionRule>,
    action: FleetAdmissionMutationActionModel,
    selector: &FleetAdmissionSelector,
    principal: Principal,
) -> Result<bool, FleetAdmissionMutationPolicyError> {
    let principal_in_fleet = fleet_principals.binary_search(&principal).is_ok();
    let rule_position = rules.binary_search_by(|rule| rule.selector.cmp(selector));
    match (action, rule_position) {
        (FleetAdmissionMutationActionModel::Add, Err(_)) => {
            if principal_in_fleet {
                Ok(false)
            } else {
                Err(FleetAdmissionMutationPolicyError::RuleWidensFleet)
            }
        }
        (FleetAdmissionMutationActionModel::Add, Ok(rule_index)) => {
            if !principal_in_fleet {
                return Err(FleetAdmissionMutationPolicyError::RuleWidensFleet);
            }
            let principal_position = rules[rule_index].principals.binary_search(&principal);
            let Err(principal_index) = principal_position else {
                return Ok(false);
            };
            rules[rule_index]
                .principals
                .insert(principal_index, principal);
            if rules[rule_index].principals == fleet_principals {
                rules.remove(rule_index);
            }
            Ok(true)
        }
        (FleetAdmissionMutationActionModel::Remove, Ok(rule_index)) => {
            let principal_position = rules[rule_index].principals.binary_search(&principal);
            let Ok(principal_index) = principal_position else {
                return Ok(false);
            };
            rules[rule_index].principals.remove(principal_index);
            Ok(true)
        }
        (FleetAdmissionMutationActionModel::Remove, Err(rule_index)) => {
            if !principal_in_fleet {
                return Ok(false);
            }
            let mut principals = fleet_principals.to_vec();
            let principal_index = principals
                .binary_search(&principal)
                .expect("Fleet membership was checked above");
            principals.remove(principal_index);
            rules.insert(
                rule_index,
                FleetAdmissionRule {
                    selector: selector.clone(),
                    principals,
                },
            );
            Ok(true)
        }
    }
}

fn effective_principals(
    fleet_principals: &[Principal],
    rules: &[FleetAdmissionRule],
    target: &FleetAdmissionTarget,
) -> Vec<Principal> {
    let mut effective = fleet_principals.to_vec();
    for rule in rules {
        if selector_matches(&rule.selector, target) {
            effective.retain(|principal| rule.principals.binary_search(principal).is_ok());
        }
    }
    effective
}

fn selector_matches(selector: &FleetAdmissionSelector, target: &FleetAdmissionTarget) -> bool {
    match selector {
        FleetAdmissionSelector::Fleet => true,
        FleetAdmissionSelector::ComponentSpec(component_spec) => {
            component_spec == &target.component_spec
        }
        FleetAdmissionSelector::ComponentInstance(component_instance) => target
            .component_instance
            .as_ref()
            .is_some_and(|target| target == component_instance),
        FleetAdmissionSelector::FleetSubnetRoot(fleet_subnet_root) => {
            fleet_subnet_root == &target.fleet_subnet_root
        }
    }
}

#[cfg(test)]
mod tests;
