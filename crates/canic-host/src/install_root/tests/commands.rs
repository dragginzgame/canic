use super::*;

#[test]
fn icp_canister_command_carries_selected_environment() {
    let mut command = icp_canister_command(Path::new("/tmp/canic-icp-root"));
    command.args(["status", "root"]);
    add_icp_environment_target(&mut command, "ic", None);

    assert_eq!(command.get_program(), "icp");
    assert_eq!(
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        [
            "--project-root-override",
            "/tmp/canic-icp-root",
            "canister",
            "status",
            "root",
            "-e",
            "ic"
        ]
    );
}

#[test]
fn local_canister_command_uses_http_target_when_configured() {
    let target = LocalReplicaTarget {
        url: "http://127.0.0.1:8000".to_string(),
        root_key: "abcd".to_string(),
    };
    let mut command = icp_canister_command(Path::new("/tmp/canic-icp-root"));
    command.env("ICP_ENVIRONMENT", "local");
    command.args(["status", "root"]);
    add_icp_environment_target(&mut command, "local", Some(&target));

    assert_eq!(
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        [
            "--project-root-override",
            "/tmp/canic-icp-root",
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
    let target = LocalReplicaTarget {
        url: "http://127.0.0.1:8000".to_string(),
        root_key: "abcd".to_string(),
    };
    let mut command = icp_canister_command(Path::new("/tmp/canic-icp-root"));
    add_create_root_target(&mut command, "root", Some(&target));

    assert_eq!(
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        [
            "--project-root-override",
            "/tmp/canic-icp-root",
            "canister",
            "create",
            "--detached",
            "--json"
        ]
    );
}

#[test]
fn environment_create_uses_named_root() {
    let mut command = icp_canister_command(Path::new("/tmp/canic-icp-root"));
    add_create_root_target(&mut command, "root", None);

    assert_eq!(
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        [
            "--project-root-override",
            "/tmp/canic-icp-root",
            "canister",
            "create",
            "root",
            "--json"
        ]
    );
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
fn local_root_create_adds_configured_cycle_funding() {
    let workspace_root = write_temp_workspace_config(
        r#"
[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[roles.project_registry]
kind = "canister"
package = "project_registry"

[roles.oracle_pokemon]
kind = "canister"
package = "oracle_pokemon"

[roles.user_hub]
kind = "canister"
package = "user_hub"

[roles.user_shard]
kind = "canister"
package = "user_shard"

[roles.scale_hub]
kind = "canister"
package = "scale_hub"

[roles.scale_replica]
kind = "canister"
package = "scale"

[roles.role_baseline]
kind = "canister"
package = "role_baseline"

[roles.worker]
kind = "canister"
package = "worker"

[subnets.default.canisters.root]
kind = "root"

[subnets.default.canisters.app]
kind = "service"
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
fn nonlocal_root_create_does_not_add_cycle_funding() {
    let workspace_root = write_temp_workspace_config(
        r#"
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[roles.project_registry]
kind = "canister"
package = "project_registry"

[roles.oracle_pokemon]
kind = "canister"
package = "oracle_pokemon"

[roles.user_hub]
kind = "canister"
package = "user_hub"

[roles.user_shard]
kind = "canister"
package = "user_shard"

[roles.scale_hub]
kind = "canister"
package = "scale_hub"

[roles.scale_replica]
kind = "canister"
package = "scale"

[roles.role_baseline]
kind = "canister"
package = "role_baseline"

[roles.worker]
kind = "canister"
package = "worker"

[subnets.default.canisters.root]
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
