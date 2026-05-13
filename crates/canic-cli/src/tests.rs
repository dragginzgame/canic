use super::*;
use crate::{
    cli::globals::{INTERNAL_ICP_OPTION, INTERNAL_NETWORK_OPTION},
    cli::help::{color_command, color_group, color_heading, strip_ansi},
};

// Ensure top-level help stays compact as command surfaces grow.
#[test]
fn usage_lists_command_families() {
    let text = usage();
    let plain = strip_ansi(&text);

    assert!(plain.contains(&format!(
        "Canic Operator CLI v{}",
        env!("CARGO_PKG_VERSION")
    )));
    assert!(plain.contains("Usage: canic [OPTIONS] <COMMAND>"));
    assert!(plain.contains("\nCommands:\n"));
    assert!(plain.contains("Global commands"));
    assert!(plain.contains("Fleet commands"));
    assert!(plain.contains("Backup and restore commands"));
    assert!(plain.find("    status") < plain.find("    fleet"));
    assert!(plain.find("    fleet") < plain.find("    replica"));
    assert!(plain.find("    replica") < plain.find("    install"));
    assert!(plain.find("    install") < plain.find("    config"));
    assert!(plain.find("    config") < plain.find("    list"));
    assert!(plain.find("    list") < plain.find("    endpoints"));
    assert!(plain.find("    endpoints") < plain.find("    medic"));
    assert!(plain.find("    medic") < plain.find("    cycles"));
    assert!(plain.find("    cycles") < plain.find("    metrics"));
    assert!(plain.find("    metrics") < plain.find("    snapshot"));
    assert!(plain.find("    snapshot") < plain.find("    backup"));
    assert!(plain.find("    backup") < plain.find("    manifest"));
    assert!(plain.find("    manifest") < plain.find("    restore"));
    assert!(plain.contains("Options:"));
    assert!(plain.contains("--icp <path>"));
    assert!(plain.contains("--network <name>"));
    assert!(!plain.contains("    scaffold"));
    assert!(plain.contains("config"));
    assert!(plain.contains("list"));
    assert!(plain.contains("endpoints"));
    assert!(plain.contains("cycles"));
    assert!(plain.contains("metrics"));
    assert!(!plain.contains("    build"));
    assert!(!plain.contains("    network"));
    assert!(!plain.contains("    defaults"));
    assert!(plain.contains("    status"));
    assert!(plain.contains("fleet"));
    assert!(plain.contains("replica"));
    assert!(plain.contains("install"));
    assert!(plain.contains("snapshot"));
    assert!(plain.contains("backup"));
    assert!(plain.contains("manifest"));
    assert!(plain.contains("medic"));
    assert!(plain.contains("restore"));
    assert!(plain.contains("Tip: Run `canic <command> help`"));
    assert!(text.contains(color_heading()));
    assert!(text.contains(color_group()));
    assert!(text.contains(color_command()));
}

// Ensure command-family help paths return successfully instead of erroring.
#[test]
fn command_family_help_returns_ok() {
    for args in [
        &["backup", "help"][..],
        &["backup", "create", "help"],
        &["backup", "inspect", "help"],
        &["backup", "list", "help"],
        &["backup", "status", "help"],
        &["backup", "verify", "help"],
        &["config", "help"],
        &["cycles", "help"],
        &["endpoints", "help"],
        &["install", "help"],
        &["fleet"],
        &["fleet", "help"],
        &["fleet", "create", "help"],
        &["fleet", "list", "help"],
        &["fleet", "sync", "help"],
        &["fleet", "delete", "help"],
        &["replica"],
        &["replica", "help"],
        &["replica", "start", "help"],
        &["replica", "status", "help"],
        &["replica", "stop", "help"],
        &["list", "help"],
        &["restore", "help"],
        &["restore", "plan", "help"],
        &["restore", "apply", "help"],
        &["restore", "run", "help"],
        &["manifest", "help"],
        &["manifest", "validate", "help"],
        &["medic", "help"],
        &["metrics", "help"],
        &["snapshot", "help"],
        &["snapshot", "download", "help"],
        &["status", "help"],
    ] {
        assert_run_ok(args);
    }
}

