use super::*;

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
fn deployment_state_path_is_scoped_by_network() {
    assert_eq!(
        deployment_install_state_path(&PathBuf::from("/tmp/canic-project"), "local", "demo"),
        PathBuf::from("/tmp/canic-project/.canic/local/deployments/demo.json")
    );
}

#[test]
fn install_state_round_trips_from_project_state_dir() {
    let root = temp_dir("canic-install-state");
    let state = sample_install_state(&root, "demo", "demo");

    let path = write_install_state(&root, "local", &state).expect("write state");
    let named = read_deployment_install_state(&root, "local", "demo")
        .expect("read named deployment")
        .expect("named deployment exists");

    assert_eq!(path, root.join(".canic/local/deployments/demo.json"));
    assert_eq!(named, state);

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_state_rejects_stale_duplicate_fleet_field() {
    let root = temp_dir("canic-install-state-stale-fleet-field");
    let path = deployment_install_state_path(&root, "local", "demo-local");
    fs::create_dir_all(path.parent().expect("state parent")).expect("create state dir");
    fs::write(
        &path,
        serde_json::to_vec_pretty(&json!({
            "schema_version": INSTALL_STATE_SCHEMA_VERSION,
            "fleet": "demo",
            "deployment_name": "demo-local",
            "fleet_template": "demo",
            "created_at_unix_secs": 42,
            "updated_at_unix_secs": 42,
            "network": "local",
            "root_target": "root",
            "root_canister_id": "uxrrr-q7777-77774-qaaaq-cai",
            "root_verification": "verified",
            "root_build_target": "root",
            "workspace_root": root.display().to_string(),
            "icp_root": root.display().to_string(),
            "config_path": root.join("fleets/demo/canic.toml").display().to_string(),
            "release_set_manifest_path": root
                .join(".icp/local/canisters/root/root.release-set.json")
                .display()
                .to_string()
        }))
        .expect("encode stale state"),
    )
    .expect("write stale state");

    let err = read_deployment_install_state(&root, "local", "demo-local")
        .expect_err("stale duplicate fleet field must fail closed");
    let message = err.to_string();

    assert!(message.contains("unknown field `fleet`"));

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_state_rejects_stale_installed_timestamp_field() {
    let root = temp_dir("canic-install-state-stale-installed-at");
    let path = deployment_install_state_path(&root, "local", "demo-local");
    fs::create_dir_all(path.parent().expect("state parent")).expect("create state dir");
    fs::write(
        &path,
        serde_json::to_vec_pretty(&json!({
            "schema_version": INSTALL_STATE_SCHEMA_VERSION,
            "deployment_name": "demo-local",
            "fleet_template": "demo",
            "installed_at_unix_secs": 42,
            "network": "local",
            "root_target": "root",
            "root_canister_id": "uxrrr-q7777-77774-qaaaq-cai",
            "root_verification": "verified",
            "root_build_target": "root",
            "workspace_root": root.display().to_string(),
            "icp_root": root.display().to_string(),
            "config_path": root.join("fleets/demo/canic.toml").display().to_string(),
            "release_set_manifest_path": root
                .join(".icp/local/canisters/root/root.release-set.json")
                .display()
                .to_string()
        }))
        .expect("encode stale state"),
    )
    .expect("write stale state");

    let err = read_deployment_install_state(&root, "local", "demo-local")
        .expect_err("stale installed timestamp field must fail closed");
    let message = err.to_string();

    assert!(message.contains("unknown field `installed_at_unix_secs`"));

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn deploy_register_writes_minimal_unverified_deployment_state() {
    let root = temp_dir("canic-register-state");
    let path = register_deployment_state(RegisterDeploymentStateOptions {
        deployment_name: "demo-local".to_string(),
        fleet_template: "demo".to_string(),
        root_canister_id: "uxrrr-q7777-77774-qaaaq-cai".to_string(),
        network: "local".to_string(),
        allow_unverified: true,
        icp_root: Some(root.clone()),
        workspace_root: Some(root.clone()),
    })
    .expect("register deployment state");
    let state = read_deployment_install_state(&root, "local", "demo-local")
        .expect("read registered state")
        .expect("state exists");

    assert_eq!(path, root.join(".canic/local/deployments/demo-local.json"));
    assert_eq!(state.deployment_name, "demo-local");
    assert_eq!(state.fleet_template, "demo");
    assert_eq!(state.root_canister_id, "uxrrr-q7777-77774-qaaaq-cai");
    assert_eq!(state.root_verification, RootVerificationStatus::NotVerified);
    assert_eq!(state.created_at_unix_secs, state.updated_at_unix_secs);
    assert!(state.config_path.ends_with("fleets/demo/canic.toml"));

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn deploy_register_requires_explicit_unverified_acknowledgement() {
    let root = temp_dir("canic-register-state-requires-ack");
    let err = register_deployment_state(RegisterDeploymentStateOptions {
        deployment_name: "demo-local".to_string(),
        fleet_template: "demo".to_string(),
        root_canister_id: "uxrrr-q7777-77774-qaaaq-cai".to_string(),
        network: "local".to_string(),
        allow_unverified: false,
        icp_root: Some(root.clone()),
        workspace_root: Some(root.clone()),
    })
    .expect_err("registration without acknowledgement must fail");

    assert!(err.to_string().contains("--allow-unverified"));

    if root.exists() {
        fs::remove_dir_all(root).expect("clean temp dir");
    }
}
