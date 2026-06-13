use super::*;

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

#[test]
fn promotion_artifact_identity_report_round_trips_through_json() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let report = promotion_artifact_identity_report("promotion-identity-1", &[input]);

    assert_json_round_trip(&report);
    let encoded = serde_json::to_value(&report).expect("identity report should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "report_id",
            "artifact_identity_report_digest",
            "status",
            "summary",
            "roles",
            "identity_groups",
            "blockers",
        ],
    );
    assert_object_keys(
        &encoded["summary"],
        &[
            "role_count",
            "identity_group_count",
            "shared_identity_group_count",
            "digest_pinned_role_count",
            "source_build_role_count",
            "deferred_identity_role_count",
        ],
    );
    assert!(
        encoded["artifact_identity_report_digest"]
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
            "identity_kind",
            "digest_pinned",
            "wasm_sha256",
            "wasm_gz_sha256",
            "candid_sha256",
            "canonical_embedded_config_sha256",
        ],
    );
    let group = &encoded["identity_groups"][0];
    assert_object_keys(
        group,
        &[
            "identity_key",
            "identity_kind",
            "roles",
            "source_kinds",
            "source_locators",
            "digest_pinned",
            "wasm_sha256",
            "wasm_gz_sha256",
            "candid_sha256",
            "canonical_embedded_config_sha256",
        ],
    );
}

#[test]
fn promotion_artifact_identity_report_distinguishes_source_kind_from_identity_kind() {
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.source.kind = RoleArtifactSourceKindV1::LocalWasm;
    input.source.expected_wasm_gz_sha256 = None;

    let report =
        promotion_artifact_identity_report_from_inputs(PromotionArtifactIdentityReportRequest {
            report_id: "promotion-identity-1".to_string(),
            inputs: vec![input],
        })
        .expect("identity report should be produced");

    assert_eq!(report.status, PromotionReadinessStatusV1::Ready);
    assert_eq!(report.roles.len(), 1);
    assert_eq!(
        report.roles[0].source_kind,
        RoleArtifactSourceKindV1::LocalWasm
    );
    assert_eq!(
        report.roles[0].identity_kind,
        PromotionArtifactIdentityKindV1::SealedWasm
    );
    assert!(report.roles[0].digest_pinned);
}

#[test]
fn promotion_artifact_identity_report_groups_roles_by_identity_key() {
    let mut root = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    root.source.kind = RoleArtifactSourceKindV1::LocalWasm;
    root.source.locator = Some("artifacts/root.wasm".to_string());
    root.source.expected_wasm_gz_sha256 = None;

    let mut worker = root.clone();
    worker.role = "worker".to_string();
    worker.source.role = "worker".to_string();
    worker.source.kind = RoleArtifactSourceKindV1::PreviousReceiptArtifact;
    worker.source.locator = Some("receipts/worker.json".to_string());
    worker.source.previous_receipt_kind = Some(PreviousArtifactReceiptKindV1::DeploymentReceipt);

    let report = promotion_artifact_identity_report("promotion-identity-1", &[root, worker]);

    assert_eq!(report.roles.len(), 2);
    assert_eq!(report.identity_groups.len(), 1);
    assert_eq!(report.summary.role_count, 2);
    assert_eq!(report.summary.identity_group_count, 1);
    assert_eq!(report.summary.shared_identity_group_count, 1);
    assert_eq!(report.summary.digest_pinned_role_count, 2);
    let group = &report.identity_groups[0];
    assert_eq!(
        group.identity_kind,
        PromotionArtifactIdentityKindV1::SealedWasm
    );
    assert_eq!(group.roles, vec!["root".to_string(), "worker".to_string()]);
    assert_eq!(
        group.source_kinds,
        vec![
            RoleArtifactSourceKindV1::LocalWasm,
            RoleArtifactSourceKindV1::PreviousReceiptArtifact
        ]
    );
    assert_eq!(
        group.source_locators,
        vec![
            "artifacts/root.wasm".to_string(),
            "receipts/worker.json".to_string()
        ]
    );
    assert_eq!(group.wasm_sha256, Some(sample_sha256("d")));
    assert!(group.identity_key.starts_with("sealed:wasm="));
    validate_promotion_artifact_identity_report(&report)
        .expect("grouped identity report should validate");
}

#[test]
fn promotion_artifact_identity_report_marks_source_build_identity() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);

    let report = promotion_artifact_identity_report("promotion-identity-1", &[input]);

    assert_eq!(report.status, PromotionReadinessStatusV1::Ready);
    assert_eq!(
        report.roles[0].identity_kind,
        PromotionArtifactIdentityKindV1::SourceBuild
    );
    assert_eq!(report.summary.source_build_role_count, 1);
}

#[test]
fn promotion_artifact_identity_report_records_invalid_source_as_blocker() {
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.source.expected_wasm_sha256 = None;
    input.source.expected_wasm_gz_sha256 = None;

    let report = promotion_artifact_identity_report("promotion-identity-1", &[input]);

    assert_eq!(report.status, PromotionReadinessStatusV1::Blocked);
    assert_eq!(report.blockers.len(), 1);
    assert_eq!(report.blockers[0].code, "promotion_artifact_source_invalid");
    assert_eq!(
        report.roles[0].identity_kind,
        PromotionArtifactIdentityKindV1::Deferred
    );
    assert_eq!(report.summary.deferred_identity_role_count, 1);
    validate_promotion_artifact_identity_report(&report)
        .expect("blocked report should still validate");
}

