use super::*;
use crate::{
    cli::globals::{INTERNAL_ICP_OPTION, INTERNAL_NETWORK_OPTION},
    info::InfoCommandError,
};

fn strip_ansi(text: &str) -> String {
    let mut plain = String::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            for ch in chars.by_ref() {
                if ch == 'm' {
                    break;
                }
            }
            continue;
        }
        plain.push(ch);
    }
    plain
}

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
    assert!(plain.contains("Project commands"));
    assert!(plain.contains("Deployment commands"));
    assert!(plain.contains("ICP wallet commands"));
    assert!(plain.contains("Backup and restore commands"));
    assert!(plain.find("    status") < plain.find("    fleet"));
    assert!(plain.find("    fleet") < plain.find("    scaffold"));
    assert!(plain.find("    scaffold") < plain.find("    replica"));
    assert!(plain.find("    replica") < plain.find("    install"));
    assert!(plain.find("    install") < plain.find("    build"));
    assert!(plain.find("    build") < plain.find("    deploy"));
    assert!(plain.find("    deploy") < plain.find("    evidence"));
    assert!(plain.find("    evidence") < plain.find("    info"));
    assert!(plain.find("    info") < plain.find("    endpoints"));
    assert!(plain.find("    endpoints") < plain.find("    medic"));
    assert!(plain.find("    medic") < plain.find("    metrics"));
    assert!(plain.find("    metrics") < plain.find("    cycles"));
    assert!(plain.find("    cycles") < plain.find("    token"));
    assert!(plain.find("    metrics") < plain.find("    snapshot"));
    assert!(plain.find("    snapshot") < plain.find("    backup"));
    assert!(plain.find("    backup") < plain.find("    restore"));
    assert!(plain.contains("Options:"));
    assert!(plain.contains("--icp <path>"));
    assert!(plain.contains("--network <name>"));
    assert!(plain.contains("    scaffold"));
    assert!(plain.contains("cycles"));
    assert!(plain.contains("token"));
    assert!(plain.contains("info"));
    assert!(plain.contains("endpoints"));
    assert!(plain.contains("metrics"));
    assert!(plain.contains("    build"));
    assert!(plain.contains("    deploy"));
    assert!(!plain.contains("    network"));
    assert!(!plain.contains("    defaults"));
    assert!(plain.contains("    status"));
    assert!(plain.contains("fleet"));
    assert!(plain.contains("replica"));
    assert!(plain.contains("install"));
    assert!(plain.contains("snapshot"));
    assert!(plain.contains("backup"));
    assert!(plain.contains("medic"));
    assert!(plain.contains("restore"));
    assert!(plain.contains("Tip: Run `canic <command> help`"));
}

// Ensure command-family help paths return successfully instead of erroring.
#[test]
fn command_family_help_returns_ok() {
    for args in [
        &["backup", "help"][..],
        &["backup", "create", "help"],
        &["backup", "inspect", "help"],
        &["backup", "list", "help"],
        &["backup", "manifest", "help"],
        &["backup", "manifest", "validate", "help"],
        &["backup", "status", "help"],
        &["backup", "verify", "help"],
        &["build", "help"],
        &["cycles", "help"],
        &["cycles", "balance", "help"],
        &["cycles", "mint", "help"],
        &["cycles", "transfer", "help"],
        &["cycles", "topup", "help"],
        &["deploy", "help"],
        &["deploy", "check", "help"],
        &["deploy", "diff", "help"],
        &["deploy", "inventory", "help"],
        &["deploy", "plan", "help"],
        &["deploy", "report", "help"],
        &["info", "help"],
        &["info", "list", "help"],
        &["info", "cycles", "help"],
        &["endpoints", "help"],
        &["evidence", "help"],
        &["evidence", "compare", "help"],
        &["install", "help"],
        &["fleet"],
        &["fleet", "help"],
        &["fleet", "check", "help"],
        &["fleet", "config", "help"],
        &["fleet", "create", "help"],
        &["fleet", "list", "help"],
        &["fleet", "delete", "help"],
        &["scaffold"],
        &["scaffold", "help"],
        &["scaffold", "canister", "help"],
        &["replica"],
        &["replica", "help"],
        &["replica", "start", "help"],
        &["replica", "status", "help"],
        &["replica", "stop", "help"],
        &["restore", "help"],
        &["restore", "plan", "help"],
        &["restore", "apply", "help"],
        &["restore", "run", "help"],
        &["medic", "help"],
        &["metrics", "help"],
        &["token", "help"],
        &["token", "balance", "help"],
        &["token", "icp", "balance", "help"],
        &["token", "transfer", "help"],
        &["snapshot", "help"],
        &["snapshot", "download", "help"],
        &["status", "help"],
    ] {
        assert_run_ok(args);
    }
}

