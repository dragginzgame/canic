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

fn write_candid(root: &std::path::Path, network: &str, role: &str, candid: &str) {
    let path = local_canister_candid_path(root, network, role);
    fs::create_dir_all(path.parent().expect("candid parent")).expect("create candid parent");
    fs::write(path, candid).expect("write candid");
}
