use super::*;
use std::collections::BTreeSet;

/// Build a dry-run authority reconciliation plan from the current deployment
/// truth check. The plan is observational; it does not mutate controller state.
#[must_use]
pub fn build_authority_reconciliation_plan(
    check: &DeploymentCheckV1,
) -> AuthorityReconciliationPlanV1 {
    let mut canister_actions = Vec::new();
    let mut hard_failures = Vec::new();
    let mut external_actions_required = Vec::new();

    for expected in &check.plan.expected_canisters {
        let action = reconcile_expected_canister(&check.plan, &check.inventory, expected);
        record_authority_outcome(
            action,
            &mut canister_actions,
            &mut hard_failures,
            &mut external_actions_required,
        );
    }

    for observed in &check.inventory.observed_canisters {
        if observed_expected_by_plan(&check.plan, observed) {
            continue;
        }
        let action = observed_unplanned_canister_action(observed);
        record_authority_outcome(
            action,
            &mut canister_actions,
            &mut hard_failures,
            &mut external_actions_required,
        );
    }

    AuthorityReconciliationPlanV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        plan_id: check.plan.plan_id.clone(),
        inventory_id: check.inventory.inventory_id.clone(),
        authority_profile_hash: check
            .plan
            .deployment_identity
            .authority_profile_hash
            .clone(),
        canister_actions,
        hard_failures,
        external_actions_required,
    }
}

fn reconcile_expected_canister(
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
            action: AuthorityActionV1::UnknownObservation,
            state: AuthorityReconciliationStateV1::Unknown,
            can_apply: false,
            reason: "controller set was not observed".to_string(),
        };
    }

    let missing = difference(&desired_controllers, &observed_controllers);
    let extra = difference(&observed_controllers, &desired_controllers);
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

fn observed_unplanned_canister_action(observed: &ObservedCanisterV1) -> CanisterAuthorityActionV1 {
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
        action,
        state,
        can_apply: false,
        reason: reason.to_string(),
    }
}

fn record_authority_outcome(
    action: CanisterAuthorityActionV1,
    canister_actions: &mut Vec<CanisterAuthorityActionV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    external_actions_required: &mut Vec<AuthorityExternalActionV1>,
) {
    if matches!(
        action.state,
        AuthorityReconciliationStateV1::RequiresExternalAction
            | AuthorityReconciliationStateV1::UnsafeBlocked
            | AuthorityReconciliationStateV1::Unknown
    ) {
        external_actions_required.push(AuthorityExternalActionV1 {
            canister_id: action.canister_id.clone(),
            role: action.role.clone(),
            action: action.action,
            reason: action.reason.clone(),
        });
    }
    if action.state == AuthorityReconciliationStateV1::UnsafeBlocked {
        hard_failures.push(SafetyFindingV1 {
            code: "authority_unsafe_blocked".to_string(),
            message: action.reason.clone(),
            severity: SafetySeverityV1::HardFailure,
            subject: action_subject(&action),
        });
    }
    canister_actions.push(action);
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

fn observed_expected_by_plan(plan: &DeploymentPlanV1, observed: &ObservedCanisterV1) -> bool {
    plan.expected_canisters.iter().any(|expected| {
        expected
            .canister_id
            .as_deref()
            .is_some_and(|id| id == observed.canister_id)
            || observed.role.as_deref() == Some(expected.role.as_str())
    })
}

fn difference(left: &[String], right: &[String]) -> Vec<String> {
    left.iter()
        .filter(|value| !right.iter().any(|candidate| candidate == *value))
        .cloned()
        .collect()
}

fn sorted_unique(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn controller_delta_reason(missing: &[String], extra: &[String]) -> String {
    match (missing.is_empty(), extra.is_empty()) {
        (false, true) => format!("missing desired controllers: {}", missing.join(",")),
        (true, false) => format!("extra observed controllers: {}", extra.join(",")),
        (false, false) => format!(
            "controller set differs: missing {}; extra {}",
            missing.join(","),
            extra.join(",")
        ),
        (true, true) => "observed controller set already matches desired authority".to_string(),
    }
}

fn action_subject(action: &CanisterAuthorityActionV1) -> Option<String> {
    action
        .canister_id
        .clone()
        .or_else(|| action.role.as_ref().map(|role| format!("role:{role}")))
}
