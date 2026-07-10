use super::super::*;

#[test]
fn artifact_promotion_execution_receipt_round_trips_through_json() {
    let receipt = sample_artifact_promotion_execution_receipt();

    assert_json_round_trip(&receipt);
    let encoded = serde_json::to_value(&receipt).expect("execution receipt should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "receipt_id",
            "execution_receipt_digest",
            "artifact_promotion_plan_id",
            "artifact_promotion_plan_digest",
            "provenance_report_id",
            "provenance_report_digest",
            "provenance_status",
            "promoted_plan_id",
            "promotion_plan_lineage_digest",
            "operation_id",
            "operation_status",
            "command_result",
            "started_at",
            "finished_at",
            "deployment_receipt",
            "roles",
        ],
    );
    assert_eq!(encoded["receipt_id"], "promotion-execution-receipt-1");
    assert!(
        encoded["execution_receipt_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    assert_eq!(
        encoded["artifact_promotion_plan_id"],
        "artifact-promotion-plan-1"
    );
    assert!(
        encoded["artifact_promotion_plan_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    assert_eq!(encoded["provenance_report_id"], "promotion-provenance-1");
    assert!(
        encoded["provenance_report_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    assert_eq!(encoded["provenance_status"], "Ready");
    assert_eq!(encoded["promoted_plan_id"], "promoted-plan-1");
    assert_eq!(encoded["operation_id"], "promoted-operation-1");
    assert_eq!(encoded["roles"][0]["role"], "root");
    assert!(encoded["roles"][0]["materialization_evidence_digest"].is_string());
    assert!(encoded["roles"][0]["wasm_store_catalog_observation_digest"].is_string());
}

#[test]
fn artifact_promotion_execution_receipt_links_deployment_receipt() {
    let receipt = sample_artifact_promotion_execution_receipt();

    assert_eq!(
        receipt.operation_status,
        DeploymentExecutionStatusV1::Complete
    );
    assert_eq!(receipt.command_result, DeploymentCommandResultV1::Succeeded);
    assert_eq!(receipt.deployment_receipt.plan_id, receipt.promoted_plan_id);
    assert_eq!(
        receipt.deployment_receipt.operation_id,
        receipt.operation_id
    );
    assert_eq!(receipt.artifact_promotion_plan_digest.len(), 64);
    assert_eq!(
        receipt.roles[0].role_phase_result,
        Some(RolePhaseResultV1::Applied)
    );
    assert_eq!(receipt.provenance_report_digest.len(), 64);
    assert_eq!(receipt.execution_receipt_digest.len(), 64);
    assert_eq!(
        receipt.roles[0].artifact_digest.as_deref(),
        Some(sample_sha256("5").as_str())
    );
    assert!(
        receipt.roles[0]
            .materialization_evidence_digest
            .as_deref()
            .is_some_and(|digest| digest.len() == 64)
    );
    assert_eq!(
        receipt.roles[0].observed_module_hash_after.as_deref(),
        Some(sample_sha256("7").as_str())
    );
    assert!(
        receipt.roles[0]
            .wasm_store_catalog_observation_digest
            .as_deref()
            .is_some_and(|digest| digest.len() == 64)
    );
}

#[test]
fn artifact_promotion_execution_receipt_validation_rejects_stale_digest() {
    let mut receipt = sample_artifact_promotion_execution_receipt();
    receipt.execution_receipt_digest = sample_sha256("9");

    let err = validate_artifact_promotion_execution_receipt(&receipt)
        .expect_err("stale execution receipt digest should fail");

    std::assert_matches!(
        err,
        ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "execution_receipt_digest"
        }
    );
}

#[test]
fn artifact_promotion_execution_receipt_validation_rejects_stale_materialization_digest() {
    let mut receipt = sample_artifact_promotion_execution_receipt();
    receipt.roles[0].materialization_evidence_digest = Some(sample_sha256("9"));

    let err = validate_artifact_promotion_execution_receipt(&receipt)
        .expect_err("stale role materialization digest should fail");

    std::assert_matches!(
        err,
        ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "execution_receipt_digest"
        }
    );
}

#[test]
fn artifact_promotion_execution_receipt_validation_rejects_stale_plan_digest_link() {
    let mut receipt = sample_artifact_promotion_execution_receipt();
    receipt.artifact_promotion_plan_digest = sample_sha256("9");

    let err = validate_artifact_promotion_execution_receipt(&receipt)
        .expect_err("stale cited plan digest should fail");

    std::assert_matches!(
        err,
        ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "execution_receipt_digest"
        }
    );
}

#[test]
fn artifact_promotion_execution_receipt_validation_rejects_nested_receipt_drift() {
    let mut receipt = sample_artifact_promotion_execution_receipt();
    receipt.deployment_receipt.phase_receipts[0]
        .verified_postcondition
        .evidence
        .push("stale:evidence".to_string());

    let err = validate_artifact_promotion_execution_receipt(&receipt)
        .expect_err("nested deployment receipt drift should fail");

    std::assert_matches!(
        err,
        ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "execution_receipt_digest"
        }
    );
}

