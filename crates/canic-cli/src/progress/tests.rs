use super::*;
use clap::Parser as _;

#[test]
fn upgrade_follow_up_preserves_authority_and_selects_recovery_from_retained_evidence() {
    use canic_management_protocol::{
        Failure,
        upgrade::{Phase, Status},
    };
    let parsed = crate::args::Cli::try_parse_from([
        "canic",
        "fleet",
        "apply",
        "original",
        "--config",
        "project's config.json",
        "--app",
        "profiles",
        "--fleet",
        "local",
        "--state-dir",
        "journal path",
        "--identity",
        "profile operator",
        "--icp-bin",
        "signer path",
    ])
    .unwrap();
    let crate::args::Commands::Fleet(command) = parsed.command else {
        panic!("fleet")
    };
    for (phase, failure, action) in [
        (Phase::Scheduled, None, "apply"),
        (Phase::Checked, None, "apply"),
        (Phase::Uploaded, None, "apply"),
        (Phase::Installed, None, "apply"),
        (Phase::Verified, None, "apply"),
        (Phase::Ready, None, "apply"),
        (Phase::UploadIssued, None, "reconcile"),
        (Phase::InstallIssued, None, "reconcile"),
        (
            Phase::InstallIssued,
            Some(Failure::Rejected(5)),
            "reconcile",
        ),
        (Phase::Verified, Some(Failure::Mismatch), "reconcile"),
        (Phase::Expired, Some(Failure::Uncertain), "status"),
        (Phase::ReserveBlocked, None, "status"),
        (Phase::Empty, None, "status"),
    ] {
        let outcome = ApplyOutcome::UpgradePending {
            maintenance: None,
            operation: "original".into(),
            root: "root".into(),
            upgrade: Box::new(Status {
                phase,
                failure,
                ..Default::default()
            }),
        };
        let before = serde_json::to_value(&outcome).unwrap();
        let output = applied(&outcome, &command);
        let next = output.split("\nNext:\n").nth(1).unwrap();
        assert!(
            next.starts_with(&format!("canic fleet {action} original ")),
            "{output}"
        );
        assert!(next.contains("--config 'project'\"'\"'s config.json'"));
        assert!(next.contains("--state-dir 'journal path' --app profiles --fleet local"));
        assert_eq!(
            next.contains("--identity 'profile operator' --icp-bin 'signer path'"),
            action != "status"
        );
        assert_eq!(serde_json::to_value(&outcome).unwrap(), before);
        assert!(output.contains("completion not yet retained"));
        if matches!(phase, Phase::UploadIssued | Phase::InstallIssued) {
            assert!(output.contains("does not authorize another installation"));
        }
    }
}

#[test]
fn follow_up_retains_scope_and_quotes_shell_input() {
    let cli = crate::args::Cli::try_parse_from([
        "canic",
        "fleet",
        "plan",
        "--config",
        "project's config.json",
        "--app",
        "demo",
        "--fleet",
        "mainnet",
        "--state-dir",
        "journal path",
        "--identity",
        "operator",
        "--icp-bin",
        "signer path",
    ])
    .unwrap();
    let crate::args::Commands::Fleet(command) = cli.command else {
        panic!("fleet")
    };
    let output = next(&command, "digest", "apply", Some("operator"));
    assert!(output.contains("--config 'project'\"'\"'s config.json'"));
    assert!(output.contains("--state-dir 'journal path' --app demo --fleet mainnet"));
    assert!(output.contains("--identity operator --icp-bin 'signer path'"));
    assert_eq!(quote("$(touch nope)"), "'$(touch nope)'");
}

#[test]
fn planning_requires_exactly_one_controller_source() {
    let base = ["canic", "fleet", "plan"];
    for args in [
        vec![],
        vec!["--identity", "one", "--controller", "2vxsx-fae"],
        vec!["--controller", "2vxsx-fae", "--icp-bin", "unused"],
    ] {
        assert!(crate::args::Cli::try_parse_from(base.into_iter().chain(args)).is_err());
    }
    for args in [vec!["--identity", "one"], vec!["--controller", "2vxsx-fae"]] {
        assert!(crate::args::Cli::try_parse_from(base.into_iter().chain(args)).is_ok());
    }
}
