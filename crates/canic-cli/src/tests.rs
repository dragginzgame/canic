use super::*;
use crate::{
    cli::globals::{INTERNAL_ICP_OPTION, INTERNAL_NETWORK_OPTION},
    info::InfoCommandError,
};

#[cfg(unix)]
use crate::test_support::TempDir;

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
    assert!(plain.find("    status") < plain.find("    medic"));
    assert!(plain.find("    medic") < plain.find("    state"));
    assert!(plain.find("    state") < plain.find("    fleet"));
    assert!(plain.find("    fleet") < plain.find("    scaffold"));
    assert!(plain.find("    scaffold") < plain.find("    replica"));
    assert!(plain.find("    replica") < plain.find("    install"));
    assert!(plain.find("    install") < plain.find("    build"));
    assert!(plain.find("    build") < plain.find("    deploy"));
    assert!(plain.find("    deploy") < plain.find("    evidence"));
    assert!(plain.find("    evidence") < plain.find("    info"));
    assert!(plain.find("    info") < plain.find("    cycles"));
    assert!(plain.find("    cycles") < plain.find("    token"));
    assert!(plain.find("    token") < plain.find("    snapshot"));
    assert!(plain.find("    snapshot") < plain.find("    backup"));
    assert!(plain.find("    backup") < plain.find("    restore"));
    assert!(plain.contains("Options:"));
    assert!(plain.contains("--icp <path>"));
    assert!(plain.contains("--network <name>"));
    assert!(plain.contains("Diagnose project and deployment preflight readiness"));
    assert!(plain.contains("Audit declared Canic state metadata"));
    assert!(plain.contains("    scaffold"));
    assert!(plain.contains("Inspect runtime-observed status for one deployed canister"));
    assert!(plain.contains("cycles"));
    assert!(plain.contains("token"));
    assert!(plain.contains("info"));
    assert!(plain.contains("    build"));
    assert!(plain.contains("    deploy"));
    assert!(plain.contains("Manage Canic fleets and roles"));
    assert!(plain.contains("Check, inspect, register, and install deployments"));
    assert!(plain.contains("Plan, inspect, and verify backups"));
    assert!(!plain.contains("Check deployment truth before mutation"));
    assert!(!plain.contains("    network"));
    assert!(!plain.contains("    defaults"));
    assert!(plain.contains("    status"));
    assert!(plain.contains("    medic"));
    assert!(plain.contains("    state"));
    assert!(plain.contains("fleet"));
    assert!(plain.contains("replica"));
    assert!(plain.contains("install"));
    assert!(plain.contains("snapshot"));
    assert!(plain.contains("backup"));
    assert!(plain.contains("restore"));
    assert!(!plain.contains("    endpoints"));
    assert!(!plain.contains("    metrics"));
    assert!(plain.contains("Tip: Run `canic <command> help`"));
}

