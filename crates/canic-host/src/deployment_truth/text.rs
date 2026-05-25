use super::*;

/// Render an execution preflight as operator text.
#[must_use]
pub fn deployment_execution_preflight_text(preflight: &DeploymentExecutionPreflightV1) -> String {
    let mut lines = vec![
        "Deployment execution preflight".to_string(),
        "mode: passive".to_string(),
        format!(
            "status: {}",
            deployment_execution_preflight_status_label(preflight.status)
        ),
        format!("plan_id: {}", preflight.plan_id),
        format!("safety_report_id: {}", preflight.safety_report_id),
        format!("authority_plan_id: {}", preflight.authority_plan_id),
        format!("backend: {:?}", preflight.backend),
        String::new(),
        "counts:".to_string(),
        format!("  planned_phases: {}", preflight.planned_phases.len()),
        format!(
            "  required_capabilities: {}",
            preflight.required_capabilities.len()
        ),
        format!(
            "  missing_capabilities: {}",
            preflight.missing_capabilities.len()
        ),
        format!("  blockers: {}", preflight.blockers.len()),
    ];

    append_string_items(&mut lines, "planned_phases", &preflight.planned_phases);
    append_capability_items(
        &mut lines,
        "required_capabilities",
        &preflight.required_capabilities,
    );
    append_capability_items(
        &mut lines,
        "missing_capabilities",
        &preflight.missing_capabilities,
    );
    append_hard_failure_items(&mut lines, "blockers", &preflight.blockers);
    lines.join("\n")
}

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
        format!("promoted_plan_id: {}", report.promoted_plan_id),
        format!(
            "promotion_plan_lineage_digest: {}",
            report.promotion_plan_lineage_digest
        ),
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
            "  materialization_identity: {}",
            report
                .materialization_identity_report_id
                .as_deref()
                .unwrap_or("none")
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
                "  {} {:?}/{:?}: materialization={} wasm_store={}",
                role.role,
                role.promotion_level,
                role.source_kind,
                role.materialization_evidence_id
                    .as_deref()
                    .unwrap_or("none"),
                role.wasm_store_locator.as_deref().unwrap_or("none")
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
            "artifact_promotion_plan_id: {}",
            receipt.artifact_promotion_plan_id
        ),
        format!("provenance_report_id: {}", receipt.provenance_report_id),
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
                "  {} {:?}: result={} artifact={} observed_module={}",
                role.role,
                role.promotion_level,
                role.role_phase_result
                    .map_or_else(|| "none".to_string(), |result| format!("{result:?}")),
                role.artifact_digest.as_deref().unwrap_or("none"),
                role.observed_module_hash_after.as_deref().unwrap_or("none")
            ));
        }
    }
    lines.join("\n")
}

/// Render an authority reconciliation plan as read-only operator text.
#[must_use]
pub fn authority_plan_text(plan: &AuthorityReconciliationPlanV1) -> String {
    let state_counts = authority_plan_state_counts(plan);
    let mut lines = vec![
        "Authority reconciliation plan".to_string(),
        "mode: dry_run".to_string(),
        format!("plan_id: {}", plan.plan_id),
        format!("inventory_id: {}", plan.inventory_id),
        format!(
            "authority_profile_hash: {}",
            plan.authority_profile_hash
                .as_deref()
                .unwrap_or("not recorded")
        ),
        String::new(),
        "counts:".to_string(),
        format!("  canister_actions: {}", plan.canister_actions.len()),
        format!("  automatic_actions: {}", plan.automatic_actions.len()),
        format!(
            "  external_actions_required: {}",
            plan.external_actions_required.len()
        ),
        format!("  hard_failures: {}", plan.hard_failures.len()),
        String::new(),
        "states:".to_string(),
        format!("  already_correct: {}", state_counts.already_correct),
        format!(
            "  can_apply_automatically: {}",
            state_counts.can_apply_automatically
        ),
        format!(
            "  requires_external_action: {}",
            state_counts.requires_external_action
        ),
        format!("  unsafe_blocked: {}", state_counts.unsafe_blocked),
        format!("  unknown: {}", state_counts.unknown),
    ];

    append_plan_canister_actions(&mut lines, plan);
    append_plan_action_preview(&mut lines, plan);
    lines.join("\n")
}

