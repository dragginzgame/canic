use super::*;
use crate::test_support::temp_dir;
use crate::{CliError, cli_error_exit_code, render_cli_error};
use serde_json::Value as JsonValue;
use std::fs;

// Ensure bare top-level medic selects the project scope without inventing a deployment.
#[test]
fn parses_bare_project_medic_options() {
    let options = MedicOptions::parse([
        OsString::from(crate::cli::globals::INTERNAL_ICP_OPTION),
        OsString::from("/tmp/icp"),
    ])
    .expect("parse medic options");

    assert_eq!(options.scope, MedicScope::Project);
    assert_eq!(options.deployment, None);
    assert_eq!(options.network, None);
    assert_eq!(options.icp, "/tmp/icp");
}

// Ensure explicit project medic keeps the same scope and accepts JSON output.
#[test]
fn parses_project_medic_options() {
    let options = MedicOptions::parse([OsString::from("project"), OsString::from("--json")])
        .expect("parse medic project options");

    assert_eq!(options.scope, MedicScope::Project);
    assert!(options.json);
    assert_eq!(options.deployment, None);
}

// Ensure deployment medic parses target, network, and ICP selectors.
#[test]
fn parses_deployment_medic_options() {
    let options = MedicOptions::parse([
        OsString::from("deployment"),
        OsString::from("demo"),
        OsString::from(crate::cli::globals::INTERNAL_NETWORK_OPTION),
        OsString::from("local"),
        OsString::from(crate::cli::globals::INTERNAL_ICP_OPTION),
        OsString::from("/tmp/icp"),
    ])
    .expect("parse medic deployment options");

    assert_eq!(options.scope, MedicScope::Deployment);
    assert_eq!(options.deployment.as_deref(), Some("demo"));
    assert_eq!(options.network.as_deref(), Some("local"));
    assert_eq!(options.icp, "/tmp/icp");
}

// Ensure targeted blob-storage medic diagnostics are deployment-only.
#[test]
fn parses_deployment_blob_storage_medic_target() {
    let options = MedicOptions::parse([
        OsString::from("deployment"),
        OsString::from("demo"),
        OsString::from("--blob-storage"),
        OsString::from("backend"),
    ])
    .expect("parse medic options");

    assert_eq!(options.deployment.as_deref(), Some("demo"));
    assert_eq!(options.blob_storage.as_deref(), Some("backend"));
}

// Ensure targeted auth-renewal medic diagnostics are deployment-only.
#[test]
fn parses_deployment_auth_renewal_medic_target() {
    let options = MedicOptions::parse([
        OsString::from("deployment"),
        OsString::from("demo"),
        OsString::from("--auth-renewal"),
        OsString::from("rrkah-fqaaa-aaaaa-aaaaq-cai"),
    ])
    .expect("parse medic options");

    assert_eq!(options.deployment.as_deref(), Some("demo"));
    assert_eq!(
        options.auth_renewal.as_deref(),
        Some("rrkah-fqaaa-aaaaa-aaaaq-cai")
    );
}

// Ensure hard-cut rejected command forms do not parse as compatibility aliases.
#[test]
fn rejects_legacy_or_shorthand_medic_forms() {
    for args in [
        vec![OsString::from("demo")],
        vec![OsString::from("--blob-storage"), OsString::from("backend")],
        vec![
            OsString::from("--auth-renewal"),
            OsString::from("rrkah-fqaaa-aaaaa-aaaaq-cai"),
        ],
        vec![
            OsString::from("project"),
            OsString::from("--blob-storage"),
            OsString::from("backend"),
        ],
        vec![
            OsString::from("project"),
            OsString::from("--auth-renewal"),
            OsString::from("rrkah-fqaaa-aaaaa-aaaaq-cai"),
        ],
    ] {
        assert!(matches!(
            MedicOptions::parse(args),
            Err(MedicCommandError::Usage(_))
        ));
    }
}

// Ensure medic help explains the new top-level command surface.
#[test]
fn medic_usage_includes_top_level_examples() {
    let text = usage();

    assert!(text.contains("Diagnose Canic project and deployment preflight readiness"));
    assert!(text.contains("Usage: canic medic"));
    assert!(text.contains("canic medic project"));
    assert!(text.contains("canic medic deployment test"));
    assert!(text.contains("canic medic deployment test --blob-storage backend"));
    assert!(text.contains("canic medic deployment test --auth-renewal"));
    assert!(text.contains("--json"));
    assert!(!text.contains("canic info medic"));
}

// Ensure subcommand help requests stop before project or deployment checks run.
#[test]
fn medic_subcommand_help_requests_are_not_targets() {
    assert!(medic_subcommand_help_requested(&[
        OsString::from("project"),
        OsString::from("help")
    ]));
    assert!(medic_subcommand_help_requested(&[
        OsString::from("project"),
        OsString::from("--help")
    ]));
    assert!(medic_subcommand_help_requested(&[
        OsString::from("deployment"),
        OsString::from("help")
    ]));
    assert!(medic_subcommand_help_requested(&[
        OsString::from("deployment"),
        OsString::from("-h")
    ]));
    assert!(medic_subcommand_help_requested(&[
        OsString::from("--json"),
        OsString::from("deployment"),
        OsString::from("help")
    ]));
    assert!(medic_subcommand_help_requested(&[
        OsString::from("deployment"),
        OsString::from("--json"),
        OsString::from("help")
    ]));
    assert!(medic_subcommand_help_requested(&[
        OsString::from("project"),
        OsString::from("--json"),
        OsString::from("help")
    ]));
    assert!(medic_subcommand_help_requested(&[
        OsString::from(crate::cli::globals::INTERNAL_NETWORK_OPTION),
        OsString::from("local"),
        OsString::from("deployment"),
        OsString::from("help")
    ]));
    assert!(medic_subcommand_help_requested(&[
        OsString::from(crate::cli::globals::INTERNAL_ICP_OPTION),
        OsString::from("/tmp/icp"),
        OsString::from("project"),
        OsString::from("help")
    ]));
    assert!(medic_subcommand_help_requested(&[
        OsString::from("deployment"),
        OsString::from(crate::cli::globals::INTERNAL_NETWORK_OPTION),
        OsString::from("local"),
        OsString::from("help")
    ]));
    assert!(!medic_subcommand_help_requested(&[
        OsString::from("deployment"),
        OsString::from("demo")
    ]));
    assert!(!medic_subcommand_help_requested(&[
        OsString::from("--json"),
        OsString::from("deployment"),
        OsString::from("demo")
    ]));
    assert!(!medic_subcommand_help_requested(&[
        OsString::from("deployment"),
        OsString::from("demo"),
        OsString::from("help")
    ]));
}

