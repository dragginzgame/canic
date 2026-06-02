use super::super::register as deploy_register;
use super::*;

#[test]
fn deploy_register_command_dispatches_register() {
    let parsed = parse_subcommand(
        deploy_command(),
        [
            OsString::from("register"),
            OsString::from("demo-local"),
            OsString::from("--fleet-template"),
            OsString::from("demo"),
            OsString::from("--root"),
            OsString::from("uxrrr-q7777-77774-qaaaq-cai"),
            OsString::from("--allow-unverified"),
        ],
    )
    .expect("parse deploy register")
    .expect("register command");

    assert_eq!(parsed.0, "register");

    let options =
        deploy_register::DeployRegisterOptions::parse(parsed.1).expect("parse register options");
    assert_eq!(options.deployment, "demo-local");
    assert_eq!(options.fleet_template, "demo");
    assert_eq!(options.root, "uxrrr-q7777-77774-qaaaq-cai");
    assert!(options.allow_unverified);
}

#[test]
fn deploy_register_builds_minimal_registration_options() {
    let options = deploy_register::DeployRegisterOptions {
        deployment: "demo-local".to_string(),
        fleet_template: "demo".to_string(),
        root: "uxrrr-q7777-77774-qaaaq-cai".to_string(),
        network: "local".to_string(),
        allow_unverified: true,
    }
    .into_register_options(Some(PathBuf::from("/tmp/icp")));

    assert_eq!(options.deployment_name, "demo-local");
    assert_eq!(options.fleet_template, "demo");
    assert_eq!(options.root_canister_id, "uxrrr-q7777-77774-qaaaq-cai");
    assert_eq!(options.network, "local");
    assert!(options.allow_unverified);
    assert_eq!(options.icp_root, Some(PathBuf::from("/tmp/icp")));
    assert_eq!(options.workspace_root, None);
}

#[test]
fn deploy_register_requires_unverified_acknowledgement_flag() {
    let err = deploy_register::DeployRegisterOptions::parse([
        OsString::from("demo-local"),
        OsString::from("--fleet-template"),
        OsString::from("demo"),
        OsString::from("--root"),
        OsString::from("uxrrr-q7777-77774-qaaaq-cai"),
    ])
    .expect_err("register without acknowledgement should fail usage");

    std::assert_matches!(err, DeployCommandError::Usage(_));
}