#[test]
fn artifact_promotion_execution_receipt_rejects_other_promoted_plan() {
    let err = artifact_promotion_execution_receipt(ArtifactPromotionExecutionReceiptRequest {
        receipt_id: "promotion-execution-receipt-1".to_string(),
        provenance_report: sample_artifact_promotion_provenance_report(),
        deployment_receipt: sample_receipt_with_phase(
            "other-plan",
            Some("aaaaa-aa"),
            ObservationStatusV1::Observed,
            RolePhaseResultV1::Applied,
        ),
    })
    .expect_err("deployment receipt must match promoted plan");

    std::assert_matches!(
        err,
        ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "deployment_receipt.plan_id"
        }
    );
}

#[test]
fn artifact_promotion_execution_receipt_rejects_blocked_provenance() {
    let mut receipt = sample_wasm_store_staging_receipt();
    receipt.role = "unknown".to_string();
    let wasm_store_report = promotion_wasm_store_identity_report_from_staging(
        PromotionWasmStoreIdentityReportRequest {
            report_id: "wasm-store-identity-1".to_string(),
            staging_receipts: vec![receipt],
        },
    )
    .expect("wasm-store identity report should validate");
    let provenance_report =
        artifact_promotion_provenance_report(ArtifactPromotionProvenanceReportRequest {
            report_id: "promotion-provenance-1".to_string(),
            artifact_promotion_plan: sample_artifact_promotion_plan(),
            wasm_store_identity_report: Some(wasm_store_report),
            wasm_store_catalog_verification: None,
            materialization_identity_report: None,
        })
        .expect("blocked provenance report should still be reportable");

    let err = artifact_promotion_execution_receipt(ArtifactPromotionExecutionReceiptRequest {
        receipt_id: "promotion-execution-receipt-1".to_string(),
        provenance_report,
        deployment_receipt: sample_promoted_deployment_receipt(),
    })
    .expect_err("blocked provenance cannot become execution receipt");

    std::assert_matches!(
        err,
        ArtifactPromotionExecutionReceiptError::ProvenanceNotReady {
            status: PromotionReadinessStatusV1::Blocked
        }
    );
}

#[test]
fn artifact_promotion_execution_receipt_validation_rejects_stale_operation_status() {
    let mut receipt = sample_artifact_promotion_execution_receipt();
    receipt.operation_status = DeploymentExecutionStatusV1::FailedBeforeMutation;

    let err = validate_artifact_promotion_execution_receipt(&receipt)
        .expect_err("wrapper status must match nested deployment receipt");

    std::assert_matches!(
        err,
        ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "operation_status"
        }
    );
}

#[test]
fn artifact_promotion_execution_receipt_validation_rejects_stale_provenance_status() {
    let mut receipt = sample_artifact_promotion_execution_receipt();
    receipt.provenance_status = PromotionReadinessStatusV1::Blocked;

    let err = validate_artifact_promotion_execution_receipt(&receipt)
        .expect_err("archived execution receipt must preserve ready provenance");

    std::assert_matches!(
        err,
        ArtifactPromotionExecutionReceiptError::ProvenanceNotReady {
            status: PromotionReadinessStatusV1::Blocked
        }
    );
}

#[test]
fn artifact_promotion_execution_receipt_validation_rejects_missing_deployment_role() {
    let mut receipt = sample_artifact_promotion_execution_receipt();
    receipt.deployment_receipt.role_phase_receipts.clear();

    let err = validate_artifact_promotion_execution_receipt(&receipt)
        .expect_err("promotion execution receipt must cite deployment role evidence");

    std::assert_matches!(
        err,
        ArtifactPromotionExecutionReceiptError::MissingDeploymentRole { role } if role == "root"
    );
}

#[test]
fn artifact_promotion_execution_receipt_validation_rejects_unknown_deployment_role() {
    let mut receipt = sample_artifact_promotion_execution_receipt();
    let mut extra = receipt.deployment_receipt.role_phase_receipts[0].clone();
    extra.role = "worker".to_string();
    receipt.deployment_receipt.role_phase_receipts.push(extra);

    let err = validate_artifact_promotion_execution_receipt(&receipt)
        .expect_err("deployment receipt cannot add roles outside promotion provenance");

    std::assert_matches!(
        err,
        ArtifactPromotionExecutionReceiptError::UnknownDeploymentRole { role } if role == "worker"
    );
}

#[test]
fn artifact_promotion_execution_receipt_text_reports_execution_summary() {
    let receipt = sample_artifact_promotion_execution_receipt();

    let text = artifact_promotion_execution_receipt_text(&receipt);

    assert!(text.contains("Artifact promotion execution receipt"));
    assert!(text.contains("mode: execution_receipt"));
    assert!(text.contains("receipt_id: promotion-execution-receipt-1"));
    assert!(text.contains("execution_receipt_digest:"));
    assert!(text.contains("artifact_promotion_plan_id: artifact-promotion-plan-1"));
    assert!(text.contains("artifact_promotion_plan_digest:"));
    assert!(text.contains("provenance_report_id: promotion-provenance-1"));
    assert!(text.contains("provenance_report_digest:"));
    assert!(text.contains("promoted_plan_id: promoted-plan-1"));
    assert!(text.contains("operation_id: promoted-operation-1"));
    assert!(text.contains("provenance_status: ready"));
    assert!(text.contains("operation_status: Complete"));
    assert!(text.contains("command_result: Succeeded"));
    assert!(text.contains("deployment_phase_receipts: 1"));
    assert!(text.contains("root SealedWasm: result=Applied"));
    assert!(text.contains("catalog_digest="));
}