// Ensure aggregate status follows the 0.78 report contract.
#[test]
fn aggregate_status_follows_report_contract() {
    assert_eq!(aggregate_status(&[]), MedicStatus::NotEvaluated);
    assert_eq!(
        aggregate_status(&[MedicCheck::not_evaluated(
            MedicCategory::DeploymentState,
            "deployment_not_selected",
            "deployment",
            "none",
            "none",
            MedicSource::Command,
        )]),
        MedicStatus::NotEvaluated
    );
    assert_eq!(
        aggregate_status(&[
            sample_check(MedicStatus::Pass),
            sample_check(MedicStatus::NotEvaluated)
        ]),
        MedicStatus::Pass
    );
    assert_eq!(
        aggregate_status(&[
            sample_check(MedicStatus::Pass),
            sample_check(MedicStatus::Warn)
        ]),
        MedicStatus::Warn
    );
    assert_eq!(
        aggregate_status(&[
            sample_check(MedicStatus::Warn),
            sample_check(MedicStatus::Fail)
        ]),
        MedicStatus::Fail
    );
}

// Ensure the text report carries status, category, code, detail, next, and source.
#[test]
fn renders_medic_text_report() {
    let report = MedicReport::new(
        &MedicOptions::project(false, None, "icp".to_string()),
        vec![
            MedicCheck::warn(
                MedicCategory::ProjectConfig,
                "local_network_implicit",
                "network",
                "no network was selected",
                "select an explicit network before deployment checks",
                MedicSource::IcpConfig,
            ),
            MedicCheck::pass(
                MedicCategory::Environment,
                "icp_cli_ok",
                "icp",
                "icp 1.0.0",
                "none",
                MedicSource::IcpCli,
            ),
        ],
    );
    let rendered = render_medic_text(&report);

    assert!(rendered.starts_with("canic medic project\nstatus: warn"));
    assert!(rendered.contains("network: not selected"));
    assert!(rendered.contains("environment [pass] icp_cli_ok"));
    assert!(rendered.contains("project_config [warn] local_network_implicit"));
    assert!(rendered.contains("  detail: no network was selected"));
    assert!(rendered.contains("  next: select an explicit network"));
    assert!(rendered.contains("  source: icp_config"));
}

// Ensure JSON output emits schema_version and stable top-level fields.
#[test]
fn renders_medic_json_report() {
    let report = MedicReport::new(
        &MedicOptions::project(true, None, "icp".to_string()),
        vec![sample_check(MedicStatus::Pass)],
    );
    let rendered = render_medic_json(&report).expect("render json");
    let value: JsonValue = serde_json::from_str(&rendered).expect("parse json");

    assert!(rendered.trim_start().starts_with('{'));
    assert!(!rendered.contains("status:"));
    assert!(!rendered.contains("detail:"));
    assert!(!rendered.contains("source:"));
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["command"], "canic medic project");
    assert_eq!(value["scope"], "project");
    assert_eq!(value["network"], JsonValue::Null);
    assert_eq!(value["deployment"], JsonValue::Null);
    assert_eq!(value["status"], "pass");
    assert!(value["checks"].is_array());
}

// Ensure medic errors keep the process-level exit-code contract from the 0.78 design.
#[test]
fn medic_cli_errors_map_to_designed_exit_codes() {
    let usage = CliError::from(MedicCommandError::Usage("bad medic args".to_string()));
    let report_failed = CliError::from(MedicCommandError::ReportFailed);
    let json = CliError::from(MedicCommandError::Json(
        serde_json::from_str::<JsonValue>("{").expect_err("invalid json"),
    ));

    assert_eq!(cli_error_exit_code(&usage), 2);
    assert_eq!(cli_error_exit_code(&report_failed), 1);
    assert_eq!(cli_error_exit_code(&json), 3);
}

// Ensure aggregate fail reports do not add duplicate human diagnostics to stderr.
#[test]
fn failed_medic_report_suppresses_cli_stderr() {
    let cli_error = CliError::from(MedicCommandError::ReportFailed);

    assert_eq!(cli_error_exit_code(&cli_error), 1);
    assert_eq!(render_cli_error(&cli_error), "");
}

// Ensure usage and internal errors still produce stderr diagnostics.
#[test]
fn medic_usage_and_internal_errors_render_cli_stderr() {
    let usage = CliError::from(MedicCommandError::Usage("bad medic args".to_string()));
    let json = CliError::from(MedicCommandError::Json(
        serde_json::from_str::<JsonValue>("{").expect_err("invalid json"),
    ));

    assert_eq!(render_cli_error(&usage), "medic: bad medic args");
    assert!(render_cli_error(&json).contains("medic: failed to render medic JSON output"));
}

