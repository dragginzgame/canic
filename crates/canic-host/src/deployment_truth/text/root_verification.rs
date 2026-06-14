use super::super::*;
use super::{append_hard_failure_items, append_string_items, append_warning_items};

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
        format!("evidence_status: {:?}", report.evidence_status),
        format!("state_transition: {:?}", report.state_transition),
        format!("report_id: {}", report.report_id),
        format!("report_digest: {}", report.report_digest),
        format!("requested_at: {}", report.requested_at),
        format!("deployment: {}", report.deployment_name),
        format!("network: {}", report.network),
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
                .map_or_else(|| "missing".to_string(), |source| format!("{source:?}"))
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

/// Render a deployment-root verification receipt as operator text.
#[must_use]
pub fn deployment_root_verification_receipt_text(
    receipt: &DeploymentRootVerificationReceiptV1,
) -> String {
    let mut lines = vec![
        "Deployment root verification receipt".to_string(),
        "mode: local-state-write".to_string(),
        "canister_execution: none".to_string(),
        "local_state_write: recorded".to_string(),
        format!("state_transition: {:?}", receipt.state_transition),
        format!("receipt_id: {}", receipt.receipt_id),
        format!("receipt_digest: {}", receipt.receipt_digest),
        format!("deployment: {}", receipt.deployment_name),
        format!("network: {}", receipt.network),
        format!("fleet_template: {}", receipt.fleet_template),
        format!("root_principal: {}", receipt.root_principal),
        format!(
            "previous_root_verification: {:?}",
            receipt.previous_root_verification
        ),
        format!("new_root_verification: {:?}", receipt.new_root_verification),
        format!("source_report_id: {}", receipt.source_report_id),
        format!("source_report_digest: {}", receipt.source_report_digest),
        format!(
            "source_report_requested_at: {}",
            receipt.source_report_requested_at
        ),
        format!("source_report_source: {:?}", receipt.source_report_source),
        format!(
            "source_report_evidence_status: {:?}",
            receipt.source_report_evidence_status
        ),
        format!(
            "source_report_current_root_verification: {:?}",
            receipt.source_report_current_root_verification
        ),
        format!(
            "source_report_state_transition: {:?}",
            receipt.source_report_state_transition
        ),
        format!(
            "source_root_observation_source: {:?}",
            receipt.source_root_observation_source
        ),
        format!(
            "source_observed_root_canister_id: {}",
            receipt.source_observed_root_canister_id
        ),
        format!("source_check_id: {}", receipt.source_check_id),
        format!("source_check_digest: {}", receipt.source_check_digest),
        format!("source_inventory_id: {}", receipt.source_inventory_id),
        format!(
            "source_inventory_digest: {}",
            receipt.source_inventory_digest
        ),
        format!("verified_at_unix_secs: {}", receipt.verified_at_unix_secs),
        format!("local_state_path: {}", receipt.local_state_path),
        format!(
            "local_state_digest_before: {}",
            receipt.local_state_digest_before
        ),
        format!(
            "local_state_digest_after: {}",
            receipt.local_state_digest_after
        ),
    ];

    append_warning_items(&mut lines, "warnings", &receipt.warnings);
    lines.join("\n")
}

fn append_root_verification_check_items(
    lines: &mut Vec<String>,
    label: &str,
    items: &[DeploymentRootVerificationCheckV1],
) {
    if items.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push(format!("{label}:"));
    for item in items {
        lines.push(format!(
            "  - {} expected={} observed={} satisfied={}",
            item.name,
            item.expected.as_deref().unwrap_or("missing"),
            item.observed.as_deref().unwrap_or("missing"),
            item.satisfied
        ));
    }
}
