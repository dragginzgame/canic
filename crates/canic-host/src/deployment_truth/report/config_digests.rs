use super::super::*;
use super::{diff_item, finding};

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
                "raw_config_plan_inconsistent",
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
        "raw_config_sha256",
        "deployment",
        Some(expected.to_string()),
        Some(observed.to_string()),
        SafetySeverityV1::HardFailure,
    ));
    hard_failures.push(finding(
        "raw_config_digest_mismatch",
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
        "canonical_config",
        "deployment",
        Some(expected.to_string()),
        Some(observed.to_string()),
        SafetySeverityV1::HardFailure,
    ));
    hard_failures.push(finding(
        "canonical_config_mismatch",
        "canonical runtime config digest differs from the plan",
        SafetySeverityV1::HardFailure,
        Some("local_config".to_string()),
    ));
}

fn record_canonical_config_unobserved(warnings: &mut Vec<SafetyFindingV1>) {
    warnings.push(finding(
        "canonical_config_unobserved",
        "canonical runtime config digest was not observed",
        SafetySeverityV1::Warning,
        Some("local_config".to_string()),
    ));
}
