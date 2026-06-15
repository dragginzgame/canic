use super::super::*;

#[test]
fn promotion_readiness_reports_ready_role_and_restage_warning() {
    let plan = sample_promotion_target_plan();
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.target_store_has_artifact = Some(false);

    let readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);

    assert_eq!(readiness.schema_version, DEPLOYMENT_TRUTH_SCHEMA_VERSION);
    assert_eq!(readiness.target_plan_id, plan.plan_id);
    assert_eq!(readiness.status, PromotionReadinessStatusV1::Ready);
    assert!(readiness.blockers.is_empty());
    assert_eq!(readiness.warnings.len(), 1);
    assert_eq!(
        readiness.warnings[0].code,
        "promotion_target_store_restage_required"
    );
    assert_eq!(readiness.roles.len(), 1);
    assert_eq!(readiness.roles[0].byte_identical_wasm, Some(true));
    assert_eq!(readiness.roles[0].embedded_config_identical, Some(true));
    assert!(readiness.roles[0].restage_required);
    validate_promotion_readiness(&readiness).expect("readiness artifact should validate");
}

#[test]
fn promotion_readiness_blocks_sealed_wasm_embedded_config_mismatch() {
    let plan = sample_promotion_target_plan();
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.source.expected_canonical_embedded_config_sha256 = Some(sample_sha256("e"));

    let readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);

    assert_eq!(readiness.status, PromotionReadinessStatusV1::Blocked);
    assert!(readiness.blockers.iter().any(|finding| {
        finding.code == "promotion_sealed_wasm_embedded_config_mismatch"
            && finding.subject.as_deref() == Some("root")
    }));
}

#[test]
fn source_build_promotion_allows_target_config_digest_change() {
    let plan = sample_promotion_target_plan();
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    input.source.expected_canonical_embedded_config_sha256 = Some(sample_sha256("e"));

    let readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);

    assert_eq!(readiness.status, PromotionReadinessStatusV1::Ready);
    assert!(readiness.blockers.is_empty());
    assert_eq!(readiness.roles[0].embedded_config_identical, Some(false));
    validate_promotion_readiness(&readiness).expect("source-build readiness should validate");
}

#[test]
fn check_promotion_readiness_validates_and_returns_artifact() {
    let request = PromotionReadinessRequest {
        readiness_id: "promotion-ready-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };

    let readiness = check_promotion_readiness(&request).expect("readiness should be valid");

    assert_eq!(readiness.readiness_id, "promotion-ready-1");
    assert_eq!(readiness.status, PromotionReadinessStatusV1::Ready);
    assert_eq!(readiness.roles.len(), 1);
}

#[test]
fn promotion_readiness_with_policy_blocks_source_build_when_sealed_bytes_are_required() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    let policy = sample_role_promotion_policy();

    let readiness = promotion_readiness_from_inputs_with_policy(
        "promotion-ready-1",
        &sample_promotion_target_plan(),
        &[input],
        &[policy],
    );

    assert_eq!(readiness.status, PromotionReadinessStatusV1::Blocked);
    assert!(readiness.blockers.iter().any(|finding| {
        finding.code == "promotion_policy_level_not_allowed"
            || finding.code == "promotion_policy_must_use_sealed_bytes"
    }));
    validate_promotion_readiness(&readiness).expect("policy-blocked readiness should validate");
}

#[test]
fn promotion_readiness_with_policy_accepts_byte_identical_source_build_policy() {
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    input.require_byte_identical_wasm = true;
    let mut policy = sample_role_promotion_policy();
    policy.allowed_promotion_levels = vec![PromotionArtifactLevelV1::SourceBuild];
    policy.requirements = vec![PromotionPolicyRequirementV1::ByteIdenticalWasm];

    let readiness = check_promotion_readiness_with_policy(&PromotionReadinessWithPolicyRequest {
        readiness_id: "promotion-ready-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![input],
        policies: vec![policy],
    })
    .expect("source-build policy readiness should validate");

    assert_eq!(readiness.status, PromotionReadinessStatusV1::Ready);
    assert!(readiness.blockers.is_empty());
}

#[test]
fn promotion_readiness_with_policy_reports_missing_role_policy() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);

    let readiness = promotion_readiness_from_inputs_with_policy(
        "promotion-ready-1",
        &sample_promotion_target_plan(),
        &[input],
        &[],
    );

    assert_eq!(readiness.status, PromotionReadinessStatusV1::Blocked);
    assert!(readiness.blockers.iter().any(|finding| {
        finding.code == "promotion_policy_missing" && finding.subject.as_deref() == Some("root")
    }));
    validate_promotion_readiness(&readiness).expect("missing-policy readiness should validate");
}

