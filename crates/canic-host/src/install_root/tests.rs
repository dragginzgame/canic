use super::{
    BuildInstallTargetsOperation, CompletedInstallPhase, EmitRootManifestOperation,
    EnsureRootCyclesOperation, INSTALL_STATE_SCHEMA_VERSION, InstallPhaseOperation,
    InstallReceiptScope, InstallRootOptions, InstallRootWasmOperation, InstallState,
    InstallTimingSummary, ResolveRootCanisterOperation, ResumeBootstrapOperation,
    StageReleaseSetOperation, WaitRootReadyOperation, add_create_root_target,
    add_icp_environment_target, add_local_root_create_cycles_arg, check_install_deployment_truth,
    check_install_execution_preflight, config_selection_error,
    current_install_deployment_truth_check_at, current_install_execution_context,
    current_install_executor_missing_capabilities, current_install_staging_evidence,
    discover_canic_config_choices, discover_project_canic_config_choices,
    enforce_install_deployment_truth_gate, fleet_install_state_path,
    icp_canister_command_in_network, install_deployment_truth_gate_lines,
    install_deployment_truth_gate_receipt, install_deployment_truth_phase_receipt,
    install_deployment_truth_receipt_path, is_missing_canister_id_error,
    latest_deployment_truth_receipt_path_from_root, parse_bootstrap_status_value,
    parse_canister_id_json, parse_created_canister_id, parse_cycle_balance_response,
    parse_root_ready_value, read_fleet_install_state, render_install_timing_summary,
    resolve_install_config_path, root_init_args, validate_expected_fleet_name,
    write_completed_install_phase_receipt, write_current_install_execution_preflight_receipt,
    write_install_deployment_truth_receipt, write_install_state,
    write_install_state_with_deployment_truth_receipt,
};
use crate::canister_build::CanisterBuildProfile;
use crate::deployment_truth::{
    CanisterControlClassV1, DeploymentCheckV1, DeploymentExecutionContextV1,
    DeploymentExecutionPreflightStatusV1, DeploymentExecutionStatusV1, DeploymentExecutorBackendV1,
    DeploymentExecutorCapabilityV1, DeploymentReceiptV1, ObservationStatusV1, ObservedCanisterV1,
    SafetyFindingV1, SafetySeverityV1, SafetyStatusV1, artifact_gate_phase_receipt,
    artifact_gate_role_phase_receipts, compare_plan_to_inventory, safety_report_from_diff,
};
use crate::icp::{CANIC_ICP_LOCAL_NETWORK_URL_ENV, CANIC_ICP_LOCAL_ROOT_KEY_ENV};
use crate::release_set::{ReleaseSetEntry, RootReleaseSetManifest, configured_install_targets};
use crate::test_support::temp_dir;
use serde_json::json;
use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::Duration,
};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[test]
fn parse_root_ready_accepts_plain_true() {
    assert!(parse_root_ready_value(&json!(true)));
}

#[test]
fn parse_root_ready_accepts_wrapped_ok_true() {
    assert!(parse_root_ready_value(&json!({ "Ok": true })));
}

#[test]
fn parse_root_ready_accepts_icp_cli_response_candid_true() {
    assert!(parse_root_ready_value(&json!({
        "response_candid": "(true)"
    })));
}

#[test]
fn parse_root_ready_rejects_false_shapes() {
    assert!(!parse_root_ready_value(&json!(false)));
    assert!(!parse_root_ready_value(&json!({ "Ok": false })));
    assert!(!parse_root_ready_value(&json!({ "Err": "nope" })));
}

#[test]
fn parse_bootstrap_status_accepts_plain_record() {
    let status = parse_bootstrap_status_value(&json!({
        "ready": false,
        "phase": "root:init:create_canisters",
        "last_error": null
    }))
    .expect("plain bootstrap status must parse");

    assert!(!status.ready);
    assert_eq!(status.phase, "root:init:create_canisters");
    assert_eq!(status.last_error, None);
}

#[test]
fn parse_bootstrap_status_accepts_wrapped_ok_record() {
    let status = parse_bootstrap_status_value(&json!({
        "Ok": {
            "ready": false,
            "phase": "failed",
            "last_error": "registry phase failed"
        }
    }))
    .expect("wrapped bootstrap status must parse");

    assert!(!status.ready);
    assert_eq!(status.phase, "failed");
    assert_eq!(status.last_error.as_deref(), Some("registry phase failed"));
}

#[test]
fn parse_bootstrap_status_accepts_icp_cli_response_candid() {
    let status = parse_bootstrap_status_value(&json!({
        "response_candid": r#"(
  record {
    89_620_959 = opt "registry phase failed";
    3_253_282_875 = "failed";
    3_870_990_435 = false;
  },
)"#
    }))
    .expect("icp cli response_candid bootstrap status must parse");

    assert!(!status.ready);
    assert_eq!(status.phase, "failed");
    assert_eq!(status.last_error.as_deref(), Some("registry phase failed"));
}

#[test]
fn icp_canister_command_carries_selected_network() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    unsafe {
        env::remove_var(CANIC_ICP_LOCAL_NETWORK_URL_ENV);
        env::remove_var(CANIC_ICP_LOCAL_ROOT_KEY_ENV);
    }
    let mut command = icp_canister_command_in_network(Path::new("/tmp/canic-icp-root"));
    command.args(["status", "root"]);
    add_icp_environment_target(&mut command, "ic");

    assert_eq!(command.get_program(), "icp");
    assert_eq!(
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        ["canister", "status", "root", "-e", "ic"]
    );
}

#[test]
fn local_canister_command_uses_http_target_when_configured() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    unsafe {
        env::set_var(CANIC_ICP_LOCAL_NETWORK_URL_ENV, "http://127.0.0.1:8000");
        env::set_var(CANIC_ICP_LOCAL_ROOT_KEY_ENV, "abcd");
    }
    let mut command = icp_canister_command_in_network(Path::new("/tmp/canic-icp-root"));
    command.env("ICP_ENVIRONMENT", "local");
    command.args(["status", "root"]);
    add_icp_environment_target(&mut command, "local");
    unsafe {
        env::remove_var(CANIC_ICP_LOCAL_NETWORK_URL_ENV);
        env::remove_var(CANIC_ICP_LOCAL_ROOT_KEY_ENV);
    }

    assert_eq!(
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        [
            "canister",
            "status",
            "root",
            "-n",
            "http://127.0.0.1:8000",
            "-k",
            "abcd"
        ]
    );
    assert!(
        command
            .get_envs()
            .any(|(key, value)| key == "ICP_ENVIRONMENT" && value.is_none())
    );
}

#[test]
fn local_http_fallback_creates_detached_root() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    unsafe {
        env::set_var(CANIC_ICP_LOCAL_NETWORK_URL_ENV, "http://127.0.0.1:8000");
    }
    let mut command = icp_canister_command_in_network(Path::new("/tmp/canic-icp-root"));
    add_create_root_target(&mut command, "root");
    unsafe {
        env::remove_var(CANIC_ICP_LOCAL_NETWORK_URL_ENV);
    }

    assert_eq!(
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        ["canister", "create", "--detached", "--json"]
    );
}

#[test]
fn environment_create_uses_named_root() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    unsafe {
        env::remove_var(CANIC_ICP_LOCAL_NETWORK_URL_ENV);
    }
    let mut command = icp_canister_command_in_network(Path::new("/tmp/canic-icp-root"));
    add_create_root_target(&mut command, "root");

    assert_eq!(
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        ["canister", "create", "root", "--json"]
    );
}

#[test]
fn parses_quiet_canister_create_output() {
    assert_eq!(
        parse_created_canister_id("Created canister:\nt63gs-up777-77776-aaaba-cai\n"),
        Some("t63gs-up777-77776-aaaba-cai".to_string())
    );
    assert_eq!(parse_created_canister_id("created root\n"), None);
}