#[test]
fn promotion_artifact_identity_report_text_reports_passive_summary() {
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.source.kind = RoleArtifactSourceKindV1::LocalWasm;
    input.source.expected_wasm_gz_sha256 = None;
    let report = promotion_artifact_identity_report("promotion-identity-1", &[input]);

    let text = promotion_artifact_identity_report_text(&report);

    assert!(text.contains("Promotion artifact identity report"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("status: ready"));
    assert!(text.contains("report_id: promotion-identity-1"));
    assert!(text.contains("artifact_identity_report_digest:"));
    assert!(text.contains("identity_groups: 1"));
    assert!(text.contains("shared_identity_groups: 0"));
    assert!(text.contains("digest_pinned_roles: 1"));
    assert!(text.contains("source_build_roles: 0"));
    assert!(text.contains("deferred_identity_roles: 0"));
    assert!(text.contains("identity groups:"));
    assert!(
        text.contains("root SealedWasm/LocalWasm: identity_kind=SealedWasm digest_pinned=true")
    );
    assert!(text.contains("source_locator: artifacts/root.wasm.gz"));
    assert!(text.contains("wasm_gz_sha256: not recorded"));
}

#[test]
fn promotion_artifact_identity_report_validation_rejects_stale_summary() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let mut report = promotion_artifact_identity_report("promotion-identity-1", &[input]);
    report.summary.identity_group_count = 2;

    let err = validate_promotion_artifact_identity_report(&report)
        .expect_err("stale summary should fail");

    std::assert_matches!(
        err,
        PromotionArtifactIdentityReportError::SummaryMismatch {
            field: "identity_group_count"
        }
    );
}

#[test]
fn promotion_artifact_identity_report_validation_rejects_stale_digest() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let mut report = promotion_artifact_identity_report("promotion-identity-1", &[input]);
    report.artifact_identity_report_digest = sample_sha256("9");

    let err = validate_promotion_artifact_identity_report(&report)
        .expect_err("stale identity report digest should fail");

    std::assert_matches!(
        err,
        PromotionArtifactIdentityReportError::LinkageMismatch {
            field: "artifact_identity_report_digest"
        }
    );
}

#[test]
fn promotion_artifact_identity_report_validation_rejects_duplicate_roles() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let mut report = promotion_artifact_identity_report("promotion-identity-1", &[input]);
    report.roles.push(report.roles[0].clone());

    let err = validate_promotion_artifact_identity_report(&report)
        .expect_err("duplicate role should fail");
    std::assert_matches!(
        err,
        PromotionArtifactIdentityReportError::DuplicateRole { role } if role == "root"
    );
}

#[test]
fn promotion_artifact_identity_report_validation_rejects_status_blocker_mismatch() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let mut report = promotion_artifact_identity_report("promotion-identity-1", &[input]);
    report.status = PromotionReadinessStatusV1::Blocked;

    let err = validate_promotion_artifact_identity_report(&report)
        .expect_err("status blocker mismatch should fail");
    std::assert_matches!(
        err,
        PromotionArtifactIdentityReportError::StatusBlockerMismatch { .. }
    );
}

#[test]
fn promotion_artifact_identity_report_validation_rejects_bad_digest_shape() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let mut report = promotion_artifact_identity_report("promotion-identity-1", &[input]);
    report.roles[0].wasm_gz_sha256 = Some("NOT-A-DIGEST".to_string());

    let err =
        validate_promotion_artifact_identity_report(&report).expect_err("bad digest should fail");
    std::assert_matches!(
        err,
        PromotionArtifactIdentityReportError::InvalidSha256Digest {
            field: "wasm_gz_sha256"
        }
    );
}

#[test]
fn promotion_artifact_identity_report_validation_rejects_stale_group_key() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let mut report = promotion_artifact_identity_report("promotion-identity-1", &[input]);
    report.identity_groups[0].identity_key = "sealed:stale".to_string();

    let err = validate_promotion_artifact_identity_report(&report)
        .expect_err("stale group key should fail");
    std::assert_matches!(
        err,
        PromotionArtifactIdentityReportError::IdentityGroupKeyMismatch { .. }
    );
}

#[test]
fn promotion_artifact_identity_report_validation_rejects_ungrouped_role() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let mut report = promotion_artifact_identity_report("promotion-identity-1", &[input]);
    report.identity_groups.clear();

    let err = validate_promotion_artifact_identity_report(&report)
        .expect_err("ungrouped role should fail");
    std::assert_matches!(
        err,
        PromotionArtifactIdentityReportError::MissingGroupedRole { role } if role == "root"
    );
}

#[test]
fn build_recipe_identity_round_trips_through_json() {
    let recipe = sample_build_recipe_identity();

    validate_build_recipe_identity(&recipe).expect("recipe identity should validate");
    assert_json_round_trip(&recipe);
    let encoded = serde_json::to_value(&recipe).expect("recipe identity should encode");
    assert_object_keys(
        &encoded,
        &[
            "recipe_id",
            "source_kind",
            "source_revision",
            "source_tree_clean",
            "package_or_role_selector",
            "cargo_profile",
            "cargo_features_digest",
            "cargo_lock_digest",
            "rust_toolchain",
            "builder_version",
            "target_triple",
            "linker_identity",
            "deterministic_build_mode",
            "wasm_opt_version",
            "compression_identity",
        ],
    );
}

#[test]
fn build_recipe_identity_validation_rejects_dirty_ambiguous_revision() {
    let mut recipe = sample_build_recipe_identity();
    recipe.source_revision = " ".to_string();

    let err = validate_build_recipe_identity(&recipe).expect_err("blank revision should fail");

    std::assert_matches!(
        err,
        PromotionMaterializationIdentityError::MissingRequiredField {
            field: "source_revision"
        }
    );
}

#[test]
fn build_materialization_input_round_trips_through_json() {
    let input = sample_build_materialization_input();

    validate_build_materialization_input(&input).expect("materialization input should validate");
    assert_json_round_trip(&input);
    let encoded = serde_json::to_value(&input).expect("materialization input should encode");
    assert_object_keys(
        &encoded,
        &[
            "materialization_input_id",
            "build_recipe_id",
            "canonical_embedded_config_sha256",
            "network",
            "root_trust_anchor",
            "runtime_variant",
        ],
    );
}

