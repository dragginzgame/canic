use super::*;
use crate::deployment_truth::multi::{
    DEPLOYMENT_COMPARISON_DRIFT_CODE, DEPLOYMENT_COMPARISON_INPUT_BLOCKED_CODE,
    DEPLOYMENT_COMPARISON_INPUT_DIFF_STALE_CODE, DEPLOYMENT_COMPARISON_INPUT_REPORT_STALE_CODE,
    DEPLOYMENT_COMPARISON_INPUT_WARNING_CODE,
};

#[test]
fn deployment_comparison_report_detects_cross_deployment_drift() {
    let left = sample_check(sample_plan(), sample_matching_inventory());
    let mut right_plan = sample_plan();
    right_plan.plan_id = "plan-prod-root".to_string();
    right_plan.deployment_identity.deployment_name = "prod".to_string();
    right_plan.deployment_identity.environment = "ic".to_string();
    right_plan.trust_domain.root_trust_anchor = Some("prod-root".to_string());
    right_plan.role_artifacts[0].wasm_sha256 = Some(sample_sha256("b"));
    let mut right_inventory = sample_matching_inventory();
    right_inventory.inventory_id = "inventory-prod".to_string();
    right_inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    right_inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    right_inventory.observed_canisters[0].module_hash = Some("prod-module".to_string());
    right_inventory.observed_canisters[0].canonical_embedded_config_digest =
        Some("prod-config".to_string());
    let mut right = sample_check(right_plan, right_inventory);
    right.check_id = "check-prod".to_string();

    let report = deployment_comparison_report_from_checks(
        "comparison-1",
        "2026-05-26T00:00:00Z",
        "staging",
        "prod",
        &left,
        &right,
    );

    assert_eq!(report.schema_version, DEPLOYMENT_TRUTH_SCHEMA_VERSION);
    assert_eq!(report.report_id, "comparison-1");
    assert_eq!(report.report_digest.len(), 64);
    assert_eq!(report.left.label, "staging");
    assert_eq!(report.right.label, "prod");
    assert_eq!(report.status, SafetyStatusV1::Blocked);
    assert!(!report.identity_diff.is_empty());
    assert!(!report.artifact_diff.is_empty());
    assert!(!report.module_hash_diff.is_empty());
    assert!(!report.embedded_config_diff.is_empty());
    assert!(!report.authority_diff.is_empty());
    assert!(!report.external_lifecycle_diff.is_empty());
    assert!(
        report
            .hard_failures
            .iter()
            .any(|failure| failure.code == DEPLOYMENT_COMPARISON_INPUT_BLOCKED_CODE)
    );
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.code == DEPLOYMENT_COMPARISON_DRIFT_CODE)
    );
    validate_deployment_comparison_report(&report).expect("comparison should validate");
}

#[test]
fn deployment_comparison_report_validation_rejects_digest_drift() {
    let left = sample_check(sample_plan(), sample_matching_inventory());
    let right = sample_check(sample_plan(), sample_matching_inventory());
    let mut report = deployment_comparison_report_from_checks(
        "comparison-1",
        "2026-05-26T00:00:00Z",
        "left",
        "right",
        &left,
        &right,
    );

    report.next_actions.push("stale action".to_string());

    let err = validate_deployment_comparison_report(&report)
        .expect_err("stale comparison digest should fail");
    assert_eq!(
        err,
        DeploymentComparisonReportError::DigestMismatch {
            field: "report_digest"
        }
    );
}

#[test]
fn deployment_comparison_report_requires_target_deployment_identity() {
    let left = sample_check(sample_plan(), sample_matching_inventory());
    let right = sample_check(sample_plan(), sample_matching_inventory());
    let mut report = deployment_comparison_report_from_checks(
        "comparison-1",
        "2026-05-26T00:00:00Z",
        "left",
        "right",
        &left,
        &right,
    );

    report.left.deployment_identity.deployment_name.clear();

    let err = validate_deployment_comparison_report(&report)
        .expect_err("missing comparison target deployment name should fail");
    assert_eq!(
        err,
        DeploymentComparisonReportError::MissingRequiredField {
            field: "left.deployment_identity.deployment_name"
        }
    );
}