#[test]
fn report_status_errors_delegate_suppression_and_exit_codes() {
    let cases = [
        CliError::Deploy(deploy::DeployCommandError::PlanBlocked(
            "blocked".to_string(),
        )),
        CliError::Inspect(inspect::InspectCommandError::ReportStatus(
            "failing".to_string(),
        )),
        CliError::Medic(medic::MedicCommandError::ReportFailed),
        CliError::State(state::StateCommandError::AuditFailed),
    ];

    for error in cases {
        assert_eq!(render_cli_error(&error), "");
        assert_eq!(cli_error_exit_code(&error), 1);
    }
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
        &["deploy", "plan", "help"],
        &["deploy", "plan", "--help"],
        &["deploy", "inspect", "help"],
        &["deploy", "inspect", "diff", "help"],
        &["deploy", "inspect", "inventory", "help"],
        &["deploy", "inspect", "plan", "help"],
        &["deploy", "inspect", "report", "help"],
        &["deploy", "inspect", "compare", "help"],
        &["deploy", "inspect", "catalog", "help"],
        &["deploy", "inspect", "catalog", "list", "help"],
        &["deploy", "inspect", "catalog", "inspect", "help"],
        &["deploy", "inspect", "root", "help"],
        &["deploy", "inspect", "resume-report", "help"],
        &["deploy", "root", "help"],
        &["deploy", "root", "verify", "help"],
        &["info", "help"],
        &["info", "list", "help"],
        &["info", "cycles", "help"],
        &["info", "metrics", "help"],
        &["info", "endpoints", "help"],
        &["info", "env", "help"],
        &["medic", "help"],
        &["medic", "project", "help"],
        &["medic", "--json", "project", "help"],
        &["medic", "project", "--json", "help"],
        &["medic", "deployment", "help"],
        &["medic", "deployment", "--help"],
        &["medic", "--json", "deployment", "help"],
        &["medic", "deployment", "--json", "help"],
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
        &["inspect", "help"],
        &["inspect", "canister", "help"],
        &["inspect", "deployment", "help"],
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
        &["state", "help"],
        &["state", "audit", "help"],
        &["state", "manifest", "help"],
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

#[test]
fn info_help_uses_deployment_target_wording() {
    let err = run([OsString::from("info")]).expect_err("info needs a subcommand");
    let CliError::Info(InfoCommandError::Usage(text)) = err else {
        panic!("expected info usage error");
    };

    assert!(text.contains("installed-deployment information commands"));
    assert!(text.contains("List installed deployment canisters"));
    assert!(text.contains("Summarize deployment cycle history"));
    assert!(text.contains("Query Canic runtime telemetry"));
    assert!(text.contains("List callable Candid endpoints"));
    assert!(text.contains("Print sourceable canister ID exports"));
    assert!(!text.contains("canic info medic"));
    assert!(!text.contains("deployed-fleet"));
    assert!(!text.contains("deployed fleet"));
}

#[cfg(unix)]
#[test]
fn icp_backed_command_rejects_unparseable_icp_cli_before_running_subcommand() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    let root = TempDir::new("canic-cli-unsupported-icp");
    fs::create_dir_all(&root).expect("create temp dir");
    let icp_path = root.join("icp");
    fs::write(
        &icp_path,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'icp development build'; exit 0; fi\necho 'unsupported replica command ran' >&2\nexit 42\n",
    )
    .expect("write fake icp");
    fs::set_permissions(&icp_path, fs::Permissions::from_mode(0o755)).expect("chmod fake icp");

    let err = run([
        OsString::from("--icp"),
        icp_path.into_os_string(),
        OsString::from("replica"),
        OsString::from("status"),
    ])
    .expect_err("unsupported icp rejected");
    let text = err.to_string();

    assert!(text.contains("unsupported icp-cli version"));
    assert!(text.contains("found: icp development build"));
    assert!(text.contains("required: icp-cli >=1.0.0, <2.0.0"));
    assert!(!text.contains("unsupported replica command ran"));
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
    assert!(run([OsString::from("install"), OsString::from("--version")]).is_ok());
    assert!(run([OsString::from("inspect"), OsString::from("--version")]).is_ok());
    assert!(run([OsString::from("medic"), OsString::from("--version")]).is_ok());
    assert!(run([OsString::from("fleet"), OsString::from("--version")]).is_ok());
    assert!(run([OsString::from("replica"), OsString::from("--version")]).is_ok());
    assert!(run([OsString::from("state"), OsString::from("--version")]).is_ok());
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
fn info_version_flags_return_ok() {
    assert!(run([OsString::from("info"), OsString::from("--version")]).is_ok());
    for leaf in ["list", "cycles", "metrics", "endpoints", "env"] {
        assert!(
            run([
                OsString::from("info"),
                OsString::from(leaf),
                OsString::from("--version")
            ])
            .is_ok()
        );
    }
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
            OsString::from("plan"),
            OsString::from("--version")
        ])
        .is_ok()
    );
    assert!(
        run([
            OsString::from("deploy"),
            OsString::from("inspect"),
            OsString::from("diff"),
            OsString::from("--version")
        ])
        .is_ok()
    );
    assert!(
        run([
            OsString::from("deploy"),
            OsString::from("inspect"),
            OsString::from("inventory"),
            OsString::from("--version")
        ])
        .is_ok()
    );
    assert!(
        run([
            OsString::from("deploy"),
            OsString::from("inspect"),
            OsString::from("plan"),
            OsString::from("--version")
        ])
        .is_ok()
    );
    assert!(
        run([
            OsString::from("deploy"),
            OsString::from("inspect"),
            OsString::from("report"),
            OsString::from("--version")
        ])
        .is_ok()
    );
}

