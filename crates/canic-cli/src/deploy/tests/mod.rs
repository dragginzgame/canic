mod authority;
mod deploy_check;
mod external;
mod fixtures;
mod promote;
mod root;

use super::*;
use canic_host::deployment_truth::{
    ArtifactDigestSourceV1, ArtifactSourceV1, AuthorityProfileV1, CanisterControlClassV1,
    ConsentChannelKindV1, DEPLOYMENT_TRUTH_SCHEMA_VERSION, DeploymentDiffV1, DeploymentIdentityV1,
    DeploymentInventoryV1, DeploymentPlanV1, DeploymentRootObservationSourceV1,
    DeploymentRootObservationV1, DeploymentRootVerificationEvidenceStatusV1,
    DeploymentRootVerificationRequestV1, DeploymentRootVerificationSourceV1,
    DeploymentRootVerificationStateTransitionV1, DeploymentRootVerificationStateV1,
    ExpectedCanisterV1, ExternalUpgradeCompletionReportRequest, ExternalUpgradeCompletionStatusV1,
    ExternalUpgradeConsentEvidenceRequest, ExternalUpgradeConsentStateV1,
    ExternalUpgradeVerificationCheckRequest, ExternalUpgradeVerificationObservationV1,
    ExternalUpgradeVerificationPolicyRequest, ExternalUpgradeVerificationReportRequest,
    ExternalUpgradeVerificationRequirementStatusV1, ExternalUpgradeVerificationResultV1,
    ExternalVerificationObservationSourceV1, LifecycleVerificationRequirementV1,
    LocalDeploymentConfigV1, ObservationStatusV1, ObservedArtifactV1, ObservedCanisterV1,
    PreviousArtifactReceiptKindV1, PromotionArtifactLevelV1, ResumeSafetyV1,
    RoleArtifactSourceKindV1, RoleArtifactSourceV1, RoleArtifactV1, RolePromotionInputV1,
    SafetyReportV1, SafetyStatusV1, TrustDomainV1, VerifierReadinessExpectationV1,
    VerifierReadinessObservationV1, compare_plan_to_inventory,
    external_upgrade_receipt_from_observation, promotion_readiness_from_inputs,
    safety_report_from_diff,
};
use fixtures::*;

#[test]
fn deploy_catalog_options_parse_list_defaults_to_text() {
    let options = catalog::DeployCatalogOptions::parse_list_test([
        OsString::from("--__canic-network"),
        OsString::from("local"),
    ])
    .expect("parse catalog list");

    assert_eq!(options.deployment, None);
    assert_eq!(options.network, "local");
    assert_eq!(options.format, output_format::CatalogOutputFormat::Text);
    assert_eq!(options.output, None);
}

#[test]
fn deploy_catalog_options_parse_inspect_json_output() {
    let options = catalog::DeployCatalogOptions::parse_inspect_test([
        OsString::from("demo-local"),
        OsString::from("--format"),
        OsString::from("json"),
        OsString::from("--output"),
        OsString::from("catalog.json"),
    ])
    .expect("parse catalog inspect");

    assert_eq!(options.deployment.as_deref(), Some("demo-local"));
    assert_eq!(options.network, "local");
    assert_eq!(options.format, output_format::CatalogOutputFormat::Json);
    assert_eq!(options.output, Some(PathBuf::from("catalog.json")));
}

#[test]
fn deploy_catalog_rejects_unknown_format() {
    let err = catalog::DeployCatalogOptions::parse_list_test([
        OsString::from("--format"),
        OsString::from("envelope-json"),
    ])
    .expect_err("catalog format is narrow in 0.54.0");

    std::assert_matches!(
        err,
        DeployCommandError::Usage(message)
            if message.contains("invalid deployment catalog output format")
    );
}

#[test]
fn deploy_catalog_command_dispatches_list_and_inspect() {
    let parsed = parse_subcommand(
        deploy_command(),
        [OsString::from("catalog"), OsString::from("list")],
    )
    .expect("parse deploy catalog")
    .expect("catalog command");

    assert_eq!(parsed.0, "catalog");
    let nested = parse_subcommand(catalog::command(), parsed.1)
        .expect("parse nested catalog")
        .expect("catalog list command");
    assert_eq!(nested.0, "list");

    let parsed = parse_subcommand(
        deploy_command(),
        [
            OsString::from("catalog"),
            OsString::from("inspect"),
            OsString::from("demo-local"),
        ],
    )
    .expect("parse deploy catalog inspect")
    .expect("catalog command");

    assert_eq!(parsed.0, "catalog");
    let nested = parse_subcommand(catalog::command(), parsed.1)
        .expect("parse nested catalog inspect")
        .expect("catalog inspect command");
    assert_eq!(nested.0, "inspect");
    assert_eq!(nested.1, vec![OsString::from("demo-local")]);
}

