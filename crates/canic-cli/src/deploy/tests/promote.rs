use super::super::output_format::PromotionOutputFormat;
use super::super::promote::*;
use super::*;

#[test]
fn deploy_promote_leaf_commands_default_to_json() {
    for (command, usage, request) in promote_leaf_commands() {
        let options = DeployPromoteReportOptions::parse(
            [OsString::from("--request"), OsString::from(request)],
            command,
            usage,
        )
        .expect("parse promote leaf command");

        assert_eq!(options.request, PathBuf::from(request));
        assert_eq!(options.format, PromotionOutputFormat::Json);
    }
}

#[test]
fn deploy_promote_leaf_commands_parse_text_format() {
    for (command, usage, request) in promote_leaf_commands() {
        let options = DeployPromoteReportOptions::parse(
            [
                OsString::from("--request"),
                OsString::from(request),
                OsString::from("--format"),
                OsString::from("text"),
            ],
            command,
            usage,
        )
        .expect("parse promote leaf command text");

        assert_eq!(options.request, PathBuf::from(request));
        assert_eq!(options.format, PromotionOutputFormat::Text);
    }
}

#[test]
fn deploy_promote_help_documents_passive_scope() {
    let help = promote_usage();
    let readiness_help = promote_readiness_usage();
    let check_help = promote_check_usage();
    let artifact_identity_help = promote_artifact_identity_usage();
    let transform_help = promote_transform_usage();
    let diff_help = promote_diff_usage();
    let transform_evidence_help = promote_transform_evidence_usage();
    let target_lineage_help = promote_target_lineage_usage();
    let plan_help = promote_plan_usage();
    let provenance_help = promote_provenance_usage();
    let wasm_store_identity_help = promote_wasm_store_identity_usage();
    let catalog_verification_help = promote_catalog_verification_usage();
    let execution_receipt_help = promote_execution_receipt_usage();
    let policy_help = promote_policy_check_usage();
    let materialization_help = promote_materialization_identity_usage();

    assert!(help.contains("Build passive artifact promotion reports"));
    assert!(help.contains("Build a passive artifact promotion readiness check"));
    assert!(help.contains("Build a passive artifact promotion diff"));
    assert!(help.contains("do not install"));
    assert!(help.contains("mutate deployment/controller state"));
    assert!(readiness_help.contains("PromotionReadinessRequest-shaped JSON"));
    assert!(check_help.contains("PromotionReadinessRequest-shaped JSON"));
    assert!(artifact_identity_help.contains("PromotionArtifactIdentityReportRequest-shaped JSON"));
    assert!(transform_help.contains("PromotionPlanTransformRequest-shaped JSON"));
    assert!(diff_help.contains("PromotionPlanTransformRequest-shaped JSON"));
    assert!(transform_evidence_help.contains("PromotionPlanTransformEvidenceRequest-shaped JSON"));
    assert!(target_lineage_help.contains("PromotionTargetExecutionLineageRequest-shaped JSON"));
    assert!(plan_help.contains("ArtifactPromotionPlanRequest-shaped JSON"));
    assert!(provenance_help.contains("ArtifactPromotionProvenanceReportRequest-shaped JSON"));
    assert!(
        wasm_store_identity_help.contains("PromotionWasmStoreIdentityReportRequest-shaped JSON")
    );
    assert!(
        catalog_verification_help
            .contains("PromotionWasmStoreCatalogVerificationRequest-shaped JSON")
    );
    assert!(
        execution_receipt_help.contains("ArtifactPromotionExecutionReceiptRequest-shaped JSON")
    );
    assert!(policy_help.contains("PromotionPolicyCheckRequest-shaped JSON"));
    assert!(
        materialization_help.contains("PromotionMaterializationIdentityReportRequest-shaped JSON")
    );
}

