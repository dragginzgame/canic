use super::*;
use super::{
    auth::{auth_renewal_medic_check_from_summary, auth_renewal_medic_error_check},
    blob_storage::{
        blob_storage_billing_roles_from_candid_dir, blob_storage_medic_check_from_summary,
        blob_storage_medic_error_check, candid_declares_blob_storage_billing,
    },
    command::{medic_subcommand_help_requested, usage},
    fleet::fleet_environment_selection,
    render::{MEDIC_REPORT_WIDTH, render_medic_ci_text, render_medic_json, render_medic_text},
    report::{MedicStatus, aggregate_status},
    role_contract::workspace_config_quality_checks,
    workspace::workspace_environment_selection_check,
};
use crate::{
    CliError,
    auth::{AuthCommandError, AuthRenewalMedicStatus, AuthRenewalMedicSummary},
    blob_storage::{BlobStorageCommandError, BlobStorageMedicStatus, BlobStorageMedicSummary},
    cli_error_exit_code, render_cli_error,
    test_support::temp_dir,
};
use std::{ffi::OsString, fs};

use canic_core::ids::CanisterRole;
use canic_host::{
    fleet_ensure::CurrentFleetInventoryError,
    icp::local_canister_candid_path,
    state_manifest::{StateAuditStatus, build_state_audit_report},
};
use serde_json::Value as JsonValue;

// Ensure bare top-level medic selects the workspace scope without inventing a Fleet.
#[test]
fn parses_bare_workspace_medic_options() {
    let options = MedicOptions::parse([
        OsString::from(crate::cli::globals::INTERNAL_ICP_OPTION),
        OsString::from("/tmp/icp"),
    ])
    .expect("parse medic options");

    assert_eq!(options.scope, MedicScope::Workspace);
    assert_eq!(options.fleet, None);
    assert_eq!(options.environment, None);
    assert_eq!(options.icp, "/tmp/icp");
    assert!(!options.ci);
}

// Ensure the removed project scope has no compatibility command.
#[test]
fn rejects_removed_project_medic_subcommand() {
    std::assert_matches!(
        MedicOptions::parse([OsString::from("project")]),
        Err(MedicCommandError::Usage(_))
    );
}

// Ensure Fleet medic parses target, environment, and ICP selectors.
#[test]
fn parses_fleet_medic_options() {
    let options = MedicOptions::parse([
        OsString::from("fleet"),
        OsString::from("demo"),
        OsString::from(crate::cli::globals::INTERNAL_ENVIRONMENT_OPTION),
        OsString::from("local"),
        OsString::from(crate::cli::globals::INTERNAL_ICP_OPTION),
        OsString::from("/tmp/icp"),
    ])
    .expect("parse medic Fleet options");

    assert_eq!(options.scope, MedicScope::Fleet);
    assert_eq!(options.fleet.as_deref(), Some("demo"));
    assert_eq!(options.environment.as_deref(), Some("local"));
    assert_eq!(options.icp, "/tmp/icp");
    assert!(!options.ci);
}

// Ensure targeted blob-storage medic diagnostics are Fleet-only.
#[test]
fn parses_fleet_blob_storage_medic_target() {
    let options = MedicOptions::parse([
        OsString::from("fleet"),
        OsString::from("demo"),
        OsString::from("--blob-storage"),
        OsString::from("backend"),
    ])
    .expect("parse medic options");

    assert_eq!(options.fleet.as_deref(), Some("demo"));
    assert_eq!(options.blob_storage.as_deref(), Some("backend"));
}

// Ensure targeted auth-renewal medic diagnostics are Fleet-only.
#[test]
fn parses_fleet_auth_renewal_medic_target() {
    let options = MedicOptions::parse([
        OsString::from("fleet"),
        OsString::from("demo"),
        OsString::from("--auth-renewal"),
        OsString::from("rrkah-fqaaa-aaaaa-aaaaq-cai"),
    ])
    .expect("parse medic options");

    assert_eq!(options.fleet.as_deref(), Some("demo"));
    assert_eq!(
        options.auth_renewal.as_deref(),
        Some("rrkah-fqaaa-aaaaa-aaaaq-cai")
    );
}

