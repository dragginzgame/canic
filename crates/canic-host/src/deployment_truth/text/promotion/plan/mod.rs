use super::super::super::*;
use super::super::{append_hard_failure_items, deployment_execution_preflight_text};
use super::identity::promotion_artifact_identity_report_text;
use super::readiness::promotion_readiness_text;
use super::shared::append_promotion_transform_role_items;

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
        format!(
            "backend: {}",
            lineage.execution_preflight.backend.variant_label()
        ),
        format!(
            "preflight_status: {}",
            lineage.execution_preflight.status.variant_label()
        ),
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
        format!("status: {}", plan.status.variant_label()),
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
