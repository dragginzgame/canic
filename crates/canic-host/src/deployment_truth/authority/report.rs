use super::super::*;
use super::{
    plan::build_authority_reconciliation_plan,
    shared::{AUTHORITY_UNSAFE_BLOCKED_CODE, action_subject},
};

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
        hard_failures: hard_authority_failure_count(&plan.hard_failures),
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
    if counts.unsafe_blocked > 0 {
        blockers.push(AuthorityApplyBlockerV1::UnsafeBlocked);
    }
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

fn hard_authority_failure_count(failures: &[SafetyFindingV1]) -> usize {
    failures
        .iter()
        .filter(|failure| failure.code != AUTHORITY_UNSAFE_BLOCKED_CODE)
        .count()
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
        SafetyStatusV1::Blocked => authority_blocked_summary(counts),
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

fn authority_blocked_summary(counts: &AuthorityReportCountsV1) -> String {
    let mut summary = format!(
        "authority reconciliation is blocked by {} unsafe canister(s) and {} hard authority finding(s)",
        counts.unsafe_blocked, counts.hard_failures
    );
    if counts.requires_external_action > 0 || counts.unknown > 0 {
        std::fmt::Write::write_fmt(
            &mut summary,
            format_args!(
                "; also requires {} external action(s) and has {} unknown observation(s)",
                counts.requires_external_action, counts.unknown
            ),
        )
        .expect("writing to a String cannot fail");
    }
    summary
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
        SafetyStatusV1::Blocked | SafetyStatusV1::Warning => {
            authority_report_blocker_next_actions(counts)
        }
        SafetyStatusV1::Safe => authority_automatic_next_actions(counts),
        SafetyStatusV1::NotEvaluated => {
            vec!["collect deployment inventory and rerun authority reconciliation".to_string()]
        }
    }
}

fn authority_report_blocker_next_actions(counts: &AuthorityReportCountsV1) -> Vec<String> {
    let mut actions = Vec::new();
    if counts.unsafe_blocked > 0 {
        actions.push(
            "resolve unsafe canister authority findings before applying controller changes"
                .to_string(),
        );
    }
    if counts.hard_failures > 0 {
        actions
            .push("resolve hard authority findings before applying controller changes".to_string());
    }
    if counts.requires_external_action > 0 {
        actions.push(
            "review external authority actions before applying controller changes".to_string(),
        );
    }
    if counts.unknown > 0 {
        actions.push(
            "collect missing controller observations before applying controller changes"
                .to_string(),
        );
    }
    actions.extend(authority_automatic_next_actions(counts));
    actions
}

fn authority_automatic_next_actions(counts: &AuthorityReportCountsV1) -> Vec<String> {
    if counts.can_apply_automatically > 0 {
        vec!["review automatic authority dry-run actions before enabling an apply path".to_string()]
    } else {
        Vec::new()
    }
}