// Ensure medic help explains the new top-level command surface.
#[test]
fn medic_usage_includes_top_level_examples() {
    let text = usage();

    assert!(text.contains("Diagnose local workspace and current-Fleet readiness"));
    assert!(text.contains("Usage: canic medic"));
    assert!(text.contains("canic medic"));
    assert!(text.contains("canic medic --ci"));
    assert!(text.contains("canic medic fleet test"));
    assert!(!text.contains("project"));
    assert!(text.contains("--json"));
}

// Ensure Fleet subcommand help requests stop before checks run.
#[test]
fn medic_subcommand_help_requests_are_not_targets() {
    assert!(medic_subcommand_help_requested(&[
        OsString::from("fleet"),
        OsString::from("--help")
    ]));
    assert!(medic_subcommand_help_requested(&[
        OsString::from("fleet"),
        OsString::from("-h")
    ]));
    assert!(medic_subcommand_help_requested(&[
        OsString::from("--json"),
        OsString::from("fleet"),
        OsString::from("--help")
    ]));
    assert!(medic_subcommand_help_requested(&[
        OsString::from("fleet"),
        OsString::from("--json"),
        OsString::from("--help")
    ]));
    assert!(medic_subcommand_help_requested(&[
        OsString::from(crate::cli::globals::INTERNAL_ENVIRONMENT_OPTION),
        OsString::from("local"),
        OsString::from("fleet"),
        OsString::from("--help")
    ]));
    assert!(medic_subcommand_help_requested(&[
        OsString::from("fleet"),
        OsString::from(crate::cli::globals::INTERNAL_ENVIRONMENT_OPTION),
        OsString::from("local"),
        OsString::from("--help")
    ]));
    assert!(!medic_subcommand_help_requested(&[
        OsString::from("fleet"),
        OsString::from("demo")
    ]));
    assert!(!medic_subcommand_help_requested(&[
        OsString::from("--json"),
        OsString::from("fleet"),
        OsString::from("demo")
    ]));
    assert!(!medic_subcommand_help_requested(&[
        OsString::from("fleet"),
        OsString::from("demo"),
        OsString::from("--help")
    ]));
    assert!(!medic_subcommand_help_requested(&[
        OsString::from("project"),
        OsString::from("--help")
    ]));
}

