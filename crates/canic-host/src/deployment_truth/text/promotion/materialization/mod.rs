use super::super::super::*;
use super::super::append_hard_failure_items;
use super::shared::promotion_readiness_status_label;

/// Render source/build materialization evidence as passive operator text.
#[must_use]
pub fn build_materialization_evidence_text(evidence: &BuildMaterializationEvidenceV1) -> String {
    [
        "Build materialization evidence".to_string(),
        "mode: passive".to_string(),
        format!("evidence_id: {}", evidence.evidence_id),
        format!(
            "materialization_evidence_digest: {}",
            evidence.materialization_evidence_digest
        ),
        format!("recipe_id: {}", evidence.recipe.recipe_id),
        format!(
            "materialization_input_id: {}",
            evidence.materialization_input.materialization_input_id
        ),
        format!(
            "materialization_result_id: {}",
            evidence.materialization_result.materialization_result_id
        ),
        format!(
            "computed_materialization_input_digest: {}",
            evidence.computed_materialization_input_digest
        ),
        format!(
            "recipe_id_matches_input: {}",
            evidence.recipe_id_matches_input
        ),
        format!(
            "recipe_id_matches_result: {}",
            evidence.recipe_id_matches_result
        ),
        format!(
            "materialization_input_digest_matches_result: {}",
            evidence.materialization_input_digest_matches_result
        ),
        "execution: none".to_string(),
    ]
    .join("\n")
}

/// Render source/build materialization identity as passive operator text.
#[must_use]
pub fn promotion_materialization_identity_report_text(
    report: &PromotionMaterializationIdentityReportV1,
) -> String {
    let mut lines = vec![
        "Promotion materialization identity report".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!(
            "status: {}",
            promotion_readiness_status_label(report.status)
        ),
        format!("report_id: {}", report.report_id),
        format!(
            "materialization_identity_report_digest: {}",
            report.materialization_identity_report_digest
        ),
        String::new(),
        "counts:".to_string(),
        format!("  roles: {}", report.roles.len()),
        format!("  output_groups: {}", report.output_groups.len()),
        format!("  blockers: {}", report.blockers.len()),
    ];

    append_hard_failure_items(&mut lines, "blockers", &report.blockers);
    if !report.output_groups.is_empty() {
        lines.push(String::new());
        lines.push("output groups:".to_string());
        for group in &report.output_groups {
            lines.push(format!(
                "  {} roles={} wasm={} installed={}",
                group.output_identity_key,
                group.roles.join(","),
                group.wasm_sha256,
                group.installed_module_hash
            ));
        }
    }
    if !report.roles.is_empty() {
        lines.push(String::new());
        lines.push("roles:".to_string());
        for role in &report.roles {
            lines.push(format!(
                "  {} evidence={} recipe={} input={} result={} network={} runtime={}",
                role.role,
                role.evidence_id,
                role.recipe_id,
                role.materialization_input_id,
                role.materialization_result_id,
                role.network,
                role.runtime_variant
            ));
        }
    }
    lines.join("\n")
}