#[test]
fn build_materialization_input_validation_rejects_bad_config_digest() {
    let mut input = sample_build_materialization_input();
    input.canonical_embedded_config_sha256 = "bad-digest".to_string();

    let err =
        validate_build_materialization_input(&input).expect_err("bad config digest should fail");

    std::assert_matches!(
        err,
        PromotionMaterializationIdentityError::InvalidSha256Digest {
            field: "canonical_embedded_config_sha256"
        }
    );
}

#[test]
fn build_materialization_result_round_trips_through_json() {
    let result = sample_build_materialization_result();

    validate_build_materialization_result(&result).expect("materialization result should validate");
    assert_json_round_trip(&result);
    let encoded = serde_json::to_value(&result).expect("materialization result should encode");
    assert_object_keys(
        &encoded,
        &[
            "materialization_result_id",
            "build_recipe_id",
            "materialization_input_digest",
            "wasm_sha256",
            "wasm_gz_sha256",
            "installed_module_hash",
            "candid_sha256",
        ],
    );
}

#[test]
fn build_materialization_result_validation_rejects_bad_output_digest() {
    let mut result = sample_build_materialization_result();
    result.wasm_sha256 = "BAD".to_string();

    let err =
        validate_build_materialization_result(&result).expect_err("bad output digest should fail");

    std::assert_matches!(
        err,
        PromotionMaterializationIdentityError::InvalidSha256Digest {
            field: "wasm_sha256"
        }
    );
}

