use super::super::super::*;

/// Render artifact promotion execution receipt linkage as operator text.
#[must_use]
pub fn artifact_promotion_execution_receipt_text(
    receipt: &ArtifactPromotionExecutionReceiptV1,
) -> String {
    let mut lines = vec![
        "Artifact promotion execution receipt".to_string(),
        "mode: execution_receipt".to_string(),
        format!("receipt_id: {}", receipt.receipt_id),
        format!(
            "execution_receipt_digest: {}",
            receipt.execution_receipt_digest
        ),
        format!(
            "artifact_promotion_plan_id: {}",
            receipt.artifact_promotion_plan_id
        ),
        format!(
            "artifact_promotion_plan_digest: {}",
            receipt.artifact_promotion_plan_digest
        ),
        format!("provenance_report_id: {}", receipt.provenance_report_id),
        format!(
            "provenance_report_digest: {}",
            receipt.provenance_report_digest
        ),
        format!("promoted_plan_id: {}", receipt.promoted_plan_id),
        format!(
            "promotion_plan_lineage_digest: {}",
            receipt.promotion_plan_lineage_digest
        ),
        format!("operation_id: {}", receipt.operation_id),
        format!("provenance_status: {}", receipt.provenance_status.label()),
        format!("operation_status: {:?}", receipt.operation_status),
        format!("command_result: {:?}", receipt.command_result),
        format!("started_at: {}", receipt.started_at),
        format!(
            "finished_at: {}",
            receipt.finished_at.as_deref().unwrap_or("none")
        ),
        String::new(),
        "counts:".to_string(),
        format!("  roles: {}", receipt.roles.len()),
        format!(
            "  deployment_phase_receipts: {}",
            receipt.deployment_receipt.phase_receipts.len()
        ),
        format!(
            "  deployment_role_phase_receipts: {}",
            receipt.deployment_receipt.role_phase_receipts.len()
        ),
    ];

    if !receipt.roles.is_empty() {
        lines.push(String::new());
        lines.push("roles:".to_string());
        for role in &receipt.roles {
            lines.push(format!(
                "  {} {}: result={} artifact={} observed_module={} materialization_digest={} catalog_digest={}",
                role.role,
                role.promotion_level.label(),
                role.role_phase_result
                    .map_or_else(|| "none".to_string(), |result| result.label().to_string()),
                role.artifact_digest.as_deref().unwrap_or("none"),
                role.observed_module_hash_after.as_deref().unwrap_or("none"),
                role.materialization_evidence_digest
                    .as_deref()
                    .unwrap_or("none"),
                role.wasm_store_catalog_observation_digest
                    .as_deref()
                    .unwrap_or("none")
            ));
        }
    }
    lines.join("\n")
}
