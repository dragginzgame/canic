use super::super::*;
use super::shared::{
    AUTHORITY_UNSAFE_BLOCKED_CODE, action_subject, controller_delta, controller_delta_reason,
    difference, sorted_unique,
};

pub(super) fn reconcile_expected_pool_canister(
    inventory: &DeploymentInventoryV1,
    expected: &ExpectedPoolCanisterV1,
) -> CanisterAuthorityActionV1 {
    let observed = find_observed_pool_for_expected(inventory, expected);
    let Some(observed) = observed else {
        return CanisterAuthorityActionV1 {
            canister_id: expected.canister_id.clone(),
            role: expected.role.clone(),
            control_classification: CanisterControlClassV1::CanicManagedPool,
            observed_controllers: Vec::new(),
            desired_controllers: Vec::new(),
            controller_delta: AuthorityControllerDeltaV1::default(),
            action: AuthorityActionV1::UnknownObservation,
            state: AuthorityReconciliationStateV1::Unknown,
            can_apply: false,
            reason: format!(
                "expected pool canister {} authority was not observed",
                expected.pool
            ),
        };
    };

    match observed.control_class {
        CanisterControlClassV1::CanicManagedPool => CanisterAuthorityActionV1 {
            canister_id: Some(observed.canister_id.clone()),
            role: observed.role.clone().or_else(|| expected.role.clone()),
            control_classification: observed.control_class,
            observed_controllers: Vec::new(),
            desired_controllers: Vec::new(),
            controller_delta: AuthorityControllerDeltaV1::default(),
            action: AuthorityActionV1::UnknownObservation,
            state: AuthorityReconciliationStateV1::Unknown,
            can_apply: false,
            reason: "pool canister controller set was not observed".to_string(),
        },
        CanisterControlClassV1::UnknownUnsafe => CanisterAuthorityActionV1 {
            canister_id: Some(observed.canister_id.clone()),
            role: observed.role.clone().or_else(|| expected.role.clone()),
            control_classification: observed.control_class,
            observed_controllers: Vec::new(),
            desired_controllers: Vec::new(),
            controller_delta: AuthorityControllerDeltaV1::default(),
            action: AuthorityActionV1::BlockedByPolicy,
            state: AuthorityReconciliationStateV1::UnsafeBlocked,
            can_apply: false,
            reason: "pool canister control class is unsafe or unknown".to_string(),
        },
        _ => CanisterAuthorityActionV1 {
            canister_id: Some(observed.canister_id.clone()),
            role: observed.role.clone().or_else(|| expected.role.clone()),
            control_classification: observed.control_class,
            observed_controllers: Vec::new(),
            desired_controllers: Vec::new(),
            controller_delta: AuthorityControllerDeltaV1::default(),
            action: AuthorityActionV1::RequiresExternalController,
            state: AuthorityReconciliationStateV1::RequiresExternalAction,
            can_apply: false,
            reason: "pool canister is not exclusively Canic-managed; external authority action is required".to_string(),
        },
    }
}
pub(super) fn reconcile_expected_canister(
    plan: &DeploymentPlanV1,
    inventory: &DeploymentInventoryV1,
    expected: &ExpectedCanisterV1,
) -> CanisterAuthorityActionV1 {
    let desired_controllers = sorted_unique(plan.authority_profile.expected_controllers.clone());
    let observed = find_observed_for_expected(inventory, expected);
    let Some(observed) = observed else {
        return CanisterAuthorityActionV1 {
            canister_id: expected.canister_id.clone(),
            role: Some(expected.role.clone()),
            control_classification: expected.control_class,
            observed_controllers: Vec::new(),
            desired_controllers,
            controller_delta: AuthorityControllerDeltaV1::default(),
            action: AuthorityActionV1::UnknownObservation,
            state: AuthorityReconciliationStateV1::Unknown,
            can_apply: false,
            reason: "expected canister authority was not observed".to_string(),
        };
    };

    let observed_controllers = sorted_unique(observed.controllers.clone());
    if observed_controllers.is_empty() {
        return CanisterAuthorityActionV1 {
            canister_id: Some(observed.canister_id.clone()),
            role: observed
                .role
                .clone()
                .or_else(|| Some(expected.role.clone())),
            control_classification: observed.control_class,
            observed_controllers,
            desired_controllers,
            controller_delta: AuthorityControllerDeltaV1::default(),
            action: AuthorityActionV1::UnknownObservation,
            state: AuthorityReconciliationStateV1::Unknown,
            can_apply: false,
            reason: "controller set was not observed".to_string(),
        };
    }

    let missing = difference(&desired_controllers, &observed_controllers);
    let extra = difference(&observed_controllers, &desired_controllers);
    let controller_delta = controller_delta(&missing, &extra);
    let (action, state, can_apply, reason) =
        classify_controller_reconciliation(observed.control_class, &missing, &extra);

    CanisterAuthorityActionV1 {
        canister_id: Some(observed.canister_id.clone()),
        role: observed
            .role
            .clone()
            .or_else(|| Some(expected.role.clone())),
        control_classification: observed.control_class,
        observed_controllers,
        desired_controllers,
        controller_delta,
        action,
        state,
        can_apply,
        reason,
    }
}

