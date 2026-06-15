use super::super::stable_json_sha256_hex;
use crate::deployment_truth::{
    DeploymentComparisonDiffV1, DeploymentComparisonReportV1, DeploymentComparisonTargetV1,
    SafetyFindingV1, SafetyStatusV1,
};
use serde::Serialize;

#[derive(Serialize)]
struct DeploymentComparisonReportDigestInput<'a> {
    report_id: &'a str,
    compared_at: &'a str,
    left: &'a DeploymentComparisonTargetV1,
    right: &'a DeploymentComparisonTargetV1,
    status: SafetyStatusV1,
    identity_diff: &'a [DeploymentComparisonDiffV1],
    artifact_diff: &'a [DeploymentComparisonDiffV1],
    module_hash_diff: &'a [DeploymentComparisonDiffV1],
    embedded_config_diff: &'a [DeploymentComparisonDiffV1],
    authority_diff: &'a [DeploymentComparisonDiffV1],
    pool_diff: &'a [DeploymentComparisonDiffV1],
    verifier_readiness_diff: &'a [DeploymentComparisonDiffV1],
    external_lifecycle_diff: &'a [DeploymentComparisonDiffV1],
    hard_failures: &'a [SafetyFindingV1],
    warnings: &'a [SafetyFindingV1],
    next_actions: &'a [String],
}

pub(super) fn deployment_comparison_report_digest(report: &DeploymentComparisonReportV1) -> String {
    stable_json_sha256_hex(&DeploymentComparisonReportDigestInput {
        report_id: &report.report_id,
        compared_at: &report.compared_at,
        left: &report.left,
        right: &report.right,
        status: report.status,
        identity_diff: &report.identity_diff,
        artifact_diff: &report.artifact_diff,
        module_hash_diff: &report.module_hash_diff,
        embedded_config_diff: &report.embedded_config_diff,
        authority_diff: &report.authority_diff,
        pool_diff: &report.pool_diff,
        verifier_readiness_diff: &report.verifier_readiness_diff,
        external_lifecycle_diff: &report.external_lifecycle_diff,
        hard_failures: &report.hard_failures,
        warnings: &report.warnings,
        next_actions: &report.next_actions,
    })
}
