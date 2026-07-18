use super::*;

pub(in crate::deployment_truth::tests) fn sample_role_artifact_source(
    kind: RoleArtifactSourceKindV1,
) -> RoleArtifactSourceV1 {
    RoleArtifactSourceV1 {
        role: "root".to_string(),
        kind,
        locator: Some("artifacts/root.wasm.gz".to_string()),
        previous_receipt_kind: (kind == RoleArtifactSourceKindV1::PreviousReceiptArtifact)
            .then_some(PreviousArtifactReceiptKindV1::DeploymentReceipt),
        previous_receipt_lineage_digest: (kind
            == RoleArtifactSourceKindV1::PreviousReceiptArtifact)
            .then(|| sample_sha256("9")),
        expected_wasm_sha256: Some(sample_sha256("d")),
        expected_wasm_gz_sha256: Some(sample_sha256("a")),
        expected_candid_sha256: Some(sample_sha256("b")),
        expected_canonical_embedded_config_sha256: Some(sample_sha256("c")),
    }
}

pub(in crate::deployment_truth::tests) fn sample_role_promotion_input(
    promotion_level: PromotionArtifactLevelV1,
) -> RolePromotionInputV1 {
    RolePromotionInputV1 {
        role: "root".to_string(),
        promotion_level,
        source: sample_role_artifact_source(RoleArtifactSourceKindV1::LocalWasmGz),
        require_byte_identical_wasm: promotion_level == PromotionArtifactLevelV1::SealedWasm,
        require_target_embedded_config: true,
        target_store_has_artifact: Some(true),
    }
}

pub(in crate::deployment_truth::tests) fn sample_role_promotion_policy() -> RolePromotionPolicyV1 {
    RolePromotionPolicyV1 {
        role: "root".to_string(),
        allowed_promotion_levels: vec![PromotionArtifactLevelV1::SealedWasm],
        requirements: vec![
            PromotionPolicyRequirementV1::SameSourceRevision,
            PromotionPolicyRequirementV1::SameCargoFeatures,
            PromotionPolicyRequirementV1::TargetConfigDigest,
            PromotionPolicyRequirementV1::ByteIdenticalWasm,
            PromotionPolicyRequirementV1::SealedBytes,
        ],
    }
}

pub(in crate::deployment_truth::tests) fn sample_build_recipe_identity() -> BuildRecipeIdentityV1 {
    BuildRecipeIdentityV1 {
        recipe_id: "recipe:root:debug".to_string(),
        source_kind: RoleArtifactSourceKindV1::WorkspacePackage,
        source_revision: "0123456789abcdef0123456789abcdef01234567".to_string(),
        source_tree_clean: true,
        package_or_role_selector: "root".to_string(),
        cargo_profile: "debug".to_string(),
        cargo_features_digest: sample_sha256("1"),
        cargo_lock_digest: sample_sha256("2"),
        rust_toolchain: "1.96.0".to_string(),
        builder_version: "canic-build-v1".to_string(),
        target_triple: "wasm32-unknown-unknown".to_string(),
        linker_identity: "rust-lld".to_string(),
        deterministic_build_mode: "locked".to_string(),
        wasm_opt_version: "not-used".to_string(),
        compression_identity: "gzip:default".to_string(),
    }
}

pub(in crate::deployment_truth::tests) fn sample_build_materialization_input()
-> BuildMaterializationInputV1 {
    BuildMaterializationInputV1 {
        materialization_input_id: "materialization-input:root:prod".to_string(),
        build_recipe_id: "recipe:root:debug".to_string(),
        canonical_embedded_config_sha256: sample_sha256("3"),
        environment: "ic".to_string(),
        root_trust_anchor: "aaaaa-aa".to_string(),
        runtime_variant: "prod".to_string(),
    }
}

