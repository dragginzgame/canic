use super::*;

#[test]
fn install_rejects_config_identity_mismatch() {
    validate_expected_fleet_name(Some("demo"), "test", Path::new("fleets/demo/canic.toml"))
        .expect_err("mismatched fleet identity should fail");
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
    assert_eq!(
        fs::read(&path).expect("read state bytes"),
        serde_json::to_vec_pretty(&state).expect("encode expected state")
    );
    assert_eq!(named, state);

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_state_validation_failures_are_typed() {
    std::assert_matches!(
        validate_state_name("bad/name"),
        Err(InstallStateError::InvalidStateName { name }) if name == "bad/name"
    );
    std::assert_matches!(
        validate_network_name("bad/network"),
        Err(InstallStateError::InvalidNetworkName { name }) if name == "bad/network"
    );
}

#[test]
fn install_state_read_retains_path_and_io_source() {
    let root = temp_dir("canic-install-state-read-error");
    let path = deployment_install_state_path(&root, "local", "demo");
    fs::create_dir_all(&path).expect("create directory at state path");
    fs::write(path.join("child"), b"not state").expect("make state directory non-empty");

    let error = read_deployment_install_state(&root, "local", "demo")
        .expect_err("directory state path must reject");

    std::assert_matches!(
        error,
        InstallStateError::Read { path: error_path, source }
            if error_path == path && source.kind() == std::io::ErrorKind::IsADirectory
    );

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_state_decode_retains_path_and_json_source() {
    let root = temp_dir("canic-install-state-decode-error");
    let path = deployment_install_state_path(&root, "local", "demo");
    fs::create_dir_all(path.parent().expect("state parent")).expect("create state parent");
    fs::write(&path, b"{").expect("write invalid state");

    let error = read_deployment_install_state(&root, "local", "demo")
        .expect_err("invalid state JSON must reject");

    std::assert_matches!(
        error,
        InstallStateError::Decode { path: error_path, source }
            if error_path == path && source.is_eof()
    );

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_state_read_rejects_schema_and_path_identity_mismatches() {
    let root = temp_dir("canic-install-state-identity-mismatch");
    let path = deployment_install_state_path(&root, "local", "demo");
    fs::create_dir_all(path.parent().expect("state parent")).expect("create state parent");
    let mut state = sample_install_state(&root, "demo", "demo");

    state.schema_version = INSTALL_STATE_SCHEMA_VERSION + 1;
    fs::write(
        &path,
        serde_json::to_vec_pretty(&state).expect("encode future-schema state"),
    )
    .expect("write future-schema state");
    std::assert_matches!(
        read_deployment_install_state(&root, "local", "demo"),
        Err(InstallStateError::SchemaVersionMismatch {
            state_version,
            supported_version,
        }) if state_version == INSTALL_STATE_SCHEMA_VERSION + 1
            && supported_version == INSTALL_STATE_SCHEMA_VERSION
    );

    state.schema_version = INSTALL_STATE_SCHEMA_VERSION;
    state.deployment_name = "other".to_string();
    fs::write(
        &path,
        serde_json::to_vec_pretty(&state).expect("encode wrong-deployment state"),
    )
    .expect("write wrong-deployment state");
    std::assert_matches!(
        read_deployment_install_state(&root, "local", "demo"),
        Err(InstallStateError::DeploymentMismatch {
            state_deployment,
            requested_deployment,
        }) if state_deployment == "other" && requested_deployment == "demo"
    );

    state.deployment_name = "demo".to_string();
    state.network = "staging".to_string();
    fs::write(
        &path,
        serde_json::to_vec_pretty(&state).expect("encode wrong-network state"),
    )
    .expect("write wrong-network state");
    std::assert_matches!(
        read_deployment_install_state(&root, "local", "demo"),
        Err(InstallStateError::NetworkMismatch {
            state_network,
            requested_network,
        }) if state_network == "staging" && requested_network == "local"
    );

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_state_write_retains_mismatch_and_io_failures() {
    let mismatch_root = temp_dir("canic-install-state-network-mismatch");
    let mut mismatch_state = sample_install_state(&mismatch_root, "demo", "demo");
    mismatch_state.schema_version = INSTALL_STATE_SCHEMA_VERSION + 1;
    std::assert_matches!(
        write_install_state(&mismatch_root, "local", &mismatch_state),
        Err(InstallStateError::SchemaVersionMismatch {
            state_version,
            supported_version,
        }) if state_version == INSTALL_STATE_SCHEMA_VERSION + 1
            && supported_version == INSTALL_STATE_SCHEMA_VERSION
    );

    mismatch_state.schema_version = INSTALL_STATE_SCHEMA_VERSION;
    std::assert_matches!(
        write_install_state(&mismatch_root, "staging", &mismatch_state),
        Err(InstallStateError::NetworkMismatch {
            state_network,
            requested_network,
        }) if state_network == "local" && requested_network == "staging"
    );

    let write_root = temp_dir("canic-install-state-write-error");
    let write_state = sample_install_state(&write_root, "demo", "demo");
    let path = deployment_install_state_path(&write_root, "local", "demo");
    fs::create_dir_all(&path).expect("create directory at write target");
    fs::write(path.join("child"), b"preserved").expect("make target non-empty");

    let error = write_install_state(&write_root, "local", &write_state)
        .expect_err("directory write target must reject");
    std::assert_matches!(
        error,
        InstallStateError::Write { path: error_path, .. } if error_path == path
    );

    if mismatch_root.exists() {
        fs::remove_dir_all(mismatch_root).expect("clean mismatch root");
    }
    fs::remove_dir_all(write_root).expect("clean write root");
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
    register_deployment_state(RegisterDeploymentStateOptions {
        deployment_name: "demo-local".to_string(),
        fleet_template: "demo".to_string(),
        root_canister_id: "uxrrr-q7777-77774-qaaaq-cai".to_string(),
        network: "local".to_string(),
        allow_unverified: false,
        icp_root: Some(root.clone()),
        workspace_root: Some(root.clone()),
    })
    .expect_err("registration without acknowledgement must fail");

    if root.exists() {
        fs::remove_dir_all(root).expect("clean temp dir");
    }
}
