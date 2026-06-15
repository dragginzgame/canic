use super::super::*;

#[test]
fn role_artifact_source_round_trips_through_json() {
    let source = sample_role_artifact_source(RoleArtifactSourceKindV1::LocalWasmGz);

    assert_json_round_trip(&source);
    let encoded = serde_json::to_value(&source).expect("source should encode");
    assert_object_keys(
        &encoded,
        &[
            "role",
            "kind",
            "locator",
            "previous_receipt_kind",
            "previous_receipt_lineage_digest",
            "expected_wasm_sha256",
            "expected_wasm_gz_sha256",
            "expected_candid_sha256",
            "expected_canonical_embedded_config_sha256",
        ],
    );
}

#[test]
fn role_promotion_input_round_trips_through_json() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);

    assert_json_round_trip(&input);
    let encoded = serde_json::to_value(&input).expect("input should encode");
    assert_object_keys(
        &encoded,
        &[
            "role",
            "promotion_level",
            "source",
            "require_byte_identical_wasm",
            "require_target_embedded_config",
            "target_store_has_artifact",
        ],
    );
}

#[test]
fn role_promotion_policy_round_trips_through_json() {
    let policy = sample_role_promotion_policy();

    validate_role_promotion_policy(&policy).expect("policy should validate");
    assert_json_round_trip(&policy);
    let encoded = serde_json::to_value(&policy).expect("policy should encode");
    assert_object_keys(
        &encoded,
        &["role", "allowed_promotion_levels", "requirements"],
    );
}

#[test]
fn role_promotion_policy_validation_rejects_sealed_only_policy_with_source_build_allowed() {
    let mut policy = sample_role_promotion_policy();
    policy
        .allowed_promotion_levels
        .push(PromotionArtifactLevelV1::SourceBuild);

    let err = validate_role_promotion_policy(&policy)
        .expect_err("sealed-only policy cannot allow source build");

    std::assert_matches!(
        err,
        PromotionPolicyCheckError::DecisionMismatch {
            role,
            field: "sealed_bytes"
        } if role == "root"
    );
}

#[test]
fn promotion_policy_check_accepts_sealed_wasm_policy() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let policy = sample_role_promotion_policy();

    let check = check_promotion_policy(PromotionPolicyCheckRequest {
        check_id: "promotion-policy-1".to_string(),
        inputs: vec![input],
        policies: vec![policy],
    })
    .expect("policy check should validate");

    assert_eq!(check.status, PromotionReadinessStatusV1::Ready);
    assert_eq!(check.roles.len(), 1);
    assert!(check.roles[0].policy_satisfied);
    assert!(
        check.roles[0]
            .requirements
            .contains(&PromotionPolicyRequirementV1::SealedBytes)
    );
    assert!(
        check.roles[0]
            .requirements
            .contains(&PromotionPolicyRequirementV1::ByteIdenticalWasm)
    );
    assert!(
        check.roles[0]
            .requirements
            .contains(&PromotionPolicyRequirementV1::TargetConfigDigest)
    );
    assert_json_round_trip(&check);
    let encoded = serde_json::to_value(&check).expect("policy check should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "check_id",
            "promotion_policy_check_digest",
            "status",
            "roles",
            "blockers",
        ],
    );
    assert!(
        encoded["promotion_policy_check_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    let role = &encoded["roles"][0];
    assert_object_keys(
        role,
        &[
            "role",
            "requested_promotion_level",
            "allowed_promotion_levels",
            "requirements",
            "claims",
            "level_allowed",
            "policy_satisfied",
        ],
    );
}

#[test]
fn promotion_policy_check_blocks_source_build_when_sealed_bytes_are_required() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    let policy = sample_role_promotion_policy();

    let check = promotion_policy_check_from_inputs("promotion-policy-1", &[input], &[policy]);

    assert_eq!(check.status, PromotionReadinessStatusV1::Blocked);
    assert!(!check.roles[0].policy_satisfied);
    assert!(check.blockers.iter().any(|finding| {
        finding.code == "promotion_policy_level_not_allowed"
            || finding.code == "promotion_policy_must_use_sealed_bytes"
    }));
    validate_promotion_policy_check(&check).expect("blocked policy check should validate");
}