pub(in crate::deployment_truth::tests) fn sample_build_materialization_result()
-> BuildMaterializationResultV1 {
    BuildMaterializationResultV1 {
        materialization_result_id: "materialization-result:root:prod".to_string(),
        build_recipe_id: "recipe:root:debug".to_string(),
        materialization_input_digest: sample_sha256("4"),
        wasm_sha256: sample_sha256("5"),
        wasm_gz_sha256: sample_sha256("6"),
        installed_module_hash: sample_sha256("7"),
        candid_sha256: sample_sha256("8"),
    }
}

pub(in crate::deployment_truth::tests) fn sample_build_materialization_evidence()
-> BuildMaterializationEvidenceV1 {
    let input = sample_build_materialization_input();
    let mut result = sample_build_materialization_result();
    result.materialization_input_digest = build_materialization_input_digest(&input);
    build_materialization_evidence(BuildMaterializationEvidenceRequest {
        evidence_id: "materialization-evidence-1".to_string(),
        recipe: sample_build_recipe_identity(),
        materialization_input: input,
        materialization_result: result,
    })
    .expect("sample materialization evidence should validate")
}

pub(in crate::deployment_truth::tests) fn sample_promotion_target_plan() -> DeploymentPlanV1 {
    let mut plan = sample_plan();
    plan.role_artifacts[0].wasm_sha256 = Some(sample_sha256("d"));
    plan.role_artifacts[0].wasm_gz_sha256 = Some(sample_sha256("a"));
    plan.role_artifacts[0].canonical_embedded_config_sha256 = Some(sample_sha256("c"));
    plan
}

pub(in crate::deployment_truth::tests) fn sample_promotion_transform() -> PromotionPlanTransformV1 {
    promoted_deployment_plan_transform_from_inputs(&PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    })
    .expect("sample promotion transform should validate")
}

pub(in crate::deployment_truth::tests) fn sample_execution_preflight_for_plan(
    plan_id: &str,
) -> DeploymentExecutionPreflightV1 {
    DeploymentExecutionPreflightV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        plan_id: plan_id.to_string(),
        safety_report_id: "report-1".to_string(),
        authority_plan_id: plan_id.to_string(),
        backend: DeploymentExecutorBackendV1::CurrentCli,
        status: DeploymentExecutionPreflightStatusV1::Ready,
        planned_phases: vec!["install_root".to_string(), "activate_root".to_string()],
        required_capabilities: vec![
            DeploymentExecutorCapabilityV1::StageArtifact,
            DeploymentExecutorCapabilityV1::InstallCode,
        ],
        missing_capabilities: Vec::new(),
        blockers: Vec::new(),
    }
}

pub(in crate::deployment_truth::tests) fn sample_artifact_promotion_plan() -> ArtifactPromotionPlanV1
{
    let target_plan = sample_promotion_target_plan();
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let readiness = promotion_readiness_from_inputs(
        "promotion-ready-1",
        &target_plan,
        std::slice::from_ref(&input),
    );
    let artifact_identity_report =
        promotion_artifact_identity_report_from_inputs(PromotionArtifactIdentityReportRequest {
            report_id: "promotion-artifact-identity-1".to_string(),
            inputs: vec![input],
        })
        .expect("sample artifact identity report should validate");
    let transform = sample_promotion_transform();
    let target_execution_lineage =
        promotion_target_execution_lineage(PromotionTargetExecutionLineageRequest {
            lineage_id: "target-execution-lineage-1".to_string(),
            generated_at: "2026-05-25T00:00:00Z".to_string(),
            transform: transform.clone(),
            execution_preflight: sample_execution_preflight_for_plan("promoted-plan-1"),
        })
        .expect("sample target execution lineage should validate");

    artifact_promotion_plan(ArtifactPromotionPlanRequest {
        plan_id: "artifact-promotion-plan-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        readiness,
        artifact_identity_report,
        transform,
        target_execution_lineage: Some(target_execution_lineage),
    })
    .expect("sample artifact promotion plan should validate")
}