// Ensure deployment reports include the effective network while project reports may omit it.
#[test]
fn deployment_report_includes_effective_network() {
    let report = MedicReport::new(
        &MedicOptions {
            scope: MedicScope::Deployment,
            deployment: Some("demo".to_string()),
            blob_storage: None,
            auth_renewal: None,
            json: false,
            network: None,
            icp: "icp".to_string(),
        },
        vec![sample_check(MedicStatus::Pass)],
    );

    assert_eq!(report.network.as_deref(), Some("local"));
    assert_eq!(report.deployment.as_deref(), Some("demo"));
}

// Ensure deployment medic uses a unique installed deployment record network when
// the operator does not pass an explicit network.
#[test]
fn deployment_network_selection_uses_recorded_network_before_local_default() {
    let root = temp_dir("canic-cli-medic-recorded-network");
    let mut state = sample_install_state();
    state.network = "ic".to_string();
    write_medic_install_state(&root, "ic", &state);
    let options = MedicOptions {
        scope: MedicScope::Deployment,
        deployment: Some("demo".to_string()),
        blob_storage: None,
        auth_renewal: None,
        json: false,
        network: None,
        icp: "icp".to_string(),
    };

    let (network, check) = deployment_network_selection(&options, Some(&root));
    let report = MedicReport::with_network(
        &options,
        Some(network.clone()),
        vec![sample_check(MedicStatus::Pass)],
    );

    assert_eq!(network, "ic");
    assert_eq!(check.code, "deployment_network_from_record");
    assert_eq!(check.source, MedicSource::InstalledDeployment);
    assert_eq!(report.network.as_deref(), Some("ic"));

    fs::remove_dir_all(root).expect("remove temp root");
}

// Ensure an explicit operator network still wins over discovered deployment
// records.
#[test]
fn deployment_network_selection_prefers_explicit_network() {
    let root = temp_dir("canic-cli-medic-explicit-network");
    let mut state = sample_install_state();
    state.network = "ic".to_string();
    write_medic_install_state(&root, "ic", &state);
    let options = MedicOptions {
        scope: MedicScope::Deployment,
        deployment: Some("demo".to_string()),
        blob_storage: None,
        auth_renewal: None,
        json: false,
        network: Some("local".to_string()),
        icp: "icp".to_string(),
    };

    let (network, check) = deployment_network_selection(&options, Some(&root));

    assert_eq!(network, "local");
    assert_eq!(check.code, "local_network_explicit");
    assert_eq!(check.source, MedicSource::Command);

    fs::remove_dir_all(root).expect("remove temp root");
}

// Ensure missing installed targets point operators at the no-mutation planner.
#[test]
fn deployment_target_missing_points_to_deploy_plan() {
    let root = temp_dir("canic-cli-medic-missing-target-plan");
    fs::create_dir_all(&root).expect("create temp root");
    let options = MedicOptions {
        scope: MedicScope::Deployment,
        deployment: Some("demo".to_string()),
        blob_storage: None,
        auth_renewal: None,
        json: false,
        network: Some("local".to_string()),
        icp: "icp".to_string(),
    };
    let context = DeploymentMedicContext {
        icp_root: Some(root.clone()),
        network: "local".to_string(),
        network_check: MedicCheck::pass(
            MedicCategory::Network,
            "local_network_explicit",
            "network",
            "local",
            "none",
            MedicSource::Command,
        ),
    };

    let checks = run_deployment_checks(&options, &context);
    let missing = checks
        .iter()
        .find(|check| check.code == "deployment_target_missing")
        .expect("missing deployment check");

    assert_eq!(missing.status, MedicStatus::Fail);
    assert!(missing.next.contains("canic deploy plan demo"));
    assert!(missing.next.contains("canic install <fleet-template>"));

    fs::remove_dir_all(root).expect("remove temp root");
}

// Ensure project-only network warnings do not duplicate deployment-scoped network checks.
#[test]
fn project_network_selection_check_is_project_only() {
    let project = MedicOptions::project(false, None, "icp".to_string());
    let deployment = MedicOptions {
        scope: MedicScope::Deployment,
        deployment: Some("demo".to_string()),
        blob_storage: None,
        auth_renewal: None,
        json: false,
        network: None,
        icp: "icp".to_string(),
    };

    let project_check = project_network_selection_check(&project).expect("project network check");

    assert_eq!(project_check.code, "local_network_implicit");
    assert_eq!(project_check.status, MedicStatus::Warn);
    assert!(project_network_selection_check(&deployment).is_none());
}

// Ensure deployment-state network drift is classified before live readiness probes.
#[test]
fn deployment_network_check_classifies_match_and_mismatch() {
    let mut state = sample_install_state();
    let matched = check_deployment_network(&state, "local");

    assert_eq!(matched.status, MedicStatus::Pass);
    assert_eq!(matched.code, "deployment_network_match");
    assert_eq!(matched.category, MedicCategory::DeploymentState);

    state.network = "ic".to_string();
    let mismatched = check_deployment_network(&state, "local");

    assert_eq!(mismatched.status, MedicStatus::Fail);
    assert_eq!(mismatched.code, "deployment_network_mismatch");
    assert!(mismatched.detail.contains("scoped to ic"));
    assert!(mismatched.detail.contains("selected local"));
}

// Ensure missing root IDs are caught before medic attempts a live readiness query.
#[test]
fn root_canister_id_check_classifies_present_and_missing_ids() {
    let mut state = sample_install_state();
    let present = check_root_canister_id(&state);

    assert_eq!(present.status, MedicStatus::Pass);
    assert_eq!(present.code, "root_canister_id_present");
    assert_eq!(present.detail, "aaaaa-aa");

    state.root_canister_id = "  ".to_string();
    let missing = check_root_canister_id(&state);

    assert_eq!(missing.status, MedicStatus::Fail);
    assert_eq!(missing.code, "root_canister_id_missing");
    assert!(
        missing
            .detail
            .contains("does not record a root canister id")
    );
}

