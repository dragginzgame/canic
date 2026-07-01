use super::super::*;
use super::{diff_item, finding};

pub(in crate::deployment_truth) const RAW_CONFIG_PLAN_INCONSISTENT_CODE: &str =
    "raw_config_plan_inconsistent";
pub(in crate::deployment_truth) const RAW_CONFIG_SHA256_DIFF_CATEGORY: &str = "raw_config_sha256";
pub(in crate::deployment_truth) const RAW_CONFIG_DIGEST_MISMATCH_CODE: &str =
    "raw_config_digest_mismatch";
pub(in crate::deployment_truth) const CANONICAL_CONFIG_DIFF_CATEGORY: &str = "canonical_config";
pub(in crate::deployment_truth) const CANONICAL_CONFIG_MISMATCH_CODE: &str =
    "canonical_config_mismatch";
pub(in crate::deployment_truth) const CANONICAL_CONFIG_UNOBSERVED_CODE: &str =
    "canonical_config_unobserved";

pub(super) fn compare_raw_config(
    plan: &DeploymentPlanV1,
    inventory: &DeploymentInventoryV1,
    embedded_config_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    let mut expected = plan
        .role_artifacts
        .iter()
        .filter_map(|artifact| artifact.raw_config_sha256.as_ref())
        .collect::<Vec<_>>();
    expected.sort_unstable();
    expected.dedup();
    let [expected] = expected.as_slice() else {
        if expected.len() > 1 {
            hard_failures.push(finding(
                RAW_CONFIG_PLAN_INCONSISTENT_CODE,
                "planned role artifacts disagree on raw config digest",
                SafetySeverityV1::HardFailure,
                Some("role_artifacts.raw_config_sha256".to_string()),
            ));
        }
        return;
    };

    if let Some(observed) = &inventory.local_config.raw_config_sha256
        && observed != *expected
    {
        record_raw_config_mismatch(expected, observed, embedded_config_diff, hard_failures);
    }
}

fn record_raw_config_mismatch(
    expected: &str,
    observed: &str,
    embedded_config_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    embedded_config_diff.push(diff_item(
        RAW_CONFIG_SHA256_DIFF_CATEGORY,
        "deployment",
        Some(expected.to_string()),
        Some(observed.to_string()),
        SafetySeverityV1::HardFailure,
    ));
    hard_failures.push(finding(
        RAW_CONFIG_DIGEST_MISMATCH_CODE,
        "raw local config digest changed during deployment truth check",
        SafetySeverityV1::HardFailure,
        Some("local_config.raw_sha256".to_string()),
    ));
}

pub(super) fn compare_embedded_config(
    plan: &DeploymentPlanV1,
    inventory: &DeploymentInventoryV1,
    embedded_config_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    let Some(expected) = &plan.deployment_identity.canonical_runtime_config_digest else {
        return;
    };
    match &inventory.local_config.canonical_embedded_config_sha256 {
        Some(observed) if observed != expected => {
            record_canonical_config_mismatch(
                expected,
                observed,
                embedded_config_diff,
                hard_failures,
            );
        }
        None => record_canonical_config_unobserved(warnings),
        _ => {}
    }
}

fn record_canonical_config_mismatch(
    expected: &str,
    observed: &str,
    embedded_config_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    embedded_config_diff.push(diff_item(
        CANONICAL_CONFIG_DIFF_CATEGORY,
        "deployment",
        Some(expected.to_string()),
        Some(observed.to_string()),
        SafetySeverityV1::HardFailure,
    ));
    hard_failures.push(finding(
        CANONICAL_CONFIG_MISMATCH_CODE,
        "canonical runtime config digest differs from the plan",
        SafetySeverityV1::HardFailure,
        Some("local_config".to_string()),
    ));
}

fn record_canonical_config_unobserved(warnings: &mut Vec<SafetyFindingV1>) {
    warnings.push(finding(
        CANONICAL_CONFIG_UNOBSERVED_CODE,
        "canonical runtime config digest was not observed",
        SafetySeverityV1::Warning,
        Some("local_config".to_string()),
    ));
}