#[test]
fn deploy_catalog_help_documents_passive_deployment_target_scope() {
    let help = catalog::usage();
    let list_help = catalog::list_usage();
    let inspect_help = catalog::inspect_usage();

    assert!(help.contains("deployment targets recorded under .canic/<network>/deployments"));
    assert!(help.contains("do not query"));
    assert!(help.contains("infer deployments from fleet-template names"));
    assert!(list_help.contains("--format <text|json>"));
    assert!(list_help.contains("--output <path>"));
    assert!(inspect_help.contains("deployment target, not a fleet template"));
}

#[test]
fn writes_catalog_json_output_file() {
    let out = temp_json_path("deploy-catalog-output.json");
    let options = catalog::DeployCatalogOptions {
        deployment: None,
        network: "local".to_string(),
        format: output_format::CatalogOutputFormat::Json,
        output: Some(out.clone()),
    };
    let report = sample_catalog_report();

    catalog::write_report(&options, &report).expect("write catalog");
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(&out).expect("read catalog")).expect("parse catalog");

    fs::remove_file(out).expect("clean catalog");
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["entries"][0]["deployment"], "demo-local");
    assert!(value.get("envelope_schema").is_none());
}

#[test]
fn deploy_catalog_path_has_no_live_lookup_or_mutation_primitives() {
    let catalog_source = include_str!("../catalog.rs");
    for forbidden in [
        "update_settings",
        "install_code",
        "create_canister",
        "delete_canister",
        "stop_canister",
        "uninstall_code",
        "provisional_create_canister",
        "dfx",
        "check_install_deployment_truth",
        "install_root(",
        "register_deployment_state",
        "verify_registered_deployment_root",
    ] {
        assert!(
            !catalog_source.contains(forbidden),
            "deploy catalog path must stay local-state read-only; found forbidden token {forbidden}"
        );
    }
}

#[test]
fn deploy_leaf_commands_parse_like_check() {
    let plan = DeployTruthOptions::parse(
        [OsString::from("demo")],
        truth::plan_command,
        truth::plan_usage,
    )
    .expect("parse deploy plan");
    let inventory = DeployTruthOptions::parse(
        [OsString::from("demo")],
        truth::inventory_command,
        truth::inventory_usage,
    )
    .expect("parse deploy inventory");
    let diff = DeployTruthOptions::parse(
        [OsString::from("demo")],
        truth::diff_command,
        truth::diff_usage,
    )
    .expect("parse deploy diff");
    let report = DeployTruthOptions::parse(
        [OsString::from("demo")],
        truth::report_command,
        truth::report_usage,
    )
    .expect("parse deploy report");
    let resume_report = resume_report::DeployResumeReportOptions::parse([
        OsString::from("--receipt"),
        OsString::from("receipt.json"),
        OsString::from("demo"),
    ])
    .expect("parse deploy resume-report");

    assert_eq!(plan.deployment, "demo");
    assert_eq!(inventory.deployment, "demo");
    assert_eq!(diff.deployment, "demo");
    assert_eq!(report.deployment, "demo");
    assert_eq!(resume_report.truth.deployment, "demo");
    assert_eq!(resume_report.receipt, Some(PathBuf::from("receipt.json")));
}

#[test]
fn deploy_compare_parses_artifact_paths_and_text_format() {
    let options = compare::DeployCompareOptions::parse([
        OsString::from("--left"),
        OsString::from("staging-check.json"),
        OsString::from("--right"),
        OsString::from("prod-check.json"),
        OsString::from("--left-label"),
        OsString::from("staging"),
        OsString::from("--right-label"),
        OsString::from("prod"),
        OsString::from("--format"),
        OsString::from("text"),
    ])
    .expect("parse deploy compare");

    assert_eq!(options.left, PathBuf::from("staging-check.json"));
    assert_eq!(options.right, PathBuf::from("prod-check.json"));
    assert_eq!(options.left_label.as_deref(), Some("staging"));
    assert_eq!(options.right_label.as_deref(), Some("prod"));
    assert_eq!(options.format, output_format::CompareOutputFormat::Text);
}

#[test]
fn deploy_compare_builder_uses_existing_check_artifacts() {
    let left = sample_authority_check();
    let mut right = sample_authority_check();
    right.plan.deployment_identity.deployment_name = "prod".to_string();

    let report = compare::build_report_from_checks(&left, &right, Some("stage"), None)
        .expect("comparison report should build");

    assert_eq!(report.report_id, "local:stage:prod:deployment-comparison");
    assert_eq!(report.left.label, "stage");
    assert_eq!(report.right.label, "prod");
    assert!(!report.identity_diff.is_empty());
    assert_eq!(report.report_digest.len(), 64);
}

