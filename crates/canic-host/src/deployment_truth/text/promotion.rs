use super::super::*;
use super::{
    append_hard_failure_items, append_warning_items, deployment_execution_preflight_text,
    optional_bool_label,
};

/// Render promotion readiness as passive operator text.
#[must_use]
pub fn promotion_readiness_text(readiness: &PromotionReadinessV1) -> String {
    let restage_required = readiness
        .roles
        .iter()
        .filter(|role| role.restage_required)
        .count();
    let mut lines = vec![
        "Promotion readiness report".to_string(),
        "mode: passive".to_string(),
        format!(
            "status: {}",
            promotion_readiness_status_label(readiness.status)
        ),
        format!("readiness_id: {}", readiness.readiness_id),
        format!(
            "promotion_readiness_digest: {}",
            readiness.promotion_readiness_digest
        ),
        format!("target_plan_id: {}", readiness.target_plan_id),
        String::new(),
        "counts:".to_string(),
        format!("  roles: {}", readiness.roles.len()),
        format!("  blockers: {}", readiness.blockers.len()),
        format!("  warnings: {}", readiness.warnings.len()),
        format!("  restage_required: {restage_required}"),
    ];

    append_promotion_role_items(&mut lines, &readiness.roles);
    append_hard_failure_items(&mut lines, "blockers", &readiness.blockers);
    append_warning_items(&mut lines, "warnings", &readiness.warnings);
    lines.join("\n")
}

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

/// Render a promotion policy check as passive operator text.
#[must_use]
pub fn promotion_policy_check_text(check: &PromotionPolicyCheckV1) -> String {
    let satisfied = check
        .roles
        .iter()
        .filter(|role| role.policy_satisfied)
        .count();
    let mut lines = vec![
        "Promotion policy check".to_string(),
        "mode: passive".to_string(),
        format!("status: {}", promotion_readiness_status_label(check.status)),
        format!("check_id: {}", check.check_id),
        format!(
            "promotion_policy_check_digest: {}",
            check.promotion_policy_check_digest
        ),
        String::new(),
        "counts:".to_string(),
        format!("  roles: {}", check.roles.len()),
        format!("  policy_satisfied: {satisfied}"),
        format!("  blockers: {}", check.blockers.len()),
    ];

    append_promotion_policy_decision_items(&mut lines, &check.roles);
    append_hard_failure_items(&mut lines, "blockers", &check.blockers);
    lines.join("\n")
}

/// Render a promotion artifact identity report as passive operator text.
#[must_use]
pub fn promotion_artifact_identity_report_text(
    report: &PromotionArtifactIdentityReportV1,
) -> String {
    let mut lines = vec![
        "Promotion artifact identity report".to_string(),
        "mode: passive".to_string(),
        format!(
            "status: {}",
            promotion_readiness_status_label(report.status)
        ),
        format!("report_id: {}", report.report_id),
        format!(
            "artifact_identity_report_digest: {}",
            report.artifact_identity_report_digest
        ),
        String::new(),
        "counts:".to_string(),
        format!("  roles: {}", report.summary.role_count),
        format!("  identity_groups: {}", report.summary.identity_group_count),
        format!(
            "  shared_identity_groups: {}",
            report.summary.shared_identity_group_count
        ),
        format!(
            "  digest_pinned_roles: {}",
            report.summary.digest_pinned_role_count
        ),
        format!(
            "  source_build_roles: {}",
            report.summary.source_build_role_count
        ),
        format!(
            "  deferred_identity_roles: {}",
            report.summary.deferred_identity_role_count
        ),
        format!("  blockers: {}", report.blockers.len()),
    ];

    append_promotion_artifact_identity_group_items(&mut lines, &report.identity_groups);
    append_promotion_artifact_identity_role_items(&mut lines, &report.roles);
    append_hard_failure_items(&mut lines, "blockers", &report.blockers);
    lines.join("\n")
}

