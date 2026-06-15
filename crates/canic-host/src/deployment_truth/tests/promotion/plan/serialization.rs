use super::*;

#[test]
fn promotion_readiness_round_trips_through_json() {
    let plan = sample_promotion_target_plan();
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);

    assert_json_round_trip(&readiness);
    let encoded = serde_json::to_value(&readiness).expect("readiness should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "readiness_id",
            "promotion_readiness_digest",
            "target_plan_id",
            "status",
            "roles",
            "blockers",
            "warnings",
        ],
    );
    assert!(
        encoded["promotion_readiness_digest"]
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
            "source_wasm_sha256",
            "source_wasm_gz_sha256",
            "target_wasm_sha256",
            "target_wasm_gz_sha256",
            "source_canonical_embedded_config_sha256",
            "target_canonical_embedded_config_sha256",
            "byte_identical_wasm",
            "embedded_config_identical",
            "target_store_has_artifact",
            "restage_required",
        ],
    );
}

#[test]
fn promotion_plan_transform_round_trips_through_json() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");

    assert_json_round_trip(&transform);
    let encoded = serde_json::to_value(&transform).expect("transform should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "transform_id",
            "target_plan_id",
            "promoted_plan_id",
            "promotion_plan_lineage_digest",
            "promoted_plan",
            "roles",
        ],
    );
    let role = &encoded["roles"][0];
    assert_object_keys(
        role,
        &[
            "role",
            "promotion_level",
            "source_kind",
            "source_locator",
            "artifact_source_before",
            "artifact_source_after",
            "wasm_sha256_before",
            "wasm_sha256_after",
            "wasm_gz_sha256_before",
            "wasm_gz_sha256_after",
            "candid_sha256_before",
            "candid_sha256_after",
            "canonical_embedded_config_sha256_before",
            "canonical_embedded_config_sha256_after",
            "artifact_identity_changed",
            "embedded_config_changed",
            "target_materialization_preserved",
            "source_build_materialization",
        ],
    );
}

#[test]
fn promotion_plan_transform_evidence_round_trips_through_json() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    let evidence = promotion_plan_transform_evidence(PromotionPlanTransformEvidenceRequest {
        evidence_id: "promotion-evidence-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
    })
    .expect("evidence should be produced");

    assert_json_round_trip(&evidence);
    let encoded = serde_json::to_value(&evidence).expect("evidence should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "evidence_id",
            "promotion_plan_transform_evidence_digest",
            "generated_at",
            "transform",
        ],
    );
    assert!(
        encoded["promotion_plan_transform_evidence_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
}
