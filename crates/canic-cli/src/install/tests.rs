use super::*;
use canic_host::install_root::InstallRootPhase;

// Ensure install defaults to the conventional local root canister target.
#[test]
fn install_defaults_to_root_target() {
    let options = InstallOptions::parse([
        OsString::from("demo"),
        OsString::from("demo-local"),
        OsString::from("--fleet-input"),
        OsString::from("deployments/demo-local.toml"),
    ])
    .expect("parse defaults");
    let install = options
        .clone()
        .into_install_root_options_with_icp_root(None);

    assert_eq!(options.fleet, "demo-local");
    assert_eq!(options.app, "demo");
    assert_eq!(options.environment, local_environment());
    assert_eq!(options.icp, default_icp());
    assert_eq!(options.profile, None);
    assert_eq!(options.release_build_id, None);
    assert_eq!(install.root_canister, "root");
    assert_eq!(install.root_build_target, "root");
    assert_eq!(install.icp_executable, default_icp());
    assert_eq!(install.icp_root, None);
    assert_eq!(install.build_profile, None);
    assert_eq!(install.release_build_id, None);
    assert_eq!(
        install.fleet_install_input_path,
        Some(PathBuf::from("deployments/demo-local.toml"))
    );
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
        OsString::from("--fleet-input"),
        OsString::from("deployments/demo.toml"),
    ])
    .expect("parse internal environment");

    assert_eq!(options.environment, "local");
}

#[test]
fn install_accepts_internal_icp_executable() {
    let options = InstallOptions::parse([
        OsString::from("toko"),
        OsString::from("demo"),
        OsString::from(crate::cli::globals::INTERNAL_ICP_OPTION),
        OsString::from("/opt/icp"),
        OsString::from("--fleet-input"),
        OsString::from("deployments/demo.toml"),
    ])
    .expect("parse internal ICP executable");
    let install = options
        .clone()
        .into_install_root_options_with_icp_root(None);

    assert_eq!(options.icp, "/opt/icp");
    assert_eq!(install.icp_executable, "/opt/icp");
}

#[test]
fn install_accepts_build_profile() {
    let options = InstallOptions::parse([
        OsString::from("--profile"),
        OsString::from("fast"),
        OsString::from("toko"),
        OsString::from("demo"),
        OsString::from("--fleet-input"),
        OsString::from("deployments/demo.toml"),
    ])
    .expect("parse profile");
    let install = options.into_install_root_options_with_icp_root(None);

    assert_eq!(install.build_profile, Some(CanisterBuildProfile::Fast));
}

#[test]
fn install_accepts_finalized_release_build_identity() {
    let release_build_id = "11".repeat(32);
    let options = InstallOptions::parse([
        OsString::from("toko"),
        OsString::from("demo"),
        OsString::from("--fleet-input"),
        OsString::from("deployments/demo.toml"),
        OsString::from("--release-build"),
        OsString::from(&release_build_id),
    ])
    .expect("parse finalized release build");
    let install = options.into_install_root_options_with_icp_root(None);

    assert_eq!(
        install.release_build_id.expect("release build").to_string(),
        release_build_id
    );
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

#[test]
fn install_requires_fleet_input() {
    let err = InstallOptions::parse([OsString::from("demo"), OsString::from("demo-local")])
        .expect_err("missing Fleet input should fail");

    std::assert_matches!(err, InstallCommandError::Usage(_));
}

// Ensure install help documents the App-owned source identity.
#[test]
fn install_usage_explains_app_config() {
    let text = usage();
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");

    assert!(text.contains("Install and bootstrap a Canic fleet"));
    assert!(text.contains("Usage: canic install <app> <fleet> --fleet-input <PATH>"));
    assert!(text.contains("<app>"));
    assert!(text.contains("<fleet>"));
    assert!(!text.contains("--app"));
    assert!(text.contains("--profile"));
    assert!(text.contains("--release-build"));
    assert!(text.contains("--fleet-input"));
    assert!(normalized.contains("fresh Fleet"));
    assert!(normalized.contains("App config"));
    assert!(normalized.contains("required operator-owned Fleet input"));
    assert!(!normalized.contains("existing-Fleet update flow"));
    assert!(!normalized.contains("CARGO_TARGET_DIR"));
    assert_eq!(text.matches("  canic install ").count(), 2);
}

// Ensure existing-deployment install failures point at diagnostics and the command boundary.
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
    assert!(message.contains("canic --environment academic medic fleet demo"));
    assert!(message.contains("`canic install` is for fresh Fleet creation"));
    assert!(message.contains("not code-only updates"));

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
