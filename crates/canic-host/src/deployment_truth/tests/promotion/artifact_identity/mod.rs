use super::super::*;

#[test]
fn promotion_artifact_identity_report_round_trips_through_json() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let report = promotion_artifact_identity_report("promotion-identity-1", &[input]);

    assert_json_round_trip(&report);
    let encoded = serde_json::to_value(&report).expect("identity report should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "report_id",
            "artifact_identity_report_digest",
            "status",
            "summary",
            "roles",
            "identity_groups",
            "blockers",
        ],
    );
    assert_object_keys(
        &encoded["summary"],
        &[
            "role_count",
            "identity_group_count",
            "shared_identity_group_count",
            "digest_pinned_role_count",
            "source_build_role_count",
            "deferred_identity_role_count",
        ],
    );
    assert!(
        encoded["artifact_identity_report_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    let role = &encoded["roles"][0];
    assert_object_keys(
        role,
        &[
            "role",
            "promotion_level",
            "source_kind",
            "source_locator",
            "identity_kind",
            "digest_pinned",
            "wasm_sha256",
            "wasm_gz_sha256",
            "candid_sha256",
            "canonical_embedded_config_sha256",
        ],
    );
    let group = &encoded["identity_groups"][0];
    assert_object_keys(
        group,
        &[
            "identity_key",
            "identity_kind",
            "roles",
            "source_kinds",
            "source_locators",
            "digest_pinned",
            "wasm_sha256",
            "wasm_gz_sha256",
            "candid_sha256",
            "canonical_embedded_config_sha256",
        ],
    );
}

#[test]
fn promotion_artifact_identity_report_distinguishes_source_kind_from_identity_kind() {
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.source.kind = RoleArtifactSourceKindV1::LocalWasm;
    input.source.expected_wasm_gz_sha256 = None;

    let report =
        promotion_artifact_identity_report_from_inputs(PromotionArtifactIdentityReportRequest {
            report_id: "promotion-identity-1".to_string(),
            inputs: vec![input],
        })
        .expect("identity report should be produced");

    assert_eq!(report.status, PromotionReadinessStatusV1::Ready);
    assert_eq!(report.roles.len(), 1);
    assert_eq!(
        report.roles[0].source_kind,
        RoleArtifactSourceKindV1::LocalWasm
    );
    assert_eq!(
        report.roles[0].identity_kind,
        PromotionArtifactIdentityKindV1::SealedWasm
    );
    assert!(report.roles[0].digest_pinned);
}

#[test]
fn promotion_artifact_identity_report_groups_roles_by_identity_key() {
    let mut root = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    root.source.kind = RoleArtifactSourceKindV1::LocalWasm;
    root.source.locator = Some("artifacts/root.wasm".to_string());
    root.source.expected_wasm_gz_sha256 = None;

    let mut worker = root.clone();
    worker.role = "worker".to_string();
    worker.source.role = "worker".to_string();
    worker.source.kind = RoleArtifactSourceKindV1::PreviousReceiptArtifact;
    worker.source.locator = Some("receipts/worker.json".to_string());
    worker.source.previous_receipt_kind = Some(PreviousArtifactReceiptKindV1::DeploymentReceipt);

    let report = promotion_artifact_identity_report("promotion-identity-1", &[root, worker]);

    assert_eq!(report.roles.len(), 2);
    assert_eq!(report.identity_groups.len(), 1);
    assert_eq!(report.summary.role_count, 2);
    assert_eq!(report.summary.identity_group_count, 1);
    assert_eq!(report.summary.shared_identity_group_count, 1);
    assert_eq!(report.summary.digest_pinned_role_count, 2);
    let group = &report.identity_groups[0];
    assert_eq!(
        group.identity_kind,
        PromotionArtifactIdentityKindV1::SealedWasm
    );
    assert_eq!(group.roles, vec!["root".to_string(), "worker".to_string()]);
    assert_eq!(
        group.source_kinds,
        vec![
            RoleArtifactSourceKindV1::LocalWasm,
            RoleArtifactSourceKindV1::PreviousReceiptArtifact
        ]
    );
    assert_eq!(
        group.source_locators,
        vec![
            "artifacts/root.wasm".to_string(),
            "receipts/worker.json".to_string()
        ]
    );
    assert_eq!(group.wasm_sha256, Some(sample_sha256("d")));
    assert!(group.identity_key.starts_with("sealed:wasm="));
    validate_promotion_artifact_identity_report(&report)
        .expect("grouped identity report should validate");
}

#[test]
fn promotion_artifact_identity_report_marks_source_build_identity() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);

    let report = promotion_artifact_identity_report("promotion-identity-1", &[input]);

    assert_eq!(report.status, PromotionReadinessStatusV1::Ready);
    assert_eq!(
        report.roles[0].identity_kind,
        PromotionArtifactIdentityKindV1::SourceBuild
    );
    assert_eq!(report.summary.source_build_role_count, 1);
}

