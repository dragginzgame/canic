use super::*;

// Ensure install defaults to the conventional local root canister target.
#[test]
fn install_defaults_to_root_target() {
    let options = InstallOptions::parse([OsString::from("demo")]).expect("parse defaults");
    let install = options.clone().into_install_root_options();

    assert_eq!(options.fleet, "demo");
    assert_eq!(options.network, local_network());
    assert_eq!(options.profile, None);
    assert_eq!(install.root_canister, "root");
    assert_eq!(install.root_build_target, "root");
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
    let install = options.into_install_root_options();

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

    assert!(matches!(err, InstallCommandError::Usage(_)));
}

// Ensure removed install target forms are rejected before mutation starts.
#[test]
fn install_rejects_target_overrides() {
    let root_arg = InstallOptions::parse([OsString::from("demo"), OsString::from("root")])
        .expect_err("positional root target should be removed");
    let root_flag = InstallOptions::parse([
        OsString::from("demo"),
        OsString::from("--root"),
        OsString::from("uxrrr-q7777-77774-qaaaq-cai"),
    ])
    .expect_err("root flag should be removed");
    let build_target = InstallOptions::parse([
        OsString::from("demo"),
        OsString::from("--root-build-target"),
        OsString::from("custom_root"),
    ])
    .expect_err("root build target should be removed");

    assert!(matches!(root_arg, InstallCommandError::Usage(_)));
    assert!(matches!(root_flag, InstallCommandError::Usage(_)));
    assert!(matches!(build_target, InstallCommandError::Usage(_)));
}

// Ensure removed install config selection is rejected before mutation starts.
#[test]
fn install_rejects_config_path() {
    let err = InstallOptions::parse([
        OsString::from("demo"),
        OsString::from("--config"),
        OsString::from("fleets/demo/canic.toml"),
    ])
    .expect_err("config override should be removed");

    assert!(matches!(err, InstallCommandError::Usage(_)));
}

// Ensure removed ready timeout controls are rejected before mutation starts.
#[test]
fn install_rejects_ready_timeout() {
    let err = InstallOptions::parse([
        OsString::from("demo"),
        OsString::from("--ready-timeout-seconds"),
        OsString::from("30"),
    ])
    .expect_err("ready timeout override should be removed");

    assert!(matches!(err, InstallCommandError::Usage(_)));
}

// Ensure install requires an explicit fleet argument.
#[test]
fn install_requires_fleet_argument() {
    let err = InstallOptions::parse([]).expect_err("missing fleet should fail");

    assert!(matches!(err, InstallCommandError::Usage(_)));
}

// Ensure install help documents config-owned fleet identity.
#[test]
fn install_usage_explains_fleet_config() {
    let text = usage();

    assert!(text.contains("Install and bootstrap a Canic fleet"));
    assert!(text.contains("Usage: canic install <fleet>"));
    assert!(text.contains("<fleet>"));
    assert!(!text.contains("--fleet <name>"));
    assert!(!text.contains("[name-or-principal]"));
    assert!(!text.contains("--config"));
    assert!(!text.contains("--ready-timeout-seconds"));
    assert!(!text.contains("--root <name-or-principal>"));
    assert!(!text.contains("--root-build-target"));
    assert!(!text.contains("--network"));
    assert!(text.contains("--profile"));
    assert!(text.contains("[fleet]"));
    assert!(text.contains("name = \"test\""));
}
