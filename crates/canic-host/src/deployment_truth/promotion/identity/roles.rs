use crate::deployment_truth::{
    BuildMaterializationEvidenceV1, PromotionArtifactIdentityKindV1, PromotionArtifactLevelV1,
    RoleArtifactSourceV1, RoleArtifactV1, RolePromotionArtifactIdentityV1, RolePromotionInputV1,
    RolePromotionMaterializationIdentityV1, RolePromotionPlanTransformV1,
};

pub(in crate::deployment_truth::promotion) fn artifact_identity_changed(
    before: &RoleArtifactV1,
    after: &RoleArtifactV1,
) -> bool {
    before.source != after.source
        || before.wasm_path != after.wasm_path
        || before.wasm_gz_path != after.wasm_gz_path
        || before.wasm_sha256 != after.wasm_sha256
        || before.wasm_gz_sha256 != after.wasm_gz_sha256
        || before.candid_path != after.candid_path
        || before.candid_sha256 != after.candid_sha256
}

pub(in crate::deployment_truth::promotion) fn role_materialization_identity_matches(
    before: &RoleArtifactV1,
    after: &RoleArtifactV1,
) -> bool {
    before.source == after.source
        && before.wasm_path == after.wasm_path
        && before.wasm_gz_path == after.wasm_gz_path
        && before.wasm_sha256 == after.wasm_sha256
        && before.wasm_gz_sha256 == after.wasm_gz_sha256
        && before.candid_path == after.candid_path
        && before.candid_sha256 == after.candid_sha256
        && before.canonical_embedded_config_sha256 == after.canonical_embedded_config_sha256
}

pub(super) fn role_promotion_artifact_identity(
    input: &RolePromotionInputV1,
) -> RolePromotionArtifactIdentityV1 {
    let wasm_sha256 = input.source.expected_wasm_sha256.clone();
    let wasm_gz_sha256 = input.source.expected_wasm_gz_sha256.clone();
    RolePromotionArtifactIdentityV1 {
        role: input.role.clone(),
        promotion_level: input.promotion_level,
        source_kind: input.source.kind,
        source_locator: input.source.locator.clone(),
        identity_kind: promotion_artifact_identity_kind(input.promotion_level, &input.source),
        digest_pinned: wasm_sha256.is_some() || wasm_gz_sha256.is_some(),
        wasm_sha256,
        wasm_gz_sha256,
        candid_sha256: input.source.expected_candid_sha256.clone(),
        canonical_embedded_config_sha256: input
            .source
            .expected_canonical_embedded_config_sha256
            .clone(),
    }
}

pub(in crate::deployment_truth::promotion) fn role_materialization_identity_from_evidence(
    evidence: &BuildMaterializationEvidenceV1,
) -> RolePromotionMaterializationIdentityV1 {
    RolePromotionMaterializationIdentityV1 {
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
        canonical_embedded_config_sha256: evidence
            .materialization_input
            .canonical_embedded_config_sha256
            .clone(),
        environment: evidence.materialization_input.environment.clone(),
        root_trust_anchor: evidence.materialization_input.root_trust_anchor.clone(),
        runtime_variant: evidence.materialization_input.runtime_variant.clone(),
        wasm_sha256: evidence.materialization_result.wasm_sha256.clone(),
        wasm_gz_sha256: evidence.materialization_result.wasm_gz_sha256.clone(),
        installed_module_hash: evidence
            .materialization_result
            .installed_module_hash
            .clone(),
        candid_sha256: evidence.materialization_result.candid_sha256.clone(),
    }
}

const fn promotion_artifact_identity_kind(
    promotion_level: PromotionArtifactLevelV1,
    source: &RoleArtifactSourceV1,
) -> PromotionArtifactIdentityKindV1 {
    if matches!(promotion_level, PromotionArtifactLevelV1::SourceBuild) {
        return PromotionArtifactIdentityKindV1::SourceBuild;
    }
    match (
        source.expected_wasm_sha256.is_some(),
        source.expected_wasm_gz_sha256.is_some(),
    ) {
        (true, true) => PromotionArtifactIdentityKindV1::SealedWasmAndCompressedWasm,
        (true, false) => PromotionArtifactIdentityKindV1::SealedWasm,
        (false, true) => PromotionArtifactIdentityKindV1::SealedCompressedWasm,
        (false, false) => PromotionArtifactIdentityKindV1::Deferred,
    }
}

pub(in crate::deployment_truth::promotion) fn role_summary_artifact_identity_changed(
    role: &RolePromotionPlanTransformV1,
) -> bool {
    role.artifact_source_before != role.artifact_source_after
        || role.wasm_sha256_before != role.wasm_sha256_after
        || role.wasm_gz_sha256_before != role.wasm_gz_sha256_after
        || role.candid_sha256_before != role.candid_sha256_after
}