#[test]
fn build_materialization_evidence_round_trips_through_json() {
    let input = sample_build_materialization_input();
    let mut result = sample_build_materialization_result();
    result.materialization_input_digest = build_materialization_input_digest(&input);

    let evidence = build_materialization_evidence(BuildMaterializationEvidenceRequest {
        evidence_id: "materialization-evidence-1".to_string(),
        recipe: sample_build_recipe_identity(),
        materialization_input: input,
        materialization_result: result,
    })
    .expect("materialization evidence should validate");

    assert!(evidence.recipe_id_matches_input);
    assert!(evidence.recipe_id_matches_result);
    assert!(evidence.materialization_input_digest_matches_result);
    assert_json_round_trip(&evidence);
    let encoded = serde_json::to_value(&evidence).expect("materialization evidence should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "evidence_id",
            "materialization_evidence_digest",
            "recipe",
            "materialization_input",
            "materialization_result",
            "computed_materialization_input_digest",
            "recipe_id_matches_input",
            "recipe_id_matches_result",
            "materialization_input_digest_matches_result",
        ],
    );
    assert!(
        encoded["materialization_evidence_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
}

#[test]
fn build_materialization_evidence_text_reports_passive_boundary() {
    let input = sample_build_materialization_input();
    let mut result = sample_build_materialization_result();
    result.materialization_input_digest = build_materialization_input_digest(&input);
    let evidence = build_materialization_evidence(BuildMaterializationEvidenceRequest {
        evidence_id: "materialization-evidence-1".to_string(),
        recipe: sample_build_recipe_identity(),
        materialization_input: input,
        materialization_result: result,
    })
    .expect("materialization evidence should validate");

    let text = build_materialization_evidence_text(&evidence);

    assert!(text.contains("Build materialization evidence"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("evidence_id: materialization-evidence-1"));
    assert!(text.contains("materialization_evidence_digest:"));
    assert!(text.contains("recipe_id_matches_input: true"));
    assert!(text.contains("recipe_id_matches_result: true"));
    assert!(text.contains("materialization_input_digest_matches_result: true"));
    assert!(text.contains("execution: none"));
}

#[test]
fn build_materialization_evidence_validation_rejects_stale_computed_digest() {
    let input = sample_build_materialization_input();
    let mut result = sample_build_materialization_result();
    result.materialization_input_digest = build_materialization_input_digest(&input);
    let mut evidence = build_materialization_evidence(BuildMaterializationEvidenceRequest {
        evidence_id: "materialization-evidence-1".to_string(),
        recipe: sample_build_recipe_identity(),
        materialization_input: input,
        materialization_result: result,
    })
    .expect("materialization evidence should validate");
    evidence.computed_materialization_input_digest = sample_sha256("9");

    let err = validate_build_materialization_evidence(&evidence)
        .expect_err("stale computed digest should fail");

    std::assert_matches!(
        err,
        PromotionMaterializationIdentityError::DigestMismatch {
            field: "computed_materialization_input_digest",
            ..
        }
    );
}

#[test]
fn build_materialization_evidence_validation_rejects_stale_link_flag() {
    let input = sample_build_materialization_input();
    let mut result = sample_build_materialization_result();
    result.materialization_input_digest = build_materialization_input_digest(&input);
    let mut evidence = build_materialization_evidence(BuildMaterializationEvidenceRequest {
        evidence_id: "materialization-evidence-1".to_string(),
        recipe: sample_build_recipe_identity(),
        materialization_input: input,
        materialization_result: result,
    })
    .expect("materialization evidence should validate");
    evidence.recipe_id_matches_input = false;

    let err =
        validate_build_materialization_evidence(&evidence).expect_err("stale flag should fail");

    std::assert_matches!(
        err,
        PromotionMaterializationIdentityError::LinkageMismatch {
            field: "recipe_id_matches_input"
        }
    );
}

#[test]
fn build_materialization_evidence_validation_rejects_stale_digest() {
    let input = sample_build_materialization_input();
    let mut result = sample_build_materialization_result();
    result.materialization_input_digest = build_materialization_input_digest(&input);
    let mut evidence = build_materialization_evidence(BuildMaterializationEvidenceRequest {
        evidence_id: "materialization-evidence-1".to_string(),
        recipe: sample_build_recipe_identity(),
        materialization_input: input,
        materialization_result: result,
    })
    .expect("materialization evidence should validate");
    evidence.materialization_evidence_digest = sample_sha256("9");

    let err =
        validate_build_materialization_evidence(&evidence).expect_err("stale digest should fail");

    std::assert_matches!(
        err,
        PromotionMaterializationIdentityError::LinkageMismatch {
            field: "materialization_evidence_digest"
        }
    );
}

#[test]
fn build_materialization_evidence_rejects_mismatched_result_input_digest() {
    let input = sample_build_materialization_input();
    let result = sample_build_materialization_result();

    let err = build_materialization_evidence(BuildMaterializationEvidenceRequest {
        evidence_id: "materialization-evidence-1".to_string(),
        recipe: sample_build_recipe_identity(),
        materialization_input: input,
        materialization_result: result,
    })
    .expect_err("mismatched result input digest should fail");

    std::assert_matches!(
        err,
        PromotionMaterializationIdentityError::LinkageMismatch {
            field: "materialization_input_digest_matches_result"
        }
    );
}

#[test]
fn promotion_materialization_identity_report_round_trips_through_json() {
    let report = promotion_materialization_identity_report_from_evidence(
        PromotionMaterializationIdentityReportRequest {
            report_id: "materialization-report-1".to_string(),
            evidence: vec![sample_build_materialization_evidence()],
        },
    )
    .expect("materialization report should validate");

    assert_json_round_trip(&report);
    let encoded = serde_json::to_value(&report).expect("materialization report should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "report_id",
            "materialization_identity_report_digest",
            "status",
            "roles",
            "output_groups",
            "blockers",
        ],
    );
    assert_eq!(encoded["report_id"], "materialization-report-1");
    assert!(
        encoded["materialization_identity_report_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    assert_eq!(encoded["status"], "Ready");
    assert_eq!(encoded["roles"][0]["role"], "root");
    assert!(
        encoded["roles"][0]["materialization_evidence_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    assert_eq!(encoded["output_groups"][0]["roles"][0], "root");
}

#[test]
fn promotion_materialization_identity_report_groups_roles_by_output_identity() {
    let mut recipe = sample_build_recipe_identity();
    recipe.package_or_role_selector = "user_hub".to_string();
    let second = build_materialization_evidence(BuildMaterializationEvidenceRequest {
        evidence_id: "materialization-evidence-2".to_string(),
        recipe,
        materialization_input: sample_build_materialization_input(),
        materialization_result: {
            let input = sample_build_materialization_input();
            let mut result = sample_build_materialization_result();
            result.materialization_input_digest = build_materialization_input_digest(&input);
            result
        },
    })
    .expect("second materialization evidence should validate");
    let report = promotion_materialization_identity_report_from_evidence(
        PromotionMaterializationIdentityReportRequest {
            report_id: "materialization-report-1".to_string(),
            evidence: vec![sample_build_materialization_evidence(), second],
        },
    )
    .expect("materialization report should validate");

    assert_eq!(report.roles.len(), 2);
    assert_eq!(report.output_groups.len(), 1);
    assert_eq!(
        report.output_groups[0].roles,
        vec!["root".to_string(), "user_hub".to_string()]
    );
}

#[test]
fn promotion_materialization_identity_report_validation_rejects_stale_output_group() {
    let mut report = promotion_materialization_identity_report_from_evidence(
        PromotionMaterializationIdentityReportRequest {
            report_id: "materialization-report-1".to_string(),
            evidence: vec![sample_build_materialization_evidence()],
        },
    )
    .expect("materialization report should validate");
    report.output_groups[0].output_identity_key = "stale".to_string();

    let err = validate_promotion_materialization_identity_report(&report)
        .expect_err("stale output group should fail");

    std::assert_matches!(
        err,
        PromotionMaterializationIdentityReportError::OutputGroupKeyMismatch { .. }
            | PromotionMaterializationIdentityReportError::OutputGroupRoleMismatch { .. }
    );
}

#[test]
fn promotion_materialization_identity_report_validation_rejects_stale_digest() {
    let mut report = promotion_materialization_identity_report_from_evidence(
        PromotionMaterializationIdentityReportRequest {
            report_id: "materialization-report-1".to_string(),
            evidence: vec![sample_build_materialization_evidence()],
        },
    )
    .expect("materialization report should validate");
    report.materialization_identity_report_digest = sample_sha256("9");

    let err = validate_promotion_materialization_identity_report(&report)
        .expect_err("stale materialization report digest should fail");

    std::assert_matches!(
        err,
        PromotionMaterializationIdentityReportError::LinkageMismatch {
            field: "materialization_identity_report_digest"
        }
    );
}

#[test]
fn promotion_materialization_identity_report_validation_rejects_duplicate_evidence() {
    let mut report = promotion_materialization_identity_report_from_evidence(
        PromotionMaterializationIdentityReportRequest {
            report_id: "materialization-report-1".to_string(),
            evidence: vec![sample_build_materialization_evidence()],
        },
    )
    .expect("materialization report should validate");
    let mut duplicate = report.roles[0].clone();
    duplicate.role = "user_hub".to_string();
    report.roles.push(duplicate);
    report.output_groups[0].roles.push("user_hub".to_string());

    let err = validate_promotion_materialization_identity_report(&report)
        .expect_err("duplicate evidence ids should fail");

    std::assert_matches!(
        err,
        PromotionMaterializationIdentityReportError::DuplicateEvidence { .. }
    );
}

#[test]
fn promotion_materialization_identity_report_text_reports_passive_summary() {
    let report = promotion_materialization_identity_report_from_evidence(
        PromotionMaterializationIdentityReportRequest {
            report_id: "materialization-report-1".to_string(),
            evidence: vec![sample_build_materialization_evidence()],
        },
    )
    .expect("materialization report should validate");

    let text = promotion_materialization_identity_report_text(&report);

    assert!(text.contains("Promotion materialization identity report"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("report_id: materialization-report-1"));
    assert!(text.contains("materialization_identity_report_digest:"));
    assert!(text.contains("output_groups: 1"));
    assert!(text.contains("root evidence=materialization-evidence-1 recipe=recipe:root:debug"));
}

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

#[test]
fn promotion_plan_transform_evidence_validation_accepts_generated_evidence() {
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

    validate_promotion_plan_transform_evidence(&evidence)
        .expect("generated evidence should validate");
}

#[test]
fn promotion_plan_transform_evidence_text_reports_passive_boundary() {
    let mut target_plan = sample_promotion_target_plan();
    target_plan.role_artifacts[0].wasm_gz_sha256 = Some(sample_sha256("f"));
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.require_byte_identical_wasm = false;
    let transform =
        promoted_deployment_plan_transform_from_inputs(&PromotionPlanTransformRequest {
            promoted_plan_id: "promoted-plan-1".to_string(),
            target_plan,
            inputs: vec![input],
        })
        .expect("transform should be produced");
    let evidence = promotion_plan_transform_evidence(PromotionPlanTransformEvidenceRequest {
        evidence_id: "promotion-evidence-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
    })
    .expect("evidence should be produced");

    let text = promotion_plan_transform_evidence_text(&evidence);

    assert!(text.contains("Promotion plan transform evidence"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("evidence_id: promotion-evidence-1"));
    assert!(text.contains("promotion_plan_transform_evidence_digest:"));
    assert!(text.contains("generated_at: 2026-05-25T00:00:00Z"));
    assert!(text.contains("transform_id: promotion-transform:promoted-plan-1"));
    assert!(text.contains("  Promotion plan transform"));
    assert!(text.contains("  mode: passive"));
    assert!(text.contains("  artifact_identity_changed: 1"));
}

#[test]
fn promotion_plan_transform_evidence_validation_rejects_blank_evidence_id() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    let err = promotion_plan_transform_evidence(PromotionPlanTransformEvidenceRequest {
        evidence_id: " ".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
    })
    .expect_err("blank evidence id should fail");

    std::assert_matches!(
        err,
        PromotionPlanTransformEvidenceError::MissingRequiredField {
            field: "evidence_id"
        }
    );
}

#[test]
fn promotion_plan_transform_evidence_validation_rejects_schema_drift() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    let mut evidence = promotion_plan_transform_evidence(PromotionPlanTransformEvidenceRequest {
        evidence_id: "promotion-evidence-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
    })
    .expect("evidence should be produced");
    evidence.schema_version += 1;

    let err = validate_promotion_plan_transform_evidence(&evidence)
        .expect_err("schema drift should fail");
    std::assert_matches!(
        err,
        PromotionPlanTransformEvidenceError::SchemaVersionMismatch { .. }
    );
}

#[test]
fn promotion_plan_transform_evidence_validation_rejects_stale_digest() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    let mut evidence = promotion_plan_transform_evidence(PromotionPlanTransformEvidenceRequest {
        evidence_id: "promotion-evidence-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
    })
    .expect("evidence should be produced");
    evidence.promotion_plan_transform_evidence_digest = sample_sha256("9");

    let err = validate_promotion_plan_transform_evidence(&evidence)
        .expect_err("stale evidence digest should fail");
    std::assert_matches!(
        err,
        PromotionPlanTransformEvidenceError::LinkageMismatch {
            field: "promotion_plan_transform_evidence_digest"
        }
    );
}

#[test]
fn promotion_plan_transform_evidence_validation_rejects_stale_transform() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    let mut evidence = promotion_plan_transform_evidence(PromotionPlanTransformEvidenceRequest {
        evidence_id: "promotion-evidence-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
    })
    .expect("evidence should be produced");
    evidence.transform.roles[0].artifact_identity_changed = false;

    let err = validate_promotion_plan_transform_evidence(&evidence)
        .expect_err("stale transform should fail");
    std::assert_matches!(
        err,
        PromotionPlanTransformEvidenceError::Transform(
            PromotionPlanTransformError::RoleStateMismatch {
                role,
                field: "artifact_identity_changed"
            }
        ) if role == "root"
    );
}

#[test]
fn promotion_target_execution_lineage_round_trips_through_json() {
    let transform = sample_promotion_transform();
    let preflight = sample_execution_preflight_for_plan("promoted-plan-1");

    let lineage = promotion_target_execution_lineage(PromotionTargetExecutionLineageRequest {
        lineage_id: "target-execution-lineage-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
        execution_preflight: preflight,
    })
    .expect("target execution lineage should be produced");

    assert_json_round_trip(&lineage);
    let encoded = serde_json::to_value(&lineage).expect("lineage should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "lineage_id",
            "generated_at",
            "target_execution_lineage_digest",
            "transform",
            "execution_preflight",
            "execution_attempted",
        ],
    );
    assert_eq!(encoded["lineage_id"], "target-execution-lineage-1");
    assert_eq!(encoded["execution_attempted"], false);
}

#[test]
fn promotion_target_execution_lineage_validation_accepts_generated_lineage() {
    let transform = sample_promotion_transform();
    let preflight = sample_execution_preflight_for_plan("promoted-plan-1");

    let lineage = promotion_target_execution_lineage(PromotionTargetExecutionLineageRequest {
        lineage_id: "target-execution-lineage-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
        execution_preflight: preflight,
    })
    .expect("target execution lineage should be produced");

    validate_promotion_target_execution_lineage(&lineage)
        .expect("generated lineage should validate");
}

#[test]
fn promotion_target_execution_lineage_rejects_preflight_for_other_plan() {
    let transform = sample_promotion_transform();
    let preflight = sample_execution_preflight_for_plan("other-promoted-plan");

    let err = promotion_target_execution_lineage(PromotionTargetExecutionLineageRequest {
        lineage_id: "target-execution-lineage-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
        execution_preflight: preflight,
    })
    .expect_err("preflight for another plan should fail");

    std::assert_matches!(
        err,
        PromotionTargetExecutionLineageError::LinkageMismatch {
            field: "execution_preflight.plan_id"
        }
    );
}

#[test]
fn promotion_target_execution_lineage_rejects_execution_claim() {
    let transform = sample_promotion_transform();
    let preflight = sample_execution_preflight_for_plan("promoted-plan-1");
    let mut lineage = promotion_target_execution_lineage(PromotionTargetExecutionLineageRequest {
        lineage_id: "target-execution-lineage-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
        execution_preflight: preflight,
    })
    .expect("target execution lineage should be produced");
    lineage.execution_attempted = true;

    let err = validate_promotion_target_execution_lineage(&lineage)
        .expect_err("execution claim should fail");

    std::assert_matches!(
        err,
        PromotionTargetExecutionLineageError::ExecutionAttempted
    );
}

#[test]
fn promotion_target_execution_lineage_rejects_stale_digest() {
    let transform = sample_promotion_transform();
    let preflight = sample_execution_preflight_for_plan("promoted-plan-1");
    let mut lineage = promotion_target_execution_lineage(PromotionTargetExecutionLineageRequest {
        lineage_id: "target-execution-lineage-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
        execution_preflight: preflight,
    })
    .expect("target execution lineage should be produced");
    lineage.target_execution_lineage_digest = sample_sha256("9");

    let err = validate_promotion_target_execution_lineage(&lineage)
        .expect_err("stale lineage digest should fail");

    std::assert_matches!(
        err,
        PromotionTargetExecutionLineageError::LinkageMismatch {
            field: "target_execution_lineage_digest"
        }
    );
}

#[test]
fn promotion_target_execution_lineage_text_reports_passive_boundary() {
    let transform = sample_promotion_transform();
    let preflight = sample_execution_preflight_for_plan("promoted-plan-1");
    let lineage = promotion_target_execution_lineage(PromotionTargetExecutionLineageRequest {
        lineage_id: "target-execution-lineage-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
        execution_preflight: preflight,
    })
    .expect("target execution lineage should be produced");

    let text = promotion_target_execution_lineage_text(&lineage);

    assert!(text.contains("Promotion target execution lineage"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("execution_attempted: false"));
    assert!(text.contains("lineage_id: target-execution-lineage-1"));
    assert!(text.contains("target_execution_lineage_digest: "));
    assert!(text.contains("transform_id: promotion-transform:promoted-plan-1"));
    assert!(text.contains("preflight_plan_id: promoted-plan-1"));
    assert!(text.contains("  Deployment execution preflight"));
}

#[test]
fn artifact_promotion_plan_round_trips_through_json() {
    let plan = sample_artifact_promotion_plan();

    assert_json_round_trip(&plan);
    let encoded = serde_json::to_value(&plan).expect("promotion plan should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "plan_id",
            "artifact_promotion_plan_digest",
            "generated_at",
            "status",
            "target_plan_id",
            "promoted_plan_id",
            "promotion_plan_lineage_digest",
            "readiness",
            "artifact_identity_report",
            "transform",
            "target_execution_lineage",
            "blockers",
        ],
    );
    assert_eq!(encoded["plan_id"], "artifact-promotion-plan-1");
    assert_eq!(encoded["status"], "Ready");
    assert!(
        encoded["artifact_promotion_plan_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
}

#[test]
fn artifact_promotion_plan_validation_accepts_generated_plan() {
    let plan = sample_artifact_promotion_plan();

    validate_artifact_promotion_plan(&plan).expect("generated promotion plan should validate");
}

#[test]
fn artifact_promotion_plan_validation_rejects_status_blocker_mismatch() {
    let mut plan = sample_artifact_promotion_plan();
    plan.blockers.push(SafetyFindingV1 {
        code: "promotion_blocker".to_string(),
        message: "blocked".to_string(),
        severity: SafetySeverityV1::HardFailure,
        subject: Some("root".to_string()),
    });

    let err =
        validate_artifact_promotion_plan(&plan).expect_err("ready plan with blockers should fail");

    std::assert_matches!(
        err,
        ArtifactPromotionPlanError::StatusBlockerMismatch { .. }
    );
}

#[test]
fn artifact_promotion_plan_validation_rejects_stale_lineage_copy() {
    let mut plan = sample_artifact_promotion_plan();
    plan.promotion_plan_lineage_digest = sample_sha256("9");

    let err =
        validate_artifact_promotion_plan(&plan).expect_err("stale plan lineage copy should fail");

    std::assert_matches!(
        err,
        ArtifactPromotionPlanError::LinkageMismatch {
            field: "promotion_plan_lineage_digest"
        }
    );
}

#[test]
fn artifact_promotion_plan_validation_rejects_stale_digest() {
    let mut plan = sample_artifact_promotion_plan();
    plan.artifact_promotion_plan_digest = sample_sha256("9");

    let err = validate_artifact_promotion_plan(&plan).expect_err("stale plan digest should fail");

    std::assert_matches!(
        err,
        ArtifactPromotionPlanError::LinkageMismatch {
            field: "artifact_promotion_plan_digest"
        }
    );
}

#[test]
fn artifact_promotion_plan_validation_rejects_mismatched_target_lineage() {
    let mut plan = sample_artifact_promotion_plan();
    let mut lineage = plan
        .target_execution_lineage
        .clone()
        .expect("sample plan should carry target lineage");
    lineage.transform.transform_id = "different-transform".to_string();
    plan.target_execution_lineage = Some(lineage);

    let err = validate_artifact_promotion_plan(&plan)
        .expect_err("target lineage with different transform should fail");

    std::assert_matches!(
        err,
        ArtifactPromotionPlanError::LinkageMismatch {
            field: "target_execution_lineage.transform"
        }
    );
}

#[test]
fn artifact_promotion_plan_for_check_accepts_matching_promoted_plan_check() {
    let plan = sample_artifact_promotion_plan();
    let check = sample_check(
        plan.transform.promoted_plan.clone(),
        sample_matching_inventory(),
    );

    validate_artifact_promotion_plan_for_check(&plan, &check)
        .expect("promotion plan should validate against target check");
}

#[test]
fn artifact_promotion_plan_for_check_rejects_other_target_plan() {
    let plan = sample_artifact_promotion_plan();
    let mut other_plan = sample_promotion_target_plan();
    other_plan.plan_id = "other-target-plan".to_string();
    let check = sample_check(other_plan, sample_matching_inventory());

    let err = validate_artifact_promotion_plan_for_check(&plan, &check)
        .expect_err("target check for another plan should fail");

    std::assert_matches!(
        err,
        ArtifactPromotionPlanError::LinkageMismatch {
            field: "target_check.plan"
        }
    );
}

#[test]
fn artifact_promotion_plan_for_check_rejects_missing_target_execution_lineage() {
    let sample = sample_artifact_promotion_plan();
    let promoted_plan = sample.transform.promoted_plan.clone();
    let plan = artifact_promotion_plan(ArtifactPromotionPlanRequest {
        plan_id: sample.plan_id,
        generated_at: sample.generated_at,
        readiness: sample.readiness,
        artifact_identity_report: sample.artifact_identity_report,
        transform: sample.transform,
        target_execution_lineage: None,
    })
    .expect("sample plan without lineage should still validate");
    let check = sample_check(promoted_plan, sample_matching_inventory());

    let err = validate_artifact_promotion_plan_for_check(&plan, &check)
        .expect_err("target check validation should require execution lineage");

    std::assert_matches!(
        err,
        ArtifactPromotionPlanError::MissingTargetExecutionLineage
    );
}

#[test]
fn artifact_promotion_plan_for_check_rejects_preflight_check_mismatch() {
    let plan = sample_artifact_promotion_plan();
    let mut check = sample_check(
        plan.transform.promoted_plan.clone(),
        sample_matching_inventory(),
    );
    check.report.report_id = "other-report".to_string();

    let err = validate_artifact_promotion_plan_for_check(&plan, &check)
        .expect_err("preflight mismatch should fail");

    std::assert_matches!(
        err,
        ArtifactPromotionPlanError::TargetCheck(
            DeploymentExecutionPreflightError::SourceCheckMismatch {
                field: "safety_report_id",
                ..
            }
        )
    );
}

#[test]
fn artifact_promotion_plan_text_reports_passive_summary() {
    let plan = sample_artifact_promotion_plan();

    let text = artifact_promotion_plan_text(&plan);

    assert!(text.contains("Artifact promotion plan"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("plan_id: artifact-promotion-plan-1"));
    assert!(text.contains("artifact_promotion_plan_digest:"));
    assert!(text.contains("target_execution_lineage: target-execution-lineage-1"));
    assert!(text.contains("readiness_roles: 1"));
    assert!(text.contains("artifact_identity_roles: 1"));
    assert!(text.contains("transform_roles: 1"));
    assert!(text.contains("  Promotion readiness report"));
    assert!(text.contains("  Promotion artifact identity report"));
    assert!(text.contains("  Promotion plan transform"));
}

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
    assert!(text.contains("deployment_phase_receipts: 1"));
    assert!(text.contains("root SealedWasm: result=Applied"));
    assert!(text.contains("catalog_digest="));
}

#[test]
fn promotion_plan_transform_validation_accepts_generated_transform() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");

    validate_promotion_plan_transform(&transform).expect("generated transform should validate");
}

#[test]
fn promotion_plan_transform_validation_rejects_schema_drift() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let mut transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    transform.schema_version += 1;

    let err = validate_promotion_plan_transform(&transform).expect_err("schema drift should fail");
    std::assert_matches!(
        err,
        PromotionPlanTransformError::SchemaVersionMismatch { .. }
    );
}

#[test]
fn promotion_plan_transform_validation_rejects_plan_id_mismatch() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let mut transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    transform.promoted_plan.plan_id = "different-plan".to_string();

    let err =
        validate_promotion_plan_transform(&transform).expect_err("plan id mismatch should fail");
    std::assert_matches!(
        err,
        PromotionPlanTransformError::PromotedPlanIdMismatch { .. }
    );
}

#[test]
fn promotion_plan_transform_validation_rejects_duplicate_roles() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let mut transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    transform.roles.push(transform.roles[0].clone());

    let err =
        validate_promotion_plan_transform(&transform).expect_err("duplicate role should fail");
    std::assert_matches!(
        err,
        PromotionPlanTransformError::DuplicateRole { role } if role == "root"
    );
}