#[test]
fn rejected_deploy_plan_command_forms_are_usage_errors() {
    for raw_args in [
        &["plan", "demo"][..],
        &["deploy", "plan"],
        &["deploy", "plan", "--deployment", "demo"],
        &["deploy", "diff", "demo"],
        &["apply", "demo"],
        &["apply", "demo", "--plan", "deployment-plan.json"],
        &["deploy", "plan", "demo", "--apply"],
        &["deploy", "plan", "demo", "--write-truth"],
        &["deploy", "plan", "demo", "--evidence"],
        &["deploy", "plan", "demo", "--format", "json"],
        &[
            "deploy",
            "plan",
            "demo",
            "--from-check",
            "deployment-check.json",
        ],
        &["deploy", "plan", "demo", "--observe-local"],
        &[
            "deploy",
            "plan",
            "demo",
            "--out",
            "deployment-plan.json",
            "--force",
        ],
    ] {
        let err = run(raw_args.iter().map(OsString::from))
            .expect_err("rejected deploy plan form should fail");

        assert!(
            matches!(
                err,
                CliError::Usage(_) | CliError::Deploy(deploy::DeployCommandError::Usage(_))
            ),
            "wrong error for {raw_args:?}: {err}"
        );
        assert_eq!(
            cli_error_exit_code(&err),
            2,
            "wrong exit code for {raw_args:?}: {err}"
        );
    }
}

#[test]
fn rejected_inspect_command_forms_are_usage_errors() {
    for raw_args in [
        &["inspect"][..],
        &["inspect", "demo-local"],
        &["inspect", "deployment", "demo-local"],
        &["inspect", "deployment", "demo-local", "--all"],
        &[
            "inspect",
            "deployment",
            "demo-local",
            "--role",
            "root",
            "--all",
        ],
        &["inspect", "canister", "aaaaa-aa", "--health"],
        &["inspect", "canister", "aaaaa-aa", "--readiness"],
        &["inspect", "canister", "aaaaa-aa", "--format", "json"],
        &[
            "inspect",
            "deployment",
            "demo-local",
            "--role",
            "root",
            "--format",
            "json",
        ],
    ] {
        let err =
            run(raw_args.iter().map(OsString::from)).expect_err("inspect form should be rejected");

        assert!(
            matches!(err, CliError::Usage(_) | CliError::Inspect(_)),
            "wrong error for {raw_args:?}: {err}"
        );
        assert_eq!(
            cli_error_exit_code(&err),
            2,
            "wrong exit code for {raw_args:?}: {err}"
        );
    }
}

#[test]
fn blocked_deploy_plan_report_suppresses_duplicate_cli_stderr() {
    let blocked = CliError::Deploy(deploy::DeployCommandError::PlanBlocked(
        "blocked".to_string(),
    ));
    assert_eq!(cli_error_exit_code(&blocked), 1);
    assert!(render_cli_error(&blocked).is_empty());

    let usage = CliError::Deploy(deploy::DeployCommandError::Usage("usage".to_string()));
    assert_eq!(cli_error_exit_code(&usage), 2);
    assert!(!render_cli_error(&usage).is_empty());
}

