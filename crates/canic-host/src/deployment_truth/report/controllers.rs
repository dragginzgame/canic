use super::super::*;
use super::{diff_item, finding};
use std::collections::BTreeSet;

pub(super) fn compare_authority_profile(
    plan: &DeploymentPlanV1,
    controller_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    let mut reported = BTreeSet::new();
    for controller in &plan.authority_profile.expected_controllers {
        if !is_staging_or_emergency_controller(plan, controller) {
            continue;
        }
        if !reported.insert(controller.as_str()) {
            continue;
        }
        controller_diff.push(diff_item(
            "controller_authority_overlap",
            "authority_profile",
            Some("expected-only".to_string()),
            Some(controller.clone()),
            SafetySeverityV1::HardFailure,
        ));
        hard_failures.push(finding(
            "controller_authority_overlap",
            format!(
                "controller {controller} appears in both expected and staging/emergency authority"
            ),
            SafetySeverityV1::HardFailure,
            Some("authority_profile".to_string()),
        ));
    }
}

pub(super) fn compare_role_controllers(
    plan: &DeploymentPlanV1,
    observed: &ObservedCanisterV1,
    controller_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    let role = observed.role.as_deref().unwrap_or("unknown");
    if observed.controllers.is_empty() && !observed_source_includes_live_status(observed) {
        warnings.push(finding(
            "controllers_unobserved",
            format!("controllers were not observed for role {role}"),
            SafetySeverityV1::Warning,
            Some(role.to_string()),
        ));
        return;
    }
    for expected in &plan.authority_profile.expected_controllers {
        if observed
            .controllers
            .iter()
            .any(|controller| controller == expected)
        {
            continue;
        }
        record_missing_expected_controller(
            role,
            expected,
            &observed.controllers,
            controller_diff,
            hard_failures,
        );
    }

    for observed_controller in &observed.controllers {
        if is_declared_controller(plan, observed_controller) {
            continue;
        }
        record_extra_controller(role, observed_controller, plan, controller_diff, warnings);
    }
}

fn record_missing_expected_controller(
    role: &str,
    expected: &str,
    observed_controllers: &[String],
    controller_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    controller_diff.push(diff_item(
        "controller_missing",
        role,
        Some(expected.to_string()),
        Some(controller_set_label(observed_controllers)),
        SafetySeverityV1::HardFailure,
    ));
    hard_failures.push(finding(
        "expected_controller_missing",
        format!("role {role} is missing expected controller {expected}"),
        SafetySeverityV1::HardFailure,
        Some(role.to_string()),
    ));
}

fn record_extra_controller(
    role: &str,
    observed_controller: &str,
    plan: &DeploymentPlanV1,
    controller_diff: &mut Vec<DiffItemV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    controller_diff.push(diff_item(
        "controller_extra",
        role,
        Some(controller_set_label(
            &plan.authority_profile.expected_controllers,
        )),
        Some(observed_controller.to_string()),
        SafetySeverityV1::Warning,
    ));
    warnings.push(finding(
        "extra_controller_observed",
        format!("role {role} has controller outside the expected authority profile"),
        SafetySeverityV1::Warning,
        Some(role.to_string()),
    ));
}

fn observed_source_includes_live_status(observed: &ObservedCanisterV1) -> bool {
    observed
        .role_assignment_source
        .as_deref()
        .is_some_and(|source| source.contains("icp_canister_status"))
}

fn is_declared_controller(plan: &DeploymentPlanV1, controller: &str) -> bool {
    plan.authority_profile
        .expected_controllers
        .iter()
        .chain(plan.authority_profile.staging_controllers.iter())
        .chain(plan.authority_profile.emergency_controllers.iter())
        .any(|expected| expected == controller)
}

fn is_staging_or_emergency_controller(plan: &DeploymentPlanV1, controller: &str) -> bool {
    plan.authority_profile
        .staging_controllers
        .iter()
        .chain(plan.authority_profile.emergency_controllers.iter())
        .any(|declared| declared == controller)
}

fn controller_set_label(controllers: &[String]) -> String {
    if controllers.is_empty() {
        return "<none>".to_string();
    }
    controllers.join(",")
}
