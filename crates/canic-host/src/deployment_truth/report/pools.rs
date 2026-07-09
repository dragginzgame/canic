use super::super::*;
use super::canisters::observed_canister_subject;
use super::{conflicting_assignment_groups, diff_item, duplicate_evidence_groups, finding};
use std::collections::{BTreeMap, BTreeSet};

pub(in crate::deployment_truth) const CANISTER_POOL_ROLE_CONFLICT_CODE: &str =
    "canister_pool_role_conflict";
pub(in crate::deployment_truth) const CANISTER_POOL_ROLE_CONFLICT_DIFF_CATEGORY: &str =
    "canister_pool_role_conflict";
pub(in crate::deployment_truth) const PLANNED_POOL_CONFLICT_CODE: &str = "planned_pool_conflict";
pub(in crate::deployment_truth) const PLANNED_POOL_CONFLICT_DIFF_CATEGORY: &str =
    "planned_pool_conflict";
pub(in crate::deployment_truth) const PLANNED_POOL_DUPLICATE_DIFF_CATEGORY: &str =
    "planned_pool_duplicate";
pub(in crate::deployment_truth) const DUPLICATE_PLANNED_POOL_CODE: &str = "duplicate_planned_pool";
pub(in crate::deployment_truth) const PLANNED_POOL_ID_CONFLICT_CODE: &str =
    "planned_pool_id_conflict";
pub(in crate::deployment_truth) const PLANNED_POOL_ID_CONFLICT_DIFF_CATEGORY: &str =
    "planned_pool_id_conflict";
pub(in crate::deployment_truth) const POOL_CANISTER_ID_CONFLICT_CODE: &str =
    "pool_canister_id_conflict";
pub(in crate::deployment_truth) const POOL_CANISTER_ID_CONFLICT_DIFF_CATEGORY: &str =
    "pool_canister_id_conflict";
pub(in crate::deployment_truth) const POOL_CANISTER_DUPLICATE_DIFF_CATEGORY: &str =
    "pool_canister_duplicate";
pub(in crate::deployment_truth) const DUPLICATE_POOL_CANISTER_OBSERVED_CODE: &str =
    "duplicate_pool_canister_observed";
pub(in crate::deployment_truth) const POOL_CANISTER_DIFF_CATEGORY: &str = "pool_canister";
pub(in crate::deployment_truth) const POOL_CANISTER_MISSING_CODE: &str = "pool_canister_missing";
const POOL_CANISTER_UNOBSERVED_CODE: &str = "pool_canister_unobserved";
pub(in crate::deployment_truth) const POOL_CANISTER_ID_DIFF_CATEGORY: &str = "pool_canister_id";
pub(in crate::deployment_truth) const POOL_CANISTER_ID_MISMATCH_CODE: &str =
    "pool_canister_id_mismatch";
pub(in crate::deployment_truth) const POOL_CONTROL_CLASS_DIFF_CATEGORY: &str = "pool_control_class";
pub(in crate::deployment_truth) const UNSAFE_POOL_CONTROL_CLASS_CODE: &str =
    "unsafe_pool_control_class";
pub(in crate::deployment_truth) const POOL_EXTRA_DIFF_CATEGORY: &str = "pool_extra";
pub(in crate::deployment_truth) const EXTRA_POOL_CANISTER_OBSERVED_CODE: &str =
    "extra_pool_canister_observed";