/// Render a wasm-store identity report as passive operator text.
#[must_use]
pub fn promotion_wasm_store_identity_report_text(
    report: &PromotionWasmStoreIdentityReportV1,
) -> String {
    let mut lines = vec![
        "Promotion wasm-store identity report".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!(
            "status: {}",
            promotion_readiness_status_label(report.status)
        ),
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
                "  {} artifact={} locator={} chunks={}/{} postcondition={:?}",
                role.role,
                role.artifact_identity,
                role.wasm_store_locator.as_deref().unwrap_or("none"),
                role.published_chunk_count,
                role.prepared_chunk_hashes.len(),
                role.verified_postcondition.status
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
        format!(
            "status: {}",
            promotion_readiness_status_label(verification.status)
        ),
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

/// Render a promotion plan transform as passive operator text.
#[must_use]
pub fn promotion_plan_transform_text(transform: &PromotionPlanTransformV1) -> String {
    let changed_artifacts = transform
        .roles
        .iter()
        .filter(|role| role.artifact_identity_changed)
        .count();
    let changed_configs = transform
        .roles
        .iter()
        .filter(|role| role.embedded_config_changed)
        .count();
    let preserved_materializations = transform
        .roles
        .iter()
        .filter(|role| role.target_materialization_preserved)
        .count();
    let mut lines = vec![
        "Promotion plan transform".to_string(),
        "mode: passive".to_string(),
        format!("transform_id: {}", transform.transform_id),
        format!("target_plan_id: {}", transform.target_plan_id),
        format!("promoted_plan_id: {}", transform.promoted_plan_id),
        format!(
            "promotion_plan_lineage_digest: {}",
            transform.promotion_plan_lineage_digest
        ),
        String::new(),
        "counts:".to_string(),
        format!("  roles: {}", transform.roles.len()),
        format!("  artifact_identity_changed: {changed_artifacts}"),
        format!("  embedded_config_changed: {changed_configs}"),
        format!("  target_materialization_preserved: {preserved_materializations}"),
    ];

    append_promotion_transform_role_items(&mut lines, &transform.roles);
    lines.join("\n")
}

/// Render promotion transform evidence as passive operator text.
#[must_use]
pub fn promotion_plan_transform_evidence_text(
    evidence: &PromotionPlanTransformEvidenceV1,
) -> String {
    let mut lines = vec![
        "Promotion plan transform evidence".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!("evidence_id: {}", evidence.evidence_id),
        format!(
            "promotion_plan_transform_evidence_digest: {}",
            evidence.promotion_plan_transform_evidence_digest
        ),
        format!("generated_at: {}", evidence.generated_at),
        format!("transform_id: {}", evidence.transform.transform_id),
        format!("target_plan_id: {}", evidence.transform.target_plan_id),
        format!("promoted_plan_id: {}", evidence.transform.promoted_plan_id),
        String::new(),
        "transform:".to_string(),
    ];

    lines.extend(
        promotion_plan_transform_text(&evidence.transform)
            .lines()
            .map(|line| format!("  {line}")),
    );
    lines.join("\n")
}

/// Render target execution lineage as passive operator text.
#[must_use]
pub fn promotion_target_execution_lineage_text(
    lineage: &PromotionTargetExecutionLineageV1,
) -> String {
    let mut lines = vec![
        "Promotion target execution lineage".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!("lineage_id: {}", lineage.lineage_id),
        format!("generated_at: {}", lineage.generated_at),
        format!(
            "target_execution_lineage_digest: {}",
            lineage.target_execution_lineage_digest
        ),
        format!("transform_id: {}", lineage.transform.transform_id),
        format!("target_plan_id: {}", lineage.transform.target_plan_id),
        format!("promoted_plan_id: {}", lineage.transform.promoted_plan_id),
        format!("preflight_plan_id: {}", lineage.execution_preflight.plan_id),
        format!(
            "preflight_safety_report_id: {}",
            lineage.execution_preflight.safety_report_id
        ),
        format!(
            "preflight_authority_plan_id: {}",
            lineage.execution_preflight.authority_plan_id
        ),
        format!("backend: {:?}", lineage.execution_preflight.backend),
        format!("preflight_status: {:?}", lineage.execution_preflight.status),
        format!("execution_attempted: {}", lineage.execution_attempted),
    ];

    lines.push(String::new());
    lines.push("promotion_plan:".to_string());
    lines.extend(
        promotion_plan_transform_text(&lineage.transform)
            .lines()
            .map(|line| format!("  {line}")),
    );
    lines.push(String::new());
    lines.push("execution_preflight:".to_string());
    lines.extend(
        deployment_execution_preflight_text(&lineage.execution_preflight)
            .lines()
            .map(|line| format!("  {line}")),
    );
    lines.join("\n")
}

/// Render an artifact promotion plan as passive operator text.
#[must_use]
pub fn artifact_promotion_plan_text(plan: &ArtifactPromotionPlanV1) -> String {
    let mut lines = vec![
        "Artifact promotion plan".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!("plan_id: {}", plan.plan_id),
        format!(
            "artifact_promotion_plan_digest: {}",
            plan.artifact_promotion_plan_digest
        ),
        format!("generated_at: {}", plan.generated_at),
        format!("status: {:?}", plan.status),
        format!("target_plan_id: {}", plan.target_plan_id),
        format!("promoted_plan_id: {}", plan.promoted_plan_id),
        format!(
            "promotion_plan_lineage_digest: {}",
            plan.promotion_plan_lineage_digest
        ),
        format!(
            "target_execution_lineage: {}",
            plan.target_execution_lineage
                .as_ref()
                .map_or("none", |lineage| lineage.lineage_id.as_str())
        ),
        String::new(),
        "counts:".to_string(),
        format!("  readiness_roles: {}", plan.readiness.roles.len()),
        format!(
            "  artifact_identity_roles: {}",
            plan.artifact_identity_report.roles.len()
        ),
        format!("  transform_roles: {}", plan.transform.roles.len()),
        format!("  blockers: {}", plan.blockers.len()),
    ];

    append_hard_failure_items(&mut lines, "blockers", &plan.blockers);
    lines.push(String::new());
    lines.push("readiness:".to_string());
    lines.extend(
        promotion_readiness_text(&plan.readiness)
            .lines()
            .map(|line| format!("  {line}")),
    );
    lines.push(String::new());
    lines.push("artifact_identity:".to_string());
    lines.extend(
        promotion_artifact_identity_report_text(&plan.artifact_identity_report)
            .lines()
            .map(|line| format!("  {line}")),
    );
    lines.push(String::new());
    lines.push("transform:".to_string());
    lines.extend(
        promotion_plan_transform_text(&plan.transform)
            .lines()
            .map(|line| format!("  {line}")),
    );
    if let Some(lineage) = &plan.target_execution_lineage {
        lines.push(String::new());
        lines.push("target_execution_lineage:".to_string());
        lines.extend(
            promotion_target_execution_lineage_text(lineage)
                .lines()
                .map(|line| format!("  {line}")),
        );
    }
    lines.join("\n")
}

/// Render artifact promotion provenance as passive operator text.
#[must_use]
pub fn artifact_promotion_provenance_report_text(
    report: &ArtifactPromotionProvenanceReportV1,
) -> String {
    let mut lines = vec![
        "Artifact promotion provenance report".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!(
            "status: {}",
            promotion_readiness_status_label(report.status)
        ),
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
        format!(
            "provenance_status: {}",
            promotion_readiness_status_label(receipt.provenance_status)
        ),
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
                "  {} {:?}: result={} artifact={} observed_module={} materialization_digest={} catalog_digest={}",
                role.role,
                role.promotion_level,
                role.role_phase_result
                    .map_or_else(|| "none".to_string(), |result| format!("{result:?}")),
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

fn append_promotion_role_items(lines: &mut Vec<String>, roles: &[RolePromotionReadinessV1]) {
    if roles.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push("roles:".to_string());
    for role in roles {
        lines.push(format!(
            "  - {} {:?}/{:?}: byte_identical_wasm={} embedded_config_identical={} restage_required={}",
            role.role,
            role.promotion_level,
            role.source_kind,
            optional_bool_label(role.byte_identical_wasm),
            optional_bool_label(role.embedded_config_identical),
            role.restage_required
        ));
        lines.push(format!(
            "    source_wasm_gz_sha256: {}",
            role.source_wasm_gz_sha256
                .as_deref()
                .unwrap_or("not recorded")
        ));
        lines.push(format!(
            "    target_wasm_gz_sha256: {}",
            role.target_wasm_gz_sha256
                .as_deref()
                .unwrap_or("not recorded")
        ));
        lines.push(format!(
            "    source_config_sha256: {}",
            role.source_canonical_embedded_config_sha256
                .as_deref()
                .unwrap_or("not recorded")
        ));
        lines.push(format!(
            "    target_config_sha256: {}",
            role.target_canonical_embedded_config_sha256
                .as_deref()
                .unwrap_or("not recorded")
        ));
    }
}

fn append_promotion_artifact_identity_role_items(
    lines: &mut Vec<String>,
    roles: &[RolePromotionArtifactIdentityV1],
) {
    if roles.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push("roles:".to_string());
    for role in roles {
        lines.push(format!(
            "  - {} {:?}/{:?}: identity_kind={:?} digest_pinned={}",
            role.role,
            role.promotion_level,
            role.source_kind,
            role.identity_kind,
            role.digest_pinned
        ));
        lines.push(format!(
            "    source_locator: {}",
            role.source_locator.as_deref().unwrap_or("not recorded")
        ));
        lines.push(format!(
            "    wasm_gz_sha256: {}",
            role.wasm_gz_sha256.as_deref().unwrap_or("not recorded")
        ));
        lines.push(format!(
            "    config_sha256: {}",
            role.canonical_embedded_config_sha256
                .as_deref()
                .unwrap_or("not recorded")
        ));
    }
}

fn append_promotion_artifact_identity_group_items(
    lines: &mut Vec<String>,
    groups: &[PromotionArtifactIdentityGroupV1],
) {
    if groups.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push("identity groups:".to_string());
    for group in groups {
        lines.push(format!(
            "  - {}: kind={:?} source_kinds={} roles={}",
            group.identity_key,
            group.identity_kind,
            group
                .source_kinds
                .iter()
                .map(|kind| format!("{kind:?}"))
                .collect::<Vec<_>>()
                .join(","),
            group.roles.join(",")
        ));
    }
}

fn append_promotion_policy_decision_items(
    lines: &mut Vec<String>,
    roles: &[RolePromotionPolicyDecisionV1],
) {
    if roles.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push("roles:".to_string());
    for role in roles {
        lines.push(format!(
            "  - {} {:?}: policy_satisfied={} level_allowed={} requirements={} claims={}",
            role.role,
            role.requested_promotion_level,
            role.policy_satisfied,
            role.level_allowed,
            role.requirements
                .iter()
                .map(|requirement| format!("{requirement:?}"))
                .collect::<Vec<_>>()
                .join(","),
            role.claims
                .iter()
                .map(|claim| format!("{claim:?}"))
                .collect::<Vec<_>>()
                .join(",")
        ));
    }
}

fn append_promotion_transform_role_items(
    lines: &mut Vec<String>,
    roles: &[RolePromotionPlanTransformV1],
) {
    if roles.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push("roles:".to_string());
    for role in roles {
        lines.push(format!(
            "  - {} {:?}/{:?}: artifact_identity_changed={} embedded_config_changed={} target_materialization_preserved={}",
            role.role,
            role.promotion_level,
            role.source_kind,
            role.artifact_identity_changed,
            role.embedded_config_changed,
            role.target_materialization_preserved
        ));
        lines.push(format!(
            "    artifact_source: {:?} -> {:?}",
            role.artifact_source_before, role.artifact_source_after
        ));
        lines.push(format!(
            "    wasm_gz_sha256: {} -> {}",
            role.wasm_gz_sha256_before
                .as_deref()
                .unwrap_or("not recorded"),
            role.wasm_gz_sha256_after
                .as_deref()
                .unwrap_or("not recorded")
        ));
        lines.push(format!(
            "    config_sha256: {} -> {}",
            role.canonical_embedded_config_sha256_before
                .as_deref()
                .unwrap_or("not recorded"),
            role.canonical_embedded_config_sha256_after
                .as_deref()
                .unwrap_or("not recorded")
        ));
        if let Some(materialization) = &role.source_build_materialization {
            lines.push(format!(
                "    materialization_evidence_id: {}",
                materialization.evidence_id
            ));
            lines.push(format!(
                "    materialization_evidence_digest: {}",
                materialization.materialization_evidence_digest
            ));
            lines.push(format!(
                "    materialization_input_digest: {}",
                materialization.materialization_input_digest
            ));
            lines.push(format!(
                "    materialized_wasm_gz_sha256: {}",
                materialization.wasm_gz_sha256
            ));
        }
    }
}

const fn promotion_readiness_status_label(status: PromotionReadinessStatusV1) -> &'static str {
    match status {
        PromotionReadinessStatusV1::Ready => "ready",
        PromotionReadinessStatusV1::Blocked => "blocked",
    }
}