#[test]
fn global_icp_is_forwarded_to_commands_that_use_icp() {
    let mut status_tail = Vec::new();
    let mut cycles_tail = vec![OsString::from("balance")];
    let mut medic_tail = Vec::new();
    let mut token_tail = vec![OsString::from("balance")];

    apply_global_icp("status", &mut status_tail, Some("/tmp/icp".to_string()));
    apply_global_icp("cycles", &mut cycles_tail, Some("/tmp/icp".to_string()));
    apply_global_icp("medic", &mut medic_tail, Some("/tmp/icp".to_string()));
    apply_global_icp("token", &mut token_tail, Some("/tmp/icp".to_string()));

    assert_eq!(
        status_tail,
        vec![
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
        medic_tail,
        vec![
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
        OsString::from("balance"),
        OsString::from(INTERNAL_ICP_OPTION),
        OsString::from("/bin/icp"),
    ];

    apply_global_icp("cycles", &mut tail, Some("/tmp/icp".to_string()));

    assert_eq!(
        tail,
        vec![
            OsString::from("balance"),
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
    let mut metrics_tail = vec![OsString::from("metrics"), OsString::from("test")];
    let mut endpoints_tail = vec![
        OsString::from("endpoints"),
        OsString::from("test"),
        OsString::from("app"),
    ];
    let mut env_tail = vec![OsString::from("env"), OsString::from("test")];
    let mut removed_medic_tail = vec![OsString::from("medic"), OsString::from("test")];
    let mut help_tail = vec![OsString::from("help")];
    let original_removed_medic_tail = removed_medic_tail.clone();

    apply_global_icp("info", &mut list_tail, Some("/tmp/icp".to_string()));
    apply_global_icp("info", &mut cycles_tail, Some("/tmp/icp".to_string()));
    apply_global_icp("info", &mut metrics_tail, Some("/tmp/icp".to_string()));
    apply_global_icp("info", &mut endpoints_tail, Some("/tmp/icp".to_string()));
    apply_global_icp("info", &mut env_tail, Some("/tmp/icp".to_string()));
    apply_global_icp(
        "info",
        &mut removed_medic_tail,
        Some("/tmp/icp".to_string()),
    );
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
    assert_eq!(
        metrics_tail,
        vec![
            OsString::from("metrics"),
            OsString::from("test"),
            OsString::from(INTERNAL_ICP_OPTION),
            OsString::from("/tmp/icp")
        ]
    );
    assert_eq!(
        endpoints_tail,
        vec![
            OsString::from("endpoints"),
            OsString::from("test"),
            OsString::from("app"),
            OsString::from(INTERNAL_ICP_OPTION),
            OsString::from("/tmp/icp")
        ]
    );
    assert_eq!(
        env_tail,
        vec![
            OsString::from("env"),
            OsString::from("test"),
            OsString::from(INTERNAL_ICP_OPTION),
            OsString::from("/tmp/icp")
        ]
    );
    assert_eq!(removed_medic_tail, original_removed_medic_tail);
    assert_eq!(help_tail, vec![OsString::from("help")]);
}

#[test]
fn global_icp_is_forwarded_only_to_active_auth_renewal_status() {
    let mut removed_run_once_tail = vec![
        OsString::from("renewal"),
        OsString::from("run-once"),
        OsString::from("downstream"),
    ];
    let mut status_tail = vec![
        OsString::from("renewal"),
        OsString::from("status"),
        OsString::from("downstream"),
        OsString::from("--issuer"),
        OsString::from("rrkah-fqaaa-aaaaa-aaaaq-cai"),
    ];
    let mut removed_provisioner_list_tail = vec![
        OsString::from("renewal"),
        OsString::from("provisioner"),
        OsString::from("list"),
        OsString::from("downstream"),
    ];
    let mut removed_provisioner_enable_tail = vec![
        OsString::from("renewal"),
        OsString::from("provisioner"),
        OsString::from("enable"),
        OsString::from("downstream"),
        OsString::from("rrkah-fqaaa-aaaaa-aaaaq-cai"),
    ];
    let mut help_tail = vec![OsString::from("help")];

    let original_removed_run_once_tail = removed_run_once_tail.clone();
    let original_removed_provisioner_list_tail = removed_provisioner_list_tail.clone();
    let original_removed_provisioner_enable_tail = removed_provisioner_enable_tail.clone();

    apply_global_icp(
        "auth",
        &mut removed_run_once_tail,
        Some("/tmp/icp".to_string()),
    );
    apply_global_icp("auth", &mut status_tail, Some("/tmp/icp".to_string()));
    apply_global_icp(
        "auth",
        &mut removed_provisioner_list_tail,
        Some("/tmp/icp".to_string()),
    );
    apply_global_icp(
        "auth",
        &mut removed_provisioner_enable_tail,
        Some("/tmp/icp".to_string()),
    );
    apply_global_icp("auth", &mut help_tail, Some("/tmp/icp".to_string()));

    assert_eq!(removed_run_once_tail, original_removed_run_once_tail);
    assert!(status_tail.ends_with(&[
        OsString::from(INTERNAL_ICP_OPTION),
        OsString::from("/tmp/icp")
    ]));
    assert_eq!(
        removed_provisioner_list_tail,
        original_removed_provisioner_list_tail
    );
    assert_eq!(
        removed_provisioner_enable_tail,
        original_removed_provisioner_enable_tail
    );
    assert_eq!(help_tail, vec![OsString::from("help")]);
}

#[test]
fn global_network_is_forwarded_to_commands_that_use_network() {
    let mut tail = vec![OsString::from("test")];
    let mut cycles_tail = vec![OsString::from("balance")];
    let mut medic_tail = Vec::new();
    let mut token_tail = vec![OsString::from("balance")];

    apply_global_network("install", &mut tail, Some("ic".to_string()));
    apply_global_network("cycles", &mut cycles_tail, Some("ic".to_string()));
    apply_global_network("medic", &mut medic_tail, Some("ic".to_string()));
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
        medic_tail,
        vec![
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
    for raw_tail in [
        &["check", "demo"][..],
        &["inspect", "diff", "demo"],
        &["install", "demo"],
        &["inspect", "inventory", "demo"],
        &["inspect", "plan", "demo"],
        &["register", "demo"],
        &["inspect", "report", "demo"],
        &["inspect", "resume-report", "demo"],
    ] {
        assert_global_network_forwarded_to_deploy_tail(raw_tail);
    }

    let mut family_tail = Vec::new();
    apply_global_network("deploy", &mut family_tail, Some("ic".to_string()));
    assert!(family_tail.is_empty());
}

fn assert_global_network_forwarded_to_deploy_tail(raw_tail: &[&str]) {
    let mut tail = raw_tail.iter().map(OsString::from).collect::<Vec<_>>();
    apply_global_network("deploy", &mut tail, Some("ic".to_string()));

    assert_eq!(
        tail,
        raw_tail
            .iter()
            .copied()
            .chain([INTERNAL_NETWORK_OPTION, "ic"])
            .map(OsString::from)
            .collect::<Vec<_>>()
    );
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
        &["inspect", "catalog", "list"],
        &["inspect", "catalog", "inspect", "demo"],
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
        &[
            "inspect", "compare", "--left", "a.json", "--right", "b.json",
        ][..],
        &["inspect", "root", "--request", "request.json"],
        &["external", "verify", "--request", "request.json"],
        &[
            "external",
            "inspect",
            "consent",
            "--request",
            "request.json",
        ],
        &["promote", "plan", "--request", "request.json"],
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
    let mut metrics_tail = vec![OsString::from("metrics"), OsString::from("test")];
    let mut endpoints_tail = vec![
        OsString::from("endpoints"),
        OsString::from("test"),
        OsString::from("app"),
    ];
    let mut env_tail = vec![OsString::from("env"), OsString::from("test")];
    let mut removed_medic_tail = vec![OsString::from("medic"), OsString::from("test")];
    let mut help_tail = vec![OsString::from("help")];
    let original_removed_medic_tail = removed_medic_tail.clone();

    apply_global_network("info", &mut list_tail, Some("local".to_string()));
    apply_global_network("info", &mut cycles_tail, Some("local".to_string()));
    apply_global_network("info", &mut metrics_tail, Some("local".to_string()));
    apply_global_network("info", &mut endpoints_tail, Some("local".to_string()));
    apply_global_network("info", &mut env_tail, Some("local".to_string()));
    apply_global_network("info", &mut removed_medic_tail, Some("local".to_string()));
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
    assert_eq!(
        metrics_tail,
        vec![
            OsString::from("metrics"),
            OsString::from("test"),
            OsString::from(INTERNAL_NETWORK_OPTION),
            OsString::from("local")
        ]
    );
    assert_eq!(
        endpoints_tail,
        vec![
            OsString::from("endpoints"),
            OsString::from("test"),
            OsString::from("app"),
            OsString::from(INTERNAL_NETWORK_OPTION),
            OsString::from("local")
        ]
    );
    assert_eq!(
        env_tail,
        vec![
            OsString::from("env"),
            OsString::from("test"),
            OsString::from(INTERNAL_NETWORK_OPTION),
            OsString::from("local")
        ]
    );
    assert_eq!(removed_medic_tail, original_removed_medic_tail);
    assert_eq!(help_tail, vec![OsString::from("help")]);
}

#[test]
fn global_network_is_forwarded_only_to_active_auth_renewal_status() {
    let mut removed_run_once_tail = vec![
        OsString::from("renewal"),
        OsString::from("run-once"),
        OsString::from("downstream"),
    ];
    let mut status_tail = vec![
        OsString::from("renewal"),
        OsString::from("status"),
        OsString::from("downstream"),
        OsString::from("--issuer"),
        OsString::from("rrkah-fqaaa-aaaaa-aaaaq-cai"),
    ];
    let mut removed_provisioner_list_tail = vec![
        OsString::from("renewal"),
        OsString::from("provisioner"),
        OsString::from("list"),
        OsString::from("downstream"),
    ];
    let mut removed_provisioner_disable_tail = vec![
        OsString::from("renewal"),
        OsString::from("provisioner"),
        OsString::from("disable"),
        OsString::from("downstream"),
        OsString::from("rrkah-fqaaa-aaaaa-aaaaq-cai"),
    ];
    let mut help_tail = vec![OsString::from("help")];

    let original_removed_run_once_tail = removed_run_once_tail.clone();
    let original_removed_provisioner_list_tail = removed_provisioner_list_tail.clone();
    let original_removed_provisioner_disable_tail = removed_provisioner_disable_tail.clone();

    apply_global_network(
        "auth",
        &mut removed_run_once_tail,
        Some("fixture".to_string()),
    );
    apply_global_network("auth", &mut status_tail, Some("fixture".to_string()));
    apply_global_network(
        "auth",
        &mut removed_provisioner_list_tail,
        Some("fixture".to_string()),
    );
    apply_global_network(
        "auth",
        &mut removed_provisioner_disable_tail,
        Some("fixture".to_string()),
    );
    apply_global_network("auth", &mut help_tail, Some("fixture".to_string()));

    assert_eq!(removed_run_once_tail, original_removed_run_once_tail);
    assert!(status_tail.ends_with(&[
        OsString::from(INTERNAL_NETWORK_OPTION),
        OsString::from("fixture")
    ]));
    assert_eq!(
        removed_provisioner_list_tail,
        original_removed_provisioner_list_tail
    );
    assert_eq!(
        removed_provisioner_disable_tail,
        original_removed_provisioner_disable_tail
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
            OsString::from("info"),
            OsString::from("medic"),
            OsString::from("test"),
            OsString::from("--icp"),
            OsString::from("icp")
        ]),
        Err(CliError::Usage(_))
    );
}

#[test]
fn rejected_medic_command_forms_exit_as_usage_errors() {
    for raw_args in [
        &["medic", "demo"][..],
        &["medic", "--blob-storage", "backend"],
        &[
            "medic",
            "project",
            "--auth-renewal",
            "rrkah-fqaaa-aaaaa-aaaaq-cai",
        ],
        &["info", "medic", "demo"],
    ] {
        let err =
            run(raw_args.iter().map(OsString::from)).expect_err("rejected medic form should fail");

        assert_eq!(
            cli_error_exit_code(&err),
            2,
            "wrong exit code for {raw_args:?}: {err}"
        );
        assert!(!render_cli_error(&err).is_empty());
    }
}

#[test]
fn rejected_legacy_operator_surfaces_exit_as_usage_errors() {
    for raw_args in [
        &["info", "medic", "demo"][..],
        &["state"],
        &["state", "audit", "root"],
        &["state", "manifest", "root"],
        &["state", "migrate"],
        &["state", "repair"],
        &["state", "explore"],
        &["state", "dump"],
        &["state", "audit", "--format", "json"],
        &["state", "manifest", "--format", "json"],
        &["inspect", "demo-local"],
        &["inspect", "deployment", "demo-local"],
        &["inspect", "deployment", "demo-local", "--all"],
        &["topology", "demo-local"],
        &["runtime", "status", "demo-local"],
    ] {
        let err = run(raw_args.iter().map(OsString::from))
            .expect_err("legacy operator surface should fail");

        assert_eq!(
            cli_error_exit_code(&err),
            2,
            "wrong exit code for {raw_args:?}: {err}"
        );
        assert!(!render_cli_error(&err).is_empty());
    }
}

// Assert that a CLI argv slice returns successfully.
fn assert_run_ok(raw_args: &[&str]) {
    let args = raw_args.iter().map(OsString::from).collect::<Vec<_>>();
    assert!(
        run(args).is_ok(),
        "expected successful run for {raw_args:?}"
    );
}
