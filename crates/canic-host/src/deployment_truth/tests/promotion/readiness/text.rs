use super::*;

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
