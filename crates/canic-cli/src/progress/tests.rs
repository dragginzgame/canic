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
        "project's config.toml",
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
        assert!(next.contains("--config 'project'\"'\"'s config.toml'"));
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
        "project's config.toml",
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
    assert!(output.contains("--config 'project'\"'\"'s config.toml'"));
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

#[test]
fn app_build_summary_marks_infrastructure_in_canister_column_and_keeps_codec_path_in_json() {
    let mut build = BuiltCanister {
        canister: "user_shard".into(),
        role: None,
        wasm: "artifacts/user_shard.wasm".into(),
        artifact: canic_host::Artifact {
            bytes: 956_693,
            sha256: "fixture digest".into(),
        },
        parent: Some("user_hub".into()),
        modules: vec!["DB", "Auth"],
        build_ms: 2_340,
        client: None,
    };
    let table = built(std::slice::from_ref(&build));
    assert!(!table.contains("PARENT"));
    assert!(table.contains("user_shard (parent: user_hub)"));
    assert!(table.contains("934.27 KiB"));
    assert!(table.contains("DB, Auth"));
    assert!(table.contains("2.34 s"));
    assert!(!table.contains("COMMAND CODEC"));

    let json = serde_json::to_value(&build).unwrap();
    assert_eq!(json["artifact"]["bytes"], 956_693);
    assert_eq!(json["artifact"]["sha256"], "fixture digest");

    build.client = Some("artifacts/user_shard.codec".into());
    let mut hub = build;
    hub.canister = "user_hub".into();
    hub.parent = None;
    let shard = BuiltCanister {
        canister: "user_shard".into(),
        role: None,
        parent: Some("user_hub".into()),
        wasm: "shard.wasm".into(),
        artifact: hub.artifact.clone(),
        modules: vec![],
        build_ms: 0,
        client: None,
    };
    let mut store = BuiltCanister {
        canister: "store".into(),
        role: None,
        parent: Some("user_hub".into()),
        wasm: "artifacts/wasm_store.wasm".into(),
        artifact: hub.artifact.clone(),
        modules: vec![],
        build_ms: 0,
        client: None,
    };
    assert!(!built(std::slice::from_ref(&store)).contains("[WASM_STORE]"));
    for (role, badge) in [("root", "[ROOT]"), ("coordinator", "[COORDINATOR]")] {
        store.role = Some(role.into());
        assert!(
            built(std::slice::from_ref(&store))
                .contains(&format!("store {badge} (parent: user_hub)"))
        );
    }
    store.canister = "artifacts".into();
    store.role = Some("wasm-store".into());
    store.modules = vec!["DB", "Auth"];
    assert_eq!(
        serde_json::to_value(&hub).unwrap()["client"],
        "artifacts/user_shard.codec"
    );
    let table = built(&[shard, hub, store]);
    let row = table
        .lines()
        .find(|row| row.starts_with("├─ artifacts"))
        .unwrap();
    assert!(row.starts_with("├─ artifacts [WASM_STORE]"));
    assert!(row.find("KiB").unwrap() < row.find("DB, Auth").unwrap());
    assert_eq!(row.matches("[WASM_STORE]").count(), 1);
    assert!(table.find("user_hub").unwrap() < table.find("└─ user_shard").unwrap());
    assert!(!table.contains("COMMAND CODEC"));
    assert!(!table.contains("artifacts/user_shard.codec"));
}