#[test]
fn deploy_compare_rejects_unknown_format() {
    let result = compare::DeployCompareOptions::parse([
        OsString::from("--left"),
        OsString::from("staging-check.json"),
        OsString::from("--right"),
        OsString::from("prod-check.json"),
        OsString::from("--format"),
        OsString::from("yaml"),
    ]);

    std::assert_matches!(
        result,
        Err(DeployCommandError::Usage(message))
            if message.contains("invalid deployment comparison output format: yaml")
    );
}

#[test]
fn deploy_compare_help_documents_passive_artifact_scope() {
    let help = compare::usage();

    assert!(help.contains("Compare two deployment truth check artifacts"));
    assert!(help.contains("DeploymentCheckV1 JSON artifacts"));
    assert!(help.contains("does not query live"));
    assert!(help.contains("install code"));
    assert!(help.contains("mutate deployments"));
    assert!(help.contains("embedded"));
    assert!(help.contains("revalidated"));
}

#[test]
fn deploy_compare_path_has_no_live_lookup_or_mutation_primitives() {
    let compare_source = include_str!("../compare.rs");

    for forbidden in [
        "update_settings",
        "install_code",
        "create_canister",
        "delete_canister",
        "stop_canister",
        "uninstall_code",
        "provisional_create_canister",
        "dfx",
        "load_deployment_check",
        "check_install_deployment_truth",
        "resolve_current_canic_icp_root",
    ] {
        assert!(
            !compare_source.contains(forbidden),
            "deploy compare run path must stay passive; found forbidden token {forbidden}"
        );
    }
}

#[test]
fn deploy_install_command_dispatches_plan_install() {
    let parsed = parse_subcommand(
        deploy_command(),
        [
            OsString::from("install"),
            OsString::from("demo-local"),
            OsString::from("--plan"),
            OsString::from("promoted-plan.json"),
        ],
    )
    .expect("parse deploy install")
    .expect("install command");

    assert_eq!(parsed.0, "install");
    assert_eq!(
        parsed.1,
        vec![
            OsString::from("demo-local"),
            OsString::from("--plan"),
            OsString::from("promoted-plan.json")
        ]
    );

    let options = install::DeployInstallPlanOptions::parse(parsed.1).expect("parse install plan");
    assert_eq!(options.deployment, "demo-local");
    assert_eq!(options.plan, PathBuf::from("promoted-plan.json"));
}

#[test]
fn deploy_register_command_dispatches_register() {
    let parsed = parse_subcommand(
        deploy_command(),
        [
            OsString::from("register"),
            OsString::from("demo-local"),
            OsString::from("--fleet-template"),
            OsString::from("demo"),
            OsString::from("--root"),
            OsString::from("uxrrr-q7777-77774-qaaaq-cai"),
            OsString::from("--allow-unverified"),
        ],
    )
    .expect("parse deploy register")
    .expect("register command");

    assert_eq!(parsed.0, "register");

    let options = register::DeployRegisterOptions::parse(parsed.1).expect("parse register options");
    assert_eq!(options.deployment, "demo-local");
    assert_eq!(options.fleet_template, "demo");
    assert_eq!(options.root, "uxrrr-q7777-77774-qaaaq-cai");
    assert!(options.allow_unverified);
}

#[test]
fn deploy_register_builds_minimal_registration_options() {
    let options = register::DeployRegisterOptions {
        deployment: "demo-local".to_string(),
        fleet_template: "demo".to_string(),
        root: "uxrrr-q7777-77774-qaaaq-cai".to_string(),
        network: "local".to_string(),
        allow_unverified: true,
    }
    .into_register_options(Some(PathBuf::from("/tmp/icp")));

    assert_eq!(options.deployment_name, "demo-local");
    assert_eq!(options.fleet_template, "demo");
    assert_eq!(options.root_canister_id, "uxrrr-q7777-77774-qaaaq-cai");
    assert_eq!(options.network, "local");
    assert!(options.allow_unverified);
    assert_eq!(options.icp_root, Some(PathBuf::from("/tmp/icp")));
    assert_eq!(options.workspace_root, None);
}

#[test]
fn deploy_register_requires_unverified_acknowledgement_flag() {
    let err = register::DeployRegisterOptions::parse([
        OsString::from("demo-local"),
        OsString::from("--fleet-template"),
        OsString::from("demo"),
        OsString::from("--root"),
        OsString::from("uxrrr-q7777-77774-qaaaq-cai"),
    ])
    .expect_err("register without acknowledgement should fail usage");

    std::assert_matches!(err, DeployCommandError::Usage(_));
}