// Ensure aggregate status follows the medic report contract.
#[test]
fn aggregate_status_follows_report_contract() {
    assert_eq!(aggregate_status(&[]), MedicStatus::NotEvaluated);
    assert_eq!(
        aggregate_status(&[MedicCheck::not_evaluated(
            MedicCategory::FleetState,
            "fleet_not_selected",
            "fleet",
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
        &MedicOptions::workspace(false, false, None, "icp".to_string()),
        vec![
            MedicCheck::warn(
                MedicCategory::WorkspaceConfig,
                "local_environment_implicit",
                "environment",
                "no environment was selected",
                "select an explicit environment before Fleet checks",
                MedicSource::IcpConfig,
            ),
            MedicCheck::pass(
                MedicCategory::Environment,
                "icp_cli_ok",
                "icp",
                "icp 1.2.0",
                "none",
                MedicSource::IcpCli,
            ),
        ],
    );
    let rendered = render_medic_text(&report);

    assert!(rendered.starts_with("canic medic\nstatus: warn"));
    assert!(rendered.contains("environment: not selected"));
    assert!(rendered.contains("environment [pass] icp_cli_ok"));
    assert!(rendered.contains("workspace_config [warn] local_environment_implicit"));
    assert!(rendered.contains("  detail: no environment was selected"));
    assert!(rendered.contains("  next: select an explicit environment"));
    assert!(rendered.contains("  source: icp_config"));
}

// Ensure JSON output emits schema_version and stable top-level fields.
#[test]
fn renders_medic_json_report() {
    let report = MedicReport::new(
        &MedicOptions::workspace(true, false, None, "icp".to_string()),
        vec![sample_check(MedicStatus::Pass)],
    );
    let rendered = render_medic_json(&report).expect("render json");
    let value: JsonValue = serde_json::from_str(&rendered).expect("parse json");

    assert!(rendered.trim_start().starts_with('{'));
    assert!(!rendered.contains("status:"));
    assert!(!rendered.contains("detail:"));
    assert!(!rendered.contains("source:"));
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["command"], "canic medic");
    assert_eq!(value["scope"], "workspace");
    assert_eq!(value["environment"], JsonValue::Null);
    assert_eq!(value["fleet"], JsonValue::Null);
    assert_eq!(value["status"], "pass");
    assert!(value["checks"].is_array());
}

// Ensure workspace medic summarizes state-audit metadata without owning stable state.
#[test]
fn workspace_medic_summarizes_state_audit_status() {
    let resolution = StateManifestResolution::Rejected {
        errors: vec![RoleContractFinding::RoleUnknown {
            role: CanisterRole::owned("missing".to_string()),
        }],
    };
    let state_report = build_state_audit_report(&resolution, None);
    let check = state_audit_workspace_check(&resolution);
    let (expected_status, expected_code) = match state_report.status {
        StateAuditStatus::Pass => (MedicStatus::Pass, "state_audit_pass"),
        StateAuditStatus::Warn => (MedicStatus::Warn, "state_audit_warn"),
        StateAuditStatus::Fail => (MedicStatus::Fail, "state_audit_fail"),
        StateAuditStatus::NotEvaluated => (MedicStatus::NotEvaluated, "state_audit_not_evaluated"),
    };

    assert_eq!(check.category, MedicCategory::Runtime);
    assert_eq!(check.status, expected_status);
    assert_eq!(check.code, expected_code);
    assert_eq!(check.subject, "state_manifest");
    assert!(check.detail.contains("state audit status"));
    assert_eq!(check.source, MedicSource::StateManifest);
    if check.status != MedicStatus::Pass {
        assert!(check.next.contains("canic state audit"));
    }
}

// Ensure CI text output is short and includes only actionable checks.
#[test]
fn renders_medic_ci_report_with_actionable_rows_and_consistent_counts() {
    let report = MedicReport::new(
        &MedicOptions::workspace(false, true, None, "icp".to_string()),
        vec![
            MedicCheck::pass(
                MedicCategory::Environment,
                "icp_cli_ok",
                "icp",
                "icp 1.2.0",
                "none",
                MedicSource::IcpCli,
            ),
            MedicCheck::warn(
                MedicCategory::WorkspaceConfig,
                "local_environment_implicit",
                "environment",
                "no environment was selected",
                "select an explicit environment before Fleet checks",
                MedicSource::IcpConfig,
            ),
            MedicCheck::fail(
                MedicCategory::WorkspaceConfig,
                "role_contract_required_feature_missing",
                "demo.app",
                "demo.app requires canic feature `auth-root-canister-sig-verify`",
                "edit runtime [dependencies].canic",
                MedicSource::AppConfig,
            ),
        ],
    );
    let rendered = render_medic_ci_text(&report);

    assert!(rendered.starts_with(
        "canic medic\nstatus: fail\nscope: workspace\nstate_audit: not_evaluated\nchecks: 3\nwarnings: 1\nwarning_codes: local_environment_implicit\nfailures: 1\nfailure_codes: role_contract_required_feature_missing"
    ));
    assert!(
        rendered.contains("fail workspace_config role_contract_required_feature_missing demo.app")
    );
    assert!(rendered.contains("warn workspace_config local_environment_implicit environment"));
    assert!(rendered.contains("  next: edit runtime [dependencies].canic"));
    assert!(!rendered.contains("icp_cli_ok"));
}

// Ensure CI text output remains internally consistent when only a warning exists.
#[test]
fn renders_medic_ci_report_without_failures() {
    let report = MedicReport::new(
        &MedicOptions::workspace(false, true, None, "icp".to_string()),
        vec![sample_check(MedicStatus::Warn)],
    );
    let rendered = render_medic_ci_text(&report);

    assert_eq!(
        rendered,
        "canic medic\nstatus: warn\nscope: workspace\nstate_audit: not_evaluated\nchecks: 1\nwarnings: 1\nwarning_codes: sample\nfailures: 0\nfailure_codes: none\nwarn environment sample subject\n  detail: detail\n  next: next\n  source: command"
    );
}

#[test]
fn renders_medic_ci_state_audit_result() {
    let report = MedicReport::new(
        &MedicOptions::workspace(false, true, None, "icp".to_string()),
        vec![MedicCheck::pass(
            MedicCategory::Runtime,
            "state_audit_pass",
            "state_manifest",
            "state audit status pass with 1221 checks",
            "none",
            MedicSource::StateManifest,
        )],
    );
    let rendered = render_medic_ci_text(&report);

    assert_eq!(
        rendered,
        "canic medic\nstatus: pass\nscope: workspace\nstate_audit: pass\nchecks: 1\nwarnings: 0\nwarning_codes: none\nfailures: 0\nfailure_codes: none"
    );
}

// Ensure medic errors keep the designed process-level exit-code contract.
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

// Ensure Fleet reports include the effective environment while workspace reports may omit it.
#[test]
fn fleet_report_includes_effective_environment() {
    let report = MedicReport::new(
        &MedicOptions {
            scope: MedicScope::Fleet,
            fleet: Some("demo".to_string()),
            blob_storage: None,
            auth_renewal: None,
            json: false,
            ci: false,
            environment: None,
            icp: "icp".to_string(),
        },
        vec![sample_check(MedicStatus::Pass)],
    );

    assert_eq!(report.environment.as_deref(), Some("local"));
    assert_eq!(report.fleet.as_deref(), Some("demo"));
}

// Ensure an explicit operator environment wins over the local default.
#[test]
fn fleet_environment_selection_prefers_explicit_environment() {
    let options = MedicOptions {
        scope: MedicScope::Fleet,
        fleet: Some("demo".to_string()),
        blob_storage: None,
        auth_renewal: None,
        json: false,
        ci: false,
        environment: Some("local".to_string()),
        icp: "icp".to_string(),
    };

    let (environment, check) = fleet_environment_selection(&options);

    assert_eq!(environment, "local");
    assert_eq!(check.code, "local_environment_explicit");
    assert_eq!(check.source, MedicSource::Command);
}

// Ensure missing current targets point operators at the no-effect ensure planner.
#[test]
fn fleet_missing_points_to_current_ensure_plan() {
    let root = temp_dir("canic-cli-medic-missing-target-plan");
    fs::create_dir_all(&root).expect("create temp root");
    let options = MedicOptions {
        scope: MedicScope::Fleet,
        fleet: Some("demo".to_string()),
        blob_storage: None,
        auth_renewal: None,
        json: false,
        ci: false,
        environment: Some("local".to_string()),
        icp: "icp".to_string(),
    };
    let context = FleetMedicContext {
        icp_root: Some(root.clone()),
        environment: "local".to_string(),
        environment_check: MedicCheck::pass(
            MedicCategory::TargetEnvironment,
            "local_environment_explicit",
            "environment",
            "local",
            "none",
            MedicSource::Command,
        ),
    };

    let checks = run_fleet_checks(&options, &context);
    let missing = checks
        .iter()
        .find(|check| check.code == "current_fleet_not_converged")
        .expect("missing Fleet check");

    assert_eq!(missing.status, MedicStatus::Fail);
    assert!(
        missing
            .next
            .contains("canic fleet ensure demo --desired fleets/demo.toml")
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

// Ensure workspace-only environment selection is informational and does not duplicate Fleet checks.
#[test]
fn workspace_environment_selection_check_is_workspace_only() {
    let workspace = MedicOptions::workspace(false, false, None, "icp".to_string());
    let fleet = MedicOptions {
        scope: MedicScope::Fleet,
        fleet: Some("demo".to_string()),
        blob_storage: None,
        auth_renewal: None,
        json: false,
        ci: false,
        environment: None,
        icp: "icp".to_string(),
    };

    let workspace_check =
        workspace_environment_selection_check(&workspace).expect("workspace environment check");

    assert_eq!(workspace_check.code, "local_environment_implicit");
    assert_eq!(workspace_check.status, MedicStatus::NotEvaluated);
    assert!(workspace_check.next.contains("canic medic fleet"));
    assert!(workspace_environment_selection_check(&fleet).is_none());
}

// Ensure workspace medic validates package-role metadata without spawning Cargo.
#[test]
fn workspace_config_quality_checks_validate_role_package_metadata() {
    let root = temp_dir("canic-cli-medic-workspace-config-quality");
    let config = write_medic_config(
        &root,
        r#"
[app]
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



[component_specs.app]
component_role = "app"
maximum_instances = 1
"#,
    );
    write_medic_package(&root, "root", "demo", "root");
    write_medic_package(&root, "app", "demo", "app");
    write_medic_package(&root, "store", "demo", "store");

    let checks = workspace_config_quality_checks(&root, &[config]);

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

// Ensure package metadata drift is a blocking workspace-config diagnostic.
#[test]
fn workspace_config_quality_checks_fail_on_missing_or_mismatched_package_metadata() {
    let root = temp_dir("canic-cli-medic-workspace-config-metadata-drift");
    let config = write_medic_config(
        &root,
        r#"
[app]
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



[component_specs.app]
component_role = "app"
maximum_instances = 1

[component_specs.store]
component_role = "store"
maximum_instances = 1
"#,
    );
    write_medic_package(&root, "root", "demo", "root");
    write_medic_package(&root, "app", "demo", "other");

    let checks = workspace_config_quality_checks(&root, &[config]);

    let app = checks
        .iter()
        .find(|check| check.subject == "demo.app" && check.code == "role_package_metadata_missing")
        .expect("mismatched metadata check");
    assert_eq!(app.status, MedicStatus::Fail);
    assert!(app.detail.contains("expected app=demo role=app"));

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

// Ensure workspace medic reports config-driven runtime feature requirements before startup traps.
#[test]
fn workspace_config_quality_checks_report_missing_required_canic_features() {
    let root = temp_dir("canic-cli-medic-workspace-required-features");
    let config = write_medic_config(
        &root,
        r#"
[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"



[component_specs.app]
component_role = "app"
maximum_instances = 1

[component_specs.app.auth]
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

    let checks = workspace_config_quality_checks(&root, &[config]);

    let app = checks
        .iter()
        .find(|check| {
            check.subject == "demo.app" && check.code == "role_contract_required_feature_missing"
        })
        .expect("missing feature check");
    assert_eq!(app.status, MedicStatus::Fail);
    assert!(app.detail.contains("auth.role_attestation_cache"));
    assert!(app.detail.contains("auth-root-canister-sig-verify"));
    assert!(app.next.contains("apps/demo/app/Cargo.toml"));
    assert!(app.next.contains("runtime [dependencies].canic"));
    assert!(app.next.contains("not [build-dependencies]"));
    assert!(
        app.next.contains(
            r#"canic = { workspace = true, features = ["auth-root-canister-sig-verify"] }"#
        )
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

// Ensure workspace medic accepts roles whose runtime canic dependency enables required features.
#[test]
fn workspace_config_quality_checks_accept_required_canic_features() {
    let root = temp_dir("canic-cli-medic-workspace-required-features-present");
    let config = write_medic_config(
        &root,
        r#"
[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"



[component_specs.app]
component_role = "app"
maximum_instances = 1

[component_specs.app.auth]
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

    let checks = workspace_config_quality_checks(&root, &[config]);

    assert!(checks.iter().any(|check| {
        check.subject == "demo.app"
            && check.code == "role_required_canic_feature_present"
            && check.status == MedicStatus::Pass
    }));
    assert!(!checks.iter().any(|check| {
        check.subject == "demo.app" && check.code == "role_contract_required_feature_missing"
    }));

    fs::remove_dir_all(root).expect("remove temp root");
}

// Ensure workspace medic reports cryptographic implementations enabled for a role
// whose current contract does not use them.
#[test]
fn workspace_config_quality_checks_reject_surplus_crypto_features() {
    let root = temp_dir("canic-cli-medic-workspace-surplus-crypto-features");
    let config = write_medic_config(
        &root,
        r#"
[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"
"#,
    );
    write_medic_package_with_canic_features(&root, "root", "demo", "root", &["control-plane"]);
    write_medic_package_with_canic_features(
        &root,
        "app",
        "demo",
        "app",
        &["auth-delegated-token-verify"],
    );

    let checks = workspace_config_quality_checks(&root, &[config]);

    let surplus = checks
        .iter()
        .filter(|check| {
            check.subject == "demo.app" && check.code == "role_contract_surplus_crypto_feature"
        })
        .collect::<Vec<_>>();
    assert_eq!(surplus.len(), 1);
    assert!(surplus.iter().all(|check| {
        check.status == MedicStatus::Fail
            && check.detail.contains("without a role capability")
            && check
                .next
                .contains("remove the unused Canic cryptographic feature")
    }));

    fs::remove_dir_all(root).expect("remove temp root");
}

// Ensure workspace medic rejects role features inherited from workspace dependencies.
#[test]
fn workspace_config_quality_checks_reject_workspace_canic_features() {
    let root = temp_dir("canic-cli-medic-workspace-workspace-required-features");
    let config = write_medic_config(
        &root,
        r#"
[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"



[component_specs.app]
component_role = "app"
maximum_instances = 1

[component_specs.app.auth]
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

    let checks = workspace_config_quality_checks(&root, &[config]);

    let check = checks
        .iter()
        .find(|check| {
            check.subject == "demo.app"
                && check.code == "role_contract_dependency_shape_unsupported"
        })
        .unwrap_or_else(|| panic!("workspace feature rejection: {checks:#?}"));
    assert_eq!(check.status, MedicStatus::Fail);
    assert!(
        check
            .detail
            .contains("workspace Canic dependency must not select features")
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn workspace_config_quality_checks_reject_package_feature_forwarding() {
    let root = temp_dir("canic-cli-medic-workspace-feature-forwarding");
    let config = write_medic_config(
        &root,
        r#"
[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"



[component_specs.app]
component_role = "app"
maximum_instances = 1
"#,
    );
    write_medic_package_with_canic_features(&root, "root", "demo", "root", &["control-plane"]);
    write_medic_package(&root, "app", "demo", "app");
    let manifest = root.join("apps/demo/app/Cargo.toml");
    let mut source = fs::read_to_string(&manifest).expect("read app manifest");
    source.push_str(
        r#"
[features]
default = ["storage"]
storage = ["canic/blob-storage"]
"#,
    );
    fs::write(&manifest, source).expect("write forwarded package feature");

    let checks = workspace_config_quality_checks(&root, &[config]);
    let check = checks
        .iter()
        .find(|check| {
            check.subject == "demo.app"
                && check.code == "role_contract_dependency_shape_unsupported"
        })
        .unwrap_or_else(|| panic!("package shape finding: {checks:#?}"));
    assert_eq!(check.status, MedicStatus::Fail);
    assert!(check.detail.contains("must not forward"));

    fs::remove_dir_all(root).expect("remove temp root");
}

// Ensure check ordering is deterministic by category.
#[test]
fn orders_checks_by_category() {
    let report = MedicReport::new(
        &MedicOptions::workspace(false, false, None, "icp".to_string()),
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
        &MedicOptions::workspace(false, false, None, "icp".to_string()),
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
            fleet: "demo".to_string(),
            target: "store".to_string(),
        },
        "demo",
        "store",
    );
    let ambiguous = blob_storage_medic_error_check(
        BlobStorageCommandError::AmbiguousRole {
            fleet: "demo".to_string(),
            role: "store".to_string(),
        },
        "demo",
        "store",
    );
    let not_blob_storage = blob_storage_medic_error_check(
        BlobStorageCommandError::CandidUnavailable {
            fleet: "demo".to_string(),
            target: "store".to_string(),
        },
        "demo",
        "store",
    );
    let generic = blob_storage_medic_error_check(
        BlobStorageCommandError::ResponseValueOutOfRange {
            response_kind: "status",
            field: "sample",
        },
        "demo",
        "store",
    );

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
        &MedicOptions::workspace(false, false, None, "icp".to_string()),
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
        AuthCommandError::CurrentFleet(CurrentFleetInventoryError::NotConverged {
            environment: "local".to_string(),
            fleet: "demo".to_string(),
        }),
        "demo",
        "rrkah-fqaaa-aaaaa-aaaaq-cai",
    );

    assert_eq!(invalid.status, MedicStatus::Fail);
    assert_eq!(invalid.code, "auth_renewal_issuer_invalid");
    assert_eq!(invalid.source, MedicSource::Command);
    assert_eq!(generic.code, "auth_renewal_drift_fail");
}

// Ensure default Fleet medic can discover blob-storage-capable local Candid sidecars passively.
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
        scope: MedicScope::Fleet,
        fleet: Some("demo".to_string()),
        blob_storage: None,
        auth_renewal: None,
        json: false,
        ci: false,
        environment: Some("local".to_string()),
        icp: "icp".to_string(),
    };
    let check = check_blob_storage_not_selected(&options, Some(&root), "local");

    assert_eq!(roles, vec!["backend".to_string()]);
    assert_eq!(check.status, MedicStatus::NotEvaluated);
    assert_eq!(check.code, "blob_storage_not_selected");
    assert_eq!(
        check.next,
        "run canic medic fleet demo --blob-storage backend"
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
                canic_status : (variant { Readiness }) -> (bool) query;
            }
        "
    ));
}

// Ensure long medic details and next actions wrap to terminal-readable lines.
#[test]
fn wraps_long_medic_report_fields() {
    let report = render_medic_text(&MedicReport::new(
        &MedicOptions::workspace(false, false, None, "icp".to_string()),
        vec![MedicCheck::warn(
            MedicCategory::FleetState,
            "fleet_missing",
            "fleet",
            "this is a deliberately long diagnostic message that should wrap across multiple indented lines instead of widening a terminal table",
            "run canic fleet ensure demo --desired fleets/demo.toml",
            MedicSource::CurrentEnsure,
        )],
    ));

    assert!(report.contains("fleet_state [warn] fleet_missing"));
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
        &MedicOptions::workspace(false, false, None, "icp".to_string()),
        vec![MedicCheck::warn(
            MedicCategory::FleetState,
            "fleet_missing",
            "fleet",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            MedicSource::CurrentEnsure,
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

fn write_candid(root: &std::path::Path, environment: &str, role: &str, candid: &str) {
    let path = local_canister_candid_path(root, environment, role);
    fs::create_dir_all(path.parent().expect("candid parent")).expect("create candid parent");
    fs::write(path, candid).expect("write candid");
}

fn write_medic_config(root: &std::path::Path, source: &str) -> std::path::PathBuf {
    write_medic_role_contract_workspace(root, &[]);
    let path = root.join("apps").join("demo").join("canic.toml");
    fs::create_dir_all(path.parent().expect("config parent")).expect("create config parent");
    fs::write(&path, source).expect("write config");
    path
}

fn write_medic_package(root: &std::path::Path, package: &str, fleet: &str, role: &str) {
    write_medic_package_with_canic_features(root, package, fleet, role, &[]);
}

fn write_medic_workspace_canic_features(root: &std::path::Path, features: &[&str]) {
    write_medic_role_contract_workspace(root, features);
}

fn write_medic_role_contract_workspace(root: &std::path::Path, features: &[&str]) {
    fs::create_dir_all(root).expect("create medic fixture root");
    let features = features
        .iter()
        .map(|feature| format!(r#""{feature}""#))
        .collect::<Vec<_>>()
        .join(", ");
    fs::write(
        root.join("Cargo.toml"),
        format!(
            r#"[workspace]
members = ["crates/canic", "crates/canic-core", "apps/demo/*"]
resolver = "2"

[workspace.dependencies]
canic = {{ path = "crates/canic", default-features = false, features = [{features}] }}
"#
        ),
    )
    .expect("write workspace manifest");
    let canic_root = root.join("crates/canic");
    let core_root = root.join("crates/canic-core");
    fs::create_dir_all(canic_root.join("src")).expect("create fake Canic source dir");
    fs::create_dir_all(core_root.join("src")).expect("create fake Canic core source dir");
    fs::write(canic_root.join("src/lib.rs"), "").expect("write fake Canic lib");
    fs::write(core_root.join("src/lib.rs"), "").expect("write fake Canic core lib");
    fs::write(
        canic_root.join("Cargo.toml"),
        format!(
            r#"[package]
name = "canic"
version = "{}"
edition = "2024"

[features]
default = []
control-plane = []
fleet-coordinator-canister = []
wasm-store-canister = []
blob-storage = ["canic-core/blob-storage"]
blob-storage-billing = ["blob-storage", "canic-core/blob-storage-billing"]
sharding = ["canic-core/sharding"]
auth-chain-key-ecdsa = ["canic-core/auth-chain-key-ecdsa"]
auth-chain-key-root-sign = ["canic-core/auth-chain-key-root-sign"]
auth-root-canister-sig-create = ["canic-core/auth-root-canister-sig-create"]
auth-root-canister-sig-verify = ["canic-core/auth-root-canister-sig-verify"]
auth-issuer-canister-sig-create = ["canic-core/auth-issuer-canister-sig-create"]
auth-issuer-canister-sig-verify = ["canic-core/auth-issuer-canister-sig-verify"]
auth-delegated-token-verify = ["canic-core/auth-delegated-token-verify"]
internal-test-fixtures = ["canic-core/internal-test-fixtures"]

[dependencies]
canic-core = {{ path = "../canic-core" }}
"#,
            env!("CARGO_PKG_VERSION")
        ),
    )
    .expect("write fake Canic manifest");
    fs::write(
        core_root.join("Cargo.toml"),
        format!(
            r#"[package]
name = "canic-core"
version = "{}"
edition = "2024"

[features]
default = []
sharding = []
auth-chain-key-ecdsa = []
auth-chain-key-root-sign = ["auth-chain-key-ecdsa"]
auth-root-canister-sig-create = []
auth-root-canister-sig-verify = []
auth-issuer-canister-sig-create = []
auth-issuer-canister-sig-verify = []
auth-delegated-token-verify = ["auth-chain-key-ecdsa", "auth-issuer-canister-sig-verify"]
blob-storage = []
blob-storage-billing = ["blob-storage"]
internal-test-fixtures = []
"#,
            env!("CARGO_PKG_VERSION")
        ),
    )
    .expect("write fake Canic core manifest");
}

fn write_medic_package_with_canic_features(
    root: &std::path::Path,
    package: &str,
    fleet: &str,
    role: &str,
    features: &[&str],
) {
    let path = root
        .join("apps")
        .join("demo")
        .join(package)
        .join("Cargo.toml");
    fs::create_dir_all(path.parent().expect("package parent")).expect("create package parent");
    fs::create_dir_all(path.parent().expect("package parent").join("src"))
        .expect("create package source dir");
    fs::write(
        path.parent().expect("package parent").join("src/lib.rs"),
        "",
    )
    .expect("write package lib");
    let features = features
        .iter()
        .map(|feature| format!(r#""{feature}""#))
        .collect::<Vec<_>>()
        .join(", ");
    fs::write(
        &path,
        format!(
            r#"[package]
name = "{fleet}_{role}"
edition = "2024"
version = "0.1.0"
build = "build.rs"

[dependencies]
canic = {{ workspace = true, features = [{features}] }}

[build-dependencies]
canic = {{ workspace = true, features = [] }}

[package.metadata.canic]
app = "{fleet}"
role = "{role}"
"#
        ),
    )
    .expect("write package manifest");
    fs::write(
        path.parent().expect("package parent").join("build.rs"),
        "fn main() {\n    canic::build!(\"../canic.toml\");\n}\n",
    )
    .expect("write package build script");
    generate_medic_fixture_lockfile(root);
}

fn generate_medic_fixture_lockfile(root: &std::path::Path) {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = std::process::Command::new(cargo)
        .args([
            "generate-lockfile",
            "--offline",
            "--manifest-path",
            &root.join("Cargo.toml").display().to_string(),
        ])
        .output()
        .expect("run Cargo for medic fixture lockfile");
    assert!(
        output.status.success(),
        "failed to generate medic fixture lockfile: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