// Ensure the old read-only top-level list alias is removed in favor of canic info.
#[test]
fn top_level_info_aliases_are_removed() {
    std::assert_matches!(
        run([OsString::from("list"), OsString::from("help")]),
        Err(CliError::Usage(_))
    );
}

#[test]
fn top_level_fleet_config_command_is_removed() {
    std::assert_matches!(
        run([OsString::from("config"), OsString::from("help")]),
        Err(CliError::Usage(_))
    );
}

#[test]
fn top_level_backup_manifest_command_is_removed() {
    std::assert_matches!(
        run([OsString::from("manifest"), OsString::from("help")]),
        Err(CliError::Usage(_))
    );
}

#[test]
fn info_help_uses_deployment_target_wording() {
    let err = run([OsString::from("info")]).expect_err("info needs a subcommand");
    let CliError::Info(InfoCommandError::Usage(text)) = err else {
        panic!("expected info usage error");
    };

    assert!(text.contains("installed-deployment information commands"));
    assert!(text.contains("List installed deployment canisters"));
    assert!(!text.contains("deployed-fleet"));
    assert!(!text.contains("deployed fleet"));
}

// Ensure the old fleet sync command is removed in favor of fleet check.
#[test]
fn fleet_sync_is_removed() {
    std::assert_matches!(
        run([OsString::from("fleet"), OsString::from("sync")]),
        Err(CliError::Fleets(_))
    );
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
    assert!(
        run([
            OsString::from("backup"),
            OsString::from("manifest"),
            OsString::from("--version")
        ])
        .is_ok()
    );
    assert!(run([OsString::from("build"), OsString::from("--version")]).is_ok());
    assert!(run([OsString::from("cycles"), OsString::from("--version")]).is_ok());
    assert!(run([OsString::from("info"), OsString::from("--version")]).is_ok());
    assert!(
        run([
            OsString::from("info"),
            OsString::from("list"),
            OsString::from("--version")
        ])
        .is_ok()
    );
    assert!(
        run([
            OsString::from("info"),
            OsString::from("cycles"),
            OsString::from("--version")
        ])
        .is_ok()
    );
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
            OsString::from("check"),
            OsString::from("--version")
        ])
        .is_ok()
    );
    assert!(
        run([
            OsString::from("fleet"),
            OsString::from("config"),
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
    assert!(run([OsString::from("restore"), OsString::from("--version")]).is_ok());
    assert!(run([OsString::from("medic"), OsString::from("--version")]).is_ok());
    assert!(run([OsString::from("metrics"), OsString::from("--version")]).is_ok());
    assert!(run([OsString::from("token"), OsString::from("--version")]).is_ok());
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
fn deploy_version_flags_return_ok() {
    assert!(run([OsString::from("deploy"), OsString::from("--version")]).is_ok());
    assert!(
        run([
            OsString::from("deploy"),
            OsString::from("check"),
            OsString::from("--version")
        ])
        .is_ok()
    );
    assert!(
        run([
            OsString::from("deploy"),
            OsString::from("diff"),
            OsString::from("--version")
        ])
        .is_ok()
    );
    assert!(
        run([
            OsString::from("deploy"),
            OsString::from("inventory"),
            OsString::from("--version")
        ])
        .is_ok()
    );
    assert!(
        run([
            OsString::from("deploy"),
            OsString::from("plan"),
            OsString::from("--version")
        ])
        .is_ok()
    );
    assert!(
        run([
            OsString::from("deploy"),
            OsString::from("report"),
            OsString::from("--version")
        ])
        .is_ok()
    );
}

#[test]
fn global_icp_is_forwarded_to_commands_that_use_icp() {
    let mut tail = vec![OsString::from("test")];
    let mut cycles_tail = vec![OsString::from("balance")];
    let mut token_tail = vec![OsString::from("balance")];

    apply_global_icp("medic", &mut tail, Some("/tmp/icp".to_string()));
    apply_global_icp("cycles", &mut cycles_tail, Some("/tmp/icp".to_string()));
    apply_global_icp("token", &mut token_tail, Some("/tmp/icp".to_string()));

    assert_eq!(
        tail,
        vec![
            OsString::from("test"),
            OsString::from(INTERNAL_ICP_OPTION),
            OsString::from("/tmp/icp")
        ]
    );
    assert_eq!(
        cycles_tail,
        vec![
            OsString::from("balance"),
            OsString::from(INTERNAL_ICP_OPTION),
            OsString::from("/tmp/icp")
        ]
    );
    assert_eq!(
        token_tail,
        vec![
            OsString::from("balance"),
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
fn global_icp_is_forwarded_to_info_query_commands() {
    let mut list_tail = vec![OsString::from("list"), OsString::from("test")];
    let mut cycles_tail = vec![OsString::from("cycles"), OsString::from("test")];
    let mut help_tail = vec![OsString::from("help")];

    apply_global_icp("info", &mut list_tail, Some("/tmp/icp".to_string()));
    apply_global_icp("info", &mut cycles_tail, Some("/tmp/icp".to_string()));
    apply_global_icp("info", &mut help_tail, Some("/tmp/icp".to_string()));

    assert_eq!(
        list_tail,
        vec![
            OsString::from("list"),
            OsString::from("test"),
            OsString::from(INTERNAL_ICP_OPTION),
            OsString::from("/tmp/icp")
        ]
    );
    assert_eq!(
        cycles_tail,
        vec![
            OsString::from("cycles"),
            OsString::from("test"),
            OsString::from(INTERNAL_ICP_OPTION),
            OsString::from("/tmp/icp")
        ]
    );
    assert_eq!(help_tail, vec![OsString::from("help")]);
}

#[test]
fn global_network_is_forwarded_to_commands_that_use_network() {
    let mut tail = vec![OsString::from("test")];
    let mut cycles_tail = vec![OsString::from("balance")];
    let mut token_tail = vec![OsString::from("balance")];

    apply_global_network("install", &mut tail, Some("ic".to_string()));
    apply_global_network("cycles", &mut cycles_tail, Some("ic".to_string()));
    apply_global_network("token", &mut token_tail, Some("ic".to_string()));

    assert_eq!(
        tail,
        vec![
            OsString::from("test"),
            OsString::from(INTERNAL_NETWORK_OPTION),
            OsString::from("ic")
        ]
    );
    assert_eq!(
        cycles_tail,
        vec![
            OsString::from("balance"),
            OsString::from(INTERNAL_NETWORK_OPTION),
            OsString::from("ic")
        ]
    );
    assert_eq!(
        token_tail,
        vec![
            OsString::from("balance"),
            OsString::from(INTERNAL_NETWORK_OPTION),
            OsString::from("ic")
        ]
    );
}

#[test]
fn global_network_is_forwarded_to_deploy() {
    let mut tail = vec![OsString::from("check"), OsString::from("demo")];
    let mut diff_tail = vec![OsString::from("diff"), OsString::from("demo")];
    let mut install_tail = vec![OsString::from("install"), OsString::from("demo")];
    let mut inventory_tail = vec![OsString::from("inventory"), OsString::from("demo")];
    let mut plan_tail = vec![OsString::from("plan"), OsString::from("demo")];
    let mut register_tail = vec![OsString::from("register"), OsString::from("demo")];
    let mut report_tail = vec![OsString::from("report"), OsString::from("demo")];
    let mut resume_tail = vec![OsString::from("resume-report"), OsString::from("demo")];
    let mut family_tail = Vec::new();

    apply_global_network("deploy", &mut tail, Some("ic".to_string()));
    apply_global_network("deploy", &mut diff_tail, Some("ic".to_string()));
    apply_global_network("deploy", &mut install_tail, Some("ic".to_string()));
    apply_global_network("deploy", &mut inventory_tail, Some("ic".to_string()));
    apply_global_network("deploy", &mut plan_tail, Some("ic".to_string()));
    apply_global_network("deploy", &mut register_tail, Some("ic".to_string()));
    apply_global_network("deploy", &mut report_tail, Some("ic".to_string()));
    apply_global_network("deploy", &mut resume_tail, Some("ic".to_string()));
    apply_global_network("deploy", &mut family_tail, Some("ic".to_string()));

    assert_eq!(
        tail,
        vec![
            OsString::from("check"),
            OsString::from("demo"),
            OsString::from(INTERNAL_NETWORK_OPTION),
            OsString::from("ic")
        ]
    );
    assert_eq!(
        diff_tail,
        vec![
            OsString::from("diff"),
            OsString::from("demo"),
            OsString::from(INTERNAL_NETWORK_OPTION),
            OsString::from("ic")
        ]
    );
    assert_eq!(
        install_tail,
        vec![
            OsString::from("install"),
            OsString::from("demo"),
            OsString::from(INTERNAL_NETWORK_OPTION),
            OsString::from("ic")
        ]
    );
    assert_eq!(
        inventory_tail,
        vec![
            OsString::from("inventory"),
            OsString::from("demo"),
            OsString::from(INTERNAL_NETWORK_OPTION),
            OsString::from("ic")
        ]
    );
    assert_eq!(
        plan_tail,
        vec![
            OsString::from("plan"),
            OsString::from("demo"),
            OsString::from(INTERNAL_NETWORK_OPTION),
            OsString::from("ic")
        ]
    );
    assert_eq!(
        register_tail,
        vec![
            OsString::from("register"),
            OsString::from("demo"),
            OsString::from(INTERNAL_NETWORK_OPTION),
            OsString::from("ic")
        ]
    );
    assert_eq!(
        report_tail,
        vec![
            OsString::from("report"),
            OsString::from("demo"),
            OsString::from(INTERNAL_NETWORK_OPTION),
            OsString::from("ic")
        ]
    );
    assert_eq!(
        resume_tail,
        vec![
            OsString::from("resume-report"),
            OsString::from("demo"),
            OsString::from(INTERNAL_NETWORK_OPTION),
            OsString::from("ic")
        ]
    );
    assert!(family_tail.is_empty());
}

#[test]
fn global_network_is_forwarded_to_nested_deploy_network_leaves() {
    for raw_tail in [
        &["authority", "check", "demo"][..],
        &["authority", "evidence", "demo"],
        &["authority", "receipt", "demo"],
        &["authority", "report", "demo"],
        &["external", "check", "demo"],
        &["external", "critical-fix", "demo"],
        &["external", "handoff", "demo"],
        &["external", "pending", "demo"],
        &["external", "plan", "demo"],
        &["external", "proposals", "demo"],
        &["root", "verify", "demo"],
    ] {
        let mut tail = raw_tail.iter().map(OsString::from).collect::<Vec<_>>();
        apply_global_network("deploy", &mut tail, Some("ic".to_string()));

        assert!(tail.ends_with(&[
            OsString::from(INTERNAL_NETWORK_OPTION),
            OsString::from("ic")
        ]));
    }
}

#[test]
fn global_network_is_not_forwarded_to_request_only_deploy_leaves() {
    for raw_tail in [
        &["compare", "--left", "a.json", "--right", "b.json"][..],
        &["external", "verify", "--request", "request.json"],
        &[
            "external",
            "inspect",
            "consent",
            "--request",
            "request.json",
        ],
        &["promote", "plan", "--request", "request.json"],
        &["root", "inspect", "--request", "request.json"],
    ] {
        let mut tail = raw_tail.iter().map(OsString::from).collect::<Vec<_>>();
        let original = tail.clone();
        apply_global_network("deploy", &mut tail, Some("ic".to_string()));

        assert_eq!(tail, original);
    }
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
fn global_network_is_forwarded_to_info_query_commands() {
    let mut list_tail = vec![OsString::from("list"), OsString::from("test")];
    let mut cycles_tail = vec![OsString::from("cycles"), OsString::from("test")];
    let mut help_tail = vec![OsString::from("help")];

    apply_global_network("info", &mut list_tail, Some("local".to_string()));
    apply_global_network("info", &mut cycles_tail, Some("local".to_string()));
    apply_global_network("info", &mut help_tail, Some("local".to_string()));

    assert_eq!(
        list_tail,
        vec![
            OsString::from("list"),
            OsString::from("test"),
            OsString::from(INTERNAL_NETWORK_OPTION),
            OsString::from("local")
        ]
    );
    assert_eq!(
        cycles_tail,
        vec![
            OsString::from("cycles"),
            OsString::from("test"),
            OsString::from(INTERNAL_NETWORK_OPTION),
            OsString::from("local")
        ]
    );
    assert_eq!(help_tail, vec![OsString::from("help")]);
}

#[test]
fn command_local_global_options_are_hard_rejected() {
    std::assert_matches!(
        run([
            OsString::from("status"),
            OsString::from("--network"),
            OsString::from("local")
        ]),
        Err(CliError::Usage(_))
    );
    std::assert_matches!(
        run([
            OsString::from("medic"),
            OsString::from("test"),
            OsString::from("--icp"),
            OsString::from("icp")
        ]),
        Err(CliError::Usage(_))
    );
}

// Assert that a CLI argv slice returns successfully.
fn assert_run_ok(raw_args: &[&str]) {
    let args = raw_args.iter().map(OsString::from).collect::<Vec<_>>();
    assert!(
        run(args).is_ok(),
        "expected successful run for {raw_args:?}"
    );
}