/// Render an authority report as read-only operator text.
#[must_use]
pub fn authority_report_text(report: &AuthorityReportV1) -> String {
    let mut lines = vec![
        "Authority reconciliation report".to_string(),
        "mode: dry_run".to_string(),
        format!("status: {}", safety_status_label(report.status)),
        format!("summary: {}", report.summary),
        format!("report_id: {}", report.report_id),
        format!(
            "check_id: {}",
            report.check_id.as_deref().unwrap_or("not recorded")
        ),
        format!("plan_id: {}", report.reconciliation_plan_id),
        format!("inventory_id: {}", report.inventory_id),
        format!(
            "authority_profile_hash: {}",
            report
                .authority_profile_hash
                .as_deref()
                .unwrap_or("not recorded")
        ),
        String::new(),
        "counts:".to_string(),
        format!("  already_correct: {}", report.counts.already_correct),
        format!(
            "  can_apply_automatically: {}",
            report.counts.can_apply_automatically
        ),
        format!(
            "  requires_external_action: {}",
            report.counts.requires_external_action
        ),
        format!("  unsafe_blocked: {}", report.counts.unsafe_blocked),
        format!("  unknown: {}", report.counts.unknown),
        format!("  hard_failures: {}", report.counts.hard_failures),
        String::new(),
        "apply_readiness:".to_string(),
        format!(
            "  can_apply_automatically: {}",
            report.apply_readiness.can_apply_automatically
        ),
        format!(
            "  automatic_action_count: {}",
            report.apply_readiness.automatic_action_count
        ),
    ];

    append_blockers(&mut lines, report);
    append_next_actions(&mut lines, report);
    append_hard_failure_items(&mut lines, "hard_failures", &report.hard_failures);
    append_observation_gap_items(&mut lines, "observation_gaps", &report.observation_gaps);
    append_authority_action_summary(&mut lines, report);
    lines.join("\n")
}

/// Render a complete authority evidence bundle as read-only operator text.
#[must_use]
pub fn authority_evidence_text(evidence: &AuthorityDryRunEvidenceV1) -> String {
    let mut lines = vec![
        "Authority dry-run evidence".to_string(),
        "mode: dry_run".to_string(),
        format!("evidence_id: {}", evidence.evidence_id),
        format!("check_id: {}", evidence.check_id),
        format!("generated_at: {}", evidence.generated_at),
        format!("plan_id: {}", evidence.reconciliation_plan.plan_id),
        format!("report_id: {}", evidence.authority_report.report_id),
        format!("receipt_id: {}", evidence.authority_receipt.operation_id),
        format!(
            "inventory_id: {}",
            evidence.reconciliation_plan.inventory_id
        ),
        format!(
            "authority_profile_hash: {}",
            evidence
                .reconciliation_plan
                .authority_profile_hash
                .as_deref()
                .unwrap_or("not recorded")
        ),
        String::new(),
        "report:".to_string(),
        format!(
            "  status: {}",
            safety_status_label(evidence.authority_report.status)
        ),
        format!("  summary: {}", evidence.authority_report.summary),
        format!(
            "  hard_failures: {}",
            evidence.authority_report.hard_failures.len()
        ),
        format!(
            "  external_actions_required: {}",
            evidence.authority_report.external_actions_required.len()
        ),
        format!(
            "  observation_gaps: {}",
            evidence.authority_report.observation_gaps.len()
        ),
        String::new(),
        "receipt:".to_string(),
        format!(
            "  status: {}",
            deployment_execution_status_label(evidence.authority_receipt.operation_status)
        ),
        format!(
            "  command_result: {}",
            deployment_command_result_label(&evidence.authority_receipt.command_result)
        ),
        format!(
            "  controller_mutation: {}",
            authority_receipt_mutation_label(&evidence.authority_receipt)
        ),
        format!(
            "  attempted_actions: {}",
            evidence.authority_receipt.attempted_actions.len()
        ),
        format!(
            "  verified_controller_observations: {}",
            evidence
                .authority_receipt
                .verified_controller_observations
                .len()
        ),
    ];

    append_controller_observation_items(
        &mut lines,
        "verified_controller_observations",
        &evidence.authority_receipt.verified_controller_observations,
    );
    append_next_actions(&mut lines, &evidence.authority_report);
    append_hard_failure_items(
        &mut lines,
        "hard_failures",
        &evidence.authority_report.hard_failures,
    );
    append_observation_gap_items(
        &mut lines,
        "observation_gaps",
        &evidence.authority_report.observation_gaps,
    );
    append_external_action_items(
        &mut lines,
        "external_actions_required",
        &evidence.authority_report.external_actions_required,
    );
    lines.join("\n")
}

