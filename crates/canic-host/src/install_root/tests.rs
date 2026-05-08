use super::{
    INSTALL_STATE_SCHEMA_VERSION, InstallState, LOCAL_ROOT_TARGET_CYCLES,
    add_icp_environment_target, canic_build_target_command, config_selection_error,
    discover_canic_config_choices, fleet_install_state_path, icp_canister_command_in_network,
    icp_start_local_command, icp_stop_command, install_build_session_id,
    parse_bootstrap_status_value, parse_canister_status_cycles, parse_local_icp_autostart,
    parse_root_ready_value, read_fleet_install_state, required_local_cycle_topup,
    resolve_install_config_path, write_install_state,
};
use crate::release_set::configured_install_targets;
use crate::test_support::temp_dir;
use serde_json::json;
use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
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
fn parse_canister_status_cycles_accepts_balance_line() {
    let output = "\
Canister status call result for root.
Status: Running
Balance: 9_002_999_998_056_000 Cycles
Memory Size: 1_234_567 Bytes
";

    assert_eq!(
        parse_canister_status_cycles(output),
        Some(9_002_999_998_056_000)
    );
}

#[test]
fn parse_canister_status_cycles_accepts_cycle_balance_line() {
    let output = "\
Canister status call result for root.
Cycle balance: 12_345 Cycles
";

    assert_eq!(parse_canister_status_cycles(output), Some(12_345));
}

#[test]
fn parse_canister_status_cycles_accepts_icp_cli_cycles_line() {
    let output = "\
Canister Status Report:
  Cycles: 1_499_993_904_000
  Reserved cycles: 0
  Idle cycles burned per day: 5_671
";

    assert_eq!(
        parse_canister_status_cycles(output),
        Some(1_499_993_904_000)
    );
}

#[test]
fn required_local_cycle_topup_skips_when_balance_already_meets_target() {
    assert_eq!(required_local_cycle_topup(LOCAL_ROOT_TARGET_CYCLES), None);
    assert_eq!(
        required_local_cycle_topup(LOCAL_ROOT_TARGET_CYCLES + 1_000),
        None
    );
}

#[test]
fn required_local_cycle_topup_returns_missing_delta_only() {
    assert_eq!(
        required_local_cycle_topup(3_000_000_000_000),
        Some(8_997_000_000_000_000)
    );
}

#[test]
fn canic_build_command_targets_one_canister_per_call() {
    let command = canic_build_target_command(
        Path::new("/tmp/canic-icp-root"),
        "ic",
        "user_hub",
        "install-root-test",
    );

    assert_eq!(
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        ["build", "user_hub"]
    );
    assert!(
        command
            .get_envs()
            .any(|(key, value)| key == "CANIC_BUILD_CONTEXT_SESSION" && value.is_some()),
        "canic build must carry the shared build-session marker"
    );
    assert_eq!(
        command_env(&command, "ICP_ENVIRONMENT").as_deref(),
        Some("ic")
    );
}

#[test]
fn icp_canister_command_carries_selected_network() {
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
fn install_build_session_id_is_prefixed_for_logs() {
    let session_id = install_build_session_id();
    assert!(session_id.starts_with("install-root-"));
}

#[test]
fn local_icp_autostart_defaults_to_enabled() {
    assert!(parse_local_icp_autostart(None));
    assert!(parse_local_icp_autostart(Some("")));
    assert!(parse_local_icp_autostart(Some("1")));
    assert!(parse_local_icp_autostart(Some("true")));
}

#[test]
fn local_icp_autostart_accepts_explicit_disable_values() {
    assert!(!parse_local_icp_autostart(Some("0")));
    assert!(!parse_local_icp_autostart(Some("false")));
    assert!(!parse_local_icp_autostart(Some("no")));
    assert!(!parse_local_icp_autostart(Some("off")));
}

#[test]
fn local_icp_start_command_uses_background_mode() {
    let command = icp_start_local_command(Path::new("/tmp/canic-icp-root"));

    assert_eq!(command.get_program(), "icp");
    assert_eq!(
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        ["network", "start", "local", "--background"]
    );
    assert_eq!(
        command
            .get_current_dir()
            .map(|path| path.to_string_lossy().into_owned()),
        Some("/tmp/canic-icp-root".to_string())
    );
    assert_eq!(
        command_env(&command, "ICP_ENVIRONMENT").as_deref(),
        Some("local")
    );
}

#[test]
fn local_icp_stop_command_targets_project_root() {
    let command = icp_stop_command(Path::new("/tmp/canic-icp-root"));

    assert_eq!(command.get_program(), "icp");
    assert_eq!(
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        ["network", "stop", "local"]
    );
    assert_eq!(
        command
            .get_current_dir()
            .map(|path| path.to_string_lossy().into_owned()),
        Some("/tmp/canic-icp-root".to_string())
    );
    assert_eq!(
        command_env(&command, "ICP_ENVIRONMENT").as_deref(),
        Some("local")
    );
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
        assert!(message.contains("run: canic install --config fleets/demo/canic.toml"));

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
    assert!(message.contains("run: canic install --config fleets/demo/canic.toml"));
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

[subnets.prime.canisters.scale]
kind = "singleton"

[subnets.prime.canisters.scale_hub]
kind = "singleton"
"#,
    )
    .expect("write example config");
    let message = config_selection_error(&root, &root.join("fleets/canic.toml"), &[demo, example]);

    assert!(message.contains("choose a config path explicitly:"));
    assert!(message.contains("choose a config path explicitly:\n\n#"));
    assert!(message.contains('#'));
    assert!(message.contains("CONFIG"));
    assert!(message.contains("CANISTERS"));
    assert!(message.contains("1  fleets/demo/canic.toml"));
    assert!(message.contains("2  fleets/example/canic.toml"));
    assert!(message.contains("fleets/demo/canic.toml"));
    assert!(message.contains("2 (root, app)"));
    assert!(message.contains("fleets/example/canic.toml"));
    assert!(message.contains("5 (root, scale, scale_hub, user_hub, user_shard)"));
    assert!(message.contains("5 (root, scale, scale_hub, user_hub, user_shard)\n\nrun:"));
    assert!(message.contains("run: canic install --config <path>"));
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

[subnets.prime.canisters.scale]
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

    assert!(message.contains("8 (root, app, minimal, scale, scale_hub, user_hub, ...)"));
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

fn write_temp_workspace_config(config_source: &str) -> PathBuf {
    let root = temp_dir("canic-install-test");
    fs::create_dir_all(root.join("fleets")).expect("temp fleets dir must be created");
    fs::write(root.join("fleets/canic.toml"), config_source)
        .expect("temp canic.toml must be written");
    root
}

fn command_env(command: &std::process::Command, name: &str) -> Option<String> {
    command
        .get_envs()
        .find_map(|(key, value)| (key == name).then_some(value))
        .flatten()
        .map(|value| value.to_string_lossy().into_owned())
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