fn classify_controller_reconciliation(
    control_class: CanisterControlClassV1,
    missing: &[String],
    extra: &[String],
) -> (
    AuthorityActionV1,
    AuthorityReconciliationStateV1,
    bool,
    String,
) {
    if missing.is_empty() && extra.is_empty() {
        return (
            AuthorityActionV1::None,
            AuthorityReconciliationStateV1::AlreadyCorrect,
            false,
            "observed controller set already matches desired authority".to_string(),
        );
    }

    match control_class {
        CanisterControlClassV1::DeploymentControlled => {
            let action = if !missing.is_empty() && extra.is_empty() {
                AuthorityActionV1::AddControllers
            } else if missing.is_empty() {
                AuthorityActionV1::RemoveControllers
            } else {
                AuthorityActionV1::ReplaceControllerSet
            };
            (
                action,
                AuthorityReconciliationStateV1::CanApplyAutomatically,
                true,
                controller_delta_reason(missing, extra),
            )
        }
        CanisterControlClassV1::CanicManagedPool => (
            AuthorityActionV1::RequiresExternalController,
            AuthorityReconciliationStateV1::RequiresExternalAction,
            false,
            "pool canister authority reconciliation is deferred to pool ownership planning"
                .to_string(),
        ),
        CanisterControlClassV1::ExternallyImported
        | CanisterControlClassV1::JointlyControlled
        | CanisterControlClassV1::UserControlled => (
            AuthorityActionV1::RequiresExternalController,
            AuthorityReconciliationStateV1::RequiresExternalAction,
            false,
            "canister is not exclusively deployment-controlled; external authority action is required"
                .to_string(),
        ),
        CanisterControlClassV1::UnknownUnsafe => (
            AuthorityActionV1::BlockedByPolicy,
            AuthorityReconciliationStateV1::UnsafeBlocked,
            false,
            "canister control class is unsafe or unknown".to_string(),
        ),
    }
}

pub(super) fn observed_unplanned_canister_action(
    observed: &ObservedCanisterV1,
) -> CanisterAuthorityActionV1 {
    let observed_controllers = sorted_unique(observed.controllers.clone());
    let (action, state, reason) = match observed.control_class {
        CanisterControlClassV1::DeploymentControlled => (
            AuthorityActionV1::ObserveOnly,
            AuthorityReconciliationStateV1::RequiresExternalAction,
            "observed deployment-controlled canister is not present in the plan",
        ),
        CanisterControlClassV1::CanicManagedPool => (
            AuthorityActionV1::AdoptPlanAvailable,
            AuthorityReconciliationStateV1::RequiresExternalAction,
            "observed pool canister needs explicit pool ownership reconciliation",
        ),
        CanisterControlClassV1::ExternallyImported
        | CanisterControlClassV1::JointlyControlled
        | CanisterControlClassV1::UserControlled => (
            AuthorityActionV1::ObserveOnly,
            AuthorityReconciliationStateV1::RequiresExternalAction,
            "observed canister is outside the current deployment plan",
        ),
        CanisterControlClassV1::UnknownUnsafe => (
            AuthorityActionV1::BlockedByPolicy,
            AuthorityReconciliationStateV1::UnsafeBlocked,
            "observed canister has unsafe or unknown control class",
        ),
    };

    CanisterAuthorityActionV1 {
        canister_id: Some(observed.canister_id.clone()),
        role: observed.role.clone(),
        control_classification: observed.control_class,
        observed_controllers,
        desired_controllers: Vec::new(),
        controller_delta: AuthorityControllerDeltaV1::default(),
        action,
        state,
        can_apply: false,
        reason: reason.to_string(),
    }
}