/// Render an authority dry-run receipt as read-only operator text.
#[must_use]
pub fn authority_receipt_text(receipt: &AuthorityReceiptV1) -> String {
    let mut lines = vec![
        "Authority dry-run receipt".to_string(),
        "mode: dry_run".to_string(),
        format!("operation_id: {}", receipt.operation_id),
        format!(
            "status: {}",
            deployment_execution_status_label(receipt.operation_status)
        ),
        format!(
            "command_result: {}",
            deployment_command_result_label(&receipt.command_result)
        ),
        format!(
            "check_id: {}",
            receipt.check_id.as_deref().unwrap_or("not recorded")
        ),
        format!("plan_id: {}", receipt.reconciliation_plan_id),
        format!("report_id: {}", receipt.authority_report_id),
        format!("inventory_id: {}", receipt.inventory_id),
        format!(
            "authority_profile_hash: {}",
            receipt
                .authority_profile_hash
                .as_deref()
                .unwrap_or("not recorded")
        ),
        format!("started_at: {}", receipt.started_at),
        format!(
            "finished_at: {}",
            receipt.finished_at.as_deref().unwrap_or("not recorded")
        ),
        String::new(),
        "dry_run_evidence:".to_string(),
        format!(
            "  controller_mutation: {}",
            authority_receipt_mutation_label(receipt)
        ),
        format!("  attempted_actions: {}", receipt.attempted_actions.len()),
        format!(
            "  verified_controller_observations: {}",
            receipt.verified_controller_observations.len()
        ),
        format!("  hard_failures: {}", receipt.hard_failures.len()),
        format!(
            "  unresolved_observation_gaps: {}",
            receipt.unresolved_observation_gaps.len()
        ),
        format!(
            "  unresolved_external_actions: {}",
            receipt.unresolved_external_actions.len()
        ),
    ];

    append_controller_observation_items(
        &mut lines,
        "verified_controller_observations",
        &receipt.verified_controller_observations,
    );
    append_hard_failure_items(&mut lines, "hard_failures", &receipt.hard_failures);
    append_observation_gap_items(
        &mut lines,
        "unresolved_observation_gaps",
        &receipt.unresolved_observation_gaps,
    );
    append_external_action_items(
        &mut lines,
        "unresolved_external_actions",
        &receipt.unresolved_external_actions,
    );
    lines.join("\n")
}

fn authority_plan_state_counts(plan: &AuthorityReconciliationPlanV1) -> AuthorityPlanStateCounts {
    let mut counts = AuthorityPlanStateCounts::default();
    for action in &plan.canister_actions {
        match action.state {
            AuthorityReconciliationStateV1::AlreadyCorrect => counts.already_correct += 1,
            AuthorityReconciliationStateV1::CanApplyAutomatically => {
                counts.can_apply_automatically += 1;
            }
            AuthorityReconciliationStateV1::RequiresExternalAction => {
                counts.requires_external_action += 1;
            }
            AuthorityReconciliationStateV1::UnsafeBlocked => counts.unsafe_blocked += 1,
            AuthorityReconciliationStateV1::Unknown => counts.unknown += 1,
        }
    }
    counts
}

fn append_plan_canister_actions(lines: &mut Vec<String>, plan: &AuthorityReconciliationPlanV1) {
    if plan.canister_actions.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push("canister_actions:".to_string());
    for action in &plan.canister_actions {
        lines.push(format!(
            "  - {} {:?}/{:?}: {}",
            authority_canister_action_subject(action),
            action.state,
            action.action,
            action.reason
        ));
    }
}

