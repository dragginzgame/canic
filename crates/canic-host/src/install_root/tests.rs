use super::{
    INSTALL_STATE_SCHEMA_VERSION, InstallState, InstallTimingSummary, add_create_root_target,
    add_icp_environment_target, add_local_root_create_cycles_arg, config_selection_error,
    discover_canic_config_choices, discover_project_canic_config_choices, fleet_install_state_path,
    icp_canister_command_in_network, is_missing_canister_id_error, parse_bootstrap_status_value,
    parse_canister_id_json, parse_created_canister_id, parse_cycle_balance_response,
    parse_root_ready_value, read_fleet_install_state, render_install_timing_summary,
    resolve_install_config_path, root_init_args, validate_expected_fleet_name, write_install_state,
};
use crate::icp::{CANIC_ICP_LOCAL_NETWORK_URL_ENV, CANIC_ICP_LOCAL_ROOT_KEY_ENV};
use crate::release_set::configured_install_targets;
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