pub(super) fn compare_observed_canister_pool_role_conflicts(
    inventory: &DeploymentInventoryV1,
    pool_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    let mut pools_by_id = BTreeMap::<&str, Vec<&ObservedPoolCanisterV1>>::new();
    for observed_pool in &inventory.observed_pool {
        pools_by_id
            .entry(observed_pool.canister_id.as_str())
            .or_default()
            .push(observed_pool);
    }

    for observed_canister in &inventory.observed_canisters {
        let Some(canister_role) = observed_canister.role.as_deref() else {
            continue;
        };
        let Some(observed_pools) = pools_by_id.get(observed_canister.canister_id.as_str()) else {
            continue;
        };
        for observed_pool in observed_pools {
            let Some(pool_role) = observed_pool.role.as_deref() else {
                continue;
            };
            if pool_role == canister_role {
                continue;
            }
            let observed_label = format!(
                "canister={};pool={}",
                observed_canister_subject(observed_canister),
                observed_pool_subject(observed_pool)
            );
            pool_diff.push(diff_item(
                CANISTER_POOL_ROLE_CONFLICT_DIFF_CATEGORY,
                &observed_canister.canister_id,
                None,
                Some(observed_label.clone()),
                SafetySeverityV1::HardFailure,
            ));
            hard_failures.push(finding(
                CANISTER_POOL_ROLE_CONFLICT_CODE,
                format!(
                    "observed canister {} has conflicting canister/pool roles {observed_label}",
                    observed_canister.canister_id
                ),
                SafetySeverityV1::HardFailure,
                Some(observed_canister.canister_id.clone()),
            ));
        }
    }
}

pub(super) fn compare_pools(
    plan: &DeploymentPlanV1,
    inventory: &DeploymentInventoryV1,
    pool_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    let planned_conflicts =
        compare_planned_pool_conflicts(plan, pool_diff, hard_failures, warnings);
    compare_observed_pool_id_conflicts(inventory, pool_diff, hard_failures, warnings);
    let mut matched_observed = BTreeSet::new();
    let mut compared_planned = BTreeSet::new();
    for expected in &plan.expected_pool {
        if planned_conflicts
            .subject_conflicts
            .contains(&expected_pool_subject(expected))
            || expected
                .canister_id
                .as_ref()
                .is_some_and(|id| planned_conflicts.id_conflicts.contains(id))
            || !compared_planned.insert(planned_pool_evidence_label(expected))
        {
            continue;
        }
        compare_expected_pool(
            expected,
            inventory,
            pool_diff,
            hard_failures,
            warnings,
            &mut matched_observed,
        );
    }

    for observed in &inventory.observed_pool {
        warn_extra_observed_pool(plan, observed, pool_diff, warnings, &matched_observed);
    }
}

struct PlannedPoolConflicts {
    subject_conflicts: BTreeSet<String>,
    id_conflicts: BTreeSet<String>,
}

fn compare_planned_pool_conflicts(
    plan: &DeploymentPlanV1,
    pool_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) -> PlannedPoolConflicts {
    let mut subject_conflicts = BTreeSet::new();
    let mut id_conflicts = BTreeSet::new();

    for group in duplicate_evidence_groups(
        &plan.expected_pool,
        expected_pool_subject,
        planned_pool_evidence_label,
        " | ",
    ) {
        if group.is_conflict {
            subject_conflicts.insert(group.subject.clone());
            pool_diff.push(diff_item(
                PLANNED_POOL_CONFLICT_DIFF_CATEGORY,
                &group.subject,
                Some("one planned pool canister".to_string()),
                Some(group.evidence_label.clone()),
                SafetySeverityV1::HardFailure,
            ));
            hard_failures.push(finding(
                PLANNED_POOL_CONFLICT_CODE,
                format!(
                    "planned pool {} has conflicting evidence: {}",
                    group.subject, group.evidence_label
                ),
                SafetySeverityV1::HardFailure,
                Some(group.subject),
            ));
        } else {
            pool_diff.push(diff_item(
                PLANNED_POOL_DUPLICATE_DIFF_CATEGORY,
                &group.subject,
                Some(group.evidence_label.clone()),
                Some(group.count.to_string()),
                SafetySeverityV1::Warning,
            ));
            warnings.push(finding(
                DUPLICATE_PLANNED_POOL_CODE,
                format!(
                    "planned pool {} was declared {} times with identical evidence",
                    group.subject, group.count
                ),
                SafetySeverityV1::Warning,
                Some(group.subject),
            ));
        }
    }

    for group in conflicting_assignment_groups(
        &plan.expected_pool,
        |planned| planned.canister_id.clone(),
        expected_pool_subject,
        ",",
    ) {
        id_conflicts.insert(group.subject.clone());
        pool_diff.push(diff_item(
            PLANNED_POOL_ID_CONFLICT_DIFF_CATEGORY,
            &group.subject,
            Some("one planned pool identity".to_string()),
            Some(group.evidence_label.clone()),
            SafetySeverityV1::HardFailure,
        ));
        hard_failures.push(finding(
            PLANNED_POOL_ID_CONFLICT_CODE,
            format!(
                "planned pool id {} is assigned to conflicting identities {}",
                group.subject, group.evidence_label
            ),
            SafetySeverityV1::HardFailure,
            Some(group.subject),
        ));
    }

    PlannedPoolConflicts {
        subject_conflicts,
        id_conflicts,
    }
}