#[test]
fn check_promotion_readiness_rejects_blank_readiness_id() {
    let request = PromotionReadinessRequest {
        readiness_id: " ".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };

    let err = check_promotion_readiness(&request).expect_err("blank readiness id should fail");
    std::assert_matches!(
        err,
        PromotionReadinessError::MissingRequiredField {
            field: "readiness_id"
        }
    );
}

#[test]
fn promoted_deployment_plan_applies_sealed_wasm_role_identity() {
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.source.kind = RoleArtifactSourceKindV1::LocalWasmGz;
    input.source.locator = Some("promoted/root.wasm.gz".to_string());
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![input],
    };

    let promoted =
        promoted_deployment_plan_from_inputs(&request).expect("promoted plan should be produced");

    assert_eq!(promoted.plan_id, "promoted-plan-1");
    assert_eq!(
        promoted.authority_profile,
        request.target_plan.authority_profile
    );
    assert_eq!(promoted.trust_domain, request.target_plan.trust_domain);
    let artifact = promoted
        .role_artifacts
        .iter()
        .find(|artifact| artifact.role == "root")
        .expect("root artifact should remain");
    assert_eq!(artifact.source, ArtifactSourceV1::External);
    assert_eq!(
        artifact.wasm_gz_path.as_deref(),
        Some("promoted/root.wasm.gz")
    );
    assert_eq!(artifact.wasm_sha256, Some(sample_sha256("d")));
    assert_eq!(artifact.wasm_gz_sha256, Some(sample_sha256("a")));
    assert_eq!(
        artifact.canonical_embedded_config_sha256,
        Some(sample_sha256("c"))
    );
}

#[test]
fn promoted_deployment_plan_transform_summarizes_sealed_wasm_changes() {
    let mut target_plan = sample_promotion_target_plan();
    target_plan.role_artifacts[0].wasm_gz_sha256 = Some(sample_sha256("f"));
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.require_byte_identical_wasm = false;
    input.source.kind = RoleArtifactSourceKindV1::LocalWasmGz;
    input.source.locator = Some("promoted/root.wasm.gz".to_string());
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan,
        inputs: vec![input],
    };

    let transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("sealed wasm transform should be produced");

    assert_eq!(
        transform.transform_id,
        "promotion-transform:promoted-plan-1"
    );
    assert_eq!(transform.target_plan_id, "plan-local-root");
    assert_eq!(transform.promoted_plan_id, "promoted-plan-1");
    assert_eq!(transform.roles.len(), 1);
    let role = &transform.roles[0];
    assert_eq!(role.role, "root");
    assert_eq!(role.promotion_level, PromotionArtifactLevelV1::SealedWasm);
    assert_eq!(role.source_kind, RoleArtifactSourceKindV1::LocalWasmGz);
    assert_eq!(
        role.source_locator.as_deref(),
        Some("promoted/root.wasm.gz")
    );
    assert_eq!(role.artifact_source_before, ArtifactSourceV1::LocalBuild);
    assert_eq!(role.artifact_source_after, ArtifactSourceV1::External);
    assert_eq!(role.wasm_gz_sha256_before, Some(sample_sha256("f")));
    assert_eq!(role.wasm_gz_sha256_after, Some(sample_sha256("a")));
    assert!(role.artifact_identity_changed);
    assert!(!role.embedded_config_changed);
    assert!(!role.target_materialization_preserved);
}