// Ensure version flags are accepted at the top level and command-family level.
#[test]
fn version_flags_return_ok() {
    assert_eq!(version_text(), concat!("canic ", env!("CARGO_PKG_VERSION")));
    assert!(run([OsString::from("--version")]).is_ok());
    assert!(
        run([
            OsString::from("backup"),
            OsString::from("list"),
            OsString::from("--dir"),
            OsString::from("version")
        ])
        .is_ok()
    );
    assert!(run([OsString::from("backup"), OsString::from("--version")]).is_ok());
    assert!(
        run([
            OsString::from("backup"),
            OsString::from("list"),
            OsString::from("--version")
        ])
        .is_ok()
    );
    assert!(run([OsString::from("config"), OsString::from("--version")]).is_ok());
    assert!(run([OsString::from("cycles"), OsString::from("--version")]).is_ok());
    assert!(run([OsString::from("endpoints"), OsString::from("--version")]).is_ok());
    assert!(run([OsString::from("install"), OsString::from("--version")]).is_ok());
    assert!(run([OsString::from("fleet"), OsString::from("--version")]).is_ok());
    assert!(run([OsString::from("replica"), OsString::from("--version")]).is_ok());
    assert!(run([OsString::from("status"), OsString::from("--version")]).is_ok());
    assert!(
        run([
            OsString::from("fleet"),
            OsString::from("create"),
            OsString::from("--version")
        ])
        .is_ok()
    );
    assert!(
        run([
            OsString::from("fleet"),
            OsString::from("sync"),
            OsString::from("--version")
        ])
        .is_ok()
    );
    assert!(
        run([
            OsString::from("replica"),
            OsString::from("start"),
            OsString::from("--version")
        ])
        .is_ok()
    );
    assert!(run([OsString::from("list"), OsString::from("--version")]).is_ok());
    assert!(run([OsString::from("restore"), OsString::from("--version")]).is_ok());
    assert!(run([OsString::from("manifest"), OsString::from("--version")]).is_ok());
    assert!(run([OsString::from("medic"), OsString::from("--version")]).is_ok());
    assert!(run([OsString::from("metrics"), OsString::from("--version")]).is_ok());
    assert!(run([OsString::from("snapshot"), OsString::from("--version")]).is_ok());
    assert!(
        run([
            OsString::from("snapshot"),
            OsString::from("download"),
            OsString::from("--version")
        ])
        .is_ok()
    );
}

#[test]
fn global_icp_is_forwarded_to_commands_that_use_icp() {
    let mut tail = vec![OsString::from("test")];

    apply_global_icp("medic", &mut tail, Some("/tmp/icp".to_string()));

    assert_eq!(
        tail,
        vec![
            OsString::from("test"),
            OsString::from(INTERNAL_ICP_OPTION),
            OsString::from("/tmp/icp")
        ]
    );
}

#[test]
fn global_icp_does_not_override_internal_forwarded_icp() {
    let mut tail = vec![
        OsString::from("test"),
        OsString::from(INTERNAL_ICP_OPTION),
        OsString::from("/bin/icp"),
    ];

    apply_global_icp("medic", &mut tail, Some("/tmp/icp".to_string()));

    assert_eq!(
        tail,
        vec![
            OsString::from("test"),
            OsString::from(INTERNAL_ICP_OPTION),
            OsString::from("/bin/icp")
        ]
    );
}