fn authority_canister_action_subject(action: &CanisterAuthorityActionV1) -> String {
    if let Some(role) = &action.role
        && let Some(canister_id) = &action.canister_id
    {
        return format!("{role} ({canister_id})");
    }
    if let Some(role) = &action.role {
        return role.clone();
    }
    action
        .canister_id
        .clone()
        .unwrap_or_else(|| "unknown canister".to_string())
}

fn append_plan_action_preview(lines: &mut Vec<String>, plan: &AuthorityReconciliationPlanV1) {
    if !plan.automatic_actions.is_empty() {
        lines.push(String::new());
        lines.push("automatic_actions:".to_string());
        for action in &plan.automatic_actions {
            lines.push(authority_action_line_with_delta(
                &action.subject,
                action.action,
                &action.reason,
                &action.controller_delta,
            ));
        }
    }
    append_external_action_items(
        lines,
        "external_actions_required",
        &plan.external_actions_required,
    );
    append_hard_failure_items(lines, "hard_failures", &plan.hard_failures);
}

///
/// AuthorityPlanStateCounts
///
#[derive(Default)]
struct AuthorityPlanStateCounts {
    already_correct: usize,
    can_apply_automatically: usize,
    requires_external_action: usize,
    unsafe_blocked: usize,
    unknown: usize,
}

fn append_blockers(lines: &mut Vec<String>, report: &AuthorityReportV1) {
    if report.apply_readiness.blockers.is_empty() {
        lines.push("  blockers: none".to_string());
        return;
    }
    lines.push("  blockers:".to_string());
    for blocker in &report.apply_readiness.blockers {
        lines.push(format!("    - {}", authority_apply_blocker_label(*blocker)));
    }
}

fn append_next_actions(lines: &mut Vec<String>, report: &AuthorityReportV1) {
    if report.next_actions.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push("next_actions:".to_string());
    for action in &report.next_actions {
        lines.push(format!("  - {action}"));
    }
}

fn append_observation_gap_items(
    lines: &mut Vec<String>,
    label: &str,
    gaps: &[DeploymentObservationGapV1],
) {
    if gaps.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push(format!("{label}:"));
    for gap in gaps {
        lines.push(format!("  - {}: {}", gap.key, gap.description));
    }
}

fn append_hard_failure_items(lines: &mut Vec<String>, label: &str, failures: &[SafetyFindingV1]) {
    if failures.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push(format!("{label}:"));
    for failure in failures {
        let subject = failure.subject.as_deref().unwrap_or("unknown subject");
        lines.push(format!(
            "  - [{}] {}: {}",
            failure.code, subject, failure.message
        ));
    }
}

