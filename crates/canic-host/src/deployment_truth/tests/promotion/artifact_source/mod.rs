use super::super::*;

#[test]
fn role_artifact_source_requires_digest_pins_for_executable_overrides() {
    let mut source = sample_role_artifact_source(RoleArtifactSourceKindV1::LocalWasm);
    source.expected_wasm_sha256 = None;
    source.expected_wasm_gz_sha256 = None;

    let err = validate_role_artifact_source(&source).expect_err("digest pin should be required");
    std::assert_matches!(
        err,
        PromotionArtifactSourceError::MissingDigestPin {
            kind: RoleArtifactSourceKindV1::LocalWasm
        }
    );
}

#[test]
fn role_artifact_source_rejects_invalid_digest_shape() {
    let mut source = sample_role_artifact_source(RoleArtifactSourceKindV1::LocalWasmGz);
    source.expected_wasm_gz_sha256 = Some("NOT-A-DIGEST".to_string());

    let err = validate_role_artifact_source(&source).expect_err("digest should be checked");
    std::assert_matches!(
        err,
        PromotionArtifactSourceError::InvalidSha256Digest {
            field: "expected_wasm_gz_sha256"
        }
    );
}

#[test]
fn previous_receipt_artifact_source_requires_eligible_receipt_kind() {
    let mut source = sample_role_artifact_source(RoleArtifactSourceKindV1::PreviousReceiptArtifact);
    source.previous_receipt_kind = None;

    let err = validate_role_artifact_source(&source).expect_err("receipt kind should be required");
    std::assert_matches!(
        err,
        PromotionArtifactSourceError::MissingPreviousReceiptKind
    );

    source.previous_receipt_kind = Some(PreviousArtifactReceiptKindV1::DeploymentReceipt);
    validate_role_artifact_source(&source).expect("deployment receipt artifact should be eligible");
    source.previous_receipt_kind = Some(PreviousArtifactReceiptKindV1::StagingReceipt);
    validate_role_artifact_source(&source).expect("staging receipt artifact should be eligible");
}

#[test]
fn previous_receipt_artifact_source_requires_lineage_digest() {
    let mut source = sample_role_artifact_source(RoleArtifactSourceKindV1::PreviousReceiptArtifact);
    source.previous_receipt_lineage_digest = None;

    let err = validate_role_artifact_source(&source)
        .expect_err("receipt lineage digest should be required");

    std::assert_matches!(
        err,
        PromotionArtifactSourceError::MissingPreviousReceiptLineageDigest
    );
}

#[test]
fn previous_receipt_artifact_source_rejects_invalid_lineage_digest() {
    let mut source = sample_role_artifact_source(RoleArtifactSourceKindV1::PreviousReceiptArtifact);
    source.previous_receipt_lineage_digest = Some("bad-digest".to_string());

    let err =
        validate_role_artifact_source(&source).expect_err("receipt lineage digest should validate");

    std::assert_matches!(
        err,
        PromotionArtifactSourceError::InvalidSha256Digest {
            field: "previous_receipt_lineage_digest"
        }
    );
}

#[test]
fn non_receipt_artifact_source_rejects_previous_receipt_kind() {
    let mut source = sample_role_artifact_source(RoleArtifactSourceKindV1::LocalWasmGz);
    source.previous_receipt_kind = Some(PreviousArtifactReceiptKindV1::DeploymentReceipt);

    let err =
        validate_role_artifact_source(&source).expect_err("receipt kind should be source-specific");
    std::assert_matches!(
        err,
        PromotionArtifactSourceError::UnexpectedPreviousReceiptKind {
            kind: RoleArtifactSourceKindV1::LocalWasmGz
        }
    );
}

#[test]
fn non_receipt_artifact_source_rejects_previous_receipt_lineage_digest() {
    let mut source = sample_role_artifact_source(RoleArtifactSourceKindV1::LocalWasmGz);
    source.previous_receipt_lineage_digest = Some(sample_sha256("9"));

    let err = validate_role_artifact_source(&source)
        .expect_err("receipt lineage digest should be source-specific");

    std::assert_matches!(
        err,
        PromotionArtifactSourceError::UnexpectedPreviousReceiptLineageDigest {
            kind: RoleArtifactSourceKindV1::LocalWasmGz
        }
    );
}

#[test]
fn canonical_wasm_store_default_source_does_not_require_locator_or_digest_pin() {
    let source = RoleArtifactSourceV1 {
        role: "wasm_store".to_string(),
        kind: RoleArtifactSourceKindV1::CanonicalWasmStoreDefault,
        locator: None,
        previous_receipt_kind: None,
        previous_receipt_lineage_digest: None,
        expected_wasm_sha256: None,
        expected_wasm_gz_sha256: None,
        expected_candid_sha256: None,
        expected_canonical_embedded_config_sha256: None,
    };

    validate_role_artifact_source(&source).expect("canonical source should be deferred");
}