#[test]
fn promotion_plan_transform_validation_rejects_missing_promoted_role() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let mut transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    transform.promoted_plan.role_artifacts.clear();

    let err = validate_promotion_plan_transform(&transform)
        .expect_err("missing promoted role should fail");
    std::assert_matches!(
        err,
        PromotionPlanTransformError::PromotedRoleMissing { role } if role == "root"
    );
}

#[test]
fn promotion_plan_transform_validation_rejects_stale_lineage_digest() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let mut transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    transform.promotion_plan_lineage_digest = sample_sha256("9");

    let err = validate_promotion_plan_transform(&transform)
        .expect_err("stale lineage digest should fail");
    std::assert_matches!(
        err,
        PromotionPlanTransformError::RoleStateMismatch {
            role,
            field: "promotion_plan_lineage_digest"
        } if role == "promotion_plan_lineage"
    );
}

#[test]
fn promotion_plan_lineage_digest_changes_when_materialization_link_changes() {
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
    let mut changed_roles = transform.roles.clone();
    changed_roles[0]
        .source_build_materialization
        .as_mut()
        .expect("materialization link should exist")
        .evidence_id = "different-evidence".to_string();

    let changed_digest = promotion_plan_lineage_digest(
        &transform.target_plan_id,
        &transform.promoted_plan_id,
        &transform.promoted_plan,
        &changed_roles,
    );

    assert_ne!(changed_digest, transform.promotion_plan_lineage_digest);
}

