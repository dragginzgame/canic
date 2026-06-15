use super::*;

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
