use super::super::super::*;
use super::super::append_hard_failure_items;

/// Render a wasm-store identity report as passive operator text.
#[must_use]
pub fn promotion_wasm_store_identity_report_text(
    report: &PromotionWasmStoreIdentityReportV1,
) -> String {
    let mut lines = vec![
        "Promotion wasm-store identity report".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!("status: {}", report.status.label()),
        format!("report_id: {}", report.report_id),
        format!(
            "wasm_store_identity_report_digest: {}",
            report.wasm_store_identity_report_digest
        ),
        String::new(),
        "counts:".to_string(),
        format!("  roles: {}", report.roles.len()),
        format!("  blockers: {}", report.blockers.len()),
    ];

    append_hard_failure_items(&mut lines, "blockers", &report.blockers);
    if !report.roles.is_empty() {
        lines.push(String::new());
        lines.push("roles:".to_string());
        for role in &report.roles {
            lines.push(format!(
                "  {} artifact={} locator={} chunks={}/{} postcondition={}",
                role.role,
                role.artifact_identity,
                role.wasm_store_locator.as_deref().unwrap_or("none"),
                role.published_chunk_count,
                role.prepared_chunk_hashes.len(),
                role.verified_postcondition.status.label()
            ));
        }
    }
    lines.join("\n")
}

/// Render a wasm-store catalog verification report as passive operator text.
#[must_use]
pub fn promotion_wasm_store_catalog_verification_text(
    verification: &PromotionWasmStoreCatalogVerificationV1,
) -> String {
    let matching_roles = verification
        .roles
        .iter()
        .filter(|role| role.catalog_matches)
        .count();
    let missing_roles = verification
        .roles
        .iter()
        .filter(|role| !role.catalog_entry_present)
        .count();
    let mut lines = vec![
        "Promotion wasm-store catalog verification".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!("status: {}", verification.status.label()),
        format!("verification_id: {}", verification.verification_id),
        format!(
            "wasm_store_catalog_verification_digest: {}",
            verification.wasm_store_catalog_verification_digest
        ),
        format!(
            "wasm_store_identity_report_id: {}",
            verification.wasm_store_identity_report_id
        ),
        String::new(),
        "counts:".to_string(),
        format!("  roles: {}", verification.roles.len()),
        format!("  matching_roles: {matching_roles}"),
        format!("  missing_catalog_entries: {missing_roles}"),
        format!("  blockers: {}", verification.blockers.len()),
    ];

    append_hard_failure_items(&mut lines, "blockers", &verification.blockers);
    if !verification.roles.is_empty() {
        lines.push(String::new());
        lines.push("roles:".to_string());
        for role in &verification.roles {
            lines.push(format!(
                "  {} locator={} match={} digest={} expected_artifact={} observed_artifact={}",
                role.role,
                role.wasm_store_locator,
                role.catalog_matches,
                role.catalog_observation_digest,
                role.expected_artifact_identity,
                role.observed_artifact_identity.as_deref().unwrap_or("none")
            ));
        }
    }
    lines.join("\n")
}