pub(super) fn observed_unplanned_pool_action(
    observed: &ObservedPoolCanisterV1,
) -> CanisterAuthorityActionV1 {
    let (action, state, reason) = match observed.control_class {
        CanisterControlClassV1::CanicManagedPool => (
            AuthorityActionV1::AdoptPlanAvailable,
            AuthorityReconciliationStateV1::RequiresExternalAction,
            "observed pool canister is not present in the expected pool plan",
        ),
        CanisterControlClassV1::UnknownUnsafe => (
            AuthorityActionV1::BlockedByPolicy,
            AuthorityReconciliationStateV1::UnsafeBlocked,
            "observed pool canister has unsafe or unknown control class",
        ),
        _ => (
            AuthorityActionV1::ObserveOnly,
            AuthorityReconciliationStateV1::RequiresExternalAction,
            "observed pool canister is outside the current deployment pool plan",
        ),
    };

    CanisterAuthorityActionV1 {
        canister_id: Some(observed.canister_id.clone()),
        role: observed.role.clone(),
        control_classification: observed.control_class,
        observed_controllers: Vec::new(),
        desired_controllers: Vec::new(),
        controller_delta: AuthorityControllerDeltaV1::default(),
        action,
        state,
        can_apply: false,
        reason: reason.to_string(),
    }
}

pub(super) fn record_authority_outcome(
    action: CanisterAuthorityActionV1,
    canister_actions: &mut Vec<CanisterAuthorityActionV1>,
    automatic_actions: &mut Vec<AuthorityAutomaticActionV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    external_actions_required: &mut Vec<AuthorityExternalActionV1>,
) {
    if action.state == AuthorityReconciliationStateV1::CanApplyAutomatically
        && let Some(canister_id) = action.canister_id.clone()
    {
        automatic_actions.push(AuthorityAutomaticActionV1 {
            subject: action_subject(&action).unwrap_or_else(|| canister_id.clone()),
            canister_id,
            role: action.role.clone(),
            action: action.action,
            observed_controllers: action.observed_controllers.clone(),
            desired_controllers: action.desired_controllers.clone(),
            controller_delta: action.controller_delta.clone(),
            reason: action.reason.clone(),
        });
    }
    if action.state == AuthorityReconciliationStateV1::RequiresExternalAction {
        external_actions_required.push(AuthorityExternalActionV1 {
            subject: action_subject(&action).unwrap_or_else(|| "unknown".to_string()),
            canister_id: action.canister_id.clone(),
            role: action.role.clone(),
            control_classification: action.control_classification,
            state: action.state,
            action: action.action,
            observed_controllers: action.observed_controllers.clone(),
            desired_controllers: action.desired_controllers.clone(),
            controller_delta: action.controller_delta.clone(),
            reason: action.reason.clone(),
        });
    }
    if action.state == AuthorityReconciliationStateV1::UnsafeBlocked {
        hard_failures.push(SafetyFindingV1 {
            code: AUTHORITY_UNSAFE_BLOCKED_CODE.to_string(),
            message: action.reason.clone(),
            severity: SafetySeverityV1::HardFailure,
            subject: action_subject(&action),
        });
    }
    canister_actions.push(action);
}

fn find_observed_pool_for_expected<'a>(
    inventory: &'a DeploymentInventoryV1,
    expected: &ExpectedPoolCanisterV1,
) -> Option<&'a ObservedPoolCanisterV1> {
    if let Some(canister_id) = &expected.canister_id {
        return inventory
            .observed_pool
            .iter()
            .find(|observed| &observed.canister_id == canister_id);
    }

    let mut matches = inventory.observed_pool.iter().filter(|observed| {
        observed.pool == expected.pool
            && expected
                .role
                .as_deref()
                .is_none_or(|role| observed.role.as_deref() == Some(role))
    });
    let first = matches.next()?;
    matches.next().is_none().then_some(first)
}

fn find_observed_for_expected<'a>(
    inventory: &'a DeploymentInventoryV1,
    expected: &ExpectedCanisterV1,
) -> Option<&'a ObservedCanisterV1> {
    if let Some(canister_id) = &expected.canister_id {
        return inventory
            .observed_canisters
            .iter()
            .find(|observed| &observed.canister_id == canister_id);
    }

    let mut matches = inventory
        .observed_canisters
        .iter()
        .filter(|observed| observed.role.as_deref() == Some(expected.role.as_str()));
    let first = matches.next()?;
    matches.next().is_none().then_some(first)
}

pub(super) fn observed_pool_expected_by_plan(
    plan: &DeploymentPlanV1,
    observed: &ObservedPoolCanisterV1,
) -> bool {
    plan.expected_pool.iter().any(|expected| {
        expected
            .canister_id
            .as_deref()
            .is_some_and(|id| id == observed.canister_id)
            || (expected.pool == observed.pool
                && expected
                    .role
                    .as_deref()
                    .is_none_or(|role| observed.role.as_deref() == Some(role)))
    })
}

pub(super) fn observed_expected_by_plan(
    plan: &DeploymentPlanV1,
    observed: &ObservedCanisterV1,
) -> bool {
    plan.expected_canisters.iter().any(|expected| {
        expected
            .canister_id
            .as_deref()
            .is_some_and(|id| id == observed.canister_id)
            || observed.role.as_deref() == Some(expected.role.as_str())
    })
}