#[test]
fn promotion_policy_check_distinguishes_byte_identity_from_sealed_bytes() {
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    input.require_byte_identical_wasm = true;
    let mut policy = sample_role_promotion_policy();
    policy.allowed_promotion_levels = vec![PromotionArtifactLevelV1::SourceBuild];
    policy.requirements = vec![PromotionPolicyRequirementV1::ByteIdenticalWasm];

    let check = promotion_policy_check_from_inputs("promotion-policy-1", &[input], &[policy]);

    assert_eq!(check.status, PromotionReadinessStatusV1::Ready);
    assert!(check.roles[0].policy_satisfied);
    assert!(
        !check.roles[0]
            .requirements
            .contains(&PromotionPolicyRequirementV1::SealedBytes)
    );
    assert!(
        check.roles[0]
            .requirements
            .contains(&PromotionPolicyRequirementV1::ByteIdenticalWasm)
    );
}

#[test]
fn promotion_policy_check_blocks_missing_byte_identity_claim() {
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    input.require_byte_identical_wasm = false;
    let mut policy = sample_role_promotion_policy();
    policy.allowed_promotion_levels = vec![PromotionArtifactLevelV1::SourceBuild];
    policy.requirements = vec![PromotionPolicyRequirementV1::ByteIdenticalWasm];

    let check = promotion_policy_check_from_inputs("promotion-policy-1", &[input], &[policy]);

    assert_eq!(check.status, PromotionReadinessStatusV1::Blocked);
    assert!(
        check
            .blockers
            .iter()
            .any(|finding| { finding.code == "promotion_policy_byte_identity_required" })
    );
}

#[test]
fn promotion_policy_check_blocks_duplicate_policy_roles_without_matching_input() {
    let mut duplicate_policy = sample_role_promotion_policy();
    duplicate_policy.role = "wasm_store".to_string();

    let check = promotion_policy_check_from_inputs(
        "promotion-policy-1",
        &[sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
        &[
            sample_role_promotion_policy(),
            duplicate_policy.clone(),
            duplicate_policy,
        ],
    );

    assert_eq!(check.status, PromotionReadinessStatusV1::Blocked);
    assert!(check.blockers.iter().any(|finding| {
        finding.code == "promotion_policy_duplicate"
            && finding.subject.as_deref() == Some("wasm_store")
    }));
    validate_promotion_policy_check(&check).expect("duplicate-policy blocker should validate");
}

#[test]
fn promotion_policy_check_text_reports_passive_summary() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let policy = sample_role_promotion_policy();
    let check = promotion_policy_check_from_inputs("promotion-policy-1", &[input], &[policy]);

    let text = promotion_policy_check_text(&check);

    assert!(text.contains("Promotion policy check"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("status: ready"));
    assert!(text.contains("check_id: promotion-policy-1"));
    assert!(text.contains("promotion_policy_check_digest:"));
    assert!(text.contains("policy_satisfied: 1"));
    assert!(text.contains("root SealedWasm: policy_satisfied=true"));
}

#[test]
fn promotion_policy_check_round_trips_through_json() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let policy = sample_role_promotion_policy();
    let check = promotion_policy_check_from_inputs("promotion-policy-1", &[input], &[policy]);

    assert_json_round_trip(&check);
    let encoded = serde_json::to_value(&check).expect("promotion policy check should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "check_id",
            "promotion_policy_check_digest",
            "status",
            "roles",
            "blockers",
        ],
    );
    assert_eq!(encoded["check_id"], "promotion-policy-1");
    assert!(
        encoded["promotion_policy_check_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
}

#[test]
fn promotion_policy_check_validation_rejects_stale_decision() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let policy = sample_role_promotion_policy();
    let mut check = promotion_policy_check_from_inputs("promotion-policy-1", &[input], &[policy]);
    check.roles[0].policy_satisfied = false;

    let err = validate_promotion_policy_check(&check).expect_err("stale decision should fail");

    std::assert_matches!(
        err,
        PromotionPolicyCheckError::DecisionMismatch {
            role,
            field: "policy_satisfied"
        } if role == "root"
    );
}

#[test]
fn promotion_policy_check_validation_rejects_stale_digest() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let policy = sample_role_promotion_policy();
    let mut check = promotion_policy_check_from_inputs("promotion-policy-1", &[input], &[policy]);
    check.promotion_policy_check_digest = sample_sha256("9");

    let err = validate_promotion_policy_check(&check)
        .expect_err("stale promotion policy check digest should fail");

    std::assert_matches!(
        err,
        PromotionPolicyCheckError::LinkageMismatch {
            field: "promotion_policy_check_digest"
        }
    );
}
