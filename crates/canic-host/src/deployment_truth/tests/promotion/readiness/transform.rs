use super::*;

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
