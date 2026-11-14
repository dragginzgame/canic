use super::*;
use canic_host::{FleetCanister, FleetManager, FleetOperation, Wallet, WalletBalance};
use clap::Parser as _;

fn empty() -> Fleet {
    Fleet {
        configuration: None,
        started_at_ns: 0,
        finished_at_ns: 0,
        scope: "local deployment journal only",
        canisters: vec![],
        operations: vec![],
        issues: vec![],
        managers: vec![],
        wallet: None,
    }
}

#[test]
fn terminal_preserves_unknowns_network_identity_and_uncertain_effects() {
    let mut snapshot = empty();
    for (network, known) in [("trust-a", true), ("trust-b", false)] {
        snapshot.canisters.push(FleetCanister {
            network_id: network.into(),
            network: "same URL".into(),
            canister: "same-principal".into(),
            observed_at_ns: 0,
            module_hash: known.then(|| "module-hash".into()),
            controllers: known.then(Vec::new),
            observation_error: (!known).then(|| "provider\u{1b}[2J\nforged line".into()),
        });
    }
    snapshot.operations.push(FleetOperation {
        deployment: None,
        operation: "original-operation".into(),
        network_id: "trust-b".into(),
        network: "same URL".into(),
        canister: None,
        approved_module: "approved".into(),
        approved_controller: "controller".into(),
        effects: BTreeMap::from([(
            "create".into(),
            "signed_attempt_retained_outcome_unknown".into(),
        )]),
        receipt_retained: false,
        approval_expired: true,
    });
    snapshot.issues.push("bad record\u{202e}spoof".into());
    let output = render(&snapshot, &["entry:worker".into()], true, false);
    assert_eq!(output.matches("same-principal").count(), 2);
    for expected in [
        "Network identity (diagnostic): trust-a",
        "Network identity (diagnostic): trust-b",
        "verified code/controllers",
        "UNKNOWN metadata",
        "Controlled by: none",
        "approval expired",
        "target: unresolved",
        "signed_attempt_retained_outcome_unknown",
        "canic fleet status --no-config original-operation",
        "PARTIAL INVENTORY",
        "not verified on-chain",
    ] {
        assert!(output.contains(expected), "missing {expected}: {output}");
    }
    assert!(!output.contains('\u{1b}'));
    assert!(!output.contains("\nforged line"));
    assert!(!output.contains('\u{202e}'));
    assert!(output.contains("\\u{1b}[2J\\nforged line"));
}

#[test]
fn terminal_escapes_external_unicode_controls_without_changing_evidence() {
    let mut snapshot = empty();
    let diagnostic =
        "provider café\u{61c}\u{200e}\u{200f}\u{2028}\u{2029}\u{202e}\u{2066}\n\u{1b}[2J";
    snapshot.issues.push(diagnostic.into());
    let before = serde_json::to_value(&snapshot).unwrap();
    let output = render(&snapshot, &[diagnostic.into()], false, false);
    assert!(output.contains(
        "provider café\\u{61c}\\u{200e}\\u{200f}\\u{2028}\\u{2029}\\u{202e}\\u{2066}\\n\\u{1b}[2J"
    ));
    for control in [
        '\u{61c}', '\u{200e}', '\u{200f}', '\u{2028}', '\u{2029}', '\u{202e}', '\u{2066}', '\u{1b}',
    ] {
        assert!(!output.contains(control));
    }
    assert_eq!(serde_json::to_value(&snapshot).unwrap(), before);
    assert_eq!(before["issues"][0], diagnostic);
}

#[test]
fn terminal_distinguishes_unknown_wallet_balance_and_uncertified_manager() {
    let mut snapshot = empty();
    snapshot.managers.push(FleetManager {
        network_id: "mainnet".into(),
        manager: "manager".into(),
        reported: Some(Default::default()),
        observation_error: None,
    });
    snapshot.managers[0].reported.as_mut().unwrap().phase =
        canic_host::ProfileGrowthPhase::CreateIssued;
    snapshot.wallet = Some(Wallet {
        principal: "owner".into(),
        icp_address: "receive-address".into(),
        network: "ic / mainnet",
        certified: false,
        icp: WalletBalance {
            ledger: "icp",
            amount: Some("0".into()),
            display: Some("0.00000000 ICP".into()),
            observed_at_ns: 0,
            observation_error: None,
        },
        cycles: WalletBalance {
            ledger: "cycles",
            amount: None,
            display: None,
            observed_at_ns: 0,
            observation_error: Some("ledger unavailable".into()),
        },
    });
    let output = render(&snapshot, &[], false, false);
    for expected in [
        "No known canisters",
        "uncertified report",
        "no second creation is authorized",
        "0.00000000 ICP",
        "Cycles ledger: UNKNOWN",
        "ledger unavailable",
        "exclude cycles held by canisters",
    ] {
        assert!(output.contains(expected), "missing {expected}: {output}");
    }
}

