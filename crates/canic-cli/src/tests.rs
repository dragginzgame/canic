use super::*;
use crate::cli::{
    globals::{INTERNAL_ENVIRONMENT_OPTION, INTERNAL_ICP_OPTION},
    help::usage,
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

#[test]
fn usage_lists_current_commands_alphabetically() {
    let plain = strip_ansi(&usage());
    let names = plain
        .split_once("\nCommands:\n")
        .expect("top-level commands section")
        .1
        .split_once("\nOptions:\n")
        .expect("top-level options section")
        .0
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .collect::<Vec<_>>();
    let mut sorted = names.clone();
    sorted.sort_unstable();

    assert_eq!(names, sorted);
    assert_eq!(
        names,
        [
            "admission",
            "app",
            "auth",
            "backup",
            "blob-storage",
            "build",
            "cycles",
            "diagnostic",
            "evidence",
            "fleet",
            "info",
            "inspect",
            "medic",
            "network",
            "replica",
            "restore",
            "scaffold",
            "state",
            "status",
            "token",
        ]
    );
    assert!(plain.contains("Usage: canic [OPTIONS] <COMMAND>"));
    assert!(plain.contains("Converge one Fleet from current desired state"));
    assert!(!plain.contains("  deploy"));
    assert!(!plain.contains("  install"));
    assert!(!plain.contains("retained"));
}

#[test]
fn current_command_help_and_versions_return_ok() {
    for args in [
        &["admission", "--help"][..],
        &["app", "--help"],
        &["auth", "--help"],
        &["backup", "--help"],
        &["blob-storage", "--help"],
        &["build", "--help"],
        &["cycles", "--help"],
        &["diagnostic", "--help"],
        &["evidence", "--help"],
        &["fleet", "--help"],
        &["fleet", "ensure", "--help"],
        &["info", "--help"],
        &["inspect", "--help"],
        &["inspect", "canister", "--help"],
        &["inspect", "fleet", "--help"],
        &["medic", "--help"],
        &["medic", "fleet", "--help"],
        &["network", "--help"],
        &["replica", "--help"],
        &["restore", "--help"],
        &["scaffold", "--help"],
        &["state", "--help"],
        &["status", "--help"],
        &["token", "--help"],
    ] {
        assert!(run(args.iter().map(OsString::from)).is_ok(), "{args:?}");
    }
    assert!(run([OsString::from("--version")]).is_ok());
    assert!(run([OsString::from("fleet"), OsString::from("--version")]).is_ok());
}

#[test]
fn global_options_are_forwarded_only_once() {
    let mut tail = vec![OsString::from("ensure"), OsString::from("staging")];
    apply_global_icp("fleet", &mut tail, Some("/tmp/icp".to_string()));
    apply_global_environment("fleet", &mut tail, Some("local".to_string()));
    assert_eq!(
        tail,
        vec![
            OsString::from("ensure"),
            OsString::from("staging"),
            OsString::from(INTERNAL_ICP_OPTION),
            OsString::from("/tmp/icp"),
            OsString::from(INTERNAL_ENVIRONMENT_OPTION),
            OsString::from("local"),
        ]
    );

    apply_global_icp("fleet", &mut tail, Some("/other/icp".to_string()));
    apply_global_environment("fleet", &mut tail, Some("other".to_string()));
    assert_eq!(
        tail.iter()
            .filter(|value| value.as_os_str() == INTERNAL_ICP_OPTION)
            .count(),
        1
    );
    assert_eq!(
        tail.iter()
            .filter(|value| value.as_os_str() == INTERNAL_ENVIRONMENT_OPTION)
            .count(),
        1
    );
}

#[test]
fn current_read_only_commands_receive_global_target_options() {
    for (command, raw_tail) in [
        ("admission", &["status", "staging"][..]),
        ("auth", &["renewal", "status", "staging"][..]),
        ("blob-storage", &["status", "staging", "root"]),
        ("cycles", &["balance"]),
        ("info", &["list", "staging"][..]),
        ("inspect", &["fleet", "staging", "--role", "root"]),
        ("medic", &["fleet", "staging"]),
        ("status", &[][..]),
        ("token", &["balance"]),
    ] {
        let mut tail = raw_tail.iter().map(OsString::from).collect::<Vec<_>>();
        apply_global_icp(command, &mut tail, Some("/tmp/icp".to_string()));
        apply_global_environment(command, &mut tail, Some("local".to_string()));

        assert!(tail.iter().any(|value| value == INTERNAL_ICP_OPTION));
        assert!(
            tail.iter()
                .any(|value| value == INTERNAL_ENVIRONMENT_OPTION)
        );
    }
}

#[cfg(unix)]
#[test]
fn icp_backed_command_rejects_unparseable_icp_before_effects() {
    use std::{fs, os::unix::fs::PermissionsExt};

    let root = TempDir::new("canic-cli-unsupported-icp");
    fs::create_dir_all(&root).expect("create temp dir");
    let icp = root.join("icp");
    fs::write(
        &icp,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'icp development build'; exit 0; fi\necho 'unexpected effect' >&2\nexit 42\n",
    )
    .expect("write fake icp");
    fs::set_permissions(&icp, fs::Permissions::from_mode(0o755)).expect("chmod fake icp");

    let error = run([
        OsString::from("--icp"),
        icp.into_os_string(),
        OsString::from("replica"),
        OsString::from("status"),
    ])
    .expect_err("unsupported icp rejected");
    assert!(error.to_string().contains("unsupported icp-cli version"));
    assert!(!error.to_string().contains("unexpected effect"));
}

#[test]
fn state_report_failure_remains_silent_and_nonzero() {
    let error = CliError::State(state::StateCommandError::AuditFailed);
    assert_eq!(render_cli_error(&error), "");
    assert_eq!(cli_error_exit_code(&error), 1);
}
