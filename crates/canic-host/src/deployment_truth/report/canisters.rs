use super::super::*;
use super::controllers::compare_role_controllers;
use super::{conflicting_assignment_groups, diff_item, duplicate_evidence_groups, finding};
use std::collections::BTreeSet;

pub(super) fn compare_observed_canister_id_conflicts(
    inventory: &DeploymentInventoryV1,
    controller_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    for group in duplicate_evidence_groups(
        &inventory.observed_canisters,
        |observed| observed.canister_id.as_str().to_string(),
        observed_role_label,
        ",",
    ) {
        if group.is_conflict {
            controller_diff.push(diff_item(
                "canister_id_role_conflict",
                &group.subject,
                None,
                Some(group.evidence_label.clone()),
                SafetySeverityV1::HardFailure,
            ));
            hard_failures.push(finding(
                "canister_id_role_conflict",
                format!(
                    "observed canister {} has conflicting roles {}",
                    group.subject, group.evidence_label
                ),
                SafetySeverityV1::HardFailure,
                Some(group.subject),
            ));
        } else {
            controller_diff.push(diff_item(
                "canister_duplicate",
                &group.subject,
                Some(group.evidence_label.clone()),
                Some(group.count.to_string()),
                SafetySeverityV1::Warning,
            ));
            warnings.push(finding(
                "duplicate_canister_observed",
                format!(
                    "observed canister {} was reported {} times for role {}",
                    group.subject, group.count, group.evidence_label
                ),
                SafetySeverityV1::Warning,
                Some(group.subject),
            ));
        }
    }
}

fn observed_role_label(observed: &ObservedCanisterV1) -> String {
    observed
        .role
        .clone()
        .unwrap_or_else(|| "<unknown>".to_string())
}

pub(super) fn compare_canisters(
    plan: &DeploymentPlanV1,
    inventory: &DeploymentInventoryV1,
    controller_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    let planned_conflicts =
        compare_planned_canister_conflicts(plan, controller_diff, hard_failures, warnings);
    let mut matched_observed = BTreeSet::new();
    let mut compared_planned = BTreeSet::new();
    for expected in &plan.expected_canisters {
        if planned_conflicts.role_conflicts.contains(&expected.role)
            || expected
                .canister_id
                .as_ref()
                .is_some_and(|id| planned_conflicts.id_conflicts.contains(id))
            || !compared_planned.insert(planned_canister_evidence_label(expected))
        {
            continue;
        }
        let observed = expected.canister_id.as_ref().map_or_else(
            || {
                let role_matches = inventory
                    .observed_canisters
                    .iter()
                    .filter(|canister| canister.role.as_deref() == Some(expected.role.as_str()))
                    .collect::<Vec<_>>();
                if role_matches.len() > 1 {
                    record_ambiguous_canister_role(
                        expected,
                        &role_matches,
                        controller_diff,
                        hard_failures,
                    );
                    None
                } else {
                    role_matches.into_iter().next()
                }
            },
            |id| {
                inventory
                    .observed_canisters
                    .iter()
                    .find(|canister| &canister.canister_id == id)
            },
        );
        let Some(observed) = observed else {
            record_missing_canister(expected, controller_diff, hard_failures, warnings);
            continue;
        };
        matched_observed.insert(observed.canister_id.as_str());
        compare_observed_role(expected, observed, controller_diff, hard_failures);
        record_unsafe_canister_control_class(expected, observed, controller_diff, hard_failures);
        compare_role_controllers(plan, observed, controller_diff, hard_failures, warnings);
    }
    warn_extra_observed_canisters(
        plan,
        inventory,
        controller_diff,
        warnings,
        &matched_observed,
    );
}

struct PlannedCanisterConflicts {
    role_conflicts: BTreeSet<String>,
    id_conflicts: BTreeSet<String>,
}

fn compare_planned_canister_conflicts(
    plan: &DeploymentPlanV1,
    controller_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) -> PlannedCanisterConflicts {
    let mut role_conflicts = BTreeSet::new();
    let mut id_conflicts = BTreeSet::new();

    for group in duplicate_evidence_groups(
        &plan.expected_canisters,
        |planned| planned.role.as_str().to_string(),
        planned_canister_evidence_label,
        " | ",
    ) {
        if group.is_conflict {
            role_conflicts.insert(group.subject.clone());
            controller_diff.push(diff_item(
                "planned_canister_role_conflict",
                &group.subject,
                Some("one planned canister".to_string()),
                Some(group.evidence_label.clone()),
                SafetySeverityV1::HardFailure,
            ));
            hard_failures.push(finding(
                "planned_canister_role_conflict",
                format!(
                    "planned canister role {} has conflicting evidence: {}",
                    group.subject, group.evidence_label
                ),
                SafetySeverityV1::HardFailure,
                Some(group.subject),
            ));
        } else {
            controller_diff.push(diff_item(
                "planned_canister_duplicate",
                &group.subject,
                Some(group.evidence_label.clone()),
                Some(group.count.to_string()),
                SafetySeverityV1::Warning,
            ));
            warnings.push(finding(
                "duplicate_planned_canister_role",
                format!(
                    "planned canister role {} was declared {} times with identical evidence",
                    group.subject, group.count
                ),
                SafetySeverityV1::Warning,
                Some(group.subject),
            ));
        }
    }

    for group in conflicting_assignment_groups(
        &plan.expected_canisters,
        |planned| planned.canister_id.clone(),
        |planned| planned.role.clone(),
        ",",
    ) {
        id_conflicts.insert(group.subject.clone());
        controller_diff.push(diff_item(
            "planned_canister_id_conflict",
            &group.subject,
            Some("one planned role".to_string()),
            Some(group.evidence_label.clone()),
            SafetySeverityV1::HardFailure,
        ));
        hard_failures.push(finding(
            "planned_canister_id_conflict",
            format!(
                "planned canister id {} is assigned to conflicting roles {}",
                group.subject, group.evidence_label
            ),
            SafetySeverityV1::HardFailure,
            Some(group.subject),
        ));
    }

    PlannedCanisterConflicts {
        role_conflicts,
        id_conflicts,
    }
}