// Ensure skipped root readiness is explicit when local deployment-state gates fail.
#[test]
fn root_readiness_not_evaluated_explains_skipped_live_query() {
    let network_mismatch = check_root_readiness_not_evaluated(false, true);

    assert_eq!(network_mismatch.status, MedicStatus::NotEvaluated);
    assert_eq!(network_mismatch.code, "root_readiness_not_evaluated");
    assert!(network_mismatch.detail.contains("network does not match"));

    let missing_root = check_root_readiness_not_evaluated(true, false);

    assert_eq!(missing_root.status, MedicStatus::NotEvaluated);
    assert_eq!(missing_root.code, "root_readiness_not_evaluated");
    assert!(missing_root.detail.contains("no root canister id"));
}

// Ensure readiness diagnostics identify local replica versus ICP CLI sources.
#[test]
fn root_readiness_source_tracks_selected_network() {
    assert_eq!(root_readiness_source("local"), MedicSource::LocalReplica);
    assert_eq!(root_readiness_source("ic"), MedicSource::IcpCli);
}

// Ensure deployment registry smoke checks are skipped behind local state gates.
#[test]
fn deployment_registry_not_evaluated_explains_skipped_live_query() {
    let network_mismatch = check_deployment_registry_not_evaluated(false, true);

    assert_eq!(network_mismatch.status, MedicStatus::NotEvaluated);
    assert_eq!(network_mismatch.code, "deployment_registry_not_evaluated");
    assert!(network_mismatch.detail.contains("network does not match"));

    let missing_root = check_deployment_registry_not_evaluated(true, false);

    assert_eq!(missing_root.status, MedicStatus::NotEvaluated);
    assert_eq!(missing_root.code, "deployment_registry_not_evaluated");
    assert!(missing_root.detail.contains("no root canister id"));
}

// Ensure successful deployment registry observation reports the live entry and role counts.
#[test]
fn deployment_registry_observed_check_reports_entry_count() {
    let resolution = sample_installed_deployment_resolution(vec![
        registry_entry("aaaaa-aa", Some("root")),
        registry_entry("bbbbbb-bb", Some("app")),
    ]);

    let check = deployment_registry_observed_check(&resolution);

    assert_eq!(check.status, MedicStatus::Pass);
    assert_eq!(check.code, "deployment_registry_observed");
    assert_eq!(check.source, MedicSource::LocalReplica);
    assert!(check.detail.contains("entries=2"));
    assert!(check.detail.contains("roles=2"));
}

// Ensure an empty observed registry remains visible as a warning.
#[test]
fn deployment_registry_observed_check_warns_on_empty_registry() {
    let resolution = sample_installed_deployment_resolution(Vec::new());

    let check = deployment_registry_observed_check(&resolution);

    assert_eq!(check.status, MedicStatus::Warn);
    assert_eq!(check.code, "deployment_registry_empty");
    assert!(check.next.contains("canic deploy plan demo"));
    assert!(check.next.contains("canic deploy check demo"));
}

// Ensure deployment-truth receipt diagnostics classify missing and complete local receipts.
#[test]
fn deployment_truth_receipt_check_classifies_missing_and_complete_receipts() {
    let root = temp_dir("canic-cli-medic-deployment-truth-complete");
    let state = sample_install_state();
    let missing = check_deployment_truth_receipt(Some(&root), &state, "local");

    assert_eq!(missing.status, MedicStatus::Warn);
    assert_eq!(missing.code, "deployment_truth_incomplete");
    assert!(missing.detail.contains("no deployment-truth receipt found"));
    assert!(missing.next.contains("canic deploy plan demo"));
    assert!(missing.next.contains("canic deploy check demo"));

    write_medic_deployment_receipt(
        &root,
        "local",
        "demo",
        sample_deployment_receipt(
            DeploymentExecutionStatusV1::Complete,
            DeploymentCommandResultV1::Succeeded,
            Some("inventory-1"),
        ),
    );
    let complete = check_deployment_truth_receipt(Some(&root), &state, "local");

    assert_eq!(complete.status, MedicStatus::Pass);
    assert_eq!(complete.code, "deployment_truth_complete");
    assert!(complete.detail.contains("status=complete"));
    assert!(complete.detail.contains("result=succeeded"));

    fs::remove_dir_all(root).expect("remove temp root");
}

// Ensure partial deployment-truth receipts are blocking medic diagnostics.
#[test]
fn deployment_truth_receipt_check_fails_on_partial_receipts() {
    let root = temp_dir("canic-cli-medic-deployment-truth-partial");
    let state = sample_install_state();
    write_medic_deployment_receipt(
        &root,
        "local",
        "demo",
        sample_deployment_receipt(
            DeploymentExecutionStatusV1::PartiallyApplied,
            DeploymentCommandResultV1::Failed {
                code: "install_failed".to_string(),
                message: "install failed".to_string(),
            },
            None,
        ),
    );

    let check = check_deployment_truth_receipt(Some(&root), &state, "local");

    assert_eq!(check.status, MedicStatus::Fail);
    assert_eq!(check.code, "deployment_truth_incomplete");
    assert!(check.detail.contains("status=partially_applied"));
    assert!(check.detail.contains("result=failed:install_failed"));
    assert!(check.next.contains("deploy inspect resume-report demo"));

    fs::remove_dir_all(root).expect("remove temp root");
}

// Ensure malformed deployment-truth receipt files fail closed.
#[test]
fn deployment_truth_receipt_check_fails_on_invalid_receipt_json() {
    let root = temp_dir("canic-cli-medic-deployment-truth-invalid");
    let state = sample_install_state();
    let receipt_dir = root.join(".canic/local/deployment-receipts/demo");
    fs::create_dir_all(&receipt_dir).expect("create receipt dir");
    fs::write(receipt_dir.join("unix_100-invalid.json"), "{").expect("write bad receipt");

    let check = check_deployment_truth_receipt(Some(&root), &state, "local");

    assert_eq!(check.status, MedicStatus::Fail);
    assert_eq!(check.code, "deployment_truth_incomplete");
    assert!(check.detail.contains("invalid"));

    fs::remove_dir_all(root).expect("remove temp root");
}

