use super::super::super::*;
use super::super::{append_hard_failure_items, append_string_items, append_warning_items};
use super::shared::append_root_verification_check_items;

/// Render a deployment-root verification report as passive operator text.
#[must_use]
pub fn deployment_root_verification_report_text(
    report: &DeploymentRootVerificationReportV1,
) -> String {
    let mut lines = vec![
        "Deployment root verification report".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        "local_state_write: none".to_string(),
        format!("evidence_status: {}", report.evidence_status.label()),
        format!("state_transition: {}", report.state_transition.label()),
        format!("report_id: {}", report.report_id),
        format!("report_digest: {}", report.report_digest),
        format!("requested_at: {}", report.requested_at),
        format!("deployment: {}", report.deployment_name),
        format!("environment: {}", report.environment),
        format!("fleet_template: {}", report.expected_fleet_template),
        format!("root_principal: {}", report.expected_root_principal),
        format!(
            "observed_root_canister_id: {}",
            report
                .observed_root_canister_id
                .as_deref()
                .unwrap_or("missing")
        ),
        format!(
            "observed_root_observation_source: {}",
            report
                .observed_root_observation_source
                .map_or("missing", DeploymentRootObservationSourceV1::label)
        ),
        format!("source_check_id: {}", report.source_check_id),
        format!("source_check_digest: {}", report.source_check_digest),
        format!("source_inventory_id: {}", report.source_inventory_id),
        format!(
            "source_inventory_digest: {}",
            report.source_inventory_digest
        ),
        String::new(),
        "counts:".to_string(),
        format!("  identity_checks: {}", report.identity_checks.len()),
        format!("  evidence_checks: {}", report.evidence_checks.len()),
        format!("  blockers: {}", report.blockers.len()),
        format!("  warnings: {}", report.warnings.len()),
    ];

    append_root_verification_check_items(&mut lines, "identity_checks", &report.identity_checks);
    append_root_verification_check_items(&mut lines, "evidence_checks", &report.evidence_checks);
    append_hard_failure_items(&mut lines, "blockers", &report.blockers);
    append_warning_items(&mut lines, "warnings", &report.warnings);
    append_string_items(
        &mut lines,
        "recommended_next_actions",
        &report.recommended_next_actions,
    );
    lines.join("\n")
}