#[test]
fn promotion_plan_transform_text_reports_passive_summary() {
    let mut target_plan = sample_promotion_target_plan();
    target_plan.role_artifacts[0].wasm_gz_sha256 = Some(sample_sha256("f"));
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.require_byte_identical_wasm = false;
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan,
        inputs: vec![input],
    };
    let transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");

    let text = promotion_plan_transform_text(&transform);

    assert!(text.contains("Promotion plan transform"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("transform_id: promotion-transform:promoted-plan-1"));
    assert!(text.contains("target_plan_id: plan-local-root"));
    assert!(text.contains("promoted_plan_id: promoted-plan-1"));
    assert!(text.contains("promotion_plan_lineage_digest: "));
    assert!(text.contains("artifact_identity_changed: 1"));
    assert!(text.contains("embedded_config_changed: 0"));
    assert!(text.contains("target_materialization_preserved: 0"));
    assert!(
        text.contains("root SealedWasm/LocalWasmGz: artifact_identity_changed=true embedded_config_changed=false target_materialization_preserved=false")
    );
    assert!(text.contains("wasm_gz_sha256: ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff -> aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
}

#[test]
fn promoted_deployment_plan_leaves_source_build_materialization_to_target_plan() {
    let mut target_plan = sample_promotion_target_plan();
    target_plan.role_artifacts[0].wasm_gz_sha256 = Some(sample_sha256("f"));
    target_plan.role_artifacts[0].canonical_embedded_config_sha256 = Some(sample_sha256("1"));
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    input.source.expected_canonical_embedded_config_sha256 = Some(sample_sha256("c"));
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: target_plan.clone(),
        inputs: vec![input],
    };

    let promoted = promoted_deployment_plan_from_inputs(&request)
        .expect("source-build plan should be produced");

    assert_eq!(promoted.plan_id, "promoted-plan-1");
    assert_eq!(
        promoted.role_artifacts[0].wasm_gz_sha256,
        target_plan.role_artifacts[0].wasm_gz_sha256
    );
    assert_eq!(
        promoted.role_artifacts[0].canonical_embedded_config_sha256,
        target_plan.role_artifacts[0].canonical_embedded_config_sha256
    );
}

#[test]
fn promoted_deployment_plan_transform_marks_source_build_target_materialization_preserved() {
    let mut target_plan = sample_promotion_target_plan();
    target_plan.role_artifacts[0].wasm_gz_sha256 = Some(sample_sha256("f"));
    target_plan.role_artifacts[0].canonical_embedded_config_sha256 = Some(sample_sha256("1"));
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    input.source.expected_canonical_embedded_config_sha256 = Some(sample_sha256("c"));
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan,
        inputs: vec![input],
    };

    let transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("source-build transform should be produced");

    let role = &transform.roles[0];
    assert_eq!(role.promotion_level, PromotionArtifactLevelV1::SourceBuild);
    assert_eq!(role.wasm_gz_sha256_before, Some(sample_sha256("f")));
    assert_eq!(role.wasm_gz_sha256_after, Some(sample_sha256("f")));
    assert_eq!(
        role.canonical_embedded_config_sha256_before,
        Some(sample_sha256("1"))
    );
    assert_eq!(
        role.canonical_embedded_config_sha256_after,
        Some(sample_sha256("1"))
    );
    assert!(!role.artifact_identity_changed);
    assert!(!role.embedded_config_changed);
    assert!(role.target_materialization_preserved);
}

#[test]
fn promoted_deployment_plan_transform_links_source_build_materialization_evidence() {
    let mut target_plan = sample_promotion_target_plan();
    target_plan.role_artifacts[0].wasm_sha256 = Some(sample_sha256("5"));
    target_plan.role_artifacts[0].wasm_gz_sha256 = Some(sample_sha256("6"));
    target_plan.role_artifacts[0].installed_module_hash = Some(sample_sha256("7"));
    target_plan.role_artifacts[0].candid_sha256 = Some(sample_sha256("8"));
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    input.source.expected_canonical_embedded_config_sha256 = target_plan.role_artifacts[0]
        .canonical_embedded_config_sha256
        .clone();
    let request = PromotionPlanTransformWithMaterializationRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan,
        inputs: vec![input],
        materialization_evidence: vec![sample_build_materialization_evidence()],
    };

    let transform = promoted_deployment_plan_transform_from_inputs_with_materialization(&request)
        .expect("source-build transform should link materialization evidence");

    let link = transform.roles[0]
        .source_build_materialization
        .as_ref()
        .expect("source-build role should carry materialization link");
    let expected_input_digest =
        build_materialization_input_digest(&sample_build_materialization_input());
    assert_eq!(link.role, "root");
    assert_eq!(link.evidence_id, "materialization-evidence-1");
    assert_eq!(link.materialization_evidence_digest.len(), 64);
    assert_eq!(link.materialization_input_digest, expected_input_digest);
    assert_eq!(link.wasm_gz_sha256, sample_sha256("6"));
    validate_promotion_plan_transform(&transform)
        .expect("materialization-linked transform should validate");
}

#[test]
fn promoted_deployment_plan_transform_requires_source_build_materialization_evidence() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    let request = PromotionPlanTransformWithMaterializationRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![input],
        materialization_evidence: Vec::new(),
    };

    let err = promoted_deployment_plan_transform_from_inputs_with_materialization(&request)
        .expect_err("source-build transform should require materialization evidence");

    std::assert_matches!(
        err,
        PromotionPlanTransformError::MaterializationRoleMissing { role } if role == "root"
    );
}

