use super::*;

#[test]
fn install_rejects_config_identity_mismatch() {
    validate_expected_app_id(Some("demo"), "test", Path::new("apps/demo/canic.toml"))
        .expect_err("mismatched App identity should fail");
}

#[test]
fn install_keeps_fleet_label_separate_from_source_app_identity() {
    let root = temp_dir("canic-install-distinct-fleet-app");
    let mut options = local_demo_install_options(&root);
    options.fleet_name = "demo-production".to_string();

    let identity = resolve_install_identity(&options, Path::new("apps/demo/canic.toml"), "demo")
        .expect("resolve distinct Fleet and App");

    assert_eq!(
        identity,
        ("demo".to_string(), "demo-production".to_string())
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn deployment_state_path_is_scoped_by_environment() {
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
        validate_environment_name("bad/environment"),
        Err(InstallStateError::InvalidEnvironmentName { name }) if name == "bad/environment"
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
    state.environment = "staging".to_string();
    fs::write(
        &path,
        serde_json::to_vec_pretty(&state).expect("encode wrong-environment state"),
    )
    .expect("write wrong-environment state");
    std::assert_matches!(
        read_deployment_install_state(&root, "local", "demo"),
        Err(InstallStateError::EnvironmentMismatch {
            state_environment,
            requested_environment,
        }) if state_environment == "staging" && requested_environment == "local"
    );

    fs::remove_dir_all(root).expect("clean temp dir");
}