#[test]
fn promotion_artifact_identity_report_records_invalid_source_as_blocker() {
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.source.expected_wasm_sha256 = None;
    input.source.expected_wasm_gz_sha256 = None;

    let report = promotion_artifact_identity_report("promotion-identity-1", &[input]);

    assert_eq!(report.status, PromotionReadinessStatusV1::Blocked);
    assert_eq!(report.blockers.len(), 1);
    assert_eq!(report.blockers[0].code, "promotion_artifact_source_invalid");
    assert_eq!(
        report.roles[0].identity_kind,
        PromotionArtifactIdentityKindV1::Deferred
    );
    assert_eq!(report.summary.deferred_identity_role_count, 1);
    validate_promotion_artifact_identity_report(&report)
        .expect("blocked report should still validate");
}

#[test]
fn promotion_artifact_identity_report_text_reports_passive_summary() {
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.source.kind = RoleArtifactSourceKindV1::LocalWasm;
    input.source.expected_wasm_gz_sha256 = None;
    let report = promotion_artifact_identity_report("promotion-identity-1", &[input]);

    let text = promotion_artifact_identity_report_text(&report);

    assert!(text.contains("Promotion artifact identity report"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("status: ready"));
    assert!(text.contains("report_id: promotion-identity-1"));
    assert!(text.contains("artifact_identity_report_digest:"));
    assert!(text.contains("identity_groups: 1"));
    assert!(text.contains("shared_identity_groups: 0"));
    assert!(text.contains("digest_pinned_roles: 1"));
    assert!(text.contains("source_build_roles: 0"));
    assert!(text.contains("deferred_identity_roles: 0"));
    assert!(text.contains("identity groups:"));
    assert!(
        text.contains("root SealedWasm/LocalWasm: identity_kind=SealedWasm digest_pinned=true")
    );
    assert!(text.contains("source_locator: artifacts/root.wasm.gz"));
    assert!(text.contains("wasm_gz_sha256: not recorded"));
}

#[test]
fn promotion_artifact_identity_report_validation_rejects_stale_summary() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let mut report = promotion_artifact_identity_report("promotion-identity-1", &[input]);
    report.summary.identity_group_count = 2;

    let err = validate_promotion_artifact_identity_report(&report)
        .expect_err("stale summary should fail");

    std::assert_matches!(
        err,
        PromotionArtifactIdentityReportError::SummaryMismatch {
            field: "identity_group_count"
        }
    );
}

#[test]
fn promotion_artifact_identity_report_validation_rejects_stale_digest() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let mut report = promotion_artifact_identity_report("promotion-identity-1", &[input]);
    report.artifact_identity_report_digest = sample_sha256("9");

    let err = validate_promotion_artifact_identity_report(&report)
        .expect_err("stale identity report digest should fail");

    std::assert_matches!(
        err,
        PromotionArtifactIdentityReportError::LinkageMismatch {
            field: "artifact_identity_report_digest"
        }
    );
}

#[test]
fn promotion_artifact_identity_report_validation_rejects_duplicate_roles() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let mut report = promotion_artifact_identity_report("promotion-identity-1", &[input]);
    report.roles.push(report.roles[0].clone());

    let err = validate_promotion_artifact_identity_report(&report)
        .expect_err("duplicate role should fail");
    std::assert_matches!(
        err,
        PromotionArtifactIdentityReportError::DuplicateRole { role } if role == "root"
    );
}

#[test]
fn promotion_artifact_identity_report_validation_rejects_status_blocker_mismatch() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let mut report = promotion_artifact_identity_report("promotion-identity-1", &[input]);
    report.status = PromotionReadinessStatusV1::Blocked;

    let err = validate_promotion_artifact_identity_report(&report)
        .expect_err("status blocker mismatch should fail");
    std::assert_matches!(
        err,
        PromotionArtifactIdentityReportError::StatusBlockerMismatch { .. }
    );
}

#[test]
fn promotion_artifact_identity_report_validation_rejects_bad_digest_shape() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let mut report = promotion_artifact_identity_report("promotion-identity-1", &[input]);
    report.roles[0].wasm_gz_sha256 = Some("NOT-A-DIGEST".to_string());

    let err =
        validate_promotion_artifact_identity_report(&report).expect_err("bad digest should fail");
    std::assert_matches!(
        err,
        PromotionArtifactIdentityReportError::InvalidSha256Digest {
            field: "wasm_gz_sha256"
        }
    );
}

#[test]
fn promotion_artifact_identity_report_validation_rejects_stale_group_key() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let mut report = promotion_artifact_identity_report("promotion-identity-1", &[input]);
    report.identity_groups[0].identity_key = "sealed:stale".to_string();

    let err = validate_promotion_artifact_identity_report(&report)
        .expect_err("stale group key should fail");
    std::assert_matches!(
        err,
        PromotionArtifactIdentityReportError::IdentityGroupKeyMismatch { .. }
    );
}

#[test]
fn promotion_artifact_identity_report_validation_rejects_ungrouped_role() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let mut report = promotion_artifact_identity_report("promotion-identity-1", &[input]);
    report.identity_groups.clear();

    let err = validate_promotion_artifact_identity_report(&report)
        .expect_err("ungrouped role should fail");
    std::assert_matches!(
        err,
        PromotionArtifactIdentityReportError::MissingGroupedRole { role } if role == "root"
    );
}