#[test]
fn promoted_deployment_plan_transform_rejects_duplicate_materialization_evidence() {
    let mut target_plan = sample_promotion_target_plan();
    target_plan.role_artifacts[0].wasm_sha256 = Some(sample_sha256("5"));
    target_plan.role_artifacts[0].wasm_gz_sha256 = Some(sample_sha256("6"));
    target_plan.role_artifacts[0].installed_module_hash = Some(sample_sha256("7"));
    target_plan.role_artifacts[0].candid_sha256 = Some(sample_sha256("8"));
    let request = PromotionPlanTransformWithMaterializationRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan,
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SourceBuild,
        )],
        materialization_evidence: vec![
            sample_build_materialization_evidence(),
            sample_build_materialization_evidence(),
        ],
    };

    let err = promoted_deployment_plan_transform_from_inputs_with_materialization(&request)
        .expect_err("duplicate materialization evidence should fail");

    std::assert_matches!(
        err,
        PromotionPlanTransformError::DuplicateMaterializationRole { role } if role == "root"
    );
}

#[test]
fn promotion_plan_transform_text_reports_source_build_materialization_link() {
    let mut target_plan = sample_promotion_target_plan();
    target_plan.role_artifacts[0].wasm_sha256 = Some(sample_sha256("5"));
    target_plan.role_artifacts[0].wasm_gz_sha256 = Some(sample_sha256("6"));
    target_plan.role_artifacts[0].installed_module_hash = Some(sample_sha256("7"));
    target_plan.role_artifacts[0].candid_sha256 = Some(sample_sha256("8"));
    let request = PromotionPlanTransformWithMaterializationRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan,
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SourceBuild,
        )],
        materialization_evidence: vec![sample_build_materialization_evidence()],
    };
    let transform = promoted_deployment_plan_transform_from_inputs_with_materialization(&request)
        .expect("source-build transform should link evidence");

    let text = promotion_plan_transform_text(&transform);
    let expected_input_digest =
        build_materialization_input_digest(&sample_build_materialization_input());

    assert!(text.contains("materialization_evidence_id: materialization-evidence-1"));
    assert!(text.contains(&format!(
        "materialization_input_digest: {expected_input_digest}"
    )));
    assert!(text.contains(
        "materialized_wasm_gz_sha256: 6666666666666666666666666666666666666666666666666666666666666666"
    ));
}

#[test]
fn promoted_deployment_plan_rejects_blocked_readiness() {
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.source.expected_canonical_embedded_config_sha256 = Some(sample_sha256("e"));
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![input],
    };

    let err =
        promoted_deployment_plan_from_inputs(&request).expect_err("blocked readiness should fail");
    std::assert_matches!(
        err,
        PromotionPlanTransformError::ReadinessBlocked { blocker_count: 1 }
    );
}

#[test]
fn promoted_deployment_plan_rejects_blank_plan_id() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: " ".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };

    let err =
        promoted_deployment_plan_from_inputs(&request).expect_err("blank plan id should fail");
    std::assert_matches!(
        err,
        PromotionPlanTransformError::MissingRequiredField {
            field: "promoted_plan_id"
        }
    );
}

#[test]
fn promotion_readiness_blocks_source_role_mismatch() {
    let plan = sample_promotion_target_plan();
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    input.source.role = "other".to_string();

    let readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);

    assert_eq!(readiness.status, PromotionReadinessStatusV1::Blocked);
    assert!(readiness.blockers.iter().any(|finding| {
        finding.code == "promotion_source_role_mismatch"
            && finding.subject.as_deref() == Some("root")
    }));
}

#[test]
fn promotion_readiness_blocks_missing_target_role() {
    let plan = sample_plan();
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    input.role = "missing".to_string();
    input.source.role = "missing".to_string();

    let readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);

    assert_eq!(readiness.status, PromotionReadinessStatusV1::Blocked);
    assert!(readiness.roles.is_empty());
    assert!(readiness.blockers.iter().any(|finding| {
        finding.code == "promotion_target_role_missing"
            && finding.subject.as_deref() == Some("missing")
    }));
}

#[test]
fn promotion_readiness_blocks_invalid_artifact_source() {
    let plan = sample_promotion_target_plan();
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    input.source.expected_wasm_sha256 = None;
    input.source.expected_wasm_gz_sha256 = None;

    let readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);

    assert_eq!(readiness.status, PromotionReadinessStatusV1::Blocked);
    assert!(readiness.blockers.iter().any(|finding| {
        finding.code == "promotion_artifact_source_invalid"
            && finding.subject.as_deref() == Some("root")
    }));
    validate_promotion_readiness(&readiness).expect("blocked readiness artifact should validate");
}