#[test]
fn promotion_plan_transform_validation_rejects_stale_after_summary() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let mut transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    transform.roles[0].wasm_gz_sha256_after = Some(sample_sha256("f"));

    let err = validate_promotion_plan_transform(&transform).expect_err("stale summary should fail");
    std::assert_matches!(
        err,
        PromotionPlanTransformError::RoleStateMismatch {
            role,
            field: "wasm_gz_sha256_after"
        } if role == "root"
    );
}

#[test]
fn promotion_plan_transform_validation_rejects_stale_change_flag() {
    let mut target_plan = sample_promotion_target_plan();
    target_plan.role_artifacts[0].wasm_gz_sha256 = Some(sample_sha256("f"));
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.require_byte_identical_wasm = false;
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan,
        inputs: vec![input],
    };
    let mut transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    transform.roles[0].artifact_identity_changed = false;

    let err = validate_promotion_plan_transform(&transform).expect_err("stale flag should fail");
    std::assert_matches!(
        err,
        PromotionPlanTransformError::RoleStateMismatch {
            role,
            field: "artifact_identity_changed"
        } if role == "root"
    );
}

#[test]
fn role_artifact_source_requires_digest_pins_for_executable_overrides() {
    let mut source = sample_role_artifact_source(RoleArtifactSourceKindV1::LocalWasm);
    source.expected_wasm_sha256 = None;
    source.expected_wasm_gz_sha256 = None;

    let err = validate_role_artifact_source(&source).expect_err("digest pin should be required");
    std::assert_matches!(
        err,
        PromotionArtifactSourceError::MissingDigestPin {
            kind: RoleArtifactSourceKindV1::LocalWasm
        }
    );
}

