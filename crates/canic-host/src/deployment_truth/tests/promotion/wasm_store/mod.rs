use super::super::*;

#[test]
fn promotion_wasm_store_identity_report_round_trips_through_json() {
    let report = promotion_wasm_store_identity_report_from_staging(
        PromotionWasmStoreIdentityReportRequest {
            report_id: "wasm-store-identity-1".to_string(),
            staging_receipts: vec![sample_wasm_store_staging_receipt()],
        },
    )
    .expect("wasm-store identity report should validate");

    assert_json_round_trip(&report);
    let encoded = serde_json::to_value(&report).expect("wasm-store report should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "report_id",
            "wasm_store_identity_report_digest",
            "status",
            "roles",
            "blockers",
        ],
    );
    assert_eq!(encoded["report_id"], "wasm-store-identity-1");
    assert!(
        encoded["wasm_store_identity_report_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    assert_eq!(encoded["status"], "Ready");
    assert_eq!(encoded["roles"][0]["transport"], "WasmStore");
}

#[test]
fn promotion_wasm_store_identity_report_records_staging_locator() {
    let report = promotion_wasm_store_identity_report_from_staging(
        PromotionWasmStoreIdentityReportRequest {
            report_id: "wasm-store-identity-1".to_string(),
            staging_receipts: vec![sample_wasm_store_staging_receipt()],
        },
    )
    .expect("wasm-store identity report should validate");

    validate_promotion_wasm_store_identity_report(&report)
        .expect("generated report should validate");
    assert_eq!(report.roles.len(), 1);
    assert_eq!(report.roles[0].role, "root");
    assert_eq!(
        report.roles[0].wasm_store_locator.as_deref(),
        Some("root:aaaaa-aa:bootstrap")
    );
    assert_eq!(report.roles[0].published_chunk_count, 2);
    assert_eq!(report.status, PromotionReadinessStatusV1::Ready);
    assert_eq!(report.wasm_store_identity_report_digest.len(), 64);
}

#[test]
fn promotion_wasm_store_identity_report_blocks_non_wasm_store_transport() {
    let mut receipt = sample_wasm_store_staging_receipt();
    receipt.transport = ArtifactTransportV1::LocalCli;
    let report = promotion_wasm_store_identity_report_from_staging(
        PromotionWasmStoreIdentityReportRequest {
            report_id: "wasm-store-identity-1".to_string(),
            staging_receipts: vec![receipt],
        },
    )
    .expect("blocked wasm-store identity report should still validate");

    assert_eq!(report.status, PromotionReadinessStatusV1::Blocked);
    assert!(report.blockers.iter().any(|finding| {
        finding.code == "promotion_wasm_store_transport_mismatch"
            && finding.subject.as_deref() == Some("root")
    }));
}

#[test]
fn promotion_wasm_store_identity_report_blocks_missing_locator() {
    let mut receipt = sample_wasm_store_staging_receipt();
    receipt.wasm_store_locator = None;
    let report = promotion_wasm_store_identity_report_from_staging(
        PromotionWasmStoreIdentityReportRequest {
            report_id: "wasm-store-identity-1".to_string(),
            staging_receipts: vec![receipt],
        },
    )
    .expect("blocked wasm-store identity report should still validate");

    assert_eq!(report.status, PromotionReadinessStatusV1::Blocked);
    assert!(report.blockers.iter().any(|finding| {
        finding.code == "promotion_wasm_store_locator_missing"
            && finding.subject.as_deref() == Some("root")
    }));
}

#[test]
fn promotion_wasm_store_identity_report_blocks_unobserved_postcondition() {
    let mut receipt = sample_wasm_store_staging_receipt();
    receipt.verified_postcondition.status = ObservationStatusV1::Missing;
    let report = promotion_wasm_store_identity_report_from_staging(
        PromotionWasmStoreIdentityReportRequest {
            report_id: "wasm-store-identity-1".to_string(),
            staging_receipts: vec![receipt],
        },
    )
    .expect("blocked wasm-store identity report should still validate");

    assert_eq!(report.status, PromotionReadinessStatusV1::Blocked);
    assert!(report.blockers.iter().any(|finding| {
        finding.code == "promotion_wasm_store_postcondition_not_observed"
            && finding.subject.as_deref() == Some("root")
    }));
}

#[test]
fn promotion_wasm_store_identity_report_blocks_chunk_count_mismatch() {
    let mut receipt = sample_wasm_store_staging_receipt();
    receipt.published_chunk_count = 1;
    let report = promotion_wasm_store_identity_report_from_staging(
        PromotionWasmStoreIdentityReportRequest {
            report_id: "wasm-store-identity-1".to_string(),
            staging_receipts: vec![receipt],
        },
    )
    .expect("blocked wasm-store identity report should still validate");

    assert_eq!(report.status, PromotionReadinessStatusV1::Blocked);
    assert!(report.blockers.iter().any(|finding| {
        finding.code == "promotion_wasm_store_chunk_count_mismatch"
            && finding.subject.as_deref() == Some("root")
    }));
}

#[test]
fn promotion_wasm_store_identity_report_validation_rejects_stale_blockers() {
    let mut report = promotion_wasm_store_identity_report_from_staging(
        PromotionWasmStoreIdentityReportRequest {
            report_id: "wasm-store-identity-1".to_string(),
            staging_receipts: vec![sample_wasm_store_staging_receipt()],
        },
    )
    .expect("wasm-store identity report should validate");
    report.blockers.push(SafetyFindingV1 {
        code: "stale".to_string(),
        message: "stale".to_string(),
        severity: SafetySeverityV1::HardFailure,
        subject: Some("root".to_string()),
    });
    report.status = PromotionReadinessStatusV1::Blocked;

    let err = validate_promotion_wasm_store_identity_report(&report)
        .expect_err("stale blockers should fail");

    std::assert_matches!(err, PromotionWasmStoreIdentityReportError::BlockerMismatch);
}

#[test]
fn promotion_wasm_store_identity_report_validation_rejects_stale_digest() {
    let mut report = promotion_wasm_store_identity_report_from_staging(
        PromotionWasmStoreIdentityReportRequest {
            report_id: "wasm-store-identity-1".to_string(),
            staging_receipts: vec![sample_wasm_store_staging_receipt()],
        },
    )
    .expect("wasm-store identity report should validate");
    report.wasm_store_identity_report_digest = sample_sha256("9");

    let err = validate_promotion_wasm_store_identity_report(&report)
        .expect_err("stale wasm-store identity digest should fail");

    std::assert_matches!(
        err,
        PromotionWasmStoreIdentityReportError::LinkageMismatch {
            field: "wasm_store_identity_report_digest"
        }
    );
}

#[test]
fn promotion_wasm_store_identity_report_rejects_staging_schema_drift() {
    let mut receipt = sample_wasm_store_staging_receipt();
    receipt.schema_version += 1;

    let err = promotion_wasm_store_identity_report_from_staging(
        PromotionWasmStoreIdentityReportRequest {
            report_id: "wasm-store-identity-1".to_string(),
            staging_receipts: vec![receipt],
        },
    )
    .expect_err("staging receipt schema drift should fail before projection");

    std::assert_matches!(
        err,
        PromotionWasmStoreIdentityReportError::StagingReceiptSchemaVersionMismatch { .. }
    );
}

#[test]
fn promotion_wasm_store_identity_report_text_reports_passive_summary() {
    let report = promotion_wasm_store_identity_report_from_staging(
        PromotionWasmStoreIdentityReportRequest {
            report_id: "wasm-store-identity-1".to_string(),
            staging_receipts: vec![sample_wasm_store_staging_receipt()],
        },
    )
    .expect("wasm-store identity report should validate");

    let text = promotion_wasm_store_identity_report_text(&report);

    assert!(text.contains("Promotion wasm-store identity report"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("report_id: wasm-store-identity-1"));
    assert!(text.contains("wasm_store_identity_report_digest:"));
    assert!(text.contains("roles: 1"));
    assert!(text.contains(
        "root artifact=embedded:root:0.44.0:abc123 locator=root:aaaaa-aa:bootstrap chunks=2/2 postcondition=Observed"
    ));
}

#[test]
fn promotion_wasm_store_catalog_verification_round_trips_through_json() {
    let verification = sample_wasm_store_catalog_verification();

    assert_json_round_trip(&verification);
    let encoded = serde_json::to_value(&verification).expect("catalog verification should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "verification_id",
            "wasm_store_catalog_verification_digest",
            "wasm_store_identity_report_id",
            "status",
            "roles",
            "blockers",
        ],
    );
    assert_eq!(encoded["verification_id"], "wasm-store-catalog-1");
    assert_eq!(
        encoded["wasm_store_identity_report_id"],
        "wasm-store-identity-1"
    );
    assert!(
        encoded["wasm_store_catalog_verification_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    assert_eq!(encoded["status"], "Ready");
    assert_eq!(encoded["roles"][0]["catalog_matches"], true);
    assert_object_keys(
        &encoded["roles"][0],
        &[
            "role",
            "wasm_store_locator",
            "expected_artifact_identity",
            "observed_artifact_identity",
            "expected_published_chunk_count",
            "observed_published_chunk_count",
            "catalog_entry_present",
            "catalog_matches",
            "catalog_observation_digest",
        ],
    );
}

#[test]
fn promotion_wasm_store_catalog_verification_blocks_missing_entry() {
    let report = sample_wasm_store_identity_report();

    let verification =
        promotion_wasm_store_catalog_verification(PromotionWasmStoreCatalogVerificationRequest {
            verification_id: "wasm-store-catalog-1".to_string(),
            wasm_store_identity_report: report,
            catalog_entries: Vec::new(),
        })
        .expect("missing catalog entry should still produce blocked verification");

    assert_eq!(verification.status, PromotionReadinessStatusV1::Blocked);
    assert!(verification.blockers.iter().any(|finding| {
        finding.code == "promotion_wasm_store_catalog_entry_missing"
            && finding.subject.as_deref() == Some("root")
    }));
}

#[test]
fn promotion_wasm_store_catalog_verification_blocks_artifact_mismatch() {
    let report = sample_wasm_store_identity_report();
    let mut entry = sample_wasm_store_catalog_entry();
    entry.artifact_identity = "embedded:root:0.44.0:other".to_string();

    let verification =
        promotion_wasm_store_catalog_verification(PromotionWasmStoreCatalogVerificationRequest {
            verification_id: "wasm-store-catalog-1".to_string(),
            wasm_store_identity_report: report,
            catalog_entries: vec![entry],
        })
        .expect("catalog artifact mismatch should still produce blocked verification");

    assert_eq!(verification.status, PromotionReadinessStatusV1::Blocked);
    assert!(verification.blockers.iter().any(|finding| {
        finding.code == "promotion_wasm_store_catalog_artifact_mismatch"
            && finding.subject.as_deref() == Some("root")
    }));
}

#[test]
fn promotion_wasm_store_catalog_verification_blocks_chunk_count_mismatch() {
    let report = sample_wasm_store_identity_report();
    let mut entry = sample_wasm_store_catalog_entry();
    entry.published_chunk_count = 1;

    let verification =
        promotion_wasm_store_catalog_verification(PromotionWasmStoreCatalogVerificationRequest {
            verification_id: "wasm-store-catalog-1".to_string(),
            wasm_store_identity_report: report,
            catalog_entries: vec![entry],
        })
        .expect("catalog chunk-count mismatch should still produce blocked verification");

    assert_eq!(verification.status, PromotionReadinessStatusV1::Blocked);
    assert!(verification.blockers.iter().any(|finding| {
        finding.code == "promotion_wasm_store_catalog_chunk_count_mismatch"
            && finding.subject.as_deref() == Some("root")
    }));
}

#[test]
fn promotion_wasm_store_catalog_verification_rejects_duplicate_catalog_locator() {
    let report = sample_wasm_store_identity_report();
    let entry = sample_wasm_store_catalog_entry();

    let err =
        promotion_wasm_store_catalog_verification(PromotionWasmStoreCatalogVerificationRequest {
            verification_id: "wasm-store-catalog-1".to_string(),
            wasm_store_identity_report: report,
            catalog_entries: vec![entry.clone(), entry],
        })
        .expect_err("duplicate catalog locator should fail before verification");

    std::assert_matches!(
        err,
        PromotionWasmStoreCatalogVerificationError::DuplicateLocator { locator }
            if locator == "root:aaaaa-aa:bootstrap"
    );
}

#[test]
fn promotion_wasm_store_catalog_verification_validation_rejects_stale_blockers() {
    let mut verification = sample_wasm_store_catalog_verification();
    verification.blockers.push(SafetyFindingV1 {
        code: "stale".to_string(),
        message: "stale".to_string(),
        severity: SafetySeverityV1::HardFailure,
        subject: Some("root".to_string()),
    });
    verification.status = PromotionReadinessStatusV1::Blocked;

    let err = validate_promotion_wasm_store_catalog_verification(&verification)
        .expect_err("stale catalog blockers should fail");

    std::assert_matches!(
        err,
        PromotionWasmStoreCatalogVerificationError::BlockerMismatch
    );
}

#[test]
fn promotion_wasm_store_catalog_verification_validation_rejects_stale_observation_digest() {
    let mut verification = sample_wasm_store_catalog_verification();
    verification.roles[0].catalog_observation_digest = sample_sha256("9");

    let err = validate_promotion_wasm_store_catalog_verification(&verification)
        .expect_err("stale catalog observation digest should fail");

    std::assert_matches!(
        err,
        PromotionWasmStoreCatalogVerificationError::RoleMismatch {
            role,
            field: "catalog_observation_digest"
        } if role == "root"
    );
}

#[test]
fn promotion_wasm_store_catalog_verification_validation_rejects_stale_digest() {
    let mut verification = sample_wasm_store_catalog_verification();
    verification.wasm_store_catalog_verification_digest = sample_sha256("9");

    let err = validate_promotion_wasm_store_catalog_verification(&verification)
        .expect_err("stale catalog verification digest should fail");

    std::assert_matches!(
        err,
        PromotionWasmStoreCatalogVerificationError::LinkageMismatch {
            field: "wasm_store_catalog_verification_digest"
        }
    );
}

#[test]
fn promotion_wasm_store_catalog_verification_text_reports_passive_summary() {
    let verification = sample_wasm_store_catalog_verification();

    let text = promotion_wasm_store_catalog_verification_text(&verification);

    assert!(text.contains("Promotion wasm-store catalog verification"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("verification_id: wasm-store-catalog-1"));
    assert!(text.contains("wasm_store_catalog_verification_digest:"));
    assert!(text.contains("wasm_store_identity_report_id: wasm-store-identity-1"));
    assert!(text.contains("matching_roles: 1"));
    assert!(text.contains("missing_catalog_entries: 0"));
    assert!(text.contains("root locator=root:aaaaa-aa:bootstrap match=true"));
    assert!(text.contains("digest="));
}
