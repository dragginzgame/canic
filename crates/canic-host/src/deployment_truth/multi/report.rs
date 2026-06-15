use super::super::stable_json_sha256_hex;
use super::{
    diff::{
        compare_artifact_evidence, compare_authority_evidence, compare_embedded_config_evidence,
        compare_external_lifecycle_evidence, compare_identity, compare_observed_module_hashes,
        compare_pool_evidence, compare_verifier_readiness_evidence,
    },
    digest::deployment_comparison_report_digest,
    status::{
        compare_input_check_consistency, compare_input_check_status, comparison_next_actions,
        comparison_status, comparison_warnings,
    },
};
use crate::deployment_truth::{
    DEPLOYMENT_TRUTH_SCHEMA_VERSION, DeploymentCheckV1, DeploymentComparisonReportV1,
    DeploymentComparisonTargetV1,
};

/// Build a passive 0.46 cross-deployment comparison report from two existing
/// deployment-truth checks. This is evidence comparison only; it does not
/// query live inventory or mutate deployment state.
#[must_use]
pub fn deployment_comparison_report_from_checks(
    report_id: impl Into<String>,
    compared_at: impl Into<String>,
    left_label: impl Into<String>,
    right_label: impl Into<String>,
    left: &DeploymentCheckV1,
    right: &DeploymentCheckV1,
) -> DeploymentComparisonReportV1 {
    let left_label = left_label.into();
    let right_label = right_label.into();
    let mut identity_diff = Vec::new();
    let mut artifact_diff = Vec::new();
    let mut module_hash_diff = Vec::new();
    let mut embedded_config_diff = Vec::new();
    let mut authority_diff = Vec::new();
    let mut pool_diff = Vec::new();
    let mut verifier_readiness_diff = Vec::new();
    let mut external_lifecycle_diff = Vec::new();

    compare_identity(left, right, &mut identity_diff);
    compare_artifact_evidence(left, right, &mut artifact_diff);
    compare_observed_module_hashes(left, right, &mut module_hash_diff);
    compare_embedded_config_evidence(left, right, &mut embedded_config_diff);
    compare_authority_evidence(left, right, &mut authority_diff);
    compare_pool_evidence(left, right, &mut pool_diff);
    compare_verifier_readiness_evidence(left, right, &mut verifier_readiness_diff);
    compare_external_lifecycle_evidence(left, right, &mut external_lifecycle_diff);

    let mut hard_failures = Vec::new();
    let mut warnings = Vec::new();
    compare_input_check_consistency(&left_label, left, &mut hard_failures);
    compare_input_check_consistency(&right_label, right, &mut hard_failures);
    compare_input_check_status(&left_label, &left.report, &mut hard_failures, &mut warnings);
    compare_input_check_status(
        &right_label,
        &right.report,
        &mut hard_failures,
        &mut warnings,
    );
    let diff_groups = [
        identity_diff.as_slice(),
        artifact_diff.as_slice(),
        module_hash_diff.as_slice(),
        embedded_config_diff.as_slice(),
        authority_diff.as_slice(),
        pool_diff.as_slice(),
        verifier_readiness_diff.as_slice(),
        external_lifecycle_diff.as_slice(),
    ];
    warnings.extend(comparison_warnings(&diff_groups));
    let status = comparison_status(&hard_failures, &warnings);
    let next_actions = comparison_next_actions(status);

    let mut report = DeploymentComparisonReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: report_id.into(),
        report_digest: String::new(),
        compared_at: compared_at.into(),
        left: comparison_target(left_label, left),
        right: comparison_target(right_label, right),
        status,
        identity_diff,
        artifact_diff,
        module_hash_diff,
        embedded_config_diff,
        authority_diff,
        pool_diff,
        verifier_readiness_diff,
        external_lifecycle_diff,
        hard_failures,
        warnings,
        next_actions,
    };
    report.report_digest = deployment_comparison_report_digest(&report);
    report
}

fn comparison_target(label: String, check: &DeploymentCheckV1) -> DeploymentComparisonTargetV1 {
    DeploymentComparisonTargetV1 {
        label,
        check_id: check.check_id.clone(),
        check_digest: stable_json_sha256_hex(check),
        plan_id: check.plan.plan_id.clone(),
        plan_digest: stable_json_sha256_hex(&check.plan),
        inventory_id: check.inventory.inventory_id.clone(),
        inventory_digest: stable_json_sha256_hex(&check.inventory),
        deployment_identity: check.plan.deployment_identity.clone(),
    }
}