// Ensure missing deployment targets get exact-match hints when they are likely fleet or role names.
#[test]
fn deployment_name_conflation_checks_find_fleet_and_role_names() {
    let root = temp_dir("canic-cli-medic-deployment-name-conflation");
    write_medic_config(
        &root,
        r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.app]
kind = "service"
"#,
    );
    write_medic_package(&root, "root", "demo", "root");
    write_medic_package(&root, "app", "demo", "app");

    let fleet = deployment_name_conflation_checks(&root, "demo");
    let role = deployment_name_conflation_checks(&root, "app");
    let none = deployment_name_conflation_checks(&root, "demo-local");

    assert!(fleet.iter().any(|check| {
        check.status == MedicStatus::Warn && check.code == "fleet_name_deployment_name_conflated"
    }));
    assert!(
        fleet
            .iter()
            .any(|check| check.next.contains("canic deploy plan demo"))
    );
    assert!(role.iter().any(|check| {
        check.status == MedicStatus::Warn && check.code == "role_name_deployment_name_conflated"
    }));
    assert!(none.is_empty());

    fs::remove_dir_all(root).expect("remove temp root");
}

// Ensure project medic validates package-role metadata without spawning Cargo.
#[test]
fn project_config_quality_checks_validate_role_package_metadata() {
    let root = temp_dir("canic-cli-medic-project-config-quality");
    let config = write_medic_config(
        &root,
        r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[roles.store]
kind = "canister"
package = "store"

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.app]
kind = "service"
"#,
    );
    write_medic_package(&root, "root", "demo", "root");
    write_medic_package(&root, "app", "demo", "app");
    write_medic_package(&root, "store", "demo", "store");

    let checks = project_config_quality_checks(&root, &[config]);

    assert!(checks.iter().any(|check| {
        check.status == MedicStatus::Pass
            && check.code == "role_package_metadata_present"
            && check.subject == "demo.app"
    }));
    let store = checks
        .iter()
        .find(|check| check.code == "declared_role_not_deployable")
        .expect("declared-only role check");
    assert_eq!(store.status, MedicStatus::Warn);
    assert_eq!(store.subject, "demo.store");

    fs::remove_dir_all(root).expect("remove temp root");
}

// Ensure package metadata drift is a blocking project-config diagnostic.
#[test]
fn project_config_quality_checks_fail_on_missing_or_mismatched_package_metadata() {
    let root = temp_dir("canic-cli-medic-project-config-metadata-drift");
    let config = write_medic_config(
        &root,
        r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[roles.store]
kind = "canister"
package = "store"

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.app]
kind = "service"

[subnets.prime.canisters.store]
kind = "service"
"#,
    );
    write_medic_package(&root, "root", "demo", "root");
    write_medic_package(&root, "app", "demo", "other");

    let checks = project_config_quality_checks(&root, &[config]);

    let app = checks
        .iter()
        .find(|check| check.subject == "demo.app" && check.code == "role_package_metadata_missing")
        .expect("mismatched metadata check");
    assert_eq!(app.status, MedicStatus::Fail);
    assert!(app.detail.contains("expected fleet=demo role=app"));

    let store = checks
        .iter()
        .find(|check| {
            check.subject == "demo.store" && check.code == "role_package_metadata_missing"
        })
        .expect("missing metadata check");
    assert_eq!(store.status, MedicStatus::Fail);
    assert!(store.detail.contains("failed to read"));

    fs::remove_dir_all(root).expect("remove temp root");
}

// Ensure project medic reports config-driven runtime feature requirements before startup traps.
#[test]
fn project_config_quality_checks_report_missing_required_canic_features() {
    let root = temp_dir("canic-cli-medic-project-required-features");
    let config = write_medic_config(
        &root,
        r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.app]
kind = "service"

[subnets.prime.canisters.app.auth]
role_attestation_cache = true
"#,
    );
    write_medic_package_with_canic_features(
        &root,
        "root",
        "demo",
        "root",
        &["auth-root-canister-sig-create"],
    );
    write_medic_package_with_canic_features(
        &root,
        "app",
        "demo",
        "app",
        &["auth-delegated-token-verify"],
    );

    let checks = project_config_quality_checks(&root, &[config]);

    let app = checks
        .iter()
        .find(|check| {
            check.subject == "demo.app" && check.code == "role_required_canic_feature_missing"
        })
        .expect("missing feature check");
    assert_eq!(app.status, MedicStatus::Fail);
    assert!(app.detail.contains("auth.role_attestation_cache"));
    assert!(app.detail.contains("auth-root-canister-sig-verify"));
    assert!(app.next.contains("fleets/demo/app/Cargo.toml"));

    fs::remove_dir_all(root).expect("remove temp root");
}

// Ensure project medic accepts roles whose runtime canic dependency enables required features.
#[test]
fn project_config_quality_checks_accept_required_canic_features() {
    let root = temp_dir("canic-cli-medic-project-required-features-present");
    let config = write_medic_config(
        &root,
        r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.app]
kind = "service"

[subnets.prime.canisters.app.auth]
role_attestation_cache = true
"#,
    );
    write_medic_package_with_canic_features(
        &root,
        "root",
        "demo",
        "root",
        &["auth-root-canister-sig-create"],
    );
    write_medic_package_with_canic_features(
        &root,
        "app",
        "demo",
        "app",
        &["auth-root-canister-sig-verify"],
    );

    let checks = project_config_quality_checks(&root, &[config]);

    assert!(checks.iter().any(|check| {
        check.subject == "demo.app"
            && check.code == "role_required_canic_feature_present"
            && check.status == MedicStatus::Pass
    }));
    assert!(!checks.iter().any(|check| {
        check.subject == "demo.app" && check.code == "role_required_canic_feature_missing"
    }));

    fs::remove_dir_all(root).expect("remove temp root");
}