#[test]
fn parses_json_canister_ids() {
    assert_eq!(
        parse_created_canister_id(r#"{"canister_id":"t63gs-up777-77776-aaaba-cai"}"#),
        Some("t63gs-up777-77776-aaaba-cai".to_string())
    );
    assert_eq!(
        parse_created_canister_id(r#"{"id":"t63gs-up777-77776-aaaba-cai","name":"root"}"#),
        Some("t63gs-up777-77776-aaaba-cai".to_string())
    );
    assert_eq!(
        parse_canister_id_json(&json!([{ "principal": "t63gs-up777-77776-aaaba-cai" }])),
        Some("t63gs-up777-77776-aaaba-cai".to_string())
    );
    assert_eq!(
        parse_created_canister_id(r#"{"canister_id":"not-a-principal"}"#),
        None
    );
}

#[test]
fn detects_missing_canister_id_errors() {
    assert!(is_missing_canister_id_error(
        "Error: failed to lookup canister ID for canister 'root' in environment 'local'"
    ));
    assert!(is_missing_canister_id_error(
        "could not find ID for canister 'root' in environment 'local'"
    ));
    assert!(!is_missing_canister_id_error(
        "Error: failed to connect to replica"
    ));
}

#[test]
fn install_timing_summary_uses_standard_table_format() {
    let timings = InstallTimingSummary {
        create_canisters: Duration::from_millis(1200),
        build_all: Duration::from_millis(2340),
        emit_manifest: Duration::from_millis(10),
        install_root: Duration::from_millis(20),
        fund_root: Duration::from_millis(30),
        stage_release_set: Duration::from_millis(40),
        resume_bootstrap: Duration::from_millis(50),
        wait_ready: Duration::from_millis(60),
        finalize_root_funding: Duration::from_millis(70),
    };

    let table = render_install_timing_summary(&timings, Duration::from_millis(3900));

    assert_eq!(
        table.lines().take(2).collect::<Vec<_>>(),
        vec![
            "PHASE                   ELAPSED",
            "---------------------   -------"
        ]
    );
    assert!(
        table.lines().any(
            |line| line.split_whitespace().collect::<Vec<_>>() == ["create_canisters", "1.20s"]
        )
    );
    assert!(
        table
            .lines()
            .any(|line| line.split_whitespace().collect::<Vec<_>>()
                == ["finalize_root_funding", "0.07s"])
    );
    assert!(
        table
            .lines()
            .any(|line| line.split_whitespace().collect::<Vec<_>>() == ["total", "3.90s"])
    );
}

#[test]
fn root_init_args_include_wasm_module_hash() {
    let root = temp_dir("canic-root-init-args");
    fs::create_dir_all(&root).expect("create temp root");
    let wasm = root.join("root.wasm");
    fs::write(&wasm, b"\0asm\x01\0\0\0").expect("write wasm");

    let args = root_init_args(&wasm).expect("build init args");

    fs::remove_dir_all(root).expect("remove temp root");
    assert!(args.starts_with("(variant { PrimeWithModuleHash = blob \""));
    assert!(args.ends_with("\" })"));
    assert!(args.contains("\\93\\A4\\4B\\BB"));
}

#[test]
fn configured_install_targets_use_root_subnet_release_roles_only() {
    let workspace_root = write_temp_workspace_config(
        r#"
[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.project_registry]
kind = "singleton"

[subnets.prime.canisters.user_hub]
kind = "singleton"

[subnets.extra.canisters.oracle_pokemon]
kind = "singleton"
"#,
    );

    let targets = configured_install_targets(&workspace_root.join("fleets/canic.toml"), "root")
        .expect("targets must resolve");

    assert_eq!(
        targets,
        vec![
            "root".to_string(),
            "project_registry".to_string(),
            "user_hub".to_string()
        ]
    );
}

#[test]
fn install_truth_preflight_uses_current_install_inputs_without_mutation() {
    with_guarded_env(|| {
        let root = temp_dir("canic-install-truth-preflight");
        fs::create_dir_all(root.join("fleets/demo")).expect("create config dir");
        fs::write(
            root.join("fleets/demo/canic.toml"),
            r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.user_hub]
kind = "singleton"
"#,
        )
        .expect("write config");
        write_wasm_gz_artifact(&root, "root", b"root-artifact");
        write_wasm_gz_artifact(&root, "wasm_store", b"wasm-store-artifact");
        write_wasm_gz_artifact(&root, "user_hub", b"user-hub-artifact");
        let previous_workspace_root = env::var_os("CANIC_WORKSPACE_ROOT");
        unsafe {
            env::set_var("CANIC_WORKSPACE_ROOT", &root);
        }

        let options = InstallRootOptions {
            root_canister: "root".to_string(),
            root_build_target: "root".to_string(),
            network: "local".to_string(),
            icp_root: Some(root.clone()),
            build_profile: Some(CanisterBuildProfile::Fast),
            ready_timeout_seconds: 30,
            config_path: Some("fleets/demo/canic.toml".to_string()),
            expected_fleet: Some("demo".to_string()),
            interactive_config_selection: false,
            deployment_plan_override: None,
        };

        let check = check_install_deployment_truth(&options, "2026-05-22T00:00:00Z")
            .expect("install truth preflight");
        let execution_preflight =
            check_install_execution_preflight(&options, "2026-05-22T00:00:01Z")
                .expect("install execution preflight");

        assert_eq!(check.check_id, "local:local:demo:check");
        assert_eq!(check.plan.fleet_template, "demo");
        assert_eq!(
            check
                .plan
                .role_artifacts
                .iter()
                .map(|artifact| artifact.build_profile.as_str())
                .collect::<Vec<_>>(),
            vec!["fast", "fast", "fast"]
        );
        assert_eq!(check.inventory.observed_artifacts.len(), 3);
        enforce_install_deployment_truth_gate(&check)
            .expect("complete local artifacts should pass gate");
        assert_eq!(execution_preflight.plan_id, check.plan.plan_id);
        assert_eq!(
            execution_preflight.backend,
            DeploymentExecutorBackendV1::CurrentCli
        );
        assert!(execution_preflight.missing_capabilities.is_empty());
        assert_eq!(
            execution_preflight.status,
            DeploymentExecutionPreflightStatusV1::Blocked
        );
        assert!(execution_preflight.blockers.iter().any(|finding| {
            finding.code == "authority_observation_missing"
                && finding.subject.as_deref() == Some("root")
        }));
        assert!(!root.join(".canic").exists());

        restore_env_var("CANIC_WORKSPACE_ROOT", previous_workspace_root);
        fs::remove_dir_all(root).expect("clean temp dir");
    });
}

#[test]
fn install_truth_artifact_gate_blocks_missing_built_artifacts() {
    let root = temp_dir("canic-install-truth-artifact-gate");
    let config_path = root.join("fleets/demo/canic.toml");
    fs::create_dir_all(config_path.parent().expect("config parent")).expect("create config dir");
    fs::write(
        &config_path,
        r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.user_hub]
kind = "singleton"
"#,
    )
    .expect("write config");
    write_wasm_gz_artifact(&root, "root", b"root-artifact");
    write_wasm_gz_artifact(&root, "wasm_store", b"wasm-store-artifact");

    let options = InstallRootOptions {
        root_canister: "root".to_string(),
        root_build_target: "root".to_string(),
        network: "local".to_string(),
        icp_root: Some(root.clone()),
        build_profile: Some(CanisterBuildProfile::Fast),
        ready_timeout_seconds: 30,
        config_path: Some("fleets/demo/canic.toml".to_string()),
        expected_fleet: Some("demo".to_string()),
        interactive_config_selection: false,
        deployment_plan_override: None,
    };

    let check = current_install_deployment_truth_check_at(
        &options,
        &root,
        &root,
        &config_path,
        "demo",
        "2026-05-22T00:00:00Z".to_string(),
    )
    .expect("deployment truth check");

    assert!(
        check
            .report
            .hard_failures
            .iter()
            .any(|finding| finding.code == "artifact_missing"
                && finding.subject.as_deref() == Some("user_hub"))
    );
    assert!(enforce_install_deployment_truth_gate(&check).is_err());

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_check_uses_supplied_deployment_plan_override() {
    let (root, mut check) = demo_install_deployment_truth_check(
        "canic-install-truth-supplied-deployment-plan-override",
    );
    check.plan.plan_id = "promoted-plan-1".to_string();
    let config_path = root.join("fleets/demo/canic.toml");
    let options = InstallRootOptions {
        root_canister: "root".to_string(),
        root_build_target: "root".to_string(),
        network: "local".to_string(),
        icp_root: Some(root.clone()),
        build_profile: Some(CanisterBuildProfile::Fast),
        ready_timeout_seconds: 30,
        config_path: Some("fleets/demo/canic.toml".to_string()),
        expected_fleet: Some("demo".to_string()),
        interactive_config_selection: false,
        deployment_plan_override: Some(check.plan),
    };

    let supplied_check = current_install_deployment_truth_check_at(
        &options,
        &root,
        &root,
        &config_path,
        "demo",
        "2026-05-22T00:00:00Z".to_string(),
    )
    .expect("deployment truth check");

    assert_eq!(supplied_check.plan.plan_id, "promoted-plan-1");
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_artifact_gate_blocks_materialized_digest_drift() {
    let root = temp_dir("canic-install-truth-artifact-digest-gate");
    let config_path = root.join("fleets/demo/canic.toml");
    fs::create_dir_all(config_path.parent().expect("config parent")).expect("create config dir");
    fs::write(
        &config_path,
        r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"
"#,
    )
    .expect("write config");
    write_wasm_gz_artifact(&root, "root", b"root-artifact");

    let options = InstallRootOptions {
        root_canister: "root".to_string(),
        root_build_target: "root".to_string(),
        network: "local".to_string(),
        icp_root: Some(root.clone()),
        build_profile: Some(CanisterBuildProfile::Fast),
        ready_timeout_seconds: 30,
        config_path: Some("fleets/demo/canic.toml".to_string()),
        expected_fleet: Some("demo".to_string()),
        interactive_config_selection: false,
        deployment_plan_override: None,
    };

    let mut check = current_install_deployment_truth_check_at(
        &options,
        &root,
        &root,
        &config_path,
        "demo",
        "2026-05-22T00:00:00Z".to_string(),
    )
    .expect("deployment truth check");
    check.plan.role_artifacts[0].observed_wasm_gz_file_sha256 =
        Some("different-observed-file-digest".to_string());
    check.diff = compare_plan_to_inventory(&check.plan, &check.inventory);
    check.report = safety_report_from_diff(
        "local:local:demo:report",
        Some("local:local:demo:diff".to_string()),
        &check.diff,
    );

    assert!(
        check
            .report
            .hard_failures
            .iter()
            .any(|finding| finding.code == "artifact_file_digest_mismatch")
    );
    assert!(enforce_install_deployment_truth_gate(&check).is_err());

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_gate_blocks_observed_controller_drift() {
    let root = temp_dir("canic-install-truth-controller-gate");
    let config_path = root.join("fleets/demo/canic.toml");
    fs::create_dir_all(config_path.parent().expect("config parent")).expect("create config dir");
    fs::write(
        &config_path,
        r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"
"#,
    )
    .expect("write config");
    write_wasm_gz_artifact(&root, "root", b"root-artifact");

    let options = InstallRootOptions {
        root_canister: "root".to_string(),
        root_build_target: "root".to_string(),
        network: "local".to_string(),
        icp_root: Some(root.clone()),
        build_profile: Some(CanisterBuildProfile::Fast),
        ready_timeout_seconds: 30,
        config_path: Some("fleets/demo/canic.toml".to_string()),
        expected_fleet: Some("demo".to_string()),
        interactive_config_selection: false,
        deployment_plan_override: None,
    };

    let mut check = current_install_deployment_truth_check_at(
        &options,
        &root,
        &root,
        &config_path,
        "demo",
        "2026-05-22T00:00:00Z".to_string(),
    )
    .expect("deployment truth check");
    check.plan.authority_profile.expected_controllers = vec!["aaaaa-aa".to_string()];
    check.inventory.observed_canisters = vec![ObservedCanisterV1 {
        canister_id: "aaaaa-aa".to_string(),
        role: Some("root".to_string()),
        control_class: CanisterControlClassV1::DeploymentControlled,
        controllers: vec!["external-controller".to_string()],
        module_hash: None,
        status: Some("running".to_string()),
        root_trust_anchor: Some("aaaaa-aa".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("icp_canister_status".to_string()),
    }];
    check.diff = compare_plan_to_inventory(&check.plan, &check.inventory);
    check.report = safety_report_from_diff(
        "local:local:demo:report",
        Some("local:local:demo:diff".to_string()),
        &check.diff,
    );

    assert!(
        check
            .report
            .hard_failures
            .iter()
            .any(|finding| finding.code == "expected_controller_missing")
    );
    assert!(enforce_install_deployment_truth_gate(&check).is_err());
    let receipt = install_deployment_truth_gate_receipt(
        &check,
        "start".to_string(),
        vec![artifact_gate_phase_receipt(
            &check,
            "start",
            Some("finish".into()),
        )],
        artifact_gate_role_phase_receipts(&check),
    );
    let lines = install_deployment_truth_gate_lines(&check, &receipt);
    assert!(
        lines
            .iter()
            .any(|line| line.contains("Deployment truth blocker: diff:expected_controller_missing"))
    );
    assert!(lines.iter().any(|line| {
        line.contains("Deployment truth receipt:") && line.contains("status=FailedBeforeMutation")
    }));
    let err = enforce_install_deployment_truth_gate(&check).unwrap_err();
    assert!(
        err.to_string()
            .contains("diff:expected_controller_missing:"),
        "unexpected error: {err}"
    );

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_gate_blocks_missing_expected_root_canister() {
    let root = temp_dir("canic-install-truth-missing-root-gate");
    let config_path = root.join("fleets/demo/canic.toml");
    fs::create_dir_all(config_path.parent().expect("config parent")).expect("create config dir");
    fs::write(
        &config_path,
        r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"
"#,
    )
    .expect("write config");
    write_wasm_gz_artifact(&root, "root", b"root-artifact");

    let options = InstallRootOptions {
        root_canister: "root".to_string(),
        root_build_target: "root".to_string(),
        network: "local".to_string(),
        icp_root: Some(root.clone()),
        build_profile: Some(CanisterBuildProfile::Fast),
        ready_timeout_seconds: 30,
        config_path: Some("fleets/demo/canic.toml".to_string()),
        expected_fleet: Some("demo".to_string()),
        interactive_config_selection: false,
        deployment_plan_override: None,
    };

    let mut check = current_install_deployment_truth_check_at(
        &options,
        &root,
        &root,
        &config_path,
        "demo",
        "2026-05-22T00:00:00Z".to_string(),
    )
    .expect("deployment truth check");
    check.plan.expected_canisters[0].canister_id = Some("aaaaa-aa".to_string());
    check.inventory.observed_canisters = vec![ObservedCanisterV1 {
        canister_id: "different-root".to_string(),
        role: Some("root".to_string()),
        control_class: CanisterControlClassV1::DeploymentControlled,
        controllers: vec!["aaaaa-aa".to_string()],
        module_hash: None,
        status: Some("running".to_string()),
        root_trust_anchor: Some("different-root".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("icp_canister_status".to_string()),
    }];
    check.diff = compare_plan_to_inventory(&check.plan, &check.inventory);
    check.report = safety_report_from_diff(
        "local:local:demo:report",
        Some("local:local:demo:diff".to_string()),
        &check.diff,
    );

    assert!(
        check
            .report
            .hard_failures
            .iter()
            .any(|finding| finding.code == "canister_missing")
    );
    let err = enforce_install_deployment_truth_gate(&check).unwrap_err();
    assert!(
        err.to_string().contains("canister_missing:"),
        "unexpected error: {err}"
    );

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_gate_blocks_all_safety_report_hard_failures() {
    let root = temp_dir("canic-install-truth-all-hard-failures");
    let config_path = root.join("fleets/demo/canic.toml");
    fs::create_dir_all(config_path.parent().expect("config parent")).expect("create config dir");
    fs::write(
        &config_path,
        r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"
"#,
    )
    .expect("write config");
    write_wasm_gz_artifact(&root, "root", b"root-artifact");

    let options = InstallRootOptions {
        root_canister: "root".to_string(),
        root_build_target: "root".to_string(),
        network: "local".to_string(),
        icp_root: Some(root.clone()),
        build_profile: Some(CanisterBuildProfile::Fast),
        ready_timeout_seconds: 30,
        config_path: Some("fleets/demo/canic.toml".to_string()),
        expected_fleet: Some("demo".to_string()),
        interactive_config_selection: false,
        deployment_plan_override: None,
    };

    let mut check = current_install_deployment_truth_check_at(
        &options,
        &root,
        &root,
        &config_path,
        "demo",
        "2026-05-22T00:00:00Z".to_string(),
    )
    .expect("deployment truth check");
    check.report.hard_failures.push(SafetyFindingV1 {
        code: "future_hard_failure".to_string(),
        message: "future deployment truth blocker".to_string(),
        severity: SafetySeverityV1::HardFailure,
        subject: Some("future.subject".to_string()),
    });

    let err = enforce_install_deployment_truth_gate(&check).unwrap_err();

    assert!(
        err.to_string().contains("future_hard_failure:"),
        "unexpected error: {err}"
    );

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_gate_persists_machine_readable_receipt() {
    let root = temp_dir("canic-install-truth-receipt-json");
    let config_path = root.join("fleets/demo/canic.toml");
    fs::create_dir_all(config_path.parent().expect("config parent")).expect("create config dir");
    fs::write(
        &config_path,
        r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"
"#,
    )
    .expect("write config");
    write_wasm_gz_artifact(&root, "root", b"root-artifact");

    let options = InstallRootOptions {
        root_canister: "root".to_string(),
        root_build_target: "root".to_string(),
        network: "local".to_string(),
        icp_root: Some(root.clone()),
        build_profile: Some(CanisterBuildProfile::Fast),
        ready_timeout_seconds: 30,
        config_path: Some("fleets/demo/canic.toml".to_string()),
        expected_fleet: Some("demo".to_string()),
        interactive_config_selection: false,
        deployment_plan_override: None,
    };

    let check = current_install_deployment_truth_check_at(
        &options,
        &root,
        &root,
        &config_path,
        "demo",
        "2026-05-22T00:00:00Z".to_string(),
    )
    .expect("deployment truth check");
    let receipt = install_deployment_truth_gate_receipt(
        &check,
        "unix:1770000000".to_string(),
        vec![artifact_gate_phase_receipt(
            &check,
            "unix:1770000000",
            Some("unix:1770000001".to_string()),
        )],
        artifact_gate_role_phase_receipts(&check),
    );

    let path = write_install_deployment_truth_receipt(&root, "local", "demo", &receipt)
        .expect("write deployment truth receipt");
    let expected_path = install_deployment_truth_receipt_path(&root, "local", "demo", &receipt)
        .expect("receipt path");

    assert_eq!(path, expected_path);
    assert_eq!(
        path.parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str()),
        Some("demo")
    );
    assert!(
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                !name.contains(':')
                    && Path::new(name)
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
            }),
        "unexpected receipt path: {}",
        path.display()
    );
    let decoded: DeploymentReceiptV1 =
        serde_json::from_slice(&fs::read(&path).expect("read receipt")).expect("decode receipt");
    assert_eq!(decoded, receipt);

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_phase_receipt_records_emit_manifest_evidence() {
    let root = temp_dir("canic-install-truth-emit-manifest-receipt");
    let config_path = root.join("fleets/demo/canic.toml");
    fs::create_dir_all(config_path.parent().expect("config parent")).expect("create config dir");
    fs::write(
        &config_path,
        r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"
"#,
    )
    .expect("write config");
    write_wasm_gz_artifact(&root, "root", b"root-artifact");

    let options = InstallRootOptions {
        root_canister: "root".to_string(),
        root_build_target: "root".to_string(),
        network: "local".to_string(),
        icp_root: Some(root.clone()),
        build_profile: Some(CanisterBuildProfile::Fast),
        ready_timeout_seconds: 30,
        config_path: Some("fleets/demo/canic.toml".to_string()),
        expected_fleet: Some("demo".to_string()),
        interactive_config_selection: false,
        deployment_plan_override: None,
    };
    let check = current_install_deployment_truth_check_at(
        &options,
        &root,
        &root,
        &config_path,
        "demo",
        "2026-05-22T00:00:00Z".to_string(),
    )
    .expect("deployment truth check");

    let receipt = install_deployment_truth_phase_receipt(
        &check,
        "emit_manifest",
        "unix:1770000002".to_string(),
        Some("unix:1770000003".to_string()),
        "emit root release-set manifest",
        ObservationStatusV1::Observed,
        vec!["manifest_path:/tmp/manifest.json".to_string()],
    );

    assert_eq!(
        receipt.operation_status,
        DeploymentExecutionStatusV1::Complete
    );
    assert_eq!(receipt.operation_id, "local:local:demo:check:emit_manifest");
    assert_eq!(receipt.phase_receipts.len(), 1);
    assert_eq!(receipt.phase_receipts[0].phase, "emit_manifest");
    assert_eq!(
        receipt.phase_receipts[0].verified_postcondition.status,
        ObservationStatusV1::Observed
    );
    assert_eq!(
        receipt.phase_receipts[0].verified_postcondition.evidence,
        vec!["manifest_path:/tmp/manifest.json".to_string()]
    );

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_completed_phase_receipt_records_pre_gate_evidence() {
    let (root, check) = demo_install_deployment_truth_check("canic-install-truth-pre-gate-phase");
    let execution_context = current_install_execution_context(&root, &root, "local");
    let scope = InstallReceiptScope {
        icp_root: &root,
        network: "local",
        fleet_name: "demo",
        check: &check,
        execution_context: Some(&execution_context),
    };

    let path = write_completed_install_phase_receipt(
        scope,
        CompletedInstallPhase {
            phase: "build_artifacts",
            attempted_action: "build configured install targets",
            started_at: "unix:1770000004".to_string(),
            finished_at: Some("unix:1770000005".to_string()),
            evidence: vec!["build_target:root".to_string()],
            role_names: vec!["root".to_string()],
        },
    )
    .expect("write completed phase receipt");
    let receipt: DeploymentReceiptV1 =
        serde_json::from_slice(&fs::read(path).expect("read receipt")).expect("decode receipt");

    assert_eq!(
        receipt.operation_id,
        "local:local:demo:check:build_artifacts"
    );
    assert_eq!(
        receipt.operation_status,
        DeploymentExecutionStatusV1::Complete
    );
    assert_eq!(receipt.phase_receipts[0].phase, "build_artifacts");
    assert_eq!(
        receipt.phase_receipts[0].verified_postcondition.evidence,
        vec!["build_target:root".to_string()]
    );
    assert_eq!(receipt.role_phase_receipts.len(), 1);
    assert_eq!(receipt.role_phase_receipts[0].role, "root");
    assert_eq!(receipt.role_phase_receipts[0].phase, "build_artifacts");
    assert_eq!(
        receipt.role_phase_receipts[0].result,
        crate::deployment_truth::RolePhaseResultV1::Applied
    );
    let execution_context = receipt
        .execution_context
        .expect("completed phase receipt should include execution context");
    assert_eq!(
        execution_context.backend,
        crate::deployment_truth::DeploymentExecutorBackendV1::CurrentCli
    );
    assert!(
        execution_context
            .artifact_roots
            .iter()
            .any(|root| { root.ends_with(".icp/local/canisters") })
    );

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_execution_preflight_receipt_records_ready_state() {
    let (root, mut check) =
        demo_install_deployment_truth_check("canic-install-truth-execution-preflight-ready");
    check.plan.expected_canisters.clear();
    check.report.status = SafetyStatusV1::Safe;
    check.report.summary = "deployment inventory matches plan".to_string();
    check.report.hard_failures.clear();
    let execution_context = current_install_execution_context(&root, &root, "local");

    let path = write_current_install_execution_preflight_receipt(
        &root,
        "local",
        "demo",
        &check,
        &execution_context,
    )
    .expect("write execution preflight receipt");
    let receipt: DeploymentReceiptV1 =
        serde_json::from_slice(&fs::read(path).expect("read receipt")).expect("decode receipt");

    assert_eq!(
        receipt.operation_id,
        "local:local:demo:check:execution_preflight"
    );
    assert_eq!(
        receipt.operation_status,
        DeploymentExecutionStatusV1::Complete
    );
    assert_eq!(receipt.phase_receipts[0].phase, "execution_preflight");
    assert!(
        receipt.phase_receipts[0]
            .verified_postcondition
            .evidence
            .contains(&"execution_preflight_status:Ready".to_string())
    );
    assert!(
        receipt.phase_receipts[0]
            .verified_postcondition
            .evidence
            .contains(&"blockers:0".to_string())
    );
    assert!(receipt.execution_context.is_some());

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_execution_preflight_receipt_records_blocked_state_before_error() {
    let (root, mut check) =
        demo_install_deployment_truth_check("canic-install-truth-execution-preflight-blocked");
    check.report.status = SafetyStatusV1::Blocked;
    check.report.hard_failures.push(SafetyFindingV1 {
        code: "deployment_artifact_missing".to_string(),
        message: "planned artifact was not observed".to_string(),
        severity: SafetySeverityV1::HardFailure,
        subject: Some("root".to_string()),
    });
    let execution_context = current_install_execution_context(&root, &root, "local");

    let err = write_current_install_execution_preflight_receipt(
        &root,
        "local",
        "demo",
        &check,
        &execution_context,
    )
    .expect_err("blocked execution preflight should stop install");

    assert!(
        err.to_string()
            .contains("deployment execution preflight blocked install")
    );
    let path = latest_deployment_truth_receipt_path_from_root(&root, "local", "demo")
        .expect("find latest receipt")
        .expect("blocked preflight receipt should be written");
    let receipt: DeploymentReceiptV1 =
        serde_json::from_slice(&fs::read(path).expect("read receipt")).expect("decode receipt");
    assert_eq!(
        receipt.operation_status,
        DeploymentExecutionStatusV1::FailedBeforeMutation
    );
    assert!(
        receipt.phase_receipts[0]
            .verified_postcondition
            .evidence
            .contains(&"execution_preflight_status:Blocked".to_string())
    );
    assert!(
        receipt.phase_receipts[0]
            .verified_postcondition
            .evidence
            .iter()
            .any(|line| line.starts_with("blocker:deployment_artifact_missing:"))
    );

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn current_install_staging_evidence_records_release_set_transport_facts() {
    let manifest = RootReleaseSetManifest {
        release_version: "0.43.4".to_string(),
        entries: vec![ReleaseSetEntry {
            role: "user_hub".to_string(),
            template_id: "embedded:user_hub".to_string(),
            artifact_relative_path: "local/canisters/user_hub/user_hub.wasm.gz".to_string(),
            payload_size_bytes: 42,
            payload_sha256_hex: "payload-hash".to_string(),
            chunk_size_bytes: 1_048_576,
            chunk_sha256_hex: vec!["chunk-a".to_string(), "chunk-b".to_string()],
        }],
    };

    let evidence = current_install_staging_evidence(
        "aaaaa-aa",
        Path::new("/workspace/.icp/local/canisters/root.release-set.json"),
        &manifest,
    );

    assert!(evidence.contains(&"root_canister:aaaaa-aa".to_string()));
    assert!(evidence.contains(&"release_version:0.43.4".to_string()));
    assert!(evidence.contains(&"staging_receipts:1".to_string()));
    assert!(evidence.contains(&"staging_role:user_hub".to_string()));
    assert!(evidence.contains(&"staging_transport:WasmStore".to_string()));
    assert!(evidence.contains(&"staging_chunks_prepared:2".to_string()));
    assert!(evidence.contains(&"staging_chunks_published:2".to_string()));
    assert!(evidence.contains(&"staging_postcondition:Observed".to_string()));
    assert!(evidence.contains(&"staging_wasm_store:root:aaaaa-aa:bootstrap".to_string()));
}

#[test]
fn resolve_root_canister_operation_owns_current_install_evidence() {
    let operation = ResolveRootCanisterOperation::new(
        Path::new("/workspace/.icp"),
        "local",
        "root",
        Path::new("/workspace/fleets/demo/canic.toml"),
    );

    let evidence = operation.evidence("aaaaa-aa");

    assert_eq!(evidence, ["root_target:root", "root_canister:aaaaa-aa"]);
}

#[test]
fn build_install_targets_operation_owns_current_install_evidence() {
    let operation = BuildInstallTargetsOperation::new(
        "local",
        vec!["root".to_string(), "wasm_store".to_string()],
        Some(CanisterBuildProfile::Fast),
        Path::new("/workspace/fleets/demo/canic.toml"),
        Path::new("/workspace/.icp"),
    );

    assert_eq!(
        operation.evidence(),
        ["build_target:root", "build_target:wasm_store"]
    );
    assert_eq!(operation.role_names(), ["root", "wasm_store"]);
}

#[test]
fn emit_root_manifest_operation_owns_current_install_evidence() {
    let _operation = EmitRootManifestOperation::new(
        Path::new("/workspace"),
        Path::new("/workspace/.icp"),
        "local",
        Path::new("/workspace/fleets/demo/canic.toml"),
    );

    let evidence = EmitRootManifestOperation::evidence(Path::new(
        "/workspace/.icp/local/canisters/root.release-set.json",
    ));

    assert_eq!(
        evidence,
        ["manifest_path:/workspace/.icp/local/canisters/root.release-set.json"]
    );
}

#[test]
fn stage_release_set_operation_owns_current_install_staging_evidence() {
    let manifest = RootReleaseSetManifest {
        release_version: "0.43.6".to_string(),
        entries: vec![ReleaseSetEntry {
            role: "root".to_string(),
            template_id: "embedded:root".to_string(),
            artifact_relative_path: "local/canisters/root/root.wasm.gz".to_string(),
            payload_size_bytes: 84,
            payload_sha256_hex: "payload-hash".to_string(),
            chunk_size_bytes: 1_048_576,
            chunk_sha256_hex: vec!["chunk-a".to_string()],
        }],
    };
    let operation = StageReleaseSetOperation::new(
        Path::new("/workspace/.icp"),
        "local",
        "aaaaa-aa",
        Path::new("/workspace/.icp/local/canisters/root.release-set.json"),
        manifest,
    );

    let evidence = operation.evidence();

    assert!(evidence.contains(&"root_canister:aaaaa-aa".to_string()));
    assert!(evidence.contains(&"release_version:0.43.6".to_string()));
    assert!(evidence.contains(&"staging_role:root".to_string()));
    assert!(evidence.contains(&"staging_transport:WasmStore".to_string()));
    assert!(evidence.contains(&"staging_chunks_prepared:1".to_string()));
    assert!(evidence.contains(&"staging_chunks_published:1".to_string()));
}

#[test]
fn install_root_wasm_operation_owns_current_install_evidence() {
    let operation = InstallRootWasmOperation::new(
        Path::new("/workspace/.icp"),
        "local",
        "aaaaa-aa",
        PathBuf::from("/workspace/.icp/local/canisters/root/root.wasm"),
    );

    let evidence = operation.evidence();

    assert!(evidence.contains(&"root_canister:aaaaa-aa".to_string()));
    assert!(
        evidence.contains(&"root_wasm:/workspace/.icp/local/canisters/root/root.wasm".to_string())
    );
}

#[test]
fn ensure_root_cycles_operation_owns_current_install_evidence() {
    let operation = EnsureRootCyclesOperation::new(
        Path::new("/workspace/.icp"),
        "local",
        "aaaaa-aa",
        "fund_root_pre_bootstrap",
        "ensure local root minimum cycles before bootstrap",
        "pre-bootstrap",
    );

    let evidence = operation.evidence();

    assert!(evidence.contains(&"root_canister:aaaaa-aa".to_string()));
    assert!(evidence.contains(&"minimum_cycles:100000000000000".to_string()));
    assert!(evidence.contains(&"funding_phase:pre-bootstrap".to_string()));
}

#[test]
fn resume_bootstrap_operation_owns_current_install_evidence() {
    let operation = ResumeBootstrapOperation::new("local", "aaaaa-aa");

    let evidence = operation.evidence();

    assert_eq!(evidence, ["root_canister:aaaaa-aa"]);
}

#[test]
fn wait_root_ready_operation_owns_current_install_evidence() {
    let operation = WaitRootReadyOperation::new("local", "aaaaa-aa", 30);

    let evidence = operation.evidence();

    assert!(evidence.contains(&"root_canister:aaaaa-aa".to_string()));
    assert!(evidence.contains(&"timeout_seconds:30".to_string()));
}

#[test]
fn current_install_activation_phases_use_operation_runner() {
    let source = include_str!("mod.rs");
    let activation = source_section(
        source,
        "fn run_root_activation_phases(",
        "#[derive(Clone, Copy)]",
    );

    for operation in [
        "install_operation",
        "pre_bootstrap_funding",
        "stage_operation",
        "resume_operation",
        "wait_ready_operation",
        "post_ready_funding",
    ] {
        assert!(
            activation.contains(&format!("run_operation(&{operation})")),
            "activation phase must run through operation runner: {operation}"
        );
    }
    assert!(
        !activation.contains("run_phase("),
        "activation phases must not manually wire receipt_scope.run_phase"
    );
}

#[test]
fn current_install_records_gates_before_activation_mutation() {
    let source = include_str!("mod.rs");
    let install = source_section(
        source,
        "pub fn install_root(",
        "struct PreparedInstallTruth",
    );
    assert_before(
        install,
        "prepare_install_deployment_truth(",
        "run_root_activation_phases(",
    );

    let prepare = source_section(
        source,
        "fn prepare_install_deployment_truth(",
        "fn resolve_root_canister_with_phase(",
    );
    assert_before(
        prepare,
        "ensure_current_install_executor_capabilities(execution_context)?",
        "run_install_deployment_truth_safety_gate(",
    );

    let gate = source_section(
        source,
        "fn run_install_deployment_truth_safety_gate(",
        "fn enforce_install_deployment_truth_gate(",
    );
    assert_before(
        gate,
        "enforce_install_deployment_truth_gate(&deployment_truth_check)?",
        "write_current_install_execution_preflight_receipt(",
    );
    assert_before(
        gate,
        "write_current_install_execution_preflight_receipt(",
        "Ok(deployment_truth_check)",
    );
}

fn source_section<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let start_index = source.find(start).expect("source section start exists");
    let end_index = source[start_index..]
        .find(end)
        .map(|offset| start_index + offset)
        .expect("source section end exists");
    &source[start_index..end_index]
}

fn assert_before(source: &str, before: &str, after: &str) {
    let before_index = source.find(before).expect("before marker exists");
    let after_index = source.find(after).expect("after marker exists");
    assert!(
        before_index < after_index,
        "`{before}` must appear before `{after}`"
    );
}

#[test]
fn install_truth_reports_executor_missing_required_capabilities() {
    let context = DeploymentExecutionContextV1 {
        workspace_root: Some("/workspace/canic".to_string()),
        icp_root: Some("/workspace/canic/.icp".to_string()),
        artifact_roots: vec!["/workspace/canic/.icp/local/canisters".to_string()],
        backend: DeploymentExecutorBackendV1::Other {
            name: "limited-test-backend".to_string(),
        },
        backend_capabilities: vec![DeploymentExecutorCapabilityV1::CanisterStatus],
    };

    assert_eq!(
        current_install_executor_missing_capabilities(&context),
        vec![
            DeploymentExecutorCapabilityV1::CreateCanister,
            DeploymentExecutorCapabilityV1::InstallCode,
            DeploymentExecutorCapabilityV1::Call,
            DeploymentExecutorCapabilityV1::Query,
            DeploymentExecutorCapabilityV1::StageArtifact,
        ],
    );
}

#[test]
fn install_truth_receipted_phase_records_success_and_failure() {
    let (root, check) = demo_install_deployment_truth_check("canic-install-truth-receipted-phase");
    let execution_context = current_install_execution_context(&root, &root, "local");
    let scope = InstallReceiptScope {
        icp_root: &root,
        network: "local",
        fleet_name: "demo",
        check: &check,
        execution_context: Some(&execution_context),
    };

    scope
        .run_phase(
            "install_root",
            "install root wasm",
            vec!["root_canister:aaaaa-aa".to_string()],
            || Ok(()),
        )
        .expect("successful phase should record");
    let err = scope
        .run_phase(
            "stage_release_set",
            "stage root release set",
            vec!["manifest_path:/tmp/release-set.json".to_string()],
            || Err::<(), Box<dyn std::error::Error>>("stage failed".into()),
        )
        .expect_err("failed phase should return original error");
    scope
        .run_phase(
            "wait_ready",
            "wait for root bootstrap readiness",
            vec!["timeout_seconds:30".to_string()],
            || Ok(()),
        )
        .expect("wait-ready phase should record");

    assert_eq!(err.to_string(), "stage failed");

    let receipt_dir = root.join(".canic/local/deployment-receipts/demo");
    let receipts = fs::read_dir(&receipt_dir)
        .expect("read receipts")
        .map(|entry| {
            let path = entry.expect("receipt entry").path();
            serde_json::from_slice::<DeploymentReceiptV1>(
                &fs::read(path).expect("read receipt JSON"),
            )
            .expect("decode receipt")
        })
        .collect::<Vec<_>>();
    let install = receipts
        .iter()
        .find(|receipt| receipt.operation_id.ends_with(":install_root"))
        .expect("install receipt");
    let stage = receipts
        .iter()
        .find(|receipt| receipt.operation_id.ends_with(":stage_release_set"))
        .expect("stage receipt");
    let wait = receipts
        .iter()
        .find(|receipt| receipt.operation_id.ends_with(":wait_ready"))
        .expect("wait-ready receipt");

    assert_eq!(
        install.operation_status,
        DeploymentExecutionStatusV1::Complete
    );
    assert_eq!(
        install.phase_receipts[0].verified_postcondition.status,
        ObservationStatusV1::Observed
    );
    assert_eq!(
        stage.operation_status,
        DeploymentExecutionStatusV1::FailedAfterMutation
    );
    assert_eq!(
        stage.phase_receipts[0].verified_postcondition.status,
        ObservationStatusV1::Inconclusive
    );
    assert_eq!(wait.operation_status, DeploymentExecutionStatusV1::Complete);
    assert_eq!(
        wait.phase_receipts[0].verified_postcondition.status,
        ObservationStatusV1::Observed
    );

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_latest_receipt_uses_newest_persisted_receipt() {
    let root = temp_dir("canic-install-truth-latest-receipt");
    let receipt_dir = root.join(".canic/local/deployment-receipts/demo");
    fs::create_dir_all(&receipt_dir).expect("create receipt dir");
    let older = receipt_dir.join("unix_100-local_demo_check_materialize_artifacts.json");
    let newer = receipt_dir.join("unix_200-local_demo_check_materialize_artifacts.json");
    let ignored = receipt_dir.join("unix_300-local_demo_check_materialize_artifacts.txt");
    fs::write(&older, "{}").expect("write older receipt");
    fs::write(&newer, "{}").expect("write newer receipt");
    fs::write(ignored, "{}").expect("write ignored file");

    let latest = latest_deployment_truth_receipt_path_from_root(&root, "local", "demo")
        .expect("latest receipt")
        .expect("receipt exists");

    assert_eq!(latest, newer);

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_gate_lines_include_warning_codes() {
    let root = temp_dir("canic-install-truth-warning-lines");
    let config_path = root.join("fleets/demo/canic.toml");
    fs::create_dir_all(config_path.parent().expect("config parent")).expect("create config dir");
    fs::write(
        &config_path,
        r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"
"#,
    )
    .expect("write config");
    write_wasm_gz_artifact(&root, "root", b"root-artifact");
    write_wasm_gz_artifact(&root, "wasm_store", b"wasm-store-artifact");

    let options = InstallRootOptions {
        root_canister: "root".to_string(),
        root_build_target: "root".to_string(),
        network: "local".to_string(),
        icp_root: Some(root.clone()),
        build_profile: Some(CanisterBuildProfile::Fast),
        ready_timeout_seconds: 30,
        config_path: Some("fleets/demo/canic.toml".to_string()),
        expected_fleet: Some("demo".to_string()),
        interactive_config_selection: false,
        deployment_plan_override: None,
    };

    let mut check = current_install_deployment_truth_check_at(
        &options,
        &root,
        &root,
        &config_path,
        "demo",
        "2026-05-22T00:00:00Z".to_string(),
    )
    .expect("deployment truth check");
    check.report.warnings.push(SafetyFindingV1 {
        code: "observation_gap".to_string(),
        message: "live root status was not observed".to_string(),
        severity: SafetySeverityV1::Warning,
        subject: Some("live_canister_status.root".to_string()),
    });

    let receipt = install_deployment_truth_gate_receipt(
        &check,
        "start".to_string(),
        vec![artifact_gate_phase_receipt(
            &check,
            "start",
            Some("finish".into()),
        )],
        artifact_gate_role_phase_receipts(&check),
    );
    let lines = install_deployment_truth_gate_lines(&check, &receipt);

    assert!(lines.iter().any(|line| {
        line.contains("Deployment truth receipt:") && line.contains("status=Complete")
    }));
    assert!(lines.iter().any(|line| line.contains(
        "Deployment truth warning: inventory:observation_gap:live_canister_status.root"
    )));
    assert!(lines.iter().any(|line| {
        line.contains("Deployment truth role receipt: phase=materialize_artifacts role=root")
    }));

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_gate_lines_distinguish_plan_assumptions() {
    let root = temp_dir("canic-install-truth-plan-assumption-lines");
    let config_path = root.join("fleets/demo/canic.toml");
    fs::create_dir_all(config_path.parent().expect("config parent")).expect("create config dir");
    fs::write(
        &config_path,
        r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"
"#,
    )
    .expect("write config");
    write_wasm_gz_artifact(&root, "root", b"root-artifact");

    let options = InstallRootOptions {
        root_canister: "root".to_string(),
        root_build_target: "root".to_string(),
        network: "local".to_string(),
        icp_root: Some(root.clone()),
        build_profile: Some(CanisterBuildProfile::Fast),
        ready_timeout_seconds: 30,
        config_path: Some("fleets/demo/canic.toml".to_string()),
        expected_fleet: Some("demo".to_string()),
        interactive_config_selection: false,
        deployment_plan_override: None,
    };

    let check = current_install_deployment_truth_check_at(
        &options,
        &root,
        &root,
        &config_path,
        "demo",
        "2026-05-22T00:00:00Z".to_string(),
    )
    .expect("deployment truth check");
    let receipt = install_deployment_truth_gate_receipt(
        &check,
        "start".to_string(),
        vec![artifact_gate_phase_receipt(
            &check,
            "start",
            Some("finish".into()),
        )],
        artifact_gate_role_phase_receipts(&check),
    );
    let lines = install_deployment_truth_gate_lines(&check, &receipt);

    assert!(lines.iter().any(|line| {
        line.contains("Deployment truth warning: plan:plan_assumption:local_state.root_canister_id")
    }));

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn local_root_create_adds_configured_cycle_funding() {
    let workspace_root = write_temp_workspace_config(
        r#"
[subnets.prime]
auto_create = ["app"]

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.app]
kind = "singleton"
"#,
    );
    let mut command = std::process::Command::new("icp");
    command.args(["canister", "create", "root", "-q"]);

    add_local_root_create_cycles_arg(
        &mut command,
        &workspace_root.join("fleets/canic.toml"),
        "local",
    )
    .expect("local cycles arg");

    assert_eq!(
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        [
            "canister",
            "create",
            "root",
            "-q",
            "--cycles",
            "110000000000000"
        ]
    );
}

#[test]
fn parses_root_cycle_balance_response() {
    assert_eq!(
        parse_cycle_balance_response("(variant { 17_724 = 4_487_280_757_485 : nat })"),
        Some(4_487_280_757_485)
    );
    assert_eq!(
        parse_cycle_balance_response(
            r"
(
  variant {
    Ok = 99_999_000_000_000 : nat;
  },
)
"
        ),
        Some(99_999_000_000_000)
    );
    assert_eq!(
        parse_cycle_balance_response(
            r#"{"response_candid":"(variant { Ok = 99_999_000_000_000 : nat })"}"#
        ),
        Some(99_999_000_000_000)
    );
    assert_eq!(
        parse_cycle_balance_response("(variant { Err = record { code = 1 : nat } })"),
        None
    );
}

#[test]
fn nonlocal_root_create_does_not_add_cycle_funding() {
    let workspace_root = write_temp_workspace_config(
        r#"
[subnets.prime.canisters.root]
kind = "root"
"#,
    );
    let mut command = std::process::Command::new("icp");
    command.args(["canister", "create", "root", "-q"]);

    add_local_root_create_cycles_arg(
        &mut command,
        &workspace_root.join("fleets/canic.toml"),
        "ic",
    )
    .expect("nonlocal cycles arg");

    assert_eq!(
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        ["canister", "create", "root", "-q"]
    );
}

#[test]
fn install_config_defaults_to_project_config_when_present() {
    with_guarded_env(|| {
        let root = temp_dir("canic-install-config-default");
        let config = root.join("fleets/canic.toml");
        fs::create_dir_all(config.parent().expect("config parent")).expect("create parent");
        fs::write(&config, "").expect("write config");
        let previous = env::var_os("CANIC_CONFIG_PATH");
        unsafe {
            env::remove_var("CANIC_CONFIG_PATH");
        }

        let resolved = resolve_install_config_path(&root, None, false).expect("resolve config");

        assert_eq!(resolved, config);
        restore_env_var("CANIC_CONFIG_PATH", previous);
        fs::remove_dir_all(root).expect("clean temp dir");
    });
}

#[test]
fn install_config_accepts_explicit_path() {
    let root = temp_dir("canic-install-config-explicit");
    let resolved = resolve_install_config_path(&root, Some("fleets/demo/canic.toml"), false)
        .expect("resolve config");

    assert_eq!(resolved, root.join("fleets/demo/canic.toml"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn install_config_error_lists_choices_when_project_default_missing() {
    with_guarded_env(|| {
        let root = temp_dir("canic-install-config-choices");
        let demo = root.join("fleets/demo/canic.toml");
        let test = root.join("canisters/test/runtime_probe/canic.toml");
        fs::create_dir_all(demo.parent().expect("demo parent")).expect("create demo parent");
        fs::create_dir_all(test.parent().expect("test parent")).expect("create test parent");
        fs::create_dir_all(root.join("fleets/demo/root")).expect("create demo root");
        fs::write(
            &demo,
            r#"
[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.app]
kind = "singleton"

[subnets.prime.canisters.user_hub]
kind = "singleton"
"#,
        )
        .expect("write demo config");
        fs::write(&test, "").expect("write test config");
        fs::write(root.join("fleets/demo/root/Cargo.toml"), "").expect("write demo root manifest");
        let previous = env::var_os("CANIC_CONFIG_PATH");
        unsafe {
            env::remove_var("CANIC_CONFIG_PATH");
        }

        let err = resolve_install_config_path(&root, None, false).expect_err("selection must fail");
        let message = err.to_string();

        assert!(message.contains("missing default Canic config at fleets/canic.toml"));
        assert!(!message.contains("found one install config:"));
        assert!(message.contains("fleets/demo/canic.toml"));
        assert!(message.contains("3 (root, app, user_hub)"));
        assert!(message.contains("fleets/canic.toml\n\n#"));
        assert!(message.contains("3 (root, app, user_hub)\n\nrun:"));
        assert!(!message.contains("canisters/test/runtime_probe/canic.toml"));
        assert!(message.contains("run: canic install demo"));

        restore_env_var("CANIC_CONFIG_PATH", previous);
        fs::remove_dir_all(root).expect("clean temp dir");
    });
}

#[test]
fn config_selection_error_is_whitespace_table() {
    let root = temp_dir("canic-install-config-single-table");
    let config = root.join("fleets/demo/canic.toml");
    fs::create_dir_all(config.parent().expect("config parent")).expect("create config parent");
    fs::write(
        &config,
        r#"
[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.app]
kind = "singleton"
"#,
    )
    .expect("write config");
    let message = config_selection_error(
        &root,
        &root.join("fleets/canic.toml"),
        std::slice::from_ref(&config),
    );

    assert!(message.contains('#'));
    assert!(message.contains("CONFIG"));
    assert!(message.contains("CANISTERS"));
    assert!(message.contains("fleets/demo/canic.toml"));
    assert!(message.contains("2 (root, app)"));
    assert!(message.contains("fleets/canic.toml\n\n#"));
    assert!(message.contains("2 (root, app)\n\nrun:"));
    assert!(message.contains("run: canic install demo"));
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn config_selection_error_lists_multiple_paths_with_numbered_options() {
    let root = temp_dir("canic-install-config-multiple-table");
    let demo = root.join("fleets/demo/canic.toml");
    let example = root.join("fleets/example/canic.toml");
    fs::create_dir_all(demo.parent().expect("demo parent")).expect("create demo parent");
    fs::create_dir_all(example.parent().expect("example parent")).expect("create example parent");
    fs::write(
        &demo,
        r#"
[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.app]
kind = "singleton"
"#,
    )
    .expect("write demo config");
    fs::write(
        &example,
        r#"
[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.user_hub]
kind = "singleton"

[subnets.prime.canisters.user_shard]
kind = "singleton"

[subnets.prime.canisters.scale_replica]
kind = "singleton"

[subnets.prime.canisters.scale_hub]
kind = "singleton"
"#,
    )
    .expect("write example config");
    let message = config_selection_error(&root, &root.join("fleets/canic.toml"), &[demo, example]);

    assert!(message.contains("choose a fleet explicitly:"));
    assert!(message.contains("choose a fleet explicitly:\n\n#"));
    assert!(message.contains('#'));
    assert!(message.contains("CONFIG"));
    assert!(message.contains("CANISTERS"));
    assert!(message.contains("1   fleets/demo/canic.toml"));
    assert!(message.contains("2   fleets/example/canic.toml"));
    assert!(message.contains("fleets/demo/canic.toml"));
    assert!(message.contains("2 (root, app)"));
    assert!(message.contains("fleets/example/canic.toml"));
    assert!(message.contains("5 (root, scale_hub, scale_replica, user_hub, user_shard)"));
    assert!(message.contains("5 (root, scale_hub, scale_replica, user_hub, user_shard)\n\nrun:"));
    assert!(message.contains("run: canic install <fleet>"));
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn config_selection_preview_lists_six_canisters_before_ellipsis() {
    let root = temp_dir("canic-install-config-preview-limit");
    let config = root.join("fleets/demo/canic.toml");
    fs::create_dir_all(config.parent().expect("config parent")).expect("create config parent");
    fs::write(
        &config,
        r#"
[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.app]
kind = "singleton"

[subnets.prime.canisters.minimal]
kind = "singleton"

[subnets.prime.canisters.scale_replica]
kind = "singleton"

[subnets.prime.canisters.scale_hub]
kind = "singleton"

[subnets.prime.canisters.user_hub]
kind = "singleton"

[subnets.prime.canisters.user_shard]
kind = "singleton"

[subnets.prime.canisters.worker]
kind = "singleton"
"#,
    )
    .expect("write config");

    let message = config_selection_error(
        &root,
        &root.join("fleets/canic.toml"),
        std::slice::from_ref(&config),
    );

    assert!(message.contains("8 (root, app, minimal, scale_hub, scale_replica, user_hub, ...)"));
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn discovered_install_config_choices_are_path_sorted() {
    let root = temp_dir("canic-install-config-sorted");
    let alpha = root.join("alpha/canic.toml");
    let zeta = root.join("zeta/canic.toml");
    fs::create_dir_all(alpha.parent().expect("alpha parent").join("root"))
        .expect("create alpha root");
    fs::create_dir_all(zeta.parent().expect("zeta parent").join("root")).expect("create zeta root");
    fs::write(&zeta, "").expect("write zeta config");
    fs::write(&alpha, "").expect("write alpha config");
    fs::write(
        alpha
            .parent()
            .expect("alpha parent")
            .join("root/Cargo.toml"),
        "",
    )
    .expect("write alpha root manifest");
    fs::write(
        zeta.parent().expect("zeta parent").join("root/Cargo.toml"),
        "",
    )
    .expect("write zeta root manifest");

    let choices = discover_canic_config_choices(&root).expect("discover choices");

    assert_eq!(choices, vec![alpha, zeta]);
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn discovered_install_config_choices_accept_split_source_fleet_configs() {
    let root = temp_dir("canic-install-config-split-source");
    let config = root.join("toko/canic.toml");
    fs::create_dir_all(config.parent().expect("config parent")).expect("create config parent");
    fs::write(&config, "[fleet]\nname = \"toko\"\n").expect("write config");

    let choices = discover_canic_config_choices(&root).expect("discover choices");

    assert_eq!(choices, vec![config]);
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn discovered_workspace_config_choices_accept_root_fleets() {
    let root = temp_dir("canic-install-config-root-fleets");
    let config = root.join("fleets/toko/canic.toml");
    fs::create_dir_all(config.parent().expect("config parent")).expect("create config parent");
    fs::write(&config, "[fleet]\nname = \"toko\"\n").expect("write config");

    let choices = discover_project_canic_config_choices(&root).expect("discover choices");

    assert_eq!(choices, vec![config]);
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn discovered_install_config_choices_reject_duplicate_fleet_names() {
    let root = temp_dir("canic-install-config-duplicate-fleet");
    let demo = root.join("demo/canic.toml");
    let copy = root.join("copy/canic.toml");
    fs::create_dir_all(demo.parent().expect("demo parent").join("root")).expect("create demo root");
    fs::create_dir_all(copy.parent().expect("copy parent").join("root")).expect("create copy root");
    fs::write(
        demo.parent().expect("demo parent").join("root/Cargo.toml"),
        "",
    )
    .expect("write demo root manifest");
    fs::write(
        copy.parent().expect("copy parent").join("root/Cargo.toml"),
        "",
    )
    .expect("write copy root manifest");
    let config = r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[subnets.prime.canisters.root]
kind = "root"
"#;
    fs::write(&demo, config).expect("write demo config");
    fs::write(&copy, config).expect("write copy config");

    let err = discover_canic_config_choices(&root).expect_err("duplicate fleet names should fail");
    let message = err.to_string();

    assert!(message.contains("multiple configs declare fleet demo"));
    assert!(message.contains("demo/canic.toml"));
    assert!(message.contains("copy/canic.toml"));
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_rejects_config_identity_mismatch() {
    let err =
        validate_expected_fleet_name(Some("demo"), "test", Path::new("fleets/demo/canic.toml"))
            .expect_err("mismatched fleet identity should fail");

    assert!(err.to_string().contains(
        "install requested fleet demo, but fleets/demo/canic.toml declares [fleet].name = \"test\""
    ));
}

#[test]
fn install_state_path_is_scoped_by_network() {
    assert_eq!(
        fleet_install_state_path(&PathBuf::from("/tmp/canic-project"), "local", "demo"),
        PathBuf::from("/tmp/canic-project/.canic/local/fleets/demo.json")
    );
}

#[test]
fn install_state_round_trips_from_project_state_dir() {
    let root = temp_dir("canic-install-state");
    let state = InstallState {
        schema_version: INSTALL_STATE_SCHEMA_VERSION,
        fleet: "demo".to_string(),
        installed_at_unix_secs: 42,
        network: "local".to_string(),
        root_target: "root".to_string(),
        root_canister_id: "uxrrr-q7777-77774-qaaaq-cai".to_string(),
        root_build_target: "root".to_string(),
        workspace_root: root.display().to_string(),
        icp_root: root.display().to_string(),
        config_path: root.join("fleets/canic.toml").display().to_string(),
        release_set_manifest_path: root
            .join(".icp/local/canisters/root/root.release-set.json")
            .display()
            .to_string(),
    };

    let path = write_install_state(&root, "local", &state).expect("write state");
    let named = read_fleet_install_state(&root, "local", "demo")
        .expect("read named fleet")
        .expect("named fleet exists");

    assert_eq!(path, root.join(".canic/local/fleets/demo.json"));
    assert_eq!(named, state);

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_state_write_receipt_records_local_state_path() {
    let (root, check) = demo_install_deployment_truth_check("canic-install-state-receipt");
    let state = InstallState {
        schema_version: INSTALL_STATE_SCHEMA_VERSION,
        fleet: "demo".to_string(),
        installed_at_unix_secs: 42,
        network: "local".to_string(),
        root_target: "root".to_string(),
        root_canister_id: "uxrrr-q7777-77774-qaaaq-cai".to_string(),
        root_build_target: "root".to_string(),
        workspace_root: root.display().to_string(),
        icp_root: root.display().to_string(),
        config_path: root.join("fleets/demo/canic.toml").display().to_string(),
        release_set_manifest_path: root
            .join(".icp/local/canisters/root/root.release-set.json")
            .display()
            .to_string(),
    };
    let execution_context = current_install_execution_context(&root, &root, "local");
    let scope = InstallReceiptScope {
        icp_root: &root,
        network: "local",
        fleet_name: "demo",
        check: &check,
        execution_context: Some(&execution_context),
    };

    let state_path = write_install_state_with_deployment_truth_receipt(scope, "local", &state)
        .expect("write install state and receipt");
    let receipt_dir = root.join(".canic/local/deployment-receipts/demo");
    let receipt = fs::read_dir(&receipt_dir)
        .expect("read receipts")
        .map(|entry| {
            let path = entry.expect("receipt entry").path();
            serde_json::from_slice::<DeploymentReceiptV1>(
                &fs::read(path).expect("read receipt JSON"),
            )
            .expect("decode receipt")
        })
        .find(|receipt| receipt.operation_id.ends_with(":write_install_state"))
        .expect("write install state receipt");

    assert_eq!(state_path, root.join(".canic/local/fleets/demo.json"));
    assert_eq!(
        receipt.operation_status,
        DeploymentExecutionStatusV1::Complete
    );
    assert_eq!(receipt.phase_receipts[0].phase, "write_install_state");
    assert!(
        receipt.phase_receipts[0]
            .verified_postcondition
            .evidence
            .contains(&format!("install_state:{}", state_path.display()))
    );

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_state_replaces_other_fleets_that_share_root() {
    let root = temp_dir("canic-install-state-replace");
    let demo = InstallState {
        schema_version: INSTALL_STATE_SCHEMA_VERSION,
        fleet: "demo".to_string(),
        installed_at_unix_secs: 42,
        network: "local".to_string(),
        root_target: "root".to_string(),
        root_canister_id: "uxrrr-q7777-77774-qaaaq-cai".to_string(),
        root_build_target: "root".to_string(),
        workspace_root: root.display().to_string(),
        icp_root: root.display().to_string(),
        config_path: root.join("fleets/demo/canic.toml").display().to_string(),
        release_set_manifest_path: root
            .join(".icp/local/canisters/root/root.release-set.json")
            .display()
            .to_string(),
    };
    let test = InstallState {
        fleet: "test".to_string(),
        config_path: root.join("fleets/test/canic.toml").display().to_string(),
        ..demo.clone()
    };

    write_install_state(&root, "local", &demo).expect("write demo state");
    write_install_state(&root, "local", &test).expect("write test state");

    assert!(
        read_fleet_install_state(&root, "local", "demo")
            .expect("read demo")
            .is_none()
    );
    assert_eq!(
        read_fleet_install_state(&root, "local", "test")
            .expect("read test")
            .expect("test state exists"),
        test
    );

    fs::remove_dir_all(root).expect("clean temp dir");
}

fn write_temp_workspace_config(config_source: &str) -> PathBuf {
    let root = temp_dir("canic-install-test");
    fs::create_dir_all(root.join("fleets")).expect("temp fleets dir must be created");
    fs::write(root.join("fleets/canic.toml"), config_source)
        .expect("temp canic.toml must be written");
    root
}

fn write_wasm_gz_artifact(root: &Path, role: &str, bytes: &[u8]) {
    let path = root
        .join(".icp/local/canisters")
        .join(role)
        .join(format!("{role}.wasm.gz"));
    fs::create_dir_all(path.parent().expect("artifact parent")).expect("create artifact dir");
    fs::write(path, bytes).expect("write artifact");
}

fn demo_install_deployment_truth_check(root_name: &str) -> (PathBuf, DeploymentCheckV1) {
    let root = temp_dir(root_name);
    let config_path = root.join("fleets/demo/canic.toml");
    fs::create_dir_all(config_path.parent().expect("config parent")).expect("create config dir");
    fs::write(
        &config_path,
        r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"
"#,
    )
    .expect("write config");
    write_wasm_gz_artifact(&root, "root", b"root-artifact");

    let options = InstallRootOptions {
        root_canister: "root".to_string(),
        root_build_target: "root".to_string(),
        network: "local".to_string(),
        icp_root: Some(root.clone()),
        build_profile: Some(CanisterBuildProfile::Fast),
        ready_timeout_seconds: 30,
        config_path: Some("fleets/demo/canic.toml".to_string()),
        expected_fleet: Some("demo".to_string()),
        interactive_config_selection: false,
        deployment_plan_override: None,
    };
    let check = current_install_deployment_truth_check_at(
        &options,
        &root,
        &root,
        &config_path,
        "demo",
        "2026-05-22T00:00:00Z".to_string(),
    )
    .expect("deployment truth check");
    (root, check)
}

fn with_guarded_env(run: impl FnOnce()) {
    let lock = ENV_LOCK.get_or_init(|| Mutex::new(()));
    let _guard = lock.lock().expect("env lock poisoned");
    run();
}

fn restore_env_var(key: &str, previous: Option<std::ffi::OsString>) {
    unsafe {
        if let Some(value) = previous {
            env::set_var(key, value);
        } else {
            env::remove_var(key);
        }
    }
}