#[test]
fn promotion_readiness_validation_rejects_status_blocker_mismatch() {
    let plan = sample_promotion_target_plan();
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let mut readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);
    readiness.status = PromotionReadinessStatusV1::Blocked;

    let err = validate_promotion_readiness(&readiness).expect_err("status should match blockers");
    std::assert_matches!(err, PromotionReadinessError::StatusBlockerMismatch { .. });
}

#[test]
fn promotion_readiness_validation_rejects_stale_digest() {
    let plan = sample_promotion_target_plan();
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let mut readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);
    readiness.promotion_readiness_digest = sample_sha256("9");

    let err =
        validate_promotion_readiness(&readiness).expect_err("stale readiness digest should fail");
    std::assert_matches!(
        err,
        PromotionReadinessError::LinkageMismatch {
            field: "promotion_readiness_digest"
        }
    );
}

#[test]
fn promotion_readiness_validation_rejects_duplicate_roles() {
    let plan = sample_promotion_target_plan();
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let mut readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);
    readiness.roles.push(readiness.roles[0].clone());

    let err = validate_promotion_readiness(&readiness).expect_err("duplicate role should fail");
    std::assert_matches!(
        err,
        PromotionReadinessError::DuplicateRole { role } if role == "root"
    );
}

#[test]
fn promotion_readiness_validation_rejects_restage_state_mismatch() {
    let plan = sample_promotion_target_plan();
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let mut readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);
    readiness.roles[0].target_store_has_artifact = Some(true);
    readiness.roles[0].restage_required = true;

    let err = validate_promotion_readiness(&readiness).expect_err("restage state should match");
    std::assert_matches!(
        err,
        PromotionReadinessError::RestageStateMismatch { role } if role == "root"
    );
}

#[test]
fn promotion_readiness_validation_rejects_bad_digest_shape() {
    let plan = sample_promotion_target_plan();
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let mut readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);
    readiness.roles[0].target_wasm_gz_sha256 = Some("NOT-A-DIGEST".to_string());

    let err = validate_promotion_readiness(&readiness).expect_err("digest should be checked");
    std::assert_matches!(
        err,
        PromotionReadinessError::InvalidSha256Digest {
            field: "target_wasm_gz_sha256"
        }
    );
}

#[test]
fn promotion_readiness_validation_rejects_warning_in_blockers() {
    let plan = sample_promotion_target_plan();
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.source.expected_canonical_embedded_config_sha256 = Some(sample_sha256("e"));
    let mut readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);
    readiness.blockers[0].severity = SafetySeverityV1::Warning;

    let err = validate_promotion_readiness(&readiness).expect_err("blockers must be hard failures");
    std::assert_matches!(
        err,
        PromotionReadinessError::FindingSeverityMismatch {
            field: "blockers",
            severity: SafetySeverityV1::Warning
        }
    );
}

#[test]
fn promotion_readiness_text_reports_passive_summary() {
    let plan = sample_promotion_target_plan();
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.target_store_has_artifact = Some(false);
    let readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);

    let text = promotion_readiness_text(&readiness);

    assert!(text.contains("Promotion readiness report"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("status: ready"));
    assert!(text.contains("readiness_id: promotion-ready-1"));
    assert!(text.contains("promotion_readiness_digest:"));
    assert!(text.contains("target_plan_id: plan-local-root"));
    assert!(text.contains("restage_required: 1"));
    assert!(
        text.contains("root SealedWasm/LocalWasmGz: byte_identical_wasm=true embedded_config_identical=true restage_required=true")
    );
    assert!(text.contains(
        "source_wasm_gz_sha256: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    ));
    assert!(text.contains(
        "target_wasm_gz_sha256: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    ));
    assert!(text.contains("[promotion_target_store_restage_required] root"));
}

#[test]
fn promotion_readiness_text_reports_blockers() {
    let plan = sample_promotion_target_plan();
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.source.expected_canonical_embedded_config_sha256 = Some(sample_sha256("e"));
    let readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);

    let text = promotion_readiness_text(&readiness);

    assert!(text.contains("status: blocked"));
    assert!(text.contains("blockers: 1"));
    assert!(text.contains("[promotion_sealed_wasm_embedded_config_mismatch] root"));
    assert!(text.contains("embedded_config_identical=false"));
}

#[test]
fn promotion_readiness_text_reports_policy_blockers() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    let policy = sample_role_promotion_policy();
    let readiness = promotion_readiness_from_inputs_with_policy(
        "promotion-ready-1",
        &sample_promotion_target_plan(),
        &[input],
        &[policy],
    );

    let text = promotion_readiness_text(&readiness);

    assert!(text.contains("status: blocked"));
    assert!(text.contains("promotion_policy_level_not_allowed"));
    assert!(text.contains("promotion_policy_must_use_sealed_bytes"));
}
