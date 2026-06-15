use super::super::super::*;
use super::super::append_warning_items;

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