#[test]
fn deploy_promote_help_keeps_advanced_reports_under_inspect() {
    let help = promote_usage();

    for advanced_command in [
        "target-lineage",
        "wasm-store-identity",
        "catalog-verification",
        "materialization-identity",
        "execution-receipt",
        "transform-evidence",
        "policy",
    ] {
        assert!(
            !help.contains(&format!("canic deploy promote {advanced_command}")),
            "advanced promotion report {advanced_command} must stay below inspect"
        );
    }
    assert!(help.contains("canic deploy promote inspect readiness"));
    assert!(help.contains("canic deploy promote inspect artifact-identity"));
    assert!(help.contains("canic deploy promote inspect provenance"));
}

#[test]
fn deploy_promote_command_dispatches_plan_as_public_surface() {
    let parsed = parse_subcommand(
        deploy_command(),
        [
            OsString::from("promote"),
            OsString::from("plan"),
            OsString::from("--request"),
            OsString::from("promotion-plan.json"),
        ],
    )
    .expect("parse deploy promote")
    .expect("promote command");

    assert_eq!(parsed.0, "promote");

    let nested = parse_subcommand(deploy_promote_command(), parsed.1)
        .expect("parse nested promote")
        .expect("promote plan command");
    assert_eq!(nested.0, "plan");
    assert_eq!(
        nested.1,
        vec![
            OsString::from("--request"),
            OsString::from("promotion-plan.json")
        ]
    );
}

#[test]
fn deploy_promote_command_dispatches_check_and_diff_as_public_surface() {
    for (command, request) in [
        ("check", "promotion-check.json"),
        ("diff", "promotion-diff.json"),
    ] {
        let parsed = parse_subcommand(
            deploy_command(),
            [
                OsString::from("promote"),
                OsString::from(command),
                OsString::from("--request"),
                OsString::from(request),
            ],
        )
        .expect("parse deploy promote")
        .expect("promote command");

        assert_eq!(parsed.0, "promote");

        let nested = parse_subcommand(deploy_promote_command(), parsed.1)
            .expect("parse nested promote")
            .expect("promote public command");
        assert_eq!(nested.0, command);
        assert_eq!(
            nested.1,
            vec![OsString::from("--request"), OsString::from(request)]
        );
    }
}

#[test]
fn deploy_promote_command_dispatches_inspect_namespace() {
    let parsed = parse_subcommand(
        deploy_command(),
        [OsString::from("promote"), OsString::from("inspect")],
    )
    .expect("parse deploy promote")
    .expect("promote command");

    assert_eq!(parsed.0, "promote");

    let nested = parse_subcommand(deploy_promote_command(), parsed.1)
        .expect("parse nested promote")
        .expect("promote inspect command");
    assert_eq!(nested.0, "inspect");
    assert!(nested.1.is_empty());
}

#[test]
fn deploy_promote_inspect_dispatches_leaf_commands() {
    for (_, _, command, request) in promote_inspect_leaf_commands() {
        let parsed = parse_subcommand(
            deploy_command(),
            [
                OsString::from("promote"),
                OsString::from("inspect"),
                OsString::from(command),
                OsString::from("--request"),
                OsString::from(request),
            ],
        )
        .expect("parse deploy promote inspect")
        .expect("promote command");

        assert_eq!(parsed.0, "promote");

        let promote = parse_subcommand(deploy_promote_command(), parsed.1)
            .expect("parse nested promote")
            .expect("promote inspect command");
        assert_eq!(promote.0, "inspect");

        let inspect = parse_subcommand(deploy_promote_inspect_command(), promote.1)
            .expect("parse nested inspect")
            .expect("promote inspect leaf command");
        assert_eq!(inspect.0, command);
        assert_eq!(
            inspect.1,
            vec![OsString::from("--request"), OsString::from(request)]
        );
    }
}

#[test]
fn promote_policy_check_rejects_unknown_format() {
    let result = DeployPromoteReportOptions::parse(
        [
            OsString::from("--request"),
            OsString::from("promotion-policy.json"),
            OsString::from("--format"),
            OsString::from("csv"),
        ],
        deploy_promote_policy_check_command,
        promote_policy_check_usage,
    );

    std::assert_matches!(
        result,
        Err(DeployCommandError::Usage(message))
            if message.contains("invalid promotion output format: csv")
    );
}