fn planned_canister_evidence_label(planned: &ExpectedCanisterV1) -> String {
    format!(
        "role={};id={};control={:?}",
        planned.role,
        planned.canister_id.as_deref().unwrap_or("<none>"),
        planned.control_class
    )
}

fn record_missing_canister(
    expected: &ExpectedCanisterV1,
    controller_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    let severity = if expected.canister_id.is_some() {
        SafetySeverityV1::HardFailure
    } else {
        SafetySeverityV1::Warning
    };
    controller_diff.push(diff_item(
        "canister",
        &expected.role,
        expected.canister_id.clone(),
        None,
        severity,
    ));
    let finding = finding(
        if expected.canister_id.is_some() {
            "canister_missing"
        } else {
            "canister_unobserved"
        },
        format!("missing observed canister for role {}", expected.role),
        severity,
        Some(expected.role.clone()),
    );
    if expected.canister_id.is_some() {
        hard_failures.push(finding);
    } else {
        warnings.push(finding);
    }
}

fn record_unsafe_canister_control_class(
    expected: &ExpectedCanisterV1,
    observed: &ObservedCanisterV1,
    controller_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    if !matches!(
        observed.control_class,
        CanisterControlClassV1::UnknownUnsafe | CanisterControlClassV1::UserControlled
    ) || expected.control_class != CanisterControlClassV1::DeploymentControlled
    {
        return;
    }
    controller_diff.push(diff_item(
        "control_class",
        &expected.role,
        Some("DeploymentControlled".to_string()),
        Some(format!("{:?}", observed.control_class)),
        SafetySeverityV1::HardFailure,
    ));
    hard_failures.push(finding(
        "unsafe_control_class",
        format!("role {} has unsafe observed control class", expected.role),
        SafetySeverityV1::HardFailure,
        Some(expected.role.clone()),
    ));
}

fn record_ambiguous_canister_role(
    expected: &ExpectedCanisterV1,
    observed_matches: &[&ObservedCanisterV1],
    controller_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    let observed_ids = observed_matches
        .iter()
        .map(|canister| canister.canister_id.as_str())
        .collect::<Vec<_>>()
        .join(",");
    controller_diff.push(diff_item(
        "canister_role_ambiguous",
        &expected.role,
        Some("one observed canister".to_string()),
        Some(observed_ids.clone()),
        SafetySeverityV1::HardFailure,
    ));
    hard_failures.push(finding(
        "canister_role_ambiguous",
        format!(
            "expected role {} has multiple observed canisters: {observed_ids}",
            expected.role
        ),
        SafetySeverityV1::HardFailure,
        Some(expected.role.clone()),
    ));
}

fn compare_observed_role(
    expected: &ExpectedCanisterV1,
    observed: &ObservedCanisterV1,
    controller_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    let Some(observed_role) = observed.role.as_deref() else {
        return;
    };
    if observed_role == expected.role {
        return;
    }
    controller_diff.push(diff_item(
        "role_mismatch",
        &expected.role,
        Some(expected.role.clone()),
        Some(observed_role.to_string()),
        SafetySeverityV1::HardFailure,
    ));
    hard_failures.push(finding(
        "canister_role_mismatch",
        format!(
            "expected canister {} to have role {}, observed role {observed_role}",
            observed.canister_id, expected.role
        ),
        SafetySeverityV1::HardFailure,
        Some(expected.role.clone()),
    ));
}

fn warn_extra_observed_canisters(
    plan: &DeploymentPlanV1,
    inventory: &DeploymentInventoryV1,
    controller_diff: &mut Vec<DiffItemV1>,
    warnings: &mut Vec<SafetyFindingV1>,
    matched_observed: &BTreeSet<&str>,
) {
    let expected_pool_roles = plan
        .expected_pool
        .iter()
        .filter_map(|pool| pool.role.as_deref())
        .collect::<BTreeSet<_>>();

    for observed in &inventory.observed_canisters {
        if matched_observed.contains(observed.canister_id.as_str()) {
            continue;
        }
        if let Some(role) = observed.role.as_deref()
            && expected_pool_roles.contains(role)
        {
            continue;
        }
        let subject = observed_canister_subject(observed);
        controller_diff.push(diff_item(
            "canister_extra",
            &subject,
            None,
            Some(observed.canister_id.clone()),
            SafetySeverityV1::Warning,
        ));
        warnings.push(finding(
            "extra_canister_observed",
            format!("observed undeclared canister {subject}"),
            SafetySeverityV1::Warning,
            Some(subject),
        ));
    }
}

pub(super) fn observed_canister_subject(observed: &ObservedCanisterV1) -> String {
    observed
        .role
        .clone()
        .unwrap_or_else(|| observed.canister_id.clone())
}