#[test]
fn deployment_comparison_report_blocks_stale_input_diff() {
    let mut left = sample_check(sample_plan(), sample_matching_inventory());
    left.diff.warnings.push(SafetyFindingV1 {
        code: "stale_warning".to_string(),
        message: "stale warning".to_string(),
        severity: SafetySeverityV1::Warning,
        subject: Some("root".to_string()),
    });
    let right = sample_check(sample_plan(), sample_matching_inventory());

    let report = deployment_comparison_report_from_checks(
        "comparison-1",
        "2026-05-26T00:00:00Z",
        "left",
        "right",
        &left,
        &right,
    );

    assert_eq!(report.status, SafetyStatusV1::Blocked);
    assert!(
        report
            .hard_failures
            .iter()
            .any(|failure| failure.code == DEPLOYMENT_COMPARISON_INPUT_DIFF_STALE_CODE)
    );
    validate_deployment_comparison_report(&report).expect("comparison should validate");
}

#[test]
fn deployment_comparison_report_blocks_stale_input_report() {
    let mut left_plan = sample_plan();
    left_plan
        .unresolved_assumptions
        .push(DeploymentAssumptionV1 {
            key: DeploymentAssumptionKindV1::LocalStateMissing
                .key()
                .to_string(),
            description: "root identity is unknown until install".to_string(),
        });
    let mut left = sample_check(left_plan, sample_matching_inventory());
    left.report.summary = "stale report summary".to_string();
    let right = sample_check(sample_plan(), sample_matching_inventory());

    let report = deployment_comparison_report_from_checks(
        "comparison-1",
        "2026-05-26T00:00:00Z",
        "left",
        "right",
        &left,
        &right,
    );

    assert_eq!(report.status, SafetyStatusV1::Blocked);
    assert!(
        report
            .hard_failures
            .iter()
            .any(|failure| failure.code == DEPLOYMENT_COMPARISON_INPUT_REPORT_STALE_CODE)
    );
    validate_deployment_comparison_report(&report).expect("comparison should validate");
}

#[test]
fn deployment_comparison_report_preserves_blocked_input_status() {
    let mut left_inventory = sample_matching_inventory();
    left_inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    left_inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    let left = sample_check(sample_plan(), left_inventory);
    assert_eq!(left.report.status, SafetyStatusV1::Blocked);
    let right = sample_check(sample_plan(), sample_matching_inventory());

    let report = deployment_comparison_report_from_checks(
        "comparison-1",
        "2026-05-26T00:00:00Z",
        "left",
        "right",
        &left,
        &right,
    );

    assert_eq!(report.status, SafetyStatusV1::Blocked);
    assert!(report.identity_diff.is_empty());
    assert!(
        report
            .hard_failures
            .iter()
            .any(|failure| failure.code == DEPLOYMENT_COMPARISON_INPUT_BLOCKED_CODE)
    );
    assert!(
        report
            .next_actions
            .iter()
            .any(|action| action.contains("resolve hard comparison failures"))
    );
    validate_deployment_comparison_report(&report).expect("comparison should validate");
}

#[test]
fn deployment_comparison_report_preserves_warning_input_status() {
    let mut left_plan = sample_plan();
    left_plan
        .unresolved_assumptions
        .push(DeploymentAssumptionV1 {
            key: DeploymentAssumptionKindV1::LocalStateMissing
                .key()
                .to_string(),
            description: "root identity is unknown until install".to_string(),
        });
    let left = sample_check(left_plan, sample_matching_inventory());
    assert_eq!(left.report.status, SafetyStatusV1::Warning);
    let right = sample_check(sample_plan(), sample_matching_inventory());

    let report = deployment_comparison_report_from_checks(
        "comparison-1",
        "2026-05-26T00:00:00Z",
        "left",
        "right",
        &left,
        &right,
    );

    assert_eq!(report.status, SafetyStatusV1::Warning);
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.code == DEPLOYMENT_COMPARISON_INPUT_WARNING_CODE)
    );
    validate_deployment_comparison_report(&report).expect("comparison should validate");
}

#[test]
fn deployment_comparison_report_text_is_passive() {
    let mut left = sample_check(sample_plan(), sample_matching_inventory());
    left.report.summary = "stale report summary".to_string();
    let right = sample_check(sample_plan(), sample_matching_inventory());
    let report = deployment_comparison_report_from_checks(
        "comparison-1",
        "2026-05-26T00:00:00Z",
        "left",
        "right",
        &left,
        &right,
    );

    let text = deployment_comparison_report_text(&report);

    assert!(text.contains("Deployment comparison report"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("external_lifecycle: 0"));
    assert!(text.contains("hard_failures:"));
    assert!(text.contains(DEPLOYMENT_COMPARISON_INPUT_REPORT_STALE_CODE));
    assert!(text.contains("next_actions:"));
}