#[test]
fn role_artifact_source_rejects_invalid_digest_shape() {
    let mut source = sample_role_artifact_source(RoleArtifactSourceKindV1::LocalWasmGz);
    source.expected_wasm_gz_sha256 = Some("NOT-A-DIGEST".to_string());

    let err = validate_role_artifact_source(&source).expect_err("digest should be checked");
    std::assert_matches!(
        err,
        PromotionArtifactSourceError::InvalidSha256Digest {
            field: "expected_wasm_gz_sha256"
        }
    );
}

#[test]
fn previous_receipt_artifact_source_requires_eligible_receipt_kind() {
    let mut source = sample_role_artifact_source(RoleArtifactSourceKindV1::PreviousReceiptArtifact);
    source.previous_receipt_kind = None;

    let err = validate_role_artifact_source(&source).expect_err("receipt kind should be required");
    std::assert_matches!(
        err,
        PromotionArtifactSourceError::MissingPreviousReceiptKind
    );

    source.previous_receipt_kind = Some(PreviousArtifactReceiptKindV1::DeploymentReceipt);
    validate_role_artifact_source(&source).expect("deployment receipt artifact should be eligible");
    source.previous_receipt_kind = Some(PreviousArtifactReceiptKindV1::StagingReceipt);
    validate_role_artifact_source(&source).expect("staging receipt artifact should be eligible");
}

