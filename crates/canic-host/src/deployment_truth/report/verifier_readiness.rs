use super::super::*;
use super::{diff_item, duplicate_evidence_groups, finding};
use std::collections::{BTreeMap, BTreeSet};

pub(in crate::deployment_truth) const VERIFIER_READINESS_DIFF_CATEGORY: &str = "verifier_readiness";
pub(in crate::deployment_truth) const VERIFIER_READINESS_UNOBSERVED_CODE: &str =
    "verifier_readiness_unobserved";
pub(in crate::deployment_truth) const VERIFIER_ROLE_EPOCH_DIFF_CATEGORY: &str =
    "verifier_role_epoch";
pub(in crate::deployment_truth) const VERIFIER_ROLE_EPOCH_STALE_CODE: &str =
    "verifier_role_epoch_stale";
pub(in crate::deployment_truth) const VERIFIER_ROLE_EPOCH_UNOBSERVED_CODE: &str =
    "verifier_role_epoch_unobserved";
pub(in crate::deployment_truth) const PLANNED_VERIFIER_ROLE_EPOCH_CONFLICT_CODE: &str =
    "planned_verifier_role_epoch_conflict";
pub(in crate::deployment_truth) const PLANNED_VERIFIER_ROLE_EPOCH_CONFLICT_DIFF_CATEGORY: &str =
    "planned_verifier_role_epoch_conflict";
pub(in crate::deployment_truth) const PLANNED_VERIFIER_ROLE_EPOCH_DUPLICATE_DIFF_CATEGORY: &str =
    "planned_verifier_role_epoch_duplicate";
pub(in crate::deployment_truth) const DUPLICATE_PLANNED_VERIFIER_ROLE_EPOCH_CODE: &str =
    "duplicate_planned_verifier_role_epoch";
pub(in crate::deployment_truth) const VERIFIER_ROLE_EPOCH_CONFLICT_CODE: &str =
    "verifier_role_epoch_conflict";
pub(in crate::deployment_truth) const VERIFIER_ROLE_EPOCH_CONFLICT_DIFF_CATEGORY: &str =
    "verifier_role_epoch_conflict";
pub(in crate::deployment_truth) const VERIFIER_ROLE_EPOCH_DUPLICATE_DIFF_CATEGORY: &str =
    "verifier_role_epoch_duplicate";
pub(in crate::deployment_truth) const DUPLICATE_VERIFIER_ROLE_EPOCH_OBSERVED_CODE: &str =
    "duplicate_verifier_role_epoch_observed";
pub(in crate::deployment_truth) const VERIFIER_NOT_OBSERVED_LABEL: &str = "not_observed";

pub(super) fn compare_verifier_readiness(
    plan: &DeploymentPlanV1,
    inventory: &DeploymentInventoryV1,
    verifier_readiness_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    if !plan.expected_verifier_readiness.required {
        return;
    }
    if inventory.observed_verifier_readiness.status == ObservationStatusV1::NotObserved {
        verifier_readiness_diff.push(diff_item(
            VERIFIER_READINESS_DIFF_CATEGORY,
            "deployment",
            Some("required".to_string()),
            Some(VERIFIER_NOT_OBSERVED_LABEL.to_string()),
            SafetySeverityV1::Warning,
        ));
        warnings.push(finding(
            VERIFIER_READINESS_UNOBSERVED_CODE,
            "verifier readiness was required but not observed",
            SafetySeverityV1::Warning,
            Some("verifier_readiness".to_string()),
        ));
    }

    let planned_conflicting_roles = compare_planned_verifier_epoch_conflicts(
        plan,
        verifier_readiness_diff,
        hard_failures,
        warnings,
    );
    let conflicting_roles = compare_observed_verifier_epoch_conflicts(
        inventory,
        verifier_readiness_diff,
        hard_failures,
        warnings,
    );
    let mut observed_by_role = BTreeMap::new();
    for epoch in &inventory.observed_verifier_readiness.role_epochs {
        if conflicting_roles.contains(&epoch.role) {
            continue;
        }
        observed_by_role.entry(epoch.role.as_str()).or_insert(epoch);
    }
    let mut compared_roles = BTreeSet::new();
    for expected in &plan.expected_verifier_readiness.expected_role_epochs {
        if planned_conflicting_roles.contains(&expected.role)
            || conflicting_roles.contains(&expected.role)
            || !compared_roles.insert(expected.role.as_str())
        {
            continue;
        }
        let observed = observed_by_role.get(expected.role.as_str());
        if let Some(observed_epoch) = observed.and_then(|observed| {
            (observed.status == ObservationStatusV1::Observed)
                .then_some(observed.observed_epoch)
                .flatten()
        }) {
            if observed_epoch < expected.minimum_epoch {
                record_stale_verifier_role_epoch(
                    expected,
                    observed_epoch,
                    verifier_readiness_diff,
                    hard_failures,
                );
            }
        } else {
            record_unobserved_verifier_role_epoch(expected, verifier_readiness_diff, warnings);
        }
    }
}

