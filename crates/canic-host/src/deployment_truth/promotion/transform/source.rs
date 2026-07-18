use super::super::{
    ensure::{
        ensure_digest_requirement, ensure_field, ensure_locator_requirement,
        ensure_optional_sha256, ensure_previous_receipt_lineage_digest_requirement,
        ensure_previous_receipt_requirement,
    },
    error::PromotionArtifactSourceError,
};
use crate::deployment_truth::{
    ArtifactDigestSourceV1, ArtifactSourceV1, PromotionArtifactLevelV1, RoleArtifactSourceKindV1,
    RoleArtifactSourceV1, RoleArtifactV1, RolePromotionInputV1,
};

pub fn validate_role_artifact_source(
    source: &RoleArtifactSourceV1,
) -> Result<(), PromotionArtifactSourceError> {
    ensure_field("role", &source.role)?;
    ensure_locator_requirement(source)?;
    ensure_previous_receipt_requirement(source)?;
    ensure_digest_requirement(source)?;
    ensure_previous_receipt_lineage_digest_requirement(source)?;
    ensure_optional_sha256(
        "expected_wasm_sha256",
        source.expected_wasm_sha256.as_deref(),
    )?;
    ensure_optional_sha256(
        "expected_wasm_gz_sha256",
        source.expected_wasm_gz_sha256.as_deref(),
    )?;
    ensure_optional_sha256(
        "expected_candid_sha256",
        source.expected_candid_sha256.as_deref(),
    )?;
    ensure_optional_sha256(
        "expected_canonical_embedded_config_sha256",
        source.expected_canonical_embedded_config_sha256.as_deref(),
    )?;
    ensure_optional_sha256(
        "previous_receipt_lineage_digest",
        source.previous_receipt_lineage_digest.as_deref(),
    )?;
    Ok(())
}

pub(super) fn apply_promotion_input_to_role_artifact(
    role_artifact: &mut RoleArtifactV1,
    input: &RolePromotionInputV1,
) {
    match input.promotion_level {
        PromotionArtifactLevelV1::SealedWasm => {
            role_artifact.source = artifact_source_for_promotion_source(input.source.kind);
            apply_promotion_source_locator(role_artifact, &input.source);
            role_artifact
                .wasm_sha256
                .clone_from(&input.source.expected_wasm_sha256);
            role_artifact
                .wasm_gz_sha256
                .clone_from(&input.source.expected_wasm_gz_sha256);
            role_artifact
                .candid_sha256
                .clone_from(&input.source.expected_candid_sha256);
            role_artifact
                .canonical_embedded_config_sha256
                .clone_from(&input.source.expected_canonical_embedded_config_sha256);
            normalize_executable_artifact_source(role_artifact, &input.source);
        }
        PromotionArtifactLevelV1::SourceBuild => {}
    }
}

fn normalize_executable_artifact_source(
    role_artifact: &mut RoleArtifactV1,
    source: &RoleArtifactSourceV1,
) {
    role_artifact.wasm_gz_size_bytes = None;
    match source.kind {
        RoleArtifactSourceKindV1::LocalWasm => {
            role_artifact.wasm_gz_path = None;
            role_artifact.wasm_gz_sha256 = None;
            role_artifact.wasm_gz_sha256_source = None;
            role_artifact.observed_wasm_gz_file_sha256 = None;
            role_artifact.observed_wasm_gz_file_sha256_source = None;
        }
        RoleArtifactSourceKindV1::LocalWasmGz => {
            role_artifact.wasm_path = None;
            role_artifact.wasm_gz_sha256_source = Some(ArtifactDigestSourceV1::ObservedFileDigest);
            role_artifact
                .observed_wasm_gz_file_sha256
                .clone_from(&source.expected_wasm_gz_sha256);
            role_artifact.observed_wasm_gz_file_sha256_source =
                Some(ArtifactDigestSourceV1::ObservedFileDigest);
        }
        RoleArtifactSourceKindV1::CanonicalWasmStoreDefault
        | RoleArtifactSourceKindV1::PreviousReceiptArtifact
        | RoleArtifactSourceKindV1::PublishedPackage
        | RoleArtifactSourceKindV1::WorkspacePackage => {
            role_artifact.wasm_gz_sha256_source = None;
            role_artifact
                .observed_wasm_gz_file_sha256
                .clone_from(&source.expected_wasm_gz_sha256);
            role_artifact.observed_wasm_gz_file_sha256_source = None;
        }
    }
}

const fn artifact_source_for_promotion_source(kind: RoleArtifactSourceKindV1) -> ArtifactSourceV1 {
    match kind {
        RoleArtifactSourceKindV1::WorkspacePackage => ArtifactSourceV1::LocalBuild,
        RoleArtifactSourceKindV1::CanonicalWasmStoreDefault => ArtifactSourceV1::WasmStore,
        RoleArtifactSourceKindV1::PublishedPackage
        | RoleArtifactSourceKindV1::LocalWasm
        | RoleArtifactSourceKindV1::LocalWasmGz
        | RoleArtifactSourceKindV1::PreviousReceiptArtifact => ArtifactSourceV1::External,
    }
}

fn apply_promotion_source_locator(
    role_artifact: &mut RoleArtifactV1,
    source: &RoleArtifactSourceV1,
) {
    match source.kind {
        RoleArtifactSourceKindV1::LocalWasm => {
            role_artifact.wasm_path.clone_from(&source.locator);
        }
        RoleArtifactSourceKindV1::LocalWasmGz => {
            role_artifact.wasm_gz_path.clone_from(&source.locator);
        }
        _ => {}
    }
}