#[test]
fn global_icp_is_forwarded_only_to_restore_run() {
    let mut plan_tail = vec![OsString::from("plan")];
    let mut run_tail = vec![OsString::from("run")];

    apply_global_icp("restore", &mut plan_tail, Some("/tmp/icp".to_string()));
    apply_global_icp("restore", &mut run_tail, Some("/tmp/icp".to_string()));

    assert_eq!(plan_tail, vec![OsString::from("plan")]);
    assert_eq!(
        run_tail,
        vec![
            OsString::from("run"),
            OsString::from(INTERNAL_ICP_OPTION),
            OsString::from("/tmp/icp")
        ]
    );
}

#[test]
fn global_icp_is_forwarded_only_to_replica_leaf_commands() {
    let mut family_tail = Vec::new();
    let mut start_tail = vec![OsString::from("start")];

    apply_global_icp("replica", &mut family_tail, Some("/tmp/icp".to_string()));
    apply_global_icp("replica", &mut start_tail, Some("/tmp/icp".to_string()));

    assert!(family_tail.is_empty());
    assert_eq!(
        start_tail,
        vec![
            OsString::from("start"),
            OsString::from(INTERNAL_ICP_OPTION),
            OsString::from("/tmp/icp")
        ]
    );
}

#[test]
fn global_network_is_forwarded_to_commands_that_use_network() {
    let mut tail = vec![OsString::from("test")];

    apply_global_network("install", &mut tail, Some("ic".to_string()));

    assert_eq!(
        tail,
        vec![
            OsString::from("test"),
            OsString::from(INTERNAL_NETWORK_OPTION),
            OsString::from("ic")
        ]
    );
}

#[test]
fn global_network_does_not_override_internal_forwarded_network() {
    let mut tail = vec![
        OsString::from("test"),
        OsString::from(INTERNAL_NETWORK_OPTION),
        OsString::from("local"),
    ];

    apply_global_network("install", &mut tail, Some("ic".to_string()));

    assert_eq!(
        tail,
        vec![
            OsString::from("test"),
            OsString::from(INTERNAL_NETWORK_OPTION),
            OsString::from("local")
        ]
    );
}

#[test]
fn global_network_is_forwarded_only_to_restore_run() {
    let mut plan_tail = vec![OsString::from("plan")];
    let mut run_tail = vec![OsString::from("run")];

    apply_global_network("restore", &mut plan_tail, Some("ic".to_string()));
    apply_global_network("restore", &mut run_tail, Some("ic".to_string()));

    assert_eq!(plan_tail, vec![OsString::from("plan")]);
    assert_eq!(
        run_tail,
        vec![
            OsString::from("run"),
            OsString::from(INTERNAL_NETWORK_OPTION),
            OsString::from("ic")
        ]
    );
}

#[test]
fn global_network_is_forwarded_only_to_fleet_list() {
    let mut create_tail = vec![OsString::from("create")];
    let mut list_tail = vec![OsString::from("list")];

    apply_global_network("fleet", &mut create_tail, Some("local".to_string()));
    apply_global_network("fleet", &mut list_tail, Some("local".to_string()));

    assert_eq!(create_tail, vec![OsString::from("create")]);
    assert_eq!(
        list_tail,
        vec![
            OsString::from("list"),
            OsString::from(INTERNAL_NETWORK_OPTION),
            OsString::from("local")
        ]
    );
}

#[test]
fn command_local_global_options_are_hard_rejected() {
    assert!(matches!(
        run([
            OsString::from("status"),
            OsString::from("--network"),
            OsString::from("local")
        ]),
        Err(CliError::Usage(_))
    ));
    assert!(matches!(
        run([
            OsString::from("medic"),
            OsString::from("test"),
            OsString::from("--icp"),
            OsString::from("icp")
        ]),
        Err(CliError::Usage(_))
    ));
}

// Assert that a CLI argv slice returns successfully.
fn assert_run_ok(raw_args: &[&str]) {
    let args = raw_args.iter().map(OsString::from).collect::<Vec<_>>();
    assert!(
        run(args).is_ok(),
        "expected successful run for {raw_args:?}"
    );
}