fn record_stale_verifier_role_epoch(
    expected: &RoleEpochExpectationV1,
    observed_epoch: u64,
    verifier_readiness_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    verifier_readiness_diff.push(diff_item(
        VERIFIER_ROLE_EPOCH_DIFF_CATEGORY,
        &expected.role,
        Some(expected.minimum_epoch.to_string()),
        Some(observed_epoch.to_string()),
        SafetySeverityV1::HardFailure,
    ));
    hard_failures.push(finding(
        VERIFIER_ROLE_EPOCH_STALE_CODE,
        format!(
            "verifier role {} has epoch {observed_epoch}, expected at least {}",
            expected.role, expected.minimum_epoch
        ),
        SafetySeverityV1::HardFailure,
        Some(expected.role.clone()),
    ));
}

fn record_unobserved_verifier_role_epoch(
    expected: &RoleEpochExpectationV1,
    verifier_readiness_diff: &mut Vec<DiffItemV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    verifier_readiness_diff.push(diff_item(
        VERIFIER_ROLE_EPOCH_DIFF_CATEGORY,
        &expected.role,
        Some(expected.minimum_epoch.to_string()),
        Some(VERIFIER_NOT_OBSERVED_LABEL.to_string()),
        SafetySeverityV1::Warning,
    ));
    warnings.push(finding(
        VERIFIER_ROLE_EPOCH_UNOBSERVED_CODE,
        format!("verifier role {} epoch was not observed", expected.role),
        SafetySeverityV1::Warning,
        Some(expected.role.clone()),
    ));
}

fn compare_planned_verifier_epoch_conflicts(
    plan: &DeploymentPlanV1,
    verifier_readiness_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) -> BTreeSet<String> {
    let mut conflicting_roles = BTreeSet::new();
    for group in duplicate_evidence_groups(
        &plan.expected_verifier_readiness.expected_role_epochs,
        |expected| expected.role.as_str().to_string(),
        |expected| expected.minimum_epoch.to_string(),
        ",",
    ) {
        if group.is_conflict {
            conflicting_roles.insert(group.subject.clone());
            verifier_readiness_diff.push(diff_item(
                PLANNED_VERIFIER_ROLE_EPOCH_CONFLICT_DIFF_CATEGORY,
                &group.subject,
                Some("one minimum epoch".to_string()),
                Some(group.evidence_label.clone()),
                SafetySeverityV1::HardFailure,
            ));
            hard_failures.push(finding(
                PLANNED_VERIFIER_ROLE_EPOCH_CONFLICT_CODE,
                format!(
                    "planned verifier role {} has conflicting minimum epochs: {}",
                    group.subject, group.evidence_label
                ),
                SafetySeverityV1::HardFailure,
                Some(group.subject),
            ));
        } else {
            verifier_readiness_diff.push(diff_item(
                PLANNED_VERIFIER_ROLE_EPOCH_DUPLICATE_DIFF_CATEGORY,
                &group.subject,
                Some(group.evidence_label.clone()),
                Some(group.count.to_string()),
                SafetySeverityV1::Warning,
            ));
            warnings.push(finding(
                DUPLICATE_PLANNED_VERIFIER_ROLE_EPOCH_CODE,
                format!(
                    "planned verifier role {} epoch was declared {} times with identical evidence",
                    group.subject, group.count
                ),
                SafetySeverityV1::Warning,
                Some(group.subject),
            ));
        }
    }
    conflicting_roles
}

fn compare_observed_verifier_epoch_conflicts(
    inventory: &DeploymentInventoryV1,
    verifier_readiness_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) -> BTreeSet<String> {
    let mut conflicting_roles = BTreeSet::new();
    for group in duplicate_evidence_groups(
        &inventory.observed_verifier_readiness.role_epochs,
        |observed| observed.role.as_str().to_string(),
        verifier_epoch_evidence_label,
        ",",
    ) {
        if group.is_conflict {
            conflicting_roles.insert(group.subject.clone());
            verifier_readiness_diff.push(diff_item(
                VERIFIER_ROLE_EPOCH_CONFLICT_DIFF_CATEGORY,
                &group.subject,
                Some("one epoch observation".to_string()),
                Some(group.evidence_label.clone()),
                SafetySeverityV1::HardFailure,
            ));
            hard_failures.push(finding(
                VERIFIER_ROLE_EPOCH_CONFLICT_CODE,
                format!(
                    "verifier role {} has conflicting epoch observations: {}",
                    group.subject, group.evidence_label
                ),
                SafetySeverityV1::HardFailure,
                Some(group.subject),
            ));
        } else {
            verifier_readiness_diff.push(diff_item(
                VERIFIER_ROLE_EPOCH_DUPLICATE_DIFF_CATEGORY,
                &group.subject,
                Some(group.evidence_label.clone()),
                Some(group.count.to_string()),
                SafetySeverityV1::Warning,
            ));
            warnings.push(finding(
                DUPLICATE_VERIFIER_ROLE_EPOCH_OBSERVED_CODE,
                format!(
                    "verifier role {} epoch was reported {} times with identical evidence",
                    group.subject, group.count
                ),
                SafetySeverityV1::Warning,
                Some(group.subject),
            ));
        }
    }
    conflicting_roles
}

fn verifier_epoch_evidence_label(observed: &RoleEpochObservationV1) -> String {
    format!(
        "epoch={};status={:?}",
        observed
            .observed_epoch
            .map_or_else(|| "<none>".to_string(), |epoch| epoch.to_string()),
        observed.status
    )
}