fn planned_pool_evidence_label(planned: &ExpectedPoolCanisterV1) -> String {
    format!(
        "pool={};role={};id={}",
        planned.pool,
        planned.role.as_deref().unwrap_or("<none>"),
        planned.canister_id.as_deref().unwrap_or("<none>")
    )
}

fn compare_observed_pool_id_conflicts(
    inventory: &DeploymentInventoryV1,
    pool_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    for group in duplicate_evidence_groups(
        &inventory.observed_pool,
        |observed| observed.canister_id.as_str().to_string(),
        observed_pool_subject,
        ",",
    ) {
        if group.is_conflict {
            pool_diff.push(diff_item(
                POOL_CANISTER_ID_CONFLICT_DIFF_CATEGORY,
                &group.subject,
                None,
                Some(group.evidence_label.clone()),
                SafetySeverityV1::HardFailure,
            ));
            hard_failures.push(finding(
                POOL_CANISTER_ID_CONFLICT_CODE,
                format!(
                    "observed pool canister {} has conflicting pool identities {}",
                    group.subject, group.evidence_label
                ),
                SafetySeverityV1::HardFailure,
                Some(group.subject),
            ));
        } else {
            pool_diff.push(diff_item(
                POOL_CANISTER_DUPLICATE_DIFF_CATEGORY,
                &group.subject,
                Some(group.evidence_label.clone()),
                Some(group.count.to_string()),
                SafetySeverityV1::Warning,
            ));
            warnings.push(finding(
                DUPLICATE_POOL_CANISTER_OBSERVED_CODE,
                format!(
                    "observed pool canister {} was reported {} times for {}",
                    group.subject, group.count, group.evidence_label
                ),
                SafetySeverityV1::Warning,
                Some(group.subject),
            ));
        }
    }
}

fn compare_expected_pool<'a>(
    expected: &ExpectedPoolCanisterV1,
    inventory: &'a DeploymentInventoryV1,
    pool_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
    matched_observed: &mut BTreeSet<&'a str>,
) {
    let observed = expected
        .canister_id
        .as_ref()
        .and_then(|id| {
            inventory
                .observed_pool
                .iter()
                .find(|pool| &pool.canister_id == id)
        })
        .or_else(|| {
            inventory
                .observed_pool
                .iter()
                .find(|pool| pool_matches_expected_pool(pool, expected))
        });
    let Some(observed) = observed else {
        record_missing_pool(expected, pool_diff, hard_failures, warnings);
        return;
    };

    matched_observed.insert(observed.canister_id.as_str());
    record_pool_id_mismatch(expected, observed, pool_diff, hard_failures);
    record_unsafe_pool_control_class(observed, pool_diff, hard_failures);
}

fn record_missing_pool(
    expected: &ExpectedPoolCanisterV1,
    pool_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    let severity = if expected.canister_id.is_some() {
        SafetySeverityV1::HardFailure
    } else {
        SafetySeverityV1::Warning
    };
    let subject = expected_pool_subject(expected);
    pool_diff.push(diff_item(
        POOL_CANISTER_DIFF_CATEGORY,
        &subject,
        expected.canister_id.clone(),
        None,
        severity,
    ));
    let finding = finding(
        if expected.canister_id.is_some() {
            POOL_CANISTER_MISSING_CODE
        } else {
            POOL_CANISTER_UNOBSERVED_CODE
        },
        format!("missing observed pool canister for {subject}"),
        severity,
        Some(subject),
    );
    if expected.canister_id.is_some() {
        hard_failures.push(finding);
    } else {
        warnings.push(finding);
    }
}