#[test]
fn previous_receipt_artifact_source_requires_lineage_digest() {
    let mut source = sample_role_artifact_source(RoleArtifactSourceKindV1::PreviousReceiptArtifact);
    source.previous_receipt_lineage_digest = None;

    let err = validate_role_artifact_source(&source)
        .expect_err("receipt lineage digest should be required");

    std::assert_matches!(
        err,
        PromotionArtifactSourceError::MissingPreviousReceiptLineageDigest
    );
}

#[test]
fn previous_receipt_artifact_source_rejects_invalid_lineage_digest() {
    let mut source = sample_role_artifact_source(RoleArtifactSourceKindV1::PreviousReceiptArtifact);
    source.previous_receipt_lineage_digest = Some("bad-digest".to_string());

    let err =
        validate_role_artifact_source(&source).expect_err("receipt lineage digest should validate");

    std::assert_matches!(
        err,
        PromotionArtifactSourceError::InvalidSha256Digest {
            field: "previous_receipt_lineage_digest"
        }
    );
}

#[test]
fn non_receipt_artifact_source_rejects_previous_receipt_kind() {
    let mut source = sample_role_artifact_source(RoleArtifactSourceKindV1::LocalWasmGz);
    source.previous_receipt_kind = Some(PreviousArtifactReceiptKindV1::DeploymentReceipt);

    let err =
        validate_role_artifact_source(&source).expect_err("receipt kind should be source-specific");
    std::assert_matches!(
        err,
        PromotionArtifactSourceError::UnexpectedPreviousReceiptKind {
            kind: RoleArtifactSourceKindV1::LocalWasmGz
        }
    );
}

#[test]
fn non_receipt_artifact_source_rejects_previous_receipt_lineage_digest() {
    let mut source = sample_role_artifact_source(RoleArtifactSourceKindV1::LocalWasmGz);
    source.previous_receipt_lineage_digest = Some(sample_sha256("9"));

    let err = validate_role_artifact_source(&source)
        .expect_err("receipt lineage digest should be source-specific");

    std::assert_matches!(
        err,
        PromotionArtifactSourceError::UnexpectedPreviousReceiptLineageDigest {
            kind: RoleArtifactSourceKindV1::LocalWasmGz
        }
    );
}

#[test]
fn canonical_wasm_store_default_source_does_not_require_locator_or_digest_pin() {
    let source = RoleArtifactSourceV1 {
        role: "wasm_store".to_string(),
        kind: RoleArtifactSourceKindV1::CanonicalWasmStoreDefault,
        locator: None,
        previous_receipt_kind: None,
        previous_receipt_lineage_digest: None,
        expected_wasm_sha256: None,
        expected_wasm_gz_sha256: None,
        expected_candid_sha256: None,
        expected_canonical_embedded_config_sha256: None,
    };

    validate_role_artifact_source(&source).expect("canonical source should be deferred");
}

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