type PromoteCommandFactory = fn() -> ClapCommand;
type PromoteUsageFactory = fn() -> String;

fn promote_leaf_commands() -> [(PromoteCommandFactory, PromoteUsageFactory, &'static str); 14] {
    [
        (
            deploy_promote_readiness_command,
            promote_readiness_usage,
            "promotion-readiness.json",
        ),
        (
            deploy_promote_check_command,
            promote_check_usage,
            "promotion-check.json",
        ),
        (
            deploy_promote_artifact_identity_command,
            promote_artifact_identity_usage,
            "promotion-artifacts.json",
        ),
        (
            deploy_promote_transform_command,
            promote_transform_usage,
            "promotion-transform.json",
        ),
        (
            deploy_promote_diff_command,
            promote_diff_usage,
            "promotion-diff.json",
        ),
        (
            deploy_promote_transform_evidence_command,
            promote_transform_evidence_usage,
            "transform-evidence.json",
        ),
        (
            deploy_promote_target_lineage_command,
            promote_target_lineage_usage,
            "target-lineage.json",
        ),
        (
            deploy_promote_plan_command,
            promote_plan_usage,
            "promotion-plan.json",
        ),
        (
            deploy_promote_provenance_command,
            promote_provenance_usage,
            "promotion-provenance.json",
        ),
        (
            deploy_promote_wasm_store_identity_command,
            promote_wasm_store_identity_usage,
            "wasm-store-identity.json",
        ),
        (
            deploy_promote_catalog_verification_command,
            promote_catalog_verification_usage,
            "catalog-verification.json",
        ),
        (
            deploy_promote_execution_receipt_command,
            promote_execution_receipt_usage,
            "promotion-execution-receipt.json",
        ),
        (
            deploy_promote_policy_check_command,
            promote_policy_check_usage,
            "promotion-policy.json",
        ),
        (
            deploy_promote_materialization_identity_command,
            promote_materialization_identity_usage,
            "materialization.json",
        ),
    ]
}

fn promote_inspect_leaf_commands() -> [(
    PromoteCommandFactory,
    PromoteUsageFactory,
    &'static str,
    &'static str,
); 11] {
    [
        (
            deploy_promote_readiness_command,
            promote_readiness_usage,
            "readiness",
            "promotion-readiness.json",
        ),
        (
            deploy_promote_artifact_identity_command,
            promote_artifact_identity_usage,
            "artifact-identity",
            "promotion-artifacts.json",
        ),
        (
            deploy_promote_transform_command,
            promote_transform_usage,
            "transform",
            "promotion-transform.json",
        ),
        (
            deploy_promote_transform_evidence_command,
            promote_transform_evidence_usage,
            "transform-evidence",
            "transform-evidence.json",
        ),
        (
            deploy_promote_target_lineage_command,
            promote_target_lineage_usage,
            "target-lineage",
            "target-lineage.json",
        ),
        (
            deploy_promote_provenance_command,
            promote_provenance_usage,
            "provenance",
            "promotion-provenance.json",
        ),
        (
            deploy_promote_wasm_store_identity_command,
            promote_wasm_store_identity_usage,
            "wasm-store-identity",
            "wasm-store-identity.json",
        ),
        (
            deploy_promote_catalog_verification_command,
            promote_catalog_verification_usage,
            "catalog-verification",
            "catalog-verification.json",
        ),
        (
            deploy_promote_execution_receipt_command,
            promote_execution_receipt_usage,
            "execution-receipt",
            "promotion-execution-receipt.json",
        ),
        (
            deploy_promote_policy_check_command,
            promote_policy_check_usage,
            "policy",
            "promotion-policy.json",
        ),
        (
            deploy_promote_materialization_identity_command,
            promote_materialization_identity_usage,
            "materialization-identity",
            "materialization.json",
        ),
    ]
}
