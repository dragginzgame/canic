use super::super::super::*;
use super::super::optional_bool_label;

pub(super) fn append_promotion_role_items(
    lines: &mut Vec<String>,
    roles: &[RolePromotionReadinessV1],
) {
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

pub(super) fn append_promotion_artifact_identity_role_items(
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

pub(super) fn append_promotion_artifact_identity_group_items(
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

pub(super) fn append_promotion_policy_decision_items(
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

pub(super) fn append_promotion_transform_role_items(
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
