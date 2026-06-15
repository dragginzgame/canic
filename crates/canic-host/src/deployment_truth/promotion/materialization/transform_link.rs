use crate::deployment_truth::{
    BuildMaterializationEvidenceV1, PromotionArtifactLevelV1, PromotionPlanTransformV1,
    RoleArtifactV1, RolePromotionInputV1, RolePromotionMaterializationLinkV1,
    RolePromotionPlanTransformV1,
};
use std::collections::{BTreeMap, BTreeSet};

use super::super::ensure::{ensure_materialization_sha256, ensure_transform_field};
use super::super::error::PromotionPlanTransformError;
use super::evidence::validate_build_materialization_evidence;

pub(in crate::deployment_truth::promotion) fn attach_source_build_materialization(
    transform: &mut PromotionPlanTransformV1,
    inputs: &[RolePromotionInputV1],
    evidence: &[BuildMaterializationEvidenceV1],
) -> Result<(), PromotionPlanTransformError> {
    let input_roles = inputs
        .iter()
        .map(|input| input.role.as_str())
        .collect::<BTreeSet<_>>();
    let mut links = BTreeMap::new();
    for item in evidence {
        validate_build_materialization_evidence(item)?;
        let role = item.recipe.package_or_role_selector.as_str();
        if !input_roles.contains(role) {
            return Err(PromotionPlanTransformError::UnexpectedMaterializationRole {
                role: role.to_string(),
            });
        }
        if links
            .insert(role.to_string(), materialization_link_from_evidence(item))
            .is_some()
        {
            return Err(PromotionPlanTransformError::DuplicateMaterializationRole {
                role: role.to_string(),
            });
        }
    }

    for role in &mut transform.roles {
        match role.promotion_level {
            PromotionArtifactLevelV1::SourceBuild => {
                let Some(link) = links.remove(&role.role) else {
                    return Err(PromotionPlanTransformError::MaterializationRoleMissing {
                        role: role.role.clone(),
                    });
                };
                role.source_build_materialization = Some(link);
            }
            PromotionArtifactLevelV1::SealedWasm => {
                if links.remove(&role.role).is_some() {
                    return Err(PromotionPlanTransformError::UnexpectedMaterializationRole {
                        role: role.role.clone(),
                    });
                }
            }
        }
    }

    if let Some(role) = links.keys().next() {
        return Err(PromotionPlanTransformError::UnexpectedMaterializationRole {
            role: role.clone(),
        });
    }
    Ok(())
}

fn materialization_link_from_evidence(
    evidence: &BuildMaterializationEvidenceV1,
) -> RolePromotionMaterializationLinkV1 {
    RolePromotionMaterializationLinkV1 {
        role: evidence.recipe.package_or_role_selector.clone(),
        evidence_id: evidence.evidence_id.clone(),
        materialization_evidence_digest: evidence.materialization_evidence_digest.clone(),
        recipe_id: evidence.recipe.recipe_id.clone(),
        materialization_input_id: evidence
            .materialization_input
            .materialization_input_id
            .clone(),
        materialization_result_id: evidence
            .materialization_result
            .materialization_result_id
            .clone(),
        materialization_input_digest: evidence.computed_materialization_input_digest.clone(),
        wasm_sha256: evidence.materialization_result.wasm_sha256.clone(),
        wasm_gz_sha256: evidence.materialization_result.wasm_gz_sha256.clone(),
        installed_module_hash: evidence
            .materialization_result
            .installed_module_hash
            .clone(),
        candid_sha256: evidence.materialization_result.candid_sha256.clone(),
    }
}

pub(in crate::deployment_truth::promotion) fn validate_role_materialization_link(
    role: &RolePromotionPlanTransformV1,
    promoted_role: &RoleArtifactV1,
) -> Result<(), PromotionPlanTransformError> {
    let Some(link) = &role.source_build_materialization else {
        return Ok(());
    };
    super::super::transform::ensure_role_field_matches(
        role,
        "source_build_materialization",
        role.promotion_level == PromotionArtifactLevelV1::SourceBuild,
    )?;
    super::super::transform::ensure_role_field_matches(
        role,
        "source_build_materialization.role",
        link.role == role.role,
    )?;
    ensure_transform_field(
        "source_build_materialization.evidence_id",
        &link.evidence_id,
    )?;
    ensure_materialization_sha256(
        "source_build_materialization.materialization_evidence_digest",
        &link.materialization_evidence_digest,
    )?;
    ensure_transform_field("source_build_materialization.recipe_id", &link.recipe_id)?;
    ensure_transform_field(
        "source_build_materialization.materialization_input_id",
        &link.materialization_input_id,
    )?;
    ensure_transform_field(
        "source_build_materialization.materialization_result_id",
        &link.materialization_result_id,
    )?;
    ensure_materialization_sha256(
        "source_build_materialization.materialization_input_digest",
        &link.materialization_input_digest,
    )?;
    ensure_materialization_sha256(
        "source_build_materialization.wasm_sha256",
        &link.wasm_sha256,
    )?;
    ensure_materialization_sha256(
        "source_build_materialization.wasm_gz_sha256",
        &link.wasm_gz_sha256,
    )?;
    ensure_materialization_sha256(
        "source_build_materialization.installed_module_hash",
        &link.installed_module_hash,
    )?;
    ensure_materialization_sha256(
        "source_build_materialization.candid_sha256",
        &link.candid_sha256,
    )?;
    super::super::transform::ensure_role_field_matches(
        role,
        "source_build_materialization.wasm_sha256",
        promoted_role.wasm_sha256.as_deref() == Some(link.wasm_sha256.as_str()),
    )?;
    super::super::transform::ensure_role_field_matches(
        role,
        "source_build_materialization.wasm_gz_sha256",
        promoted_role.wasm_gz_sha256.as_deref() == Some(link.wasm_gz_sha256.as_str()),
    )?;
    super::super::transform::ensure_role_field_matches(
        role,
        "source_build_materialization.installed_module_hash",
        promoted_role.installed_module_hash.as_deref() == Some(link.installed_module_hash.as_str()),
    )?;
    super::super::transform::ensure_role_field_matches(
        role,
        "source_build_materialization.candid_sha256",
        promoted_role.candid_sha256.as_deref() == Some(link.candid_sha256.as_str()),
    )
}
