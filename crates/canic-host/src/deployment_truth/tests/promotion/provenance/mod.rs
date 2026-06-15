use super::super::*;

#[test]
fn artifact_promotion_provenance_report_round_trips_through_json() {
    let report = artifact_promotion_provenance_report(ArtifactPromotionProvenanceReportRequest {
        report_id: "promotion-provenance-1".to_string(),
        artifact_promotion_plan: sample_artifact_promotion_plan(),
        wasm_store_identity_report: Some(sample_wasm_store_identity_report()),
        wasm_store_catalog_verification: Some(sample_wasm_store_catalog_verification()),
        materialization_identity_report: Some(sample_materialization_identity_report()),
    })
    .expect("promotion provenance should validate");

    assert_json_round_trip(&report);
    let encoded = serde_json::to_value(&report).expect("promotion provenance should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "report_id",
            "status",
            "artifact_promotion_plan_id",
            "artifact_promotion_plan_digest",
            "target_plan_id",
            "promoted_plan_id",
            "promotion_plan_lineage_digest",
            "provenance_report_digest",
            "readiness_id",
            "artifact_identity_report_id",
            "transform_id",
            "target_execution_lineage_id",
            "wasm_store_identity_report_id",
            "wasm_store_identity_report_digest",
            "wasm_store_catalog_verification_id",
            "wasm_store_catalog_verification_digest",
            "materialization_identity_report_id",
            "materialization_identity_report_digest",
            "execution_attempted",
            "roles",
            "blockers",
        ],
    );
    assert_eq!(encoded["report_id"], "promotion-provenance-1");
    assert_eq!(encoded["status"], "Ready");
    assert!(
        encoded["artifact_promotion_plan_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    assert!(
        encoded["provenance_report_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    assert!(
        encoded["wasm_store_identity_report_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    assert!(
        encoded["wasm_store_catalog_verification_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    assert!(
        encoded["materialization_identity_report_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    assert_eq!(encoded["execution_attempted"], false);
    assert_eq!(encoded["roles"][0]["role"], "root");
    assert!(encoded["roles"][0]["materialization_evidence_digest"].is_string());
    assert!(encoded["roles"][0]["wasm_store_catalog_observation_digest"].is_string());
}

#[test]
fn artifact_promotion_provenance_report_links_optional_reports() {
    let report = artifact_promotion_provenance_report(ArtifactPromotionProvenanceReportRequest {
        report_id: "promotion-provenance-1".to_string(),
        artifact_promotion_plan: sample_artifact_promotion_plan(),
        wasm_store_identity_report: Some(sample_wasm_store_identity_report()),
        wasm_store_catalog_verification: Some(sample_wasm_store_catalog_verification()),
        materialization_identity_report: Some(sample_materialization_identity_report()),
    })
    .expect("promotion provenance should validate");

    assert_eq!(
        report.wasm_store_identity_report_id.as_deref(),
        Some("wasm-store-identity-1")
    );
    assert!(
        report
            .wasm_store_identity_report_digest
            .as_deref()
            .is_some_and(|digest| digest.len() == 64),
        "provenance should cite the wasm-store identity report digest"
    );
    assert_eq!(
        report.wasm_store_catalog_verification_id.as_deref(),
        Some("wasm-store-catalog-1")
    );
    assert!(
        report
            .wasm_store_catalog_verification_digest
            .as_deref()
            .is_some_and(|digest| digest.len() == 64),
        "provenance should cite the wasm-store catalog verification digest"
    );
    assert_eq!(
        report.materialization_identity_report_id.as_deref(),
        Some("materialization-report-1")
    );
    assert!(
        report
            .materialization_identity_report_digest
            .as_deref()
            .is_some_and(|digest| digest.len() == 64),
        "provenance should cite the materialization report digest"
    );
    assert_eq!(
        report.artifact_promotion_plan_digest.len(),
        64,
        "provenance should cite the plan digest"
    );
    assert_eq!(
        report.roles[0].wasm_store_locator.as_deref(),
        Some("root:aaaaa-aa:bootstrap")
    );
    assert!(
        report.roles[0]
            .wasm_store_catalog_observation_digest
            .as_deref()
            .is_some_and(|digest| digest.len() == 64)
    );
    assert_eq!(
        report.roles[0].materialization_evidence_id.as_deref(),
        Some("materialization-evidence-1")
    );
    assert!(
        report.roles[0]
            .materialization_evidence_digest
            .as_deref()
            .is_some_and(|digest| digest.len() == 64)
    );
}

#[test]
fn artifact_promotion_provenance_report_blocks_unknown_report_roles() {
    let mut receipt = sample_wasm_store_staging_receipt();
    receipt.role = "unknown".to_string();
    let wasm_store_report = promotion_wasm_store_identity_report_from_staging(
        PromotionWasmStoreIdentityReportRequest {
            report_id: "wasm-store-identity-1".to_string(),
            staging_receipts: vec![receipt],
        },
    )
    .expect("wasm-store identity report should validate");

    let report = artifact_promotion_provenance_report(ArtifactPromotionProvenanceReportRequest {
        report_id: "promotion-provenance-1".to_string(),
        artifact_promotion_plan: sample_artifact_promotion_plan(),
        wasm_store_identity_report: Some(wasm_store_report),
        wasm_store_catalog_verification: None,
        materialization_identity_report: None,
    })
    .expect("unknown optional report role should become provenance blocker");

    assert_eq!(report.status, PromotionReadinessStatusV1::Blocked);
    assert!(report.blockers.iter().any(|finding| {
        finding.code == "promotion_provenance_unknown_wasm_store_role"
            && finding.subject.as_deref() == Some("unknown")
    }));
}

#[test]
fn artifact_promotion_provenance_report_blocks_catalog_identity_mismatch() {
    let other_identity = promotion_wasm_store_identity_report_from_staging(
        PromotionWasmStoreIdentityReportRequest {
            report_id: "other-wasm-store-report".to_string(),
            staging_receipts: vec![sample_wasm_store_staging_receipt()],
        },
    )
    .expect("alternate wasm-store identity report should validate");
    let catalog =
        promotion_wasm_store_catalog_verification(PromotionWasmStoreCatalogVerificationRequest {
            verification_id: "wasm-store-catalog-1".to_string(),
            wasm_store_identity_report: other_identity,
            catalog_entries: vec![sample_wasm_store_catalog_entry()],
        })
        .expect("alternate catalog verification should validate");

    let report = artifact_promotion_provenance_report(ArtifactPromotionProvenanceReportRequest {
        report_id: "promotion-provenance-1".to_string(),
        artifact_promotion_plan: sample_artifact_promotion_plan(),
        wasm_store_identity_report: Some(sample_wasm_store_identity_report()),
        wasm_store_catalog_verification: Some(catalog),
        materialization_identity_report: None,
    })
    .expect("mismatched catalog verification should become provenance blocker");

    assert_eq!(report.status, PromotionReadinessStatusV1::Blocked);
    assert!(report.blockers.iter().any(|finding| {
        finding.code == "promotion_provenance_wasm_store_catalog_identity_mismatch"
            && finding.subject.as_deref() == Some("wasm_store_catalog")
    }));
}

#[test]
fn artifact_promotion_provenance_report_blocks_catalog_locator_mismatch() {
    let mut receipt = sample_wasm_store_staging_receipt();
    receipt.wasm_store_locator = Some("root:aaaaa-aa:other".to_string());
    let other_identity = promotion_wasm_store_identity_report_from_staging(
        PromotionWasmStoreIdentityReportRequest {
            report_id: "wasm-store-identity-1".to_string(),
            staging_receipts: vec![receipt],
        },
    )
    .expect("alternate wasm-store identity report should validate");
    let catalog =
        promotion_wasm_store_catalog_verification(PromotionWasmStoreCatalogVerificationRequest {
            verification_id: "wasm-store-catalog-1".to_string(),
            wasm_store_identity_report: other_identity,
            catalog_entries: vec![PromotionWasmStoreCatalogEntryV1 {
                locator: "root:aaaaa-aa:other".to_string(),
                artifact_identity: "embedded:root:0.44.0:abc123".to_string(),
                published_chunk_count: 2,
            }],
        })
        .expect("alternate catalog verification should validate");

    let report = artifact_promotion_provenance_report(ArtifactPromotionProvenanceReportRequest {
        report_id: "promotion-provenance-1".to_string(),
        artifact_promotion_plan: sample_artifact_promotion_plan(),
        wasm_store_identity_report: Some(sample_wasm_store_identity_report()),
        wasm_store_catalog_verification: Some(catalog),
        materialization_identity_report: None,
    })
    .expect("mismatched catalog locator should become provenance blocker");

    assert_eq!(report.status, PromotionReadinessStatusV1::Blocked);
    assert!(report.blockers.iter().any(|finding| {
        finding.code == "promotion_provenance_wasm_store_catalog_locator_mismatch"
            && finding.subject.as_deref() == Some("root")
    }));
}

#[test]
fn artifact_promotion_provenance_report_rejects_stale_digest() {
    let mut report = sample_artifact_promotion_provenance_report();
    report.provenance_report_digest = sample_sha256("9");

    let err = validate_artifact_promotion_provenance_report(&report)
        .expect_err("stale provenance digest should fail");

    std::assert_matches!(
        err,
        ArtifactPromotionProvenanceReportError::LinkageMismatch {
            field: "provenance_report_digest"
        }
    );
}

#[test]
fn artifact_promotion_provenance_report_rejects_stale_plan_digest_link() {
    let mut report = sample_artifact_promotion_provenance_report();
    report.artifact_promotion_plan_digest = sample_sha256("9");

    let err = validate_artifact_promotion_provenance_report(&report)
        .expect_err("stale cited promotion plan digest should fail");

    std::assert_matches!(
        err,
        ArtifactPromotionProvenanceReportError::LinkageMismatch {
            field: "provenance_report_digest"
        }
    );
}

#[test]
fn artifact_promotion_provenance_report_rejects_stale_wasm_store_digest_link() {
    let mut report = sample_artifact_promotion_provenance_report();
    report.wasm_store_identity_report_digest = Some(sample_sha256("9"));

    let err = validate_artifact_promotion_provenance_report(&report)
        .expect_err("stale cited wasm-store identity report digest should fail");

    std::assert_matches!(
        err,
        ArtifactPromotionProvenanceReportError::LinkageMismatch {
            field: "provenance_report_digest"
        }
    );
}

#[test]
fn artifact_promotion_provenance_report_rejects_stale_wasm_store_catalog_digest_link() {
    let mut report = sample_artifact_promotion_provenance_report();
    report.wasm_store_catalog_verification_digest = Some(sample_sha256("9"));

    let err = validate_artifact_promotion_provenance_report(&report)
        .expect_err("stale cited wasm-store catalog verification digest should fail");

    std::assert_matches!(
        err,
        ArtifactPromotionProvenanceReportError::LinkageMismatch {
            field: "provenance_report_digest"
        }
    );
}

#[test]
fn artifact_promotion_provenance_report_rejects_stale_materialization_digest_link() {
    let mut report = sample_artifact_promotion_provenance_report();
    report.materialization_identity_report_digest = Some(sample_sha256("9"));

    let err = validate_artifact_promotion_provenance_report(&report)
        .expect_err("stale cited materialization report digest should fail");

    std::assert_matches!(
        err,
        ArtifactPromotionProvenanceReportError::LinkageMismatch {
            field: "provenance_report_digest"
        }
    );
}

#[test]
fn artifact_promotion_provenance_report_rejects_stale_role_materialization_digest() {
    let mut report = sample_artifact_promotion_provenance_report();
    report.roles[0].materialization_evidence_digest = Some(sample_sha256("9"));

    let err = validate_artifact_promotion_provenance_report(&report)
        .expect_err("stale role materialization evidence digest should fail");

    std::assert_matches!(
        err,
        ArtifactPromotionProvenanceReportError::LinkageMismatch {
            field: "provenance_report_digest"
        }
    );
}

#[test]
fn artifact_promotion_provenance_report_text_reports_passive_summary() {
    let report = artifact_promotion_provenance_report(ArtifactPromotionProvenanceReportRequest {
        report_id: "promotion-provenance-1".to_string(),
        artifact_promotion_plan: sample_artifact_promotion_plan(),
        wasm_store_identity_report: Some(sample_wasm_store_identity_report()),
        wasm_store_catalog_verification: Some(sample_wasm_store_catalog_verification()),
        materialization_identity_report: Some(sample_materialization_identity_report()),
    })
    .expect("promotion provenance should validate");

    let text = artifact_promotion_provenance_report_text(&report);

    assert!(text.contains("Artifact promotion provenance report"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("report_id: promotion-provenance-1"));
    assert!(text.contains("artifact_promotion_plan_id: artifact-promotion-plan-1"));
    assert!(text.contains("artifact_promotion_plan_digest:"));
    assert!(text.contains("provenance_report_digest:"));
    assert!(text.contains("wasm_store_identity: wasm-store-identity-1"));
    assert!(text.contains("wasm_store_identity_digest:"));
    assert!(text.contains("wasm_store_catalog: wasm-store-catalog-1"));
    assert!(text.contains("wasm_store_catalog_digest:"));
    assert!(text.contains("catalog_digest="));
    assert!(text.contains("materialization_identity: materialization-report-1"));
    assert!(text.contains("materialization_identity_digest:"));
    assert!(text.contains("materialization_digest="));
    assert!(text.contains("root SealedWasm/LocalWasmGz"));
}
