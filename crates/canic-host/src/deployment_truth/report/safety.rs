use super::super::*;

/// Render an operator-facing safety report from a machine deployment diff.
#[must_use]
pub fn safety_report_from_diff(
    report_id: impl Into<String>,
    diff_id: Option<String>,
    diff: &DeploymentDiffV1,
) -> SafetyReportV1 {
    let status = safety_status(&diff.hard_failures, &diff.warnings);
    SafetyReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: report_id.into(),
        diff_id,
        status,
        summary: safety_summary(status, diff.hard_failures.len(), diff.warnings.len()),
        hard_failures: diff.hard_failures.clone(),
        warnings: diff.warnings.clone(),
        next_actions: safety_next_actions(status),
    }
}

pub(in crate::deployment_truth::report) const fn safety_status(
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

pub(in crate::deployment_truth::report) fn resume_safety_reasons(
    hard_failures: &[SafetyFindingV1],
    warnings: &[SafetyFindingV1],
) -> Vec<String> {
    if !hard_failures.is_empty() {
        return hard_failures
            .iter()
            .map(|finding| finding.message.clone())
            .collect();
    }
    if !warnings.is_empty() {
        return warnings
            .iter()
            .map(|finding| finding.message.clone())
            .collect();
    }
    vec!["no blocking deployment truth differences were found".to_string()]
}

fn safety_summary(
    status: SafetyStatusV1,
    hard_failure_count: usize,
    warning_count: usize,
) -> String {
    match status {
        SafetyStatusV1::Safe => "deployment inventory matches the checked plan".to_string(),
        SafetyStatusV1::Warning => {
            format!("deployment inventory has {warning_count} warning(s)")
        }
        SafetyStatusV1::Blocked => {
            format!(
                "deployment inventory has {hard_failure_count} blocking issue(s) and {warning_count} warning(s)"
            )
        }
        SafetyStatusV1::NotEvaluated => "deployment safety has not been evaluated".to_string(),
    }
}

fn safety_next_actions(status: SafetyStatusV1) -> Vec<String> {
    match status {
        SafetyStatusV1::Safe => Vec::new(),
        SafetyStatusV1::Warning => {
            vec!["review deployment warnings before continuing".to_string()]
        }
        SafetyStatusV1::Blocked => {
            vec!["resolve blocking deployment truth differences before mutation".to_string()]
        }
        SafetyStatusV1::NotEvaluated => vec!["collect deployment inventory".to_string()],
    }
}