pub(in crate::deployment_truth::tests) fn sample_artifact_promotion_provenance_report()
-> ArtifactPromotionProvenanceReportV1 {
    artifact_promotion_provenance_report(ArtifactPromotionProvenanceReportRequest {
        report_id: "promotion-provenance-1".to_string(),
        artifact_promotion_plan: sample_artifact_promotion_plan(),
        wasm_store_identity_report: Some(sample_wasm_store_identity_report()),
        wasm_store_catalog_verification: Some(sample_wasm_store_catalog_verification()),
        materialization_identity_report: Some(sample_materialization_identity_report()),
    })
    .expect("sample promotion provenance report should validate")
}

pub(in crate::deployment_truth::tests) fn sample_artifact_promotion_execution_receipt()
-> ArtifactPromotionExecutionReceiptV1 {
    artifact_promotion_execution_receipt(ArtifactPromotionExecutionReceiptRequest {
        receipt_id: "promotion-execution-receipt-1".to_string(),
        provenance_report: sample_artifact_promotion_provenance_report(),
        deployment_receipt: sample_promoted_deployment_receipt(),
    })
    .expect("sample promotion execution receipt should validate")
}

pub(in crate::deployment_truth::tests) fn sample_promoted_deployment_receipt() -> DeploymentReceiptV1
{
    let mut receipt = sample_receipt_with_phase(
        "promoted-plan-1",
        Some("aaaaa-aa"),
        ObservationStatusV1::Observed,
        RolePhaseResultV1::Applied,
    );
    receipt.operation_id = "promoted-operation-1".to_string();
    receipt.phase_receipts[0].phase = "promote_artifacts".to_string();
    receipt.role_phase_receipts[0].phase = "install_root".to_string();
    receipt.role_phase_receipts[0].artifact_digest = Some(sample_sha256("5"));
    receipt.role_phase_receipts[0].observed_module_hash_after = Some(sample_sha256("7"));
    receipt.role_phase_receipts[0].canonical_embedded_config_sha256 = Some(sample_sha256("3"));
    receipt
}

pub(in crate::deployment_truth::tests) fn sample_wasm_store_identity_report()
-> PromotionWasmStoreIdentityReportV1 {
    promotion_wasm_store_identity_report_from_staging(PromotionWasmStoreIdentityReportRequest {
        report_id: "wasm-store-identity-1".to_string(),
        staging_receipts: vec![sample_wasm_store_staging_receipt()],
    })
    .expect("sample wasm-store identity report should validate")
}

pub(in crate::deployment_truth::tests) fn sample_wasm_store_catalog_entry()
-> PromotionWasmStoreCatalogEntryV1 {
    PromotionWasmStoreCatalogEntryV1 {
        locator: "root:aaaaa-aa:bootstrap".to_string(),
        artifact_identity: "embedded:root:0.44.0:abc123".to_string(),
        published_chunk_count: 2,
    }
}

pub(in crate::deployment_truth::tests) fn sample_wasm_store_catalog_verification()
-> PromotionWasmStoreCatalogVerificationV1 {
    promotion_wasm_store_catalog_verification(PromotionWasmStoreCatalogVerificationRequest {
        verification_id: "wasm-store-catalog-1".to_string(),
        wasm_store_identity_report: sample_wasm_store_identity_report(),
        catalog_entries: vec![sample_wasm_store_catalog_entry()],
    })
    .expect("sample wasm-store catalog verification should validate")
}

pub(in crate::deployment_truth::tests) fn sample_materialization_identity_report()
-> PromotionMaterializationIdentityReportV1 {
    promotion_materialization_identity_report_from_evidence(
        PromotionMaterializationIdentityReportRequest {
            report_id: "materialization-report-1".to_string(),
            evidence: vec![sample_build_materialization_evidence()],
        },
    )
    .expect("sample materialization identity report should validate")
}