fn record_pool_id_mismatch(
    expected: &ExpectedPoolCanisterV1,
    observed: &ObservedPoolCanisterV1,
    pool_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    if let Some(expected_id) = expected.canister_id.as_ref()
        && observed.canister_id != *expected_id
    {
        let subject = observed_pool_subject(observed);
        pool_diff.push(diff_item(
            POOL_CANISTER_ID_DIFF_CATEGORY,
            &subject,
            Some(expected_id.clone()),
            Some(observed.canister_id.clone()),
            SafetySeverityV1::HardFailure,
        ));
        hard_failures.push(finding(
            POOL_CANISTER_ID_MISMATCH_CODE,
            format!(
                "pool canister {subject} has observed id {}, expected {expected_id}",
                observed.canister_id
            ),
            SafetySeverityV1::HardFailure,
            Some(subject),
        ));
    }
}

fn record_unsafe_pool_control_class(
    observed: &ObservedPoolCanisterV1,
    pool_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    if !matches!(
        observed.control_class,
        CanisterControlClassV1::UnknownUnsafe | CanisterControlClassV1::UserControlled
    ) {
        return;
    }
    let subject = observed_pool_subject(observed);
    pool_diff.push(diff_item(
        POOL_CONTROL_CLASS_DIFF_CATEGORY,
        &subject,
        Some(CanisterControlClassV1::CanicManagedPool.label().to_string()),
        Some(observed.control_class.label().to_string()),
        SafetySeverityV1::HardFailure,
    ));
    hard_failures.push(finding(
        UNSAFE_POOL_CONTROL_CLASS_CODE,
        format!("pool canister {subject} has unsafe observed control class"),
        SafetySeverityV1::HardFailure,
        Some(subject),
    ));
}

fn warn_extra_observed_pool(
    plan: &DeploymentPlanV1,
    observed: &ObservedPoolCanisterV1,
    pool_diff: &mut Vec<DiffItemV1>,
    warnings: &mut Vec<SafetyFindingV1>,
    matched_observed: &BTreeSet<&str>,
) {
    if matched_observed.contains(observed.canister_id.as_str())
        || plan.expected_pool.iter().any(|expected| {
            expected.canister_id.as_ref() == Some(&observed.canister_id)
                || pool_matches_expected_pool(observed, expected)
        })
    {
        return;
    }
    let subject = observed_pool_subject(observed);
    pool_diff.push(diff_item(
        POOL_EXTRA_DIFF_CATEGORY,
        &subject,
        None,
        Some(observed.canister_id.clone()),
        SafetySeverityV1::Warning,
    ));
    warnings.push(finding(
        EXTRA_POOL_CANISTER_OBSERVED_CODE,
        format!("observed undeclared pool canister {subject}"),
        SafetySeverityV1::Warning,
        Some(subject),
    ));
}

fn pool_matches_expected_pool(
    observed: &ObservedPoolCanisterV1,
    expected: &ExpectedPoolCanisterV1,
) -> bool {
    observed.pool == expected.pool
        && expected
            .role
            .as_ref()
            .is_none_or(|role| observed.role.as_ref() == Some(role))
}

fn expected_pool_subject(expected: &ExpectedPoolCanisterV1) -> String {
    expected.role.as_ref().map_or_else(
        || expected.pool.clone(),
        |role| format!("{}:{role}", expected.pool),
    )
}

fn observed_pool_subject(observed: &ObservedPoolCanisterV1) -> String {
    observed.role.as_ref().map_or_else(
        || observed.pool.clone(),
        |role| format!("{}:{role}", observed.pool),
    )
}