// Ensure project medic accepts required features inherited from workspace dependencies.
#[test]
fn project_config_quality_checks_accept_workspace_required_canic_features() {
    let root = temp_dir("canic-cli-medic-project-workspace-required-features");
    let config = write_medic_config(
        &root,
        r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.app]
kind = "service"

[subnets.prime.canisters.app.auth]
role_attestation_cache = true
"#,
    );
    write_medic_workspace_canic_features(
        &root,
        &[
            "auth-root-canister-sig-create",
            "auth-root-canister-sig-verify",
        ],
    );
    write_medic_package(&root, "root", "demo", "root");
    write_medic_package(&root, "app", "demo", "app");

    let checks = project_config_quality_checks(&root, &[config]);

    assert!(checks.iter().any(|check| {
        check.subject == "demo.app"
            && check.code == "role_required_canic_feature_present"
            && check.status == MedicStatus::Pass
    }));
    assert!(!checks.iter().any(|check| {
        check.subject == "demo.app" && check.code == "role_required_canic_feature_missing"
    }));

    fs::remove_dir_all(root).expect("remove temp root");
}

// Ensure check ordering is deterministic by category.
#[test]
fn orders_checks_by_category() {
    let report = MedicReport::new(
        &MedicOptions::project(false, None, "icp".to_string()),
        vec![
            MedicCheck::pass(
                MedicCategory::BlobStorage,
                "blob_storage_not_selected",
                "blob_storage",
                "none",
                "none",
                MedicSource::Command,
            ),
            MedicCheck::pass(
                MedicCategory::Environment,
                "icp_cli_ok",
                "icp",
                "ok",
                "none",
                MedicSource::IcpCli,
            ),
        ],
    );

    assert_eq!(report.checks[0].category, MedicCategory::Environment);
    assert_eq!(report.checks[1].category, MedicCategory::BlobStorage);
}

// Ensure ICP CLI availability failures keep distinct stable medic codes.
#[test]
fn icp_cli_error_check_distinguishes_missing_cli() {
    let missing = icp_cli_error_check(IcpCommandError::MissingCli {
        executable: "icp-missing".to_string(),
    });
    let incompatible = icp_cli_error_check(IcpCommandError::IncompatibleCliVersion {
        executable: "icp".to_string(),
        found: "icp 0.1.0".to_string(),
    });

    assert_eq!(missing.status, MedicStatus::Fail);
    assert_eq!(missing.code, "icp_cli_missing");
    assert_eq!(incompatible.code, "icp_cli_incompatible");
}

// Ensure blob-storage medic uses the shared status summary without reinterpreting warnings.
#[test]
fn renders_blob_storage_medic_summary() {
    let check = blob_storage_medic_check_from_summary(BlobStorageMedicSummary {
        status: BlobStorageMedicStatus::Warning,
        detail: "readiness=warning; configured=true; gateways=0; funding=funding_needed"
            .to_string(),
        next: "canic blob-storage sync-gateways demo backend".to_string(),
    });
    let report = render_medic_text(&MedicReport::new(
        &MedicOptions::project(false, None, "icp".to_string()),
        vec![check],
    ));

    assert!(report.contains("blob_storage [warn] blob_storage_billing_unready"));
    assert!(report.contains("readiness=warning"));
    assert!(report.contains("canic blob-storage sync-gateways demo backend"));
}

// Ensure targeted blob-storage medic errors keep stable target-resolution codes.
#[test]
fn blob_storage_medic_error_check_classifies_target_errors() {
    let missing = blob_storage_medic_error_check(
        BlobStorageCommandError::UnknownTarget {
            deployment: "demo".to_string(),
            target: "store".to_string(),
        },
        "demo",
        "store",
    );
    let ambiguous = blob_storage_medic_error_check(
        BlobStorageCommandError::AmbiguousRole {
            deployment: "demo".to_string(),
            role: "store".to_string(),
        },
        "demo",
        "store",
    );
    let not_blob_storage = blob_storage_medic_error_check(
        BlobStorageCommandError::CandidUnavailable {
            deployment: "demo".to_string(),
            target: "store".to_string(),
        },
        "demo",
        "store",
    );
    let generic =
        blob_storage_medic_error_check(BlobStorageCommandError::ResponseParse, "demo", "store");

    assert_eq!(missing.code, "blob_storage_target_missing");
    assert_eq!(ambiguous.code, "blob_storage_target_ambiguous");
    assert_eq!(
        not_blob_storage.code,
        "blob_storage_target_not_blob_storage"
    );
    assert_eq!(generic.code, "blob_storage_billing_unready");
}

// Ensure auth-renewal medic uses the shared auth summary without mutating renewal state.
#[test]
fn renders_auth_renewal_medic_summary() {
    let check = auth_renewal_medic_check_from_summary(AuthRenewalMedicSummary {
        status: AuthRenewalMedicStatus::Warning,
        detail: "status=drift_detected; issuer_observation=drift_detected; drift_detected=true"
            .to_string(),
        next: "canic auth renewal status demo --issuer rrkah-fqaaa-aaaaa-aaaaq-cai".to_string(),
    });
    let report = render_medic_text(&MedicReport::new(
        &MedicOptions::project(false, None, "icp".to_string()),
        vec![check],
    ));

    assert!(report.contains("auth [warn] auth_renewal_drift_warn"));
    assert!(report.contains("status=drift_detected"));
    assert!(report.contains("canic auth renewal status demo --issuer"));
}

