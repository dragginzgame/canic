use super::super::*;
use crate::deployment_truth::report::{
    DUPLICATE_PLANNED_VERIFIER_ROLE_EPOCH_CODE, DUPLICATE_VERIFIER_ROLE_EPOCH_OBSERVED_CODE,
    PLANNED_VERIFIER_ROLE_EPOCH_CONFLICT_CODE, PLANNED_VERIFIER_ROLE_EPOCH_CONFLICT_DIFF_CATEGORY,
    PLANNED_VERIFIER_ROLE_EPOCH_DUPLICATE_DIFF_CATEGORY, VERIFIER_NOT_OBSERVED_LABEL,
    VERIFIER_ROLE_EPOCH_CONFLICT_CODE, VERIFIER_ROLE_EPOCH_CONFLICT_DIFF_CATEGORY,
    VERIFIER_ROLE_EPOCH_DIFF_CATEGORY, VERIFIER_ROLE_EPOCH_DUPLICATE_DIFF_CATEGORY,
    VERIFIER_ROLE_EPOCH_STALE_CODE, VERIFIER_ROLE_EPOCH_UNOBSERVED_CODE,
};

#[test]
fn deployment_diff_blocks_stale_verifier_role_epoch() {
    let plan = sample_plan();
    let mut inventory = sample_matching_inventory();
    inventory.observed_verifier_readiness.role_epochs = vec![RoleEpochObservationV1 {
        role: "root".to_string(),
        observed_epoch: Some(0),
        status: ObservationStatusV1::Observed,
    }];

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == VERIFIER_ROLE_EPOCH_STALE_CODE)
    );
    assert!(diff.verifier_readiness_diff.iter().any(|item| {
        item.category == VERIFIER_ROLE_EPOCH_DIFF_CATEGORY
            && item.subject == "root"
            && item.expected.as_deref() == Some("1")
            && item.observed.as_deref() == Some("0")
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_warns_when_required_verifier_role_epoch_is_unobserved() {
    let plan = sample_plan();
    let mut inventory = sample_matching_inventory();
    inventory.observed_verifier_readiness.role_epochs.clear();

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == VERIFIER_ROLE_EPOCH_UNOBSERVED_CODE)
    );
    assert!(diff.verifier_readiness_diff.iter().any(|item| {
        item.category == VERIFIER_ROLE_EPOCH_DIFF_CATEGORY
            && item.subject == "root"
            && item.expected.as_deref() == Some("1")
            && item.observed.as_deref() == Some(VERIFIER_NOT_OBSERVED_LABEL)
            && item.severity == SafetySeverityV1::Warning
    }));
}

#[test]
fn deployment_diff_blocks_conflicting_verifier_role_epoch_observations() {
    let plan = sample_plan();
    let mut inventory = sample_matching_inventory();
    inventory.observed_verifier_readiness.role_epochs = vec![
        RoleEpochObservationV1 {
            role: "root".to_string(),
            observed_epoch: Some(1),
            status: ObservationStatusV1::Observed,
        },
        RoleEpochObservationV1 {
            role: "root".to_string(),
            observed_epoch: Some(0),
            status: ObservationStatusV1::Observed,
        },
    ];

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == VERIFIER_ROLE_EPOCH_CONFLICT_CODE
                && finding.subject.as_deref() == Some("root"))
    );
    assert!(diff.verifier_readiness_diff.iter().any(|item| {
        item.category == VERIFIER_ROLE_EPOCH_CONFLICT_DIFF_CATEGORY
            && item.subject == "root"
            && item.observed.as_deref().is_some_and(|observed| {
                observed.contains("epoch=1") && observed.contains("epoch=0")
            })
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_warns_for_duplicate_identical_verifier_role_epoch_observation() {
    let mut inventory = sample_matching_inventory();
    inventory
        .observed_verifier_readiness
        .role_epochs
        .push(inventory.observed_verifier_readiness.role_epochs[0].clone());

    let diff = compare_plan_to_inventory(&sample_plan(), &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(diff.warnings.iter().any(|finding| finding.code
        == DUPLICATE_VERIFIER_ROLE_EPOCH_OBSERVED_CODE
        && finding.subject.as_deref() == Some("root")));
    assert!(diff.verifier_readiness_diff.iter().any(|item| {
        item.category == VERIFIER_ROLE_EPOCH_DUPLICATE_DIFF_CATEGORY
            && item.subject == "root"
            && item.observed.as_deref() == Some("2")
            && item.severity == SafetySeverityV1::Warning
    }));
}

#[test]
fn deployment_diff_blocks_conflicting_planned_verifier_role_epoch() {
    let mut plan = sample_plan();
    plan.expected_verifier_readiness
        .expected_role_epochs
        .push(RoleEpochExpectationV1 {
            role: "root".to_string(),
            minimum_epoch: 2,
        });

    let diff = compare_plan_to_inventory(&plan, &sample_matching_inventory());

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(diff.hard_failures.iter().any(|finding| finding.code
        == PLANNED_VERIFIER_ROLE_EPOCH_CONFLICT_CODE
        && finding.subject.as_deref() == Some("root")));
    assert!(diff.verifier_readiness_diff.iter().any(|item| {
        item.category == PLANNED_VERIFIER_ROLE_EPOCH_CONFLICT_DIFF_CATEGORY
            && item.subject == "root"
            && item
                .observed
                .as_deref()
                .is_some_and(|observed| observed.contains('1') && observed.contains('2'))
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_warns_for_duplicate_identical_planned_verifier_role_epoch() {
    let mut plan = sample_plan();
    plan.expected_verifier_readiness
        .expected_role_epochs
        .push(plan.expected_verifier_readiness.expected_role_epochs[0].clone());

    let diff = compare_plan_to_inventory(&plan, &sample_matching_inventory());

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(diff.warnings.iter().any(|finding| finding.code
        == DUPLICATE_PLANNED_VERIFIER_ROLE_EPOCH_CODE
        && finding.subject.as_deref() == Some("root")));
    assert!(diff.verifier_readiness_diff.iter().any(|item| {
        item.category == PLANNED_VERIFIER_ROLE_EPOCH_DUPLICATE_DIFF_CATEGORY
            && item.subject == "root"
            && item.observed.as_deref() == Some("2")
            && item.severity == SafetySeverityV1::Warning
    }));
}
