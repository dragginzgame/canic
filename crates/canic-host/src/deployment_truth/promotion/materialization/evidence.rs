use crate::deployment_truth::{
    BuildMaterializationEvidenceV1, BuildMaterializationInputV1, BuildMaterializationResultV1,
    BuildRecipeIdentityV1, DEPLOYMENT_TRUTH_SCHEMA_VERSION,
};

use super::super::digest::{
    build_materialization_evidence_digest, build_materialization_input_digest,
};
use super::super::ensure::{
    ensure_materialization_field, ensure_materialization_link, ensure_materialization_sha256,
};
use super::super::error::PromotionMaterializationIdentityError;
use super::super::request::BuildMaterializationEvidenceRequest;

pub fn build_materialization_evidence(
    request: BuildMaterializationEvidenceRequest,
) -> Result<BuildMaterializationEvidenceV1, PromotionMaterializationIdentityError> {
    ensure_materialization_field("evidence_id", &request.evidence_id)?;
    validate_build_recipe_identity(&request.recipe)?;
    validate_build_materialization_input(&request.materialization_input)?;
    validate_build_materialization_result(&request.materialization_result)?;
    let computed_materialization_input_digest =
        build_materialization_input_digest(&request.materialization_input);
    let mut evidence = BuildMaterializationEvidenceV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        evidence_id: request.evidence_id,
        materialization_evidence_digest: String::new(),
        recipe_id_matches_input: request.recipe.recipe_id
            == request.materialization_input.build_recipe_id,
        recipe_id_matches_result: request.recipe.recipe_id
            == request.materialization_result.build_recipe_id,
        materialization_input_digest_matches_result: computed_materialization_input_digest
            == request.materialization_result.materialization_input_digest,
        computed_materialization_input_digest,
        recipe: request.recipe,
        materialization_input: request.materialization_input,
        materialization_result: request.materialization_result,
    };
    evidence.materialization_evidence_digest = build_materialization_evidence_digest(&evidence);
    validate_build_materialization_evidence(&evidence)?;
    Ok(evidence)
}

pub fn validate_build_materialization_evidence(
    evidence: &BuildMaterializationEvidenceV1,
) -> Result<(), PromotionMaterializationIdentityError> {
    if evidence.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            PromotionMaterializationIdentityError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                found: evidence.schema_version,
            },
        );
    }
    ensure_materialization_field("evidence_id", &evidence.evidence_id)?;
    ensure_materialization_sha256(
        "materialization_evidence_digest",
        &evidence.materialization_evidence_digest,
    )?;
    validate_build_recipe_identity(&evidence.recipe)?;
    validate_build_materialization_input(&evidence.materialization_input)?;
    validate_build_materialization_result(&evidence.materialization_result)?;
    ensure_materialization_sha256(
        "computed_materialization_input_digest",
        &evidence.computed_materialization_input_digest,
    )?;
    ensure_materialization_link(
        "recipe_id_matches_input",
        evidence.recipe_id_matches_input
            == (evidence.recipe.recipe_id == evidence.materialization_input.build_recipe_id),
    )?;
    ensure_materialization_link("recipe_id_matches_input", evidence.recipe_id_matches_input)?;
    ensure_materialization_link(
        "recipe_id_matches_result",
        evidence.recipe_id_matches_result
            == (evidence.recipe.recipe_id == evidence.materialization_result.build_recipe_id),
    )?;
    ensure_materialization_link(
        "recipe_id_matches_result",
        evidence.recipe_id_matches_result,
    )?;
    let computed = build_materialization_input_digest(&evidence.materialization_input);
    if computed != evidence.computed_materialization_input_digest {
        return Err(PromotionMaterializationIdentityError::DigestMismatch {
            field: "computed_materialization_input_digest",
            expected: computed,
            found: evidence.computed_materialization_input_digest.clone(),
        });
    }
    ensure_materialization_link(
        "materialization_input_digest_matches_result",
        evidence.materialization_input_digest_matches_result
            == (evidence.computed_materialization_input_digest
                == evidence.materialization_result.materialization_input_digest),
    )?;
    ensure_materialization_link(
        "materialization_input_digest_matches_result",
        evidence.materialization_input_digest_matches_result,
    )?;
    if evidence.materialization_evidence_digest != build_materialization_evidence_digest(evidence) {
        return Err(PromotionMaterializationIdentityError::LinkageMismatch {
            field: "materialization_evidence_digest",
        });
    }
    Ok(())
}

pub fn validate_build_recipe_identity(
    recipe: &BuildRecipeIdentityV1,
) -> Result<(), PromotionMaterializationIdentityError> {
    ensure_materialization_field("recipe_id", &recipe.recipe_id)?;
    ensure_materialization_field("source_revision", &recipe.source_revision)?;
    ensure_materialization_field("package_or_role_selector", &recipe.package_or_role_selector)?;
    ensure_materialization_field("cargo_profile", &recipe.cargo_profile)?;
    ensure_materialization_sha256("cargo_features_digest", &recipe.cargo_features_digest)?;
    ensure_materialization_sha256("cargo_lock_digest", &recipe.cargo_lock_digest)?;
    ensure_materialization_field("rust_toolchain", &recipe.rust_toolchain)?;
    ensure_materialization_field("builder_version", &recipe.builder_version)?;
    ensure_materialization_field("target_triple", &recipe.target_triple)?;
    ensure_materialization_field("linker_identity", &recipe.linker_identity)?;
    ensure_materialization_field("deterministic_build_mode", &recipe.deterministic_build_mode)?;
    ensure_materialization_field("wasm_opt_version", &recipe.wasm_opt_version)?;
    ensure_materialization_field("compression_identity", &recipe.compression_identity)?;
    Ok(())
}

pub fn validate_build_materialization_input(
    input: &BuildMaterializationInputV1,
) -> Result<(), PromotionMaterializationIdentityError> {
    ensure_materialization_field("materialization_input_id", &input.materialization_input_id)?;
    ensure_materialization_field("build_recipe_id", &input.build_recipe_id)?;
    ensure_materialization_sha256(
        "canonical_embedded_config_sha256",
        &input.canonical_embedded_config_sha256,
    )?;
    ensure_materialization_field("network", &input.network)?;
    ensure_materialization_field("root_trust_anchor", &input.root_trust_anchor)?;
    ensure_materialization_field("runtime_variant", &input.runtime_variant)?;
    Ok(())
}

pub fn validate_build_materialization_result(
    result: &BuildMaterializationResultV1,
) -> Result<(), PromotionMaterializationIdentityError> {
    ensure_materialization_field(
        "materialization_result_id",
        &result.materialization_result_id,
    )?;
    ensure_materialization_field("build_recipe_id", &result.build_recipe_id)?;
    ensure_materialization_sha256(
        "materialization_input_digest",
        &result.materialization_input_digest,
    )?;
    ensure_materialization_sha256("wasm_sha256", &result.wasm_sha256)?;
    ensure_materialization_sha256("wasm_gz_sha256", &result.wasm_gz_sha256)?;
    ensure_materialization_sha256("installed_module_hash", &result.installed_module_hash)?;
    ensure_materialization_sha256("candid_sha256", &result.candid_sha256)?;
    Ok(())
}