// Ensure targeted auth-renewal medic preserves the stable invalid-issuer code.
#[test]
fn auth_renewal_medic_error_check_classifies_invalid_issuer() {
    let invalid = auth_renewal_medic_error_check(
        AuthCommandError::InvalidIssuerPrincipal {
            issuer: "not a principal".to_string(),
        },
        "demo",
        "not a principal",
    );
    let generic = auth_renewal_medic_error_check(
        AuthCommandError::InstallState("missing state".to_string()),
        "demo",
        "rrkah-fqaaa-aaaaa-aaaaq-cai",
    );

    assert_eq!(invalid.status, MedicStatus::Fail);
    assert_eq!(invalid.code, "auth_renewal_issuer_invalid");
    assert_eq!(invalid.source, MedicSource::Command);
    assert_eq!(generic.code, "auth_renewal_drift_fail");
}

// Ensure default deployment medic can discover blob-storage-capable local Candid sidecars passively.
#[test]
fn passive_blob_storage_hint_uses_local_candid_only() {
    let root = temp_dir("canic-cli-medic-blob-storage-passive");
    write_candid(
        &root,
        "local",
        "backend",
        r#"
            service : {
                get_blob_storage_status : () -> () query;
                "_immutableObjectStorageUpdateGatewayPrincipals" : () -> ();
                "_immutableObjectStorageFundFromProjectCycles" : (nat) -> ();
            }
        "#,
    );
    write_candid(
        &root,
        "local",
        "other",
        r"
            service : {
                get_blob_storage_status : () -> () query;
            }
        ",
    );

    let roles = blob_storage_billing_roles_from_candid_dir(&root, "local");
    let options = MedicOptions {
        scope: MedicScope::Deployment,
        deployment: Some("demo".to_string()),
        blob_storage: None,
        auth_renewal: None,
        json: false,
        network: Some("local".to_string()),
        icp: "icp".to_string(),
    };
    let check = check_blob_storage_not_selected(&options, Some(&root), "local");

    assert_eq!(roles, vec!["backend".to_string()]);
    assert_eq!(check.status, MedicStatus::NotEvaluated);
    assert_eq!(check.code, "blob_storage_not_selected");
    assert_eq!(
        check.next,
        "run canic medic deployment demo --blob-storage backend"
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

// Ensure passive Candid detection only accepts the full billing endpoint trio.
#[test]
fn blob_storage_passive_detection_rejects_partial_or_unrelated_candid() {
    assert!(candid_declares_blob_storage_billing(
        r#"
            service : {
                get_blob_storage_status : () -> () query;
                "_immutableObjectStorageUpdateGatewayPrincipals" : () -> ();
                "_immutableObjectStorageFundFromProjectCycles" : (nat) -> ();
            }
        "#
    ));
    assert!(!candid_declares_blob_storage_billing(
        r#"
            service : {
                get_blob_storage_status : () -> () query;
                "_immutableObjectStorageUpdateGatewayPrincipals" : () -> ();
            }
        "#
    ));
    assert!(!candid_declares_blob_storage_billing(
        r"
            service : {
                canic_ready : () -> (bool) query;
            }
        "
    ));
}

// Ensure long medic details and next actions wrap to terminal-readable lines.
#[test]
fn wraps_long_medic_report_fields() {
    let report = render_medic_text(&MedicReport::new(
        &MedicOptions::project(false, None, "icp".to_string()),
        vec![MedicCheck::warn(
            MedicCategory::DeploymentState,
            "deployment_target_missing",
            "deployment",
            "this is a deliberately long diagnostic message that should wrap across multiple indented lines instead of widening a terminal table",
            "run canic install <fleet-template> or canic deploy register <deployment> --fleet-template <fleet-template> --root <principal> --allow-unverified",
            MedicSource::InstalledDeployment,
        )],
    ));

    assert!(report.contains("deployment_state [warn] deployment_target_missing"));
    assert!(
        report
            .lines()
            .all(|line| line.chars().count() <= MEDIC_REPORT_WIDTH)
    );
    assert!(
        report
            .lines()
            .any(|line| line.starts_with("          ") && !line.trim().is_empty())
    );
}

// Ensure unbroken long values cannot widen text reports past the fixed report width.
#[test]
fn wraps_unbroken_long_medic_report_fields() {
    let report = render_medic_text(&MedicReport::new(
        &MedicOptions::project(false, None, "icp".to_string()),
        vec![MedicCheck::warn(
            MedicCategory::DeploymentState,
            "deployment_target_missing",
            "deployment",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            MedicSource::InstalledDeployment,
        )],
    ));

    assert!(
        report
            .lines()
            .all(|line| line.chars().count() <= MEDIC_REPORT_WIDTH)
    );
}

// Ensure ICP identity session guidance stays informational and versionless.
#[test]
fn icp_identity_session_cache_hint_is_informational() {
    let check = check_icp_identity_session_cache_hint();

    assert_eq!(check.status, MedicStatus::Pass);
    assert_eq!(check.code, "icp_identity_session_hint");
    assert!(check.detail.contains("PEM identities"));
    assert!(check.next.contains("icp settings session-length"));
    assert!(check.next.contains("icp identity reauth"));
    assert!(!check.next.contains("1.0.0"));
}

// Ensure host installed-deployment missing-state errors remain classifiable.
#[test]
fn missing_installed_deployment_error_is_classifiable() {
    assert!(is_missing_installed_deployment(
        "deployment target demo is not installed on network local"
    ));
    assert!(!is_missing_installed_deployment(
        "failed to read canic deployment state: bad json"
    ));
}

fn sample_check(status: MedicStatus) -> MedicCheck {
    MedicCheck::new(
        MedicCategory::Environment,
        "sample",
        status,
        "subject",
        "detail",
        "next",
        MedicSource::Command,
    )
}

fn sample_install_state() -> InstallState {
    InstallState {
        schema_version: 2,
        deployment_name: "demo".to_string(),
        fleet_template: "demo".to_string(),
        created_at_unix_secs: 1,
        updated_at_unix_secs: 1,
        network: "local".to_string(),
        root_target: "root".to_string(),
        root_canister_id: "aaaaa-aa".to_string(),
        root_verification: canic_host::install_root::RootVerificationStatus::Verified,
        root_build_target: "root".to_string(),
        workspace_root: "/workspace".to_string(),
        icp_root: "/workspace".to_string(),
        config_path: "/workspace/fleets/demo/canic.toml".to_string(),
        release_set_manifest_path: "/workspace/.icp/local/canisters/root/root.release-set.json"
            .to_string(),
    }
}

fn sample_installed_deployment_resolution(
    entries: Vec<canic_host::registry::RegistryEntry>,
) -> InstalledDeploymentResolution {
    let mut roles_by_canister = std::collections::BTreeMap::new();
    for entry in &entries {
        if let Some(role) = &entry.role {
            roles_by_canister.insert(entry.pid.clone(), role.clone());
        }
    }

    InstalledDeploymentResolution {
        source: InstalledDeploymentSource::LocalReplica,
        state: sample_install_state(),
        registry: canic_host::installed_deployment::InstalledDeploymentRegistry {
            root_canister_id: "aaaaa-aa".to_string(),
            entries,
        },
        topology: canic_host::installed_deployment::ResolvedDeploymentTopology {
            root_canister_id: "aaaaa-aa".to_string(),
            children_by_parent: std::collections::BTreeMap::new(),
            roles_by_canister,
        },
    }
}

fn registry_entry(pid: &str, role: Option<&str>) -> canic_host::registry::RegistryEntry {
    canic_host::registry::RegistryEntry {
        pid: pid.to_string(),
        role: role.map(str::to_string),
        kind: Some("service".to_string()),
        parent_pid: Some("aaaaa-aa".to_string()),
        module_hash: None,
    }
}

fn write_candid(root: &std::path::Path, network: &str, role: &str, candid: &str) {
    let path = local_canister_candid_path(root, network, role);
    fs::create_dir_all(path.parent().expect("candid parent")).expect("create candid parent");
    fs::write(path, candid).expect("write candid");
}

fn write_medic_config(root: &std::path::Path, source: &str) -> std::path::PathBuf {
    let path = root.join("fleets").join("demo").join("canic.toml");
    fs::create_dir_all(path.parent().expect("config parent")).expect("create config parent");
    fs::write(&path, source).expect("write config");
    path
}

fn write_medic_package(root: &std::path::Path, package: &str, fleet: &str, role: &str) {
    write_medic_package_with_canic_features(root, package, fleet, role, &[]);
}

fn write_medic_workspace_canic_features(root: &std::path::Path, features: &[&str]) {
    let features = features
        .iter()
        .map(|feature| format!(r#""{feature}""#))
        .collect::<Vec<_>>()
        .join(", ");
    fs::write(
        root.join("Cargo.toml"),
        format!(
            r#"[workspace]
members = ["fleets/demo/root", "fleets/demo/app"]

[workspace.dependencies]
canic = {{ path = "crates/canic", features = [{features}] }}
"#
        ),
    )
    .expect("write workspace manifest");
}

fn write_medic_package_with_canic_features(
    root: &std::path::Path,
    package: &str,
    fleet: &str,
    role: &str,
    features: &[&str],
) {
    let path = root
        .join("fleets")
        .join("demo")
        .join(package)
        .join("Cargo.toml");
    fs::create_dir_all(path.parent().expect("package parent")).expect("create package parent");
    let features = features
        .iter()
        .map(|feature| format!(r#""{feature}""#))
        .collect::<Vec<_>>()
        .join(", ");
    fs::write(
        path,
        format!(
            r#"[package]
name = "{fleet}_{role}"
edition = "2024"
version = "0.1.0"

[dependencies]
canic = {{ workspace = true, features = [{features}] }}

[package.metadata.canic]
fleet = "{fleet}"
role = "{role}"
"#
        ),
    )
    .expect("write package manifest");
}

fn write_medic_deployment_receipt(
    root: &std::path::Path,
    network: &str,
    deployment: &str,
    receipt: DeploymentReceiptV1,
) {
    let receipt_dir = root
        .join(".canic")
        .join(network)
        .join("deployment-receipts")
        .join(deployment);
    fs::create_dir_all(&receipt_dir).expect("create receipt dir");
    fs::write(
        receipt_dir.join("unix_100-medic.json"),
        serde_json::to_vec_pretty(&receipt).expect("serialize receipt"),
    )
    .expect("write receipt");
}

fn write_medic_install_state(root: &std::path::Path, network: &str, state: &InstallState) {
    let state_dir = root.join(".canic").join(network).join("deployments");
    fs::create_dir_all(&state_dir).expect("create state dir");
    fs::write(
        state_dir.join(format!("{}.json", state.deployment_name)),
        serde_json::to_vec_pretty(state).expect("serialize install state"),
    )
    .expect("write install state");
}

fn sample_deployment_receipt(
    status: DeploymentExecutionStatusV1,
    result: DeploymentCommandResultV1,
    final_inventory: Option<&str>,
) -> DeploymentReceiptV1 {
    DeploymentReceiptV1 {
        schema_version: 1,
        operation_id: "op-1".to_string(),
        plan_id: "plan-1".to_string(),
        execution_context: None,
        operation_status: status,
        started_at: "2026-07-01T00:00:00Z".to_string(),
        finished_at: Some("2026-07-01T00:00:01Z".to_string()),
        operator_principal: None,
        root_principal: Some("aaaaa-aa".to_string()),
        previous_observed_deployment_epoch: None,
        phase_receipts: Vec::new(),
        role_phase_receipts: Vec::new(),
        final_inventory_id: final_inventory.map(str::to_string),
        command_result: result,
    }
}
