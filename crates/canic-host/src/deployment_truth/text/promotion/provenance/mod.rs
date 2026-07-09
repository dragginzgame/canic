use super::super::super::*;
use super::super::append_hard_failure_items;

/// Render artifact promotion provenance as passive operator text.
#[must_use]
pub fn artifact_promotion_provenance_report_text(
    report: &ArtifactPromotionProvenanceReportV1,
) -> String {
    let mut lines = vec![
        "Artifact promotion provenance report".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!("status: {}", report.status.label()),
        format!("report_id: {}", report.report_id),
        format!(
            "artifact_promotion_plan_id: {}",
            report.artifact_promotion_plan_id
        ),
        format!(
            "artifact_promotion_plan_digest: {}",
            report.artifact_promotion_plan_digest
        ),
        format!("promoted_plan_id: {}", report.promoted_plan_id),
        format!(
            "promotion_plan_lineage_digest: {}",
            report.promotion_plan_lineage_digest
        ),
        format!(
            "provenance_report_digest: {}",
            report.provenance_report_digest
        ),
    ];
    append_artifact_promotion_provenance_linked_reports(&mut lines, report);
    lines.extend([
        String::new(),
        "counts:".to_string(),
        format!("  roles: {}", report.roles.len()),
        format!("  blockers: {}", report.blockers.len()),
    ]);

    append_hard_failure_items(&mut lines, "blockers", &report.blockers);
    if !report.roles.is_empty() {
        lines.push(String::new());
        lines.push("roles:".to_string());
        for role in &report.roles {
            lines.push(format!(
                "  {} {:?}/{:?}: materialization={} materialization_digest={} wasm_store={} catalog_digest={}",
                role.role,
                role.promotion_level,
                role.source_kind,
                role.materialization_evidence_id
                    .as_deref()
                    .unwrap_or("none"),
                role.materialization_evidence_digest
                    .as_deref()
                    .unwrap_or("none"),
                role.wasm_store_locator.as_deref().unwrap_or("none"),
                role.wasm_store_catalog_observation_digest
                    .as_deref()
                    .unwrap_or("none")
            ));
        }
    }
    lines.join("\n")
}

fn append_artifact_promotion_provenance_linked_reports(
    lines: &mut Vec<String>,
    report: &ArtifactPromotionProvenanceReportV1,
) {
    lines.extend([
        String::new(),
        "linked reports:".to_string(),
        format!("  readiness: {}", report.readiness_id),
        format!(
            "  artifact_identity: {}",
            report.artifact_identity_report_id
        ),
        format!("  transform: {}", report.transform_id),
        format!(
            "  target_execution_lineage: {}",
            report
                .target_execution_lineage_id
                .as_deref()
                .unwrap_or("none")
        ),
        format!(
            "  wasm_store_identity: {}",
            report
                .wasm_store_identity_report_id
                .as_deref()
                .unwrap_or("none")
        ),
        format!(
            "  wasm_store_identity_digest: {}",
            report
                .wasm_store_identity_report_digest
                .as_deref()
                .unwrap_or("none")
        ),
        format!(
            "  wasm_store_catalog: {}",
            report
                .wasm_store_catalog_verification_id
                .as_deref()
                .unwrap_or("none")
        ),
        format!(
            "  wasm_store_catalog_digest: {}",
            report
                .wasm_store_catalog_verification_digest
                .as_deref()
                .unwrap_or("none")
        ),
        format!(
            "  materialization_identity: {}",
            report
                .materialization_identity_report_id
                .as_deref()
                .unwrap_or("none")
        ),
        format!(
            "  materialization_identity_digest: {}",
            report
                .materialization_identity_report_digest
                .as_deref()
                .unwrap_or("none")
        ),
    ]);
}
