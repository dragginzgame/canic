use super::*;

// Ensure install defaults to the conventional local root canister target.
#[test]
fn install_defaults_to_root_target() {
    let options = InstallOptions::parse([OsString::from("demo")]).expect("parse defaults");
    let install = options
        .clone()
        .into_install_root_options_with_icp_root(None);

    assert_eq!(options.fleet, "demo");
    assert_eq!(options.network, local_network());
    assert_eq!(options.profile, None);
    assert_eq!(install.root_canister, "root");
    assert_eq!(install.root_build_target, "root");
    assert_eq!(install.icp_root, None);
    assert_eq!(install.build_profile, None);
    assert_eq!(install.ready_timeout_seconds, DEFAULT_READY_TIMEOUT_SECONDS);
    assert_eq!(
        install.config_path,
        Some("fleets/demo/canic.toml".to_string())
    );
    assert_eq!(install.expected_fleet, Some("demo".to_string()));
}

// Ensure top-level dispatch can pass network selection internally.
#[test]
fn install_accepts_internal_network() {
    let options = InstallOptions::parse([
        OsString::from("demo"),
        OsString::from(crate::cli::globals::INTERNAL_NETWORK_OPTION),
        OsString::from("local"),
    ])
    .expect("parse internal network");

    assert_eq!(options.network, "local");
}

#[test]
fn install_accepts_build_profile() {
    let options = InstallOptions::parse([
        OsString::from("--profile"),
        OsString::from("fast"),
        OsString::from("demo"),
    ])
    .expect("parse profile");
    let install = options.into_install_root_options_with_icp_root(None);

    assert_eq!(install.build_profile, Some(CanisterBuildProfile::Fast));
}

#[test]
fn install_rejects_invalid_build_profile() {
    let err = InstallOptions::parse([
        OsString::from("--profile"),
        OsString::from("tiny"),
        OsString::from("demo"),
    ])
    .expect_err("invalid profile should fail");

    std::assert_matches!(err, InstallCommandError::Usage(_));
}

#[test]
fn install_preserves_icp_root_resolution_causes() {
    let error = InstallCommandError::from(IcpConfigError::NoIcpRoot {
        start: PathBuf::from("/project"),
    });

    std::assert_matches!(
        error,
        InstallCommandError::IcpRoot(IcpConfigError::NoIcpRoot { .. })
    );
}

// Ensure install requires an explicit fleet argument.
#[test]
fn install_requires_fleet_argument() {
    let err = InstallOptions::parse([]).expect_err("missing fleet should fail");

    std::assert_matches!(err, InstallCommandError::Usage(_));
}

// Ensure install help documents config-owned fleet identity.
#[test]
fn install_usage_explains_fleet_config() {
    let text = usage();
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");

    assert!(text.contains("Install and bootstrap a Canic fleet"));
    assert!(text.contains("Usage: canic install <fleet>"));
    assert!(text.contains("<fleet>"));
    assert!(text.contains("--profile"));
    assert!(normalized.contains("fresh local creation"));
    assert!(normalized.contains("project upgrade flow"));
    assert!(normalized.contains("canic medic deployment"));
    assert!(text.contains("[fleet]"));
    assert!(text.contains("name = \"test\""));
}

// Ensure existing-deployment install failures point at diagnostics and upgrade flow.
#[test]
fn install_existing_deployment_errors_get_action_hint() {
    let err = install_error_with_context(
        Box::new(std::io::Error::other("canister already has installed code")),
        "demo",
        "academic",
    );
    let message = err.to_string();

    assert!(message.contains("canic --network academic info list demo"));
    assert!(message.contains("canic --network academic medic deployment demo"));
    assert!(message.contains("project upgrade flow"));

    std::assert_matches!(
        install_error_with_context(
            Box::new(std::io::Error::other("failed to read config")),
            "demo",
            "academic",
        ),
        InstallCommandError::Install(_)
    );
}
