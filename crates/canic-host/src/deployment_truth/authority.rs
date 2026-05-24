use super::*;
use std::collections::BTreeSet;

/// Build a dry-run authority reconciliation plan from the current deployment
/// truth check. The plan is observational; it does not mutate controller state.
#[must_use]
pub fn build_authority_reconciliation_plan(
    check: &DeploymentCheckV1,
) -> AuthorityReconciliationPlanV1 {
    let mut canister_actions = Vec::new();
    let mut automatic_actions = Vec::new();
    let mut hard_failures = authority_profile_overlap_findings(&check.plan);
    let mut external_actions_required = Vec::new();

    for expected in &check.plan.expected_canisters {
        let action = reconcile_expected_canister(&check.plan, &check.inventory, expected);
        record_authority_outcome(
            action,
            &mut canister_actions,
            &mut automatic_actions,
            &mut hard_failures,
            &mut external_actions_required,
        );
    }

    for expected in &check.plan.expected_pool {
        let action = reconcile_expected_pool_canister(&check.inventory, expected);
        record_authority_outcome(
            action,
            &mut canister_actions,
            &mut automatic_actions,
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
            &mut automatic_actions,
            &mut hard_failures,
            &mut external_actions_required,
        );
    }

    for observed in &check.inventory.observed_pool {
        if observed_pool_expected_by_plan(&check.plan, observed) {
            continue;
        }
        let action = observed_unplanned_pool_action(observed);
        record_authority_outcome(
            action,
            &mut canister_actions,
            &mut automatic_actions,
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
        automatic_actions,
        hard_failures,
        external_actions_required,
    }
}

fn reconcile_expected_pool_canister(
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

/// Render the operator-facing authority report for one reconciliation plan.
#[must_use]
pub fn authority_report_from_plan(
    report_id: impl Into<String>,
    plan: &AuthorityReconciliationPlanV1,
) -> AuthorityReportV1 {
    authority_report_from_plan_with_check_id(report_id, None, plan)
}

/// Build the dry-run authority reconciliation plan for one deployment truth
/// check and render the operator-facing report with source check provenance.
#[must_use]
pub fn authority_report_from_check(
    report_id: impl Into<String>,
    check: &DeploymentCheckV1,
) -> AuthorityReportV1 {
    let plan = build_authority_reconciliation_plan(check);
    authority_report_from_plan_with_check_id(report_id, Some(check.check_id.clone()), &plan)
}

/// Build the operator-facing authority report using the standard local
/// deployment-truth artifact identifier.
#[must_use]
pub fn authority_report_from_check_with_local_id(check: &DeploymentCheckV1) -> AuthorityReportV1 {
    authority_report_from_check(
        local_authority_artifact_id(check, "authority-report"),
        check,
    )
}

/// Render the operator-facing authority report and attach the source deployment
/// check identifier when the caller is building the report from a full check.
#[must_use]
pub fn authority_report_from_plan_with_check_id(
    report_id: impl Into<String>,
    check_id: Option<String>,
    plan: &AuthorityReconciliationPlanV1,
) -> AuthorityReportV1 {
    let counts = authority_report_counts(plan);
    let status = authority_report_status(&counts);
    let next_actions = authority_report_next_actions(status, &counts);
    let apply_readiness = authority_apply_readiness(&counts);
    AuthorityReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: report_id.into(),
        check_id,
        reconciliation_plan_id: plan.plan_id.clone(),
        inventory_id: plan.inventory_id.clone(),
        authority_profile_hash: plan.authority_profile_hash.clone(),
        status,
        summary: authority_report_summary(status, &counts),
        counts,
        apply_readiness,
        action_counts: authority_report_action_counts(plan),
        control_class_counts: authority_report_control_class_counts(plan),
        observation_gaps: authority_report_observation_gaps(plan),
        automatic_actions: plan.automatic_actions.clone(),
        hard_failures: plan.hard_failures.clone(),
        external_actions_required: plan.external_actions_required.clone(),
        next_actions,
    }
}

fn authority_report_counts(plan: &AuthorityReconciliationPlanV1) -> AuthorityReportCountsV1 {
    let mut counts = AuthorityReportCountsV1 {
        already_correct: 0,
        can_apply_automatically: 0,
        requires_external_action: 0,
        unsafe_blocked: 0,
        unknown: 0,
        hard_failures: plan.hard_failures.len(),
    };
    for action in &plan.canister_actions {
        match action.state {
            AuthorityReconciliationStateV1::AlreadyCorrect => counts.already_correct += 1,
            AuthorityReconciliationStateV1::CanApplyAutomatically => {
                counts.can_apply_automatically += 1;
            }
            AuthorityReconciliationStateV1::RequiresExternalAction => {
                counts.requires_external_action += 1;
            }
            AuthorityReconciliationStateV1::UnsafeBlocked => counts.unsafe_blocked += 1,
            AuthorityReconciliationStateV1::Unknown => counts.unknown += 1,
        }
    }
    counts
}

fn authority_apply_readiness(counts: &AuthorityReportCountsV1) -> AuthorityApplyReadinessV1 {
    let mut blockers = Vec::new();
    if counts.hard_failures > 0 {
        blockers.push(AuthorityApplyBlockerV1::HardFailures);
    }
    if counts.unknown > 0 {
        blockers.push(AuthorityApplyBlockerV1::ObservationGaps);
    }
    if counts.requires_external_action > 0 {
        blockers.push(AuthorityApplyBlockerV1::ExternalActions);
    }
    let can_apply_automatically = counts.can_apply_automatically > 0 && blockers.is_empty();
    AuthorityApplyReadinessV1 {
        can_apply_automatically,
        automatic_action_count: counts.can_apply_automatically,
        blockers,
    }
}

fn authority_profile_overlap_findings(plan: &DeploymentPlanV1) -> Vec<SafetyFindingV1> {
    let expected = sorted_unique(plan.authority_profile.expected_controllers.clone());
    let staging = authority_category_overlaps(
        "staging",
        &expected,
        &plan.authority_profile.staging_controllers,
    );
    let emergency = authority_category_overlaps(
        "emergency",
        &expected,
        &plan.authority_profile.emergency_controllers,
    );

    staging.into_iter().chain(emergency).collect()
}

fn authority_category_overlaps(
    category: &str,
    expected_controllers: &[String],
    category_controllers: &[String],
) -> Vec<SafetyFindingV1> {
    let overlaps = sorted_unique(
        category_controllers
            .iter()
            .filter(|controller| {
                expected_controllers
                    .iter()
                    .any(|expected| expected == *controller)
            })
            .cloned()
            .collect(),
    );

    overlaps
        .into_iter()
        .map(|principal| SafetyFindingV1 {
            code: "authority_profile_overlap".to_string(),
            message: format!(
                "{category} authority principal {principal} overlaps the normal expected controller set"
            ),
            severity: SafetySeverityV1::HardFailure,
            subject: Some(principal),
        })
        .collect()
}

const fn authority_report_status(counts: &AuthorityReportCountsV1) -> SafetyStatusV1 {
    if counts.unsafe_blocked > 0 || counts.hard_failures > 0 {
        SafetyStatusV1::Blocked
    } else if counts.requires_external_action > 0 || counts.unknown > 0 {
        SafetyStatusV1::Warning
    } else {
        SafetyStatusV1::Safe
    }
}

fn authority_report_summary(status: SafetyStatusV1, counts: &AuthorityReportCountsV1) -> String {
    match status {
        SafetyStatusV1::Blocked => format!(
            "authority reconciliation is blocked by {} unsafe canister(s) and {} hard authority finding(s)",
            counts.unsafe_blocked, counts.hard_failures
        ),
        SafetyStatusV1::Warning => format!(
            "authority reconciliation requires {} external action(s) and has {} unknown observation(s)",
            counts.requires_external_action, counts.unknown
        ),
        SafetyStatusV1::Safe => format!(
            "authority reconciliation is safe: {} canister(s) already correct, {} automatic dry-run action(s)",
            counts.already_correct, counts.can_apply_automatically
        ),
        SafetyStatusV1::NotEvaluated => {
            "authority reconciliation has not been evaluated".to_string()
        }
    }
}

fn authority_report_action_counts(
    plan: &AuthorityReconciliationPlanV1,
) -> Vec<AuthorityActionCountV1> {
    let mut none = 0;
    let mut add_controllers = 0;
    let mut remove_controllers = 0;
    let mut replace_controller_set = 0;
    let mut requires_external_controller = 0;
    let mut requires_destructive_import_confirmation = 0;
    let mut observe_only = 0;
    let mut adopt_plan_available = 0;
    let mut blocked_by_policy = 0;
    let mut unknown_observation = 0;

    for action in &plan.canister_actions {
        match action.action {
            AuthorityActionV1::None => none += 1,
            AuthorityActionV1::AddControllers => add_controllers += 1,
            AuthorityActionV1::RemoveControllers => remove_controllers += 1,
            AuthorityActionV1::ReplaceControllerSet => replace_controller_set += 1,
            AuthorityActionV1::RequiresExternalController => requires_external_controller += 1,
            AuthorityActionV1::RequiresDestructiveImportConfirmation => {
                requires_destructive_import_confirmation += 1;
            }
            AuthorityActionV1::ObserveOnly => observe_only += 1,
            AuthorityActionV1::AdoptPlanAvailable => adopt_plan_available += 1,
            AuthorityActionV1::BlockedByPolicy => blocked_by_policy += 1,
            AuthorityActionV1::UnknownObservation => unknown_observation += 1,
        }
    }

    [
        (AuthorityActionV1::None, none),
        (AuthorityActionV1::AddControllers, add_controllers),
        (AuthorityActionV1::RemoveControllers, remove_controllers),
        (
            AuthorityActionV1::ReplaceControllerSet,
            replace_controller_set,
        ),
        (
            AuthorityActionV1::RequiresExternalController,
            requires_external_controller,
        ),
        (
            AuthorityActionV1::RequiresDestructiveImportConfirmation,
            requires_destructive_import_confirmation,
        ),
        (AuthorityActionV1::ObserveOnly, observe_only),
        (AuthorityActionV1::AdoptPlanAvailable, adopt_plan_available),
        (AuthorityActionV1::BlockedByPolicy, blocked_by_policy),
        (AuthorityActionV1::UnknownObservation, unknown_observation),
    ]
    .into_iter()
    .filter(|(_, count)| *count > 0)
    .map(|(action, count)| AuthorityActionCountV1 { action, count })
    .collect()
}

fn authority_report_control_class_counts(
    plan: &AuthorityReconciliationPlanV1,
) -> Vec<AuthorityControlClassCountV1> {
    let mut deployment_controlled = 0;
    let mut canic_managed_pool = 0;
    let mut externally_imported = 0;
    let mut jointly_controlled = 0;
    let mut user_controlled = 0;
    let mut unknown_unsafe = 0;

    for action in &plan.canister_actions {
        match action.control_classification {
            CanisterControlClassV1::DeploymentControlled => deployment_controlled += 1,
            CanisterControlClassV1::CanicManagedPool => canic_managed_pool += 1,
            CanisterControlClassV1::ExternallyImported => externally_imported += 1,
            CanisterControlClassV1::JointlyControlled => jointly_controlled += 1,
            CanisterControlClassV1::UserControlled => user_controlled += 1,
            CanisterControlClassV1::UnknownUnsafe => unknown_unsafe += 1,
        }
    }

    [
        (
            CanisterControlClassV1::DeploymentControlled,
            deployment_controlled,
        ),
        (CanisterControlClassV1::CanicManagedPool, canic_managed_pool),
        (
            CanisterControlClassV1::ExternallyImported,
            externally_imported,
        ),
        (
            CanisterControlClassV1::JointlyControlled,
            jointly_controlled,
        ),
        (CanisterControlClassV1::UserControlled, user_controlled),
        (CanisterControlClassV1::UnknownUnsafe, unknown_unsafe),
    ]
    .into_iter()
    .filter(|(_, count)| *count > 0)
    .map(|(control_class, count)| AuthorityControlClassCountV1 {
        control_class,
        count,
    })
    .collect()
}

fn authority_report_observation_gaps(
    plan: &AuthorityReconciliationPlanV1,
) -> Vec<DeploymentObservationGapV1> {
    plan.canister_actions
        .iter()
        .filter(|action| action.state == AuthorityReconciliationStateV1::Unknown)
        .map(|action| {
            let subject = action_subject(action).unwrap_or_else(|| "unknown".to_string());
            DeploymentObservationGapV1 {
                key: format!("authority.controllers.{subject}"),
                description: action.reason.clone(),
            }
        })
        .collect()
}

fn authority_report_next_actions(
    status: SafetyStatusV1,
    counts: &AuthorityReportCountsV1,
) -> Vec<String> {
    match status {
        SafetyStatusV1::Blocked => {
            let mut actions = Vec::new();
            if counts.unsafe_blocked > 0 {
                actions.push(
                    "resolve unsafe canister authority findings before applying controller changes"
                        .to_string(),
                );
            }
            if counts.hard_failures > 0 {
                actions.push(
                    "resolve hard authority findings before applying controller changes"
                        .to_string(),
                );
            }
            actions
        }
        SafetyStatusV1::Warning => {
            let mut actions = Vec::new();
            if counts.requires_external_action > 0 {
                actions.push(
                    "review external authority actions before applying controller changes"
                        .to_string(),
                );
            }
            if counts.unknown > 0 {
                actions.push(
                    "collect missing controller observations before applying controller changes"
                        .to_string(),
                );
            }
            actions
        }
        SafetyStatusV1::Safe => {
            if counts.can_apply_automatically > 0 {
                vec![
                    "review automatic authority dry-run actions before enabling an apply path"
                        .to_string(),
                ]
            } else {
                Vec::new()
            }
        }
        SafetyStatusV1::NotEvaluated => {
            vec!["collect deployment inventory and rerun authority reconciliation".to_string()]
        }
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
        controller_delta: AuthorityControllerDeltaV1::default(),
        action,
        state,
        can_apply: false,
        reason: reason.to_string(),
    }
}

fn observed_unplanned_pool_action(observed: &ObservedPoolCanisterV1) -> CanisterAuthorityActionV1 {
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

fn record_authority_outcome(
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
            code: "authority_unsafe_blocked".to_string(),
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

fn observed_pool_expected_by_plan(
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

fn controller_delta(missing: &[String], extra: &[String]) -> AuthorityControllerDeltaV1 {
    AuthorityControllerDeltaV1 {
        add_controllers: missing.to_vec(),
        remove_controllers: extra.to_vec(),
    }
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
