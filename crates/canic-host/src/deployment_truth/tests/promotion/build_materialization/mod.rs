use super::super::*;

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
            "environment",
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
