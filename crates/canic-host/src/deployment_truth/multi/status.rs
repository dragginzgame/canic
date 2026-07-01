use super::super::{compare_plan_to_inventory, safety_report_from_diff};
use crate::deployment_truth::{
    DEPLOYMENT_TRUTH_SCHEMA_VERSION, DeploymentCheckV1, DeploymentComparisonDiffV1,
    SafetyFindingV1, SafetyReportV1, SafetySeverityV1, SafetyStatusV1,
};

pub(in crate::deployment_truth) const DEPLOYMENT_COMPARISON_DRIFT_CODE: &str =
    "deployment_comparison_drift";
pub(in crate::deployment_truth) const DEPLOYMENT_COMPARISON_INPUT_WARNING_CODE: &str =
    "deployment_comparison_input_warning";
pub(in crate::deployment_truth) const DEPLOYMENT_COMPARISON_INPUT_BLOCKED_CODE: &str =
    "deployment_comparison_input_blocked";
const DEPLOYMENT_COMPARISON_INPUT_NOT_EVALUATED_CODE: &str =
    "deployment_comparison_input_not_evaluated";
const DEPLOYMENT_COMPARISON_INPUT_SCHEMA_MISMATCH_CODE: &str =
    "deployment_comparison_input_schema_mismatch";
pub(in crate::deployment_truth) const DEPLOYMENT_COMPARISON_INPUT_DIFF_STALE_CODE: &str =
    "deployment_comparison_input_diff_stale";
pub(in crate::deployment_truth) const DEPLOYMENT_COMPARISON_INPUT_REPORT_STALE_CODE: &str =
    "deployment_comparison_input_report_stale";

pub(super) fn comparison_warnings(
    diff_groups: &[&[DeploymentComparisonDiffV1]],
) -> Vec<SafetyFindingV1> {
    let diff_count = diff_groups.iter().map(|group| group.len()).sum::<usize>();
    if diff_count == 0 {
        return Vec::new();
    }
    vec![SafetyFindingV1 {
        code: DEPLOYMENT_COMPARISON_DRIFT_CODE.to_string(),
        message: format!("deployment comparison found {diff_count} drift item(s)"),
        severity: SafetySeverityV1::Warning,
        subject: None,
    }]
}

pub(super) fn compare_input_check_status(
    label: &str,
    report: &SafetyReportV1,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    match report.status {
        SafetyStatusV1::Safe => {}
        SafetyStatusV1::Warning => warnings.push(SafetyFindingV1 {
            code: DEPLOYMENT_COMPARISON_INPUT_WARNING_CODE.to_string(),
            message: "input deployment check has warnings; comparison is drift evidence, not whole-deployment safety".to_string(),
            severity: SafetySeverityV1::Warning,
            subject: Some(format!("{label}:{}", report.report_id)),
        }),
        SafetyStatusV1::Blocked => hard_failures.push(SafetyFindingV1 {
            code: DEPLOYMENT_COMPARISON_INPUT_BLOCKED_CODE.to_string(),
            message: "input deployment check is blocked; comparison cannot be used as ready deployment evidence".to_string(),
            severity: SafetySeverityV1::HardFailure,
            subject: Some(format!("{label}:{}", report.report_id)),
        }),
        SafetyStatusV1::NotEvaluated => hard_failures.push(SafetyFindingV1 {
            code: DEPLOYMENT_COMPARISON_INPUT_NOT_EVALUATED_CODE.to_string(),
            message: "input deployment check was not evaluated; comparison cannot establish deployment safety".to_string(),
            severity: SafetySeverityV1::HardFailure,
            subject: Some(format!("{label}:{}", report.report_id)),
        }),
    }
}

pub(super) fn compare_input_check_consistency(
    label: &str,
    check: &DeploymentCheckV1,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    if check.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        hard_failures.push(SafetyFindingV1 {
            code: DEPLOYMENT_COMPARISON_INPUT_SCHEMA_MISMATCH_CODE.to_string(),
            message: "input deployment check schema version is unsupported".to_string(),
            severity: SafetySeverityV1::HardFailure,
            subject: Some(format!("{label}:{}", check.check_id)),
        });
        return;
    }

    let expected_diff = compare_plan_to_inventory(&check.plan, &check.inventory);
    if check.diff != expected_diff {
        hard_failures.push(SafetyFindingV1 {
            code: DEPLOYMENT_COMPARISON_INPUT_DIFF_STALE_CODE.to_string(),
            message: "input deployment check diff does not match its plan and inventory"
                .to_string(),
            severity: SafetySeverityV1::HardFailure,
            subject: Some(format!("{label}:{}", check.check_id)),
        });
        return;
    }

    let expected_report = safety_report_from_diff(
        &check.report.report_id,
        check.report.diff_id.clone(),
        &check.diff,
    );
    if check.report != expected_report {
        hard_failures.push(SafetyFindingV1 {
            code: DEPLOYMENT_COMPARISON_INPUT_REPORT_STALE_CODE.to_string(),
            message: "input deployment check report does not match its diff".to_string(),
            severity: SafetySeverityV1::HardFailure,
            subject: Some(format!("{label}:{}", check.check_id)),
        });
    }
}

pub(super) const fn comparison_status(
    hard_failures: &[SafetyFindingV1],
    warnings: &[SafetyFindingV1],
) -> SafetyStatusV1 {
    if !hard_failures.is_empty() {
        SafetyStatusV1::Blocked
    } else if !warnings.is_empty() {
        SafetyStatusV1::Warning
    } else {
        SafetyStatusV1::Safe
    }
}

pub(super) fn comparison_next_actions(status: SafetyStatusV1) -> Vec<String> {
    match status {
        SafetyStatusV1::Safe => vec!["no cross-deployment drift detected".to_string()],
        SafetyStatusV1::Warning => {
            vec!["review comparison drift before promotion, rebuild, or teardown".to_string()]
        }
        SafetyStatusV1::Blocked => {
            vec!["resolve hard comparison failures before using this evidence".to_string()]
        }
        SafetyStatusV1::NotEvaluated => vec!["run deployment comparison".to_string()],
    }
}