#[tokio::test(flavor = "current_thread")]
async fn fleet_command_selects_text_or_json_without_a_browser() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .join("target/fleet-terminal-tests");
    std::fs::create_dir_all(&root).unwrap();
    let directory = ic_host_fs::directory::create_unique_directory(
        &root,
        "empty",
        0o700,
        std::num::NonZeroUsize::new(32),
    )
    .unwrap();
    let args = [
        "canic",
        "fleet",
        "show",
        "--no-config",
        "--state-dir",
        directory.to_str().unwrap(),
    ];
    let cli = crate::Cli::try_parse_from(args).unwrap();
    let crate::Output::Text(output) = crate::run(&cli).await.unwrap() else {
        panic!("expected terminal output")
    };
    assert!(output.contains("0 canisters"));
    let cli = crate::Cli::try_parse_from(args.into_iter().chain(["--json", "--color", "always"]))
        .unwrap();
    let crate::Output::Json(output) = crate::run(&cli).await.unwrap() else {
        panic!("expected JSON output")
    };
    assert_eq!(output["canisters"], serde_json::json!([]));
    assert_eq!(
        output["scope"],
        "local deployment journal, configured identities and explicitly selected manager reports"
    );
    assert!(output.get("started_at_ns").is_some());
    assert!(!output.to_string().contains('\u{1b}'));
}

#[test]
fn roles_follow_observed_authority_without_inventing_root_or_crossing_networks() {
    let mut snapshot = empty();
    for (id, network, controllers) in [
        ("manager", "a", vec!["operator"]),
        ("worker", "a", vec!["manager"]),
        ("manager", "b", vec!["other-operator"]),
        ("entry", "a", vec!["operator"]),
        ("entry", "b", vec!["other-operator"]),
    ] {
        snapshot.canisters.push(FleetCanister {
            network_id: network.into(),
            network: "same URL".into(),
            canister: id.into(),
            observed_at_ns: 0,
            module_hash: Some("full-module-hash".into()),
            controllers: Some(controllers.into_iter().map(String::from).collect()),
            observation_error: None,
        });
    }
    snapshot.managers.push(FleetManager {
        network_id: "a".into(),
        manager: "manager".into(),
        reported: Some(Default::default()),
        observation_error: None,
    });
    snapshot.managers[0].reported.as_mut().unwrap().worker = Some("worker".into());
    let links = ["entry:worker".into()];
    let output = render(&snapshot, &links, false, true);
    assert!(output.contains("\x1b[1;35m[CONTROLLER CANISTER]\x1b[0m"));
    assert!(output.contains("\x1b[1;35m[EXTERNAL CONTROLLER]\x1b[0m operator"));
    assert!(output.contains("\x1b[1;36m[MANAGER / selected]\x1b[0m"));
    assert!(output.contains("[WORKER / reported]"));
    assert!(output.contains("[ENTRY / declared]"));
    assert!(!output.contains("ROOT"));
    assert!(!output.contains("full-module-hash"));
    assert!(!output.contains("Network identity (diagnostic):"));
    assert!(output.contains("Network #1: same URL"));
    assert!(output.contains("Network #2: same URL"));
    assert_eq!(
        roles::badges(&snapshot, &snapshot.canisters[2], &links, false),
        "[CANISTER]"
    );
    assert_eq!(
        roles::badges(&snapshot, &snapshot.canisters[4], &links, false),
        "[CANISTER]"
    );
    let plain = render(&snapshot, &links, true, false);
    assert!(!plain.contains('\x1b'));
    assert!(plain.contains("full-module-hash"));
    snapshot.canisters[1].observation_error = Some("observation failed".into());
    assert!(
        !roles::badges(&snapshot, &snapshot.canisters[0], &links, false)
            .contains("CONTROLLER CANISTER")
    );
}

#[test]
fn automatic_colours_respect_redirection_no_color_and_dumb_terminals() {
    use crate::ColorMode;
    assert!(ColorMode::Auto.enabled(true, false, false));
    assert!(!ColorMode::Auto.enabled(false, false, false));
    assert!(!ColorMode::Auto.enabled(true, true, false));
    assert!(!ColorMode::Auto.enabled(true, false, true));
    assert!(ColorMode::Always.enabled(false, true, true));
    assert!(!ColorMode::Never.enabled(true, false, false));
}
