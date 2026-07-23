use super::*;
use canic_host::install_root::InstallRootPhase;

// Ensure install defaults to the conventional local root canister target.
#[test]
fn install_defaults_to_root_target() {
    let options = InstallOptions::parse([OsString::from("demo"), OsString::from("demo-local")])
        .expect("parse defaults");
    let install = options
        .clone()
        .into_install_root_options_with_icp_root(None);

    assert_eq!(options.fleet, "demo-local");
    assert_eq!(options.app, "demo");
    assert_eq!(options.environment, local_environment());
    assert_eq!(options.profile, None);
    assert_eq!(install.root_canister, "root");
    assert_eq!(install.root_build_target, "root");
    assert_eq!(install.icp_root, None);
    assert_eq!(install.build_profile, None);
    assert_eq!(install.ready_timeout_seconds, DEFAULT_READY_TIMEOUT_SECONDS);
    assert_eq!(
        install.config_path,
        Some("apps/demo/canic.toml".to_string())
    );
    assert_eq!(install.fleet_name, "demo-local");
    assert_eq!(install.expected_app, Some("demo".to_string()));
}

// Ensure top-level dispatch can pass environment selection internally.
#[test]
fn install_accepts_internal_environment() {
    let options = InstallOptions::parse([
        OsString::from("toko"),
        OsString::from("demo"),
        OsString::from(crate::cli::globals::INTERNAL_ENVIRONMENT_OPTION),
        OsString::from("local"),
    ])
    .expect("parse internal environment");

    assert_eq!(options.environment, "local");
}

#[test]
fn install_accepts_build_profile() {
    let options = InstallOptions::parse([
        OsString::from("--profile"),
        OsString::from("fast"),
        OsString::from("toko"),
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
        OsString::from("toko"),
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

// Ensure install requires both source App and installed Fleet identities.
#[test]
fn install_requires_app_argument() {
    let err = InstallOptions::parse([]).expect_err("missing App should fail");

    std::assert_matches!(err, InstallCommandError::Usage(_));
}

#[test]
fn install_requires_fleet_argument() {
    let err =
        InstallOptions::parse([OsString::from("demo")]).expect_err("missing Fleet should fail");

    std::assert_matches!(err, InstallCommandError::Usage(_));
}

// Ensure install help documents the App-owned source identity.
#[test]
fn install_usage_explains_app_config() {
    let text = usage();
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");

    assert!(text.contains("Install and bootstrap a Canic fleet"));
    assert!(text.contains("Usage: canic install <app> <fleet>"));
    assert!(text.contains("<app>"));
    assert!(text.contains("<fleet>"));
    assert!(!text.contains("--app"));
    assert!(text.contains("--profile"));
    assert!(normalized.contains("fresh local creation"));
    assert!(normalized.contains("project upgrade flow"));
    assert!(normalized.contains("canic medic deployment"));
    assert!(text.contains("[app]"));
    assert!(text.contains("name = \"test\""));
}

// Ensure existing-deployment install failures point at diagnostics and upgrade flow.
#[test]
fn install_existing_deployment_errors_get_action_hint() {
    let err = install_error_with_context(
        InstallRootError::new(
            InstallRootPhase::Activation,
            std::io::Error::other("canister already has installed code"),
        ),
        "demo",
        "academic",
    );
    let message = err.to_string();

    assert!(message.contains("canic --environment academic info list demo"));
    assert!(message.contains("canic --environment academic medic deployment demo"));
    assert!(message.contains("project upgrade flow"));

    std::assert_matches!(
        install_error_with_context(
            InstallRootError::new(
                InstallRootPhase::Configuration,
                std::io::Error::other("failed to read config"),
            ),
            "demo",
            "academic",
        ),
        InstallCommandError::Install(_)
    );
}