#[test]
fn deploy_compare_command_dispatches_compare() {
    let parsed = parse_subcommand(
        deploy_command(),
        [
            OsString::from("compare"),
            OsString::from("--left"),
            OsString::from("staging-check.json"),
            OsString::from("--right"),
            OsString::from("prod-check.json"),
        ],
    )
    .expect("parse deploy compare")
    .expect("compare command");

    assert_eq!(parsed.0, "compare");
    assert_eq!(
        parsed.1,
        vec![
            OsString::from("--left"),
            OsString::from("staging-check.json"),
            OsString::from("--right"),
            OsString::from("prod-check.json")
        ]
    );

    let options = compare::DeployCompareOptions::parse(parsed.1).expect("parse compare options");
    assert_eq!(options.left, PathBuf::from("staging-check.json"));
    assert_eq!(options.right, PathBuf::from("prod-check.json"));
}

#[test]
fn deploy_install_path_uses_current_install_with_plan_override() {
    let source = include_str!("../install.rs");
    let install_source = source_between(source, "pub(super) fn run<I>", "pub(super) fn read_plan");

    assert!(install_source.contains("read_plan"));
    assert!(install_source.contains("into_install_root_options"));
    assert!(install_source.contains("install_root"));
    for forbidden in [
        "artifact_promotion_execution_receipt",
        "artifact_promotion_provenance_report",
        "build_artifact_promotion_plan",
        "run_promote",
    ] {
        assert!(
            !install_source.contains(forbidden),
            "deploy install path must stay current-install mediated; found forbidden token {forbidden}"
        );
    }
}

#[test]
fn deploy_resume_report_allows_latest_local_receipt_lookup() {
    let resume_report = resume_report::DeployResumeReportOptions::parse([OsString::from("demo")])
        .expect("parse deploy resume-report");

    assert_eq!(resume_report.truth.deployment, "demo");
    assert_eq!(resume_report.receipt, None);
}

#[test]
fn deploy_install_plan_builds_current_install_options_with_plan_override() {
    let mut identity = sample_deployment_identity();
    identity.deployment_name = "demo-local".to_string();
    let plan = sample_deployment_plan(identity);
    let input = install::DeployInstallPlanInput {
        deployment_plan: plan,
        artifact_promotion_plan: None,
    };
    let options = install::DeployInstallPlanOptions {
        deployment: "demo-local".to_string(),
        plan: PathBuf::from("promoted-plan.json"),
        network: "local".to_string(),
        profile: Some(CanisterBuildProfile::Fast),
    }
    .into_install_root_options(input, Some(std::path::PathBuf::from("/tmp/icp")));

    assert_eq!(options.root_canister, "aaaaa-aa");
    assert_eq!(options.root_build_target, "root");
    assert_eq!(options.network, "local");
    assert_eq!(options.deployment_name.as_deref(), Some("demo-local"));
    assert_eq!(options.build_profile, Some(CanisterBuildProfile::Fast));
    assert_eq!(
        options.config_path.as_deref(),
        Some("fleets/demo/canic.toml")
    );
    assert_eq!(options.expected_fleet.as_deref(), Some("demo"));
    assert!(options.deployment_plan_override.is_some());
}

#[test]
fn deploy_install_plan_reader_accepts_raw_deployment_plan() {
    let path = temp_json_path("deploy-install-raw-plan.json");
    let plan = sample_deployment_plan(sample_deployment_identity());
    fs::write(&path, serde_json::to_vec(&plan).expect("encode plan")).expect("write plan");

    let decoded = install::read_plan(&path).expect("decode deployment plan");

    assert_eq!(decoded.deployment_plan.plan_id, "plan-1");
    assert_eq!(decoded.artifact_promotion_plan, None);
    fs::remove_file(path).expect("clean temp plan");
}

#[test]
fn deploy_install_plan_reader_accepts_ready_promotion_envelope() {
    let path = temp_json_path("deploy-install-ready-promotion-plan.json");
    let plan = sample_artifact_promotion_plan();
    fs::write(&path, serde_json::to_vec(&plan).expect("encode plan")).expect("write plan");

    let decoded = install::read_plan(&path).expect("decode promotion plan");

    assert_eq!(decoded.deployment_plan.plan_id, "promoted-plan-1");
    assert_eq!(
        decoded
            .artifact_promotion_plan
            .as_ref()
            .map(|plan| plan.plan_id.as_str()),
        Some("artifact-promotion-plan-1")
    );
    fs::remove_file(path).expect("clean temp plan");
}

#[test]
fn deploy_install_plan_reader_rejects_blocked_promotion_envelope() {
    let path = temp_json_path("deploy-install-blocked-promotion-plan.json");
    let plan = sample_blocked_artifact_promotion_plan();
    fs::write(&path, serde_json::to_vec(&plan).expect("encode plan")).expect("write plan");

    let result = install::read_plan(&path);

    std::assert_matches!(
        result,
        Err(DeployCommandError::Blocked(message))
            if message.contains("artifact promotion plan artifact-promotion-plan-1 is not ready")
    );
    fs::remove_file(path).expect("clean temp plan");
}