fn append_warning_items(lines: &mut Vec<String>, label: &str, warnings: &[SafetyFindingV1]) {
    if warnings.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push(format!("{label}:"));
    for warning in warnings {
        let subject = warning.subject.as_deref().unwrap_or("unknown subject");
        lines.push(format!(
            "  - [{}] {}: {}",
            warning.code, subject, warning.message
        ));
    }
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

fn append_string_items(lines: &mut Vec<String>, label: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push(format!("{label}:"));
    for value in values {
        lines.push(format!("  - {value}"));
    }
}

fn append_capability_items(
    lines: &mut Vec<String>,
    label: &str,
    capabilities: &[DeploymentExecutorCapabilityV1],
) {
    if capabilities.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push(format!("{label}:"));
    for capability in capabilities {
        lines.push(format!("  - {capability:?}"));
    }
}

fn append_external_action_items(
    lines: &mut Vec<String>,
    label: &str,
    actions: &[AuthorityExternalActionV1],
) {
    if actions.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push(format!("{label}:"));
    for action in actions {
        lines.push(authority_action_line_with_delta(
            &action.subject,
            action.action,
            &action.reason,
            &action.controller_delta,
        ));
    }
}

fn append_controller_observation_items(
    lines: &mut Vec<String>,
    label: &str,
    observations: &[AuthorityControllerObservationV1],
) {
    if observations.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push(format!("{label}:"));
    for observation in observations {
        lines.push(format!(
            "  - {} {:?}/{:?}: observed=[{}] desired=[{}]{}",
            observation.subject,
            observation.state,
            observation.action,
            authority_delta_list(&observation.observed_controllers),
            authority_delta_list(&observation.desired_controllers),
            authority_delta_suffix(&observation.controller_delta)
        ));
    }
}

fn append_authority_action_summary(lines: &mut Vec<String>, report: &AuthorityReportV1) {
    if !report.automatic_actions.is_empty() {
        lines.push(String::new());
        lines.push("automatic_actions:".to_string());
        for action in &report.automatic_actions {
            lines.push(authority_action_line_with_delta(
                &action.subject,
                action.action,
                &action.reason,
                &action.controller_delta,
            ));
        }
    }
    append_external_action_items(
        lines,
        "external_actions_required",
        &report.external_actions_required,
    );
}

fn authority_action_line(subject: &str, action: AuthorityActionV1, reason: &str) -> String {
    format!("  - {subject} {action:?}: {reason}")
}

fn authority_action_line_with_delta(
    subject: &str,
    action: AuthorityActionV1,
    reason: &str,
    delta: &AuthorityControllerDeltaV1,
) -> String {
    format!(
        "{}{}",
        authority_action_line(subject, action, reason),
        authority_delta_suffix(delta)
    )
}

fn authority_delta_suffix(delta: &AuthorityControllerDeltaV1) -> String {
    let add = authority_delta_list(&delta.add_controllers);
    let remove = authority_delta_list(&delta.remove_controllers);
    format!(" [add={add}; remove={remove}]")
}

fn authority_delta_list(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(",")
    }
}

const fn safety_status_label(status: SafetyStatusV1) -> &'static str {
    match status {
        SafetyStatusV1::NotEvaluated => "not_evaluated",
        SafetyStatusV1::Safe => "safe",
        SafetyStatusV1::Warning => "warning",
        SafetyStatusV1::Blocked => "blocked",
    }
}

const fn authority_apply_blocker_label(blocker: AuthorityApplyBlockerV1) -> &'static str {
    match blocker {
        AuthorityApplyBlockerV1::UnsafeBlocked => "unsafe_blocked",
        AuthorityApplyBlockerV1::HardFailures => "hard_failures",
        AuthorityApplyBlockerV1::ObservationGaps => "observation_gaps",
        AuthorityApplyBlockerV1::ExternalActions => "external_actions",
    }
}

const fn deployment_execution_status_label(status: DeploymentExecutionStatusV1) -> &'static str {
    match status {
        DeploymentExecutionStatusV1::NotStarted => "not_started",
        DeploymentExecutionStatusV1::InProgress => "in_progress",
        DeploymentExecutionStatusV1::FailedBeforeMutation => "failed_before_mutation",
        DeploymentExecutionStatusV1::PartiallyApplied => "partially_applied",
        DeploymentExecutionStatusV1::FailedAfterMutation => "failed_after_mutation",
        DeploymentExecutionStatusV1::Complete => "complete",
    }
}

const fn deployment_execution_preflight_status_label(
    status: DeploymentExecutionPreflightStatusV1,
) -> &'static str {
    match status {
        DeploymentExecutionPreflightStatusV1::Ready => "ready",
        DeploymentExecutionPreflightStatusV1::Blocked => "blocked",
    }
}

const fn promotion_readiness_status_label(status: PromotionReadinessStatusV1) -> &'static str {
    match status {
        PromotionReadinessStatusV1::Ready => "ready",
        PromotionReadinessStatusV1::Blocked => "blocked",
    }
}

const fn optional_bool_label(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "unknown",
    }
}

fn deployment_command_result_label(result: &DeploymentCommandResultV1) -> String {
    match result {
        DeploymentCommandResultV1::NotFinished => "not_finished".to_string(),
        DeploymentCommandResultV1::Succeeded => "succeeded".to_string(),
        DeploymentCommandResultV1::Failed { code, message } => {
            format!("failed[{code}]: {message}")
        }
    }
}

const fn authority_receipt_mutation_label(receipt: &AuthorityReceiptV1) -> &'static str {
    if receipt.attempted_actions.is_empty() {
        "none_attempted"
    } else {
        "attempted_actions_present"
    }
}
