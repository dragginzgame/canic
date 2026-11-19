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
        metrics: vec![],
        stores: vec![],
        wallet: None,
    }
}

#[test]
fn child_reports_keep_slots_names_and_ambiguity_distinct() {
    use canic_management_protocol::{Approval, children::ChildId};
    let mut snapshot = empty();
    for slot in [None, Some(ChildId::SECOND)] {
        snapshot.managers.push(FleetManager {
            slot,
            upgrade: None,
            network_id: "a".into(),
            manager: "root".into(),
            reported: Some(canic_management_protocol::Status {
                worker: Some("child<untrusted>".into()),
                ..Default::default()
            }),
            observation_error: None,
        });
    }
    let check = |snapshot: &Fleet, label: &str| {
        let before = serde_json::to_value(snapshot).unwrap();
        let text = render(snapshot, &[], false, false);
        let html = super::super::render(snapshot, "");
        assert!(text.contains(&format!(" / {label}")), "{text}");
        assert!(html.contains(&format!(
            "Child: <strong>{}</strong>",
            super::super::escape(label)
        )));
        assert!(!html.contains("child<untrusted>"));
        assert_eq!(serde_json::to_value(snapshot).unwrap(), before);
    };
    check(&snapshot, "slot 1 · child<untrusted>");
    check(&snapshot, "slot 2 · child<untrusted>");
    let operation = |canister: &str| FleetOperation {
        store_root: None,
        slot: Some(ChildId::FIRST),
        upgrade: None,
        management: Some(Approval {
            manager: "root".into(),
            module_hash: [0; 32],
            creation_cycles: 1,
            execute_at_ns: 1,
            expires_at_ns: 2,
        }),
        deployment: Some(canic_host::DeploymentName {
            fleet: "fleet".into(),
            app: "app".into(),
            canister: canister.into(),
        }),
        operation: "operation".into(),
        network_id: "a".into(),
        network: "local".into(),
        canister: None,
        approved_module: String::new(),
        approved_controller: "root".into(),
        effects: BTreeMap::new(),
        receipt_retained: false,
        approval_expired: false,
    };
    snapshot.operations = vec![operation("named"), operation("named")];
    // Legacy None and explicit First denote the same physical slot.
    snapshot.operations[0].slot = None;
    check(&snapshot, "slot 1 · named");
    check(&snapshot, "slot 2 · child<untrusted>");
    snapshot.operations.push(operation("another-name"));
    check(&snapshot, "slot 1 · child<untrusted>");
    snapshot.operations.reverse();
    check(&snapshot, "slot 1 · child<untrusted>");
    snapshot.operations[0].network_id = "other-binding".into();
    check(&snapshot, "slot 1 · named");
    snapshot.operations[0].network_id = "a".into();
    snapshot.operations[0].management.as_mut().unwrap().manager = "another-root".into();
    check(&snapshot, "slot 1 · named");
    snapshot.managers[1].reported = None;
    snapshot.managers[1].observation_error = Some("unavailable".into());
    assert!(render(&snapshot, &[], false, false).contains(" / slot 2 — UNKNOWN"));
}

#[test]
fn metrics_details_keep_unknown_and_clipped_counts_separate_from_outcomes() {
    let mut snapshot = empty();
    let counter = serde_json::json!({"total":u128::MAX.to_string(), "overflowed":true});
    snapshot.metrics.push(canic_host::FleetMetrics {
        network_id: "network".into(),
        canister: "root\u{1b}[31m".into(),
        reported: Some(serde_json::from_value(serde_json::json!({
            "observed_at_ns":100,
            "timers":{
                "started_at_version":7, "started_at_ns":50,
                "accepted":counter, "completed":counter, "abandoned":counter,
                "expected_failures":counter, "deferred_submissions":counter, "provider_faults":counter,
            }
        })).unwrap()),
        observation_error: None,
    });
    snapshot.metrics.push(canic_host::FleetMetrics {
        network_id: "network".into(),
        canister: "old-root".into(),
        reported: None,
        observation_error: Some("metrics method unavailable".into()),
    });
    let detail = render(&snapshot, &[], true, false);
    assert!(detail.contains("Timer metrics (uncertified)"));
    assert!(detail.contains(&u128::MAX.to_string()));
    assert!(detail.contains("clipped"));
    assert!(detail.contains("UNKNOWN: metrics method unavailable"));
    assert!(detail.contains("counters do not prove operation outcomes"));
    assert!(!detail.contains('\u{1b}'));
    assert!(!render(&snapshot, &[], false, false).contains("Timer metrics"));
}

#[test]
fn store_maintenance_reports_keep_uncertainty_activation_and_historical_completion_distinct() {
    use canic_management_protocol::store_maintenance::{Phase, Status};
    let mut snapshot = empty();
    let status: Status = serde_json::from_value(serde_json::json!({
        "approval": { "store": "store", "previous_module": vec![1; 32], "module_hash": vec![2; 32],
            "attempt": { "sequence": 7, "operation": vec![3; 32] }, "expires_at_ns": 10_000_000_000_u64 },
        "operator": "operator", "phase": "Stopped", "stop_request": null, "failure": null,
        "preservation": null, "installation": null
    })).unwrap();
    snapshot.stores.push(canic_host::FleetStoreMaintenance {
        network_id: "network-a".into(),
        root: "root<script>\x1b[2J".into(),
        store: "store<script>\x1b[2J".into(),
        reported: Some(status),
        observation_error: None,
    });
    for (phase, wording) in [
        (Phase::Stopped, "stopped; replacement not installed"),
        (Phase::InstallIssued, "installation issued; outcome pending"),
        (Phase::Verified, "verified; awaiting explicit activation"),
        (Phase::Ready, "maintenance completed (historical)"),
    ] {
        snapshot.stores[0].reported.as_mut().unwrap().phase = phase;
        let concise = render(&snapshot, &[], false, false);
        assert!(
            concise.contains(&format!("attempt 7 — {wording}")),
            "{concise}"
        );
        assert!(!concise.contains("history limit"));
        assert!(!concise.contains("Dispatch deadline"));
        assert!(!concise.contains('\x1b'));
        let detailed = render(&snapshot, &[], true, false);
        assert!(detailed.contains("Qualified history limit: 8 retained attempts"));
        assert!(detailed.contains("Before replacement: not retained"));
        let html = super::super::render(&snapshot, "");
        assert!(html.contains(&format!("attempt 7 — {wording}")));
        assert!(!html.contains("store<script>"));
        assert!(!html.contains("root<script>"));
    }
    snapshot.stores[0]
        .reported
        .as_mut()
        .unwrap()
        .approval
        .attempt
        .sequence = 8;
    assert!(
        render(&snapshot, &[], false, false)
            .contains("attempt 8 — maintenance completed (historical)")
    );
    snapshot.stores[0].reported = None;
    assert!(render(&snapshot, &[], false, false).contains("no retained maintenance attempt"));
    assert!(!render(&snapshot, &[], false, false).contains("attempt 0"));
    snapshot.stores[0].observation_error = Some("missing method <unsafe>\x1b[2J".into());
    let text = render(&snapshot, &[], false, false);
    assert!(text.contains("UNKNOWN"));
    assert!(!text.contains("no retained maintenance attempt"));
    assert!(!text.contains('\x1b'));
    assert!(super::super::render(&snapshot, "").contains("missing method &lt;unsafe&gt;"));
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
        store_root: None,
        slot: None,
        upgrade: None,
        management: None,
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
    assert_eq!(
        output
            .lines()
            .filter(|line| line.starts_with('—') && line.contains("same-principal"))
            .count(),
        2
    );
    for expected in [
        "Network identity (diagnostic): trust-a",
        "Network identity (diagnostic): trust-b",
        "verified",
        "UNKNOWN",
        "none",
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
        slot: None,
        upgrade: None,
        network_id: "mainnet".into(),
        manager: "manager".into(),
        reported: Some(Default::default()),
        observation_error: None,
    });
    snapshot.managers[0].reported.as_mut().unwrap().phase =
        canic_host::ManagementPhase::CreateIssued;
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
        "Manager report (uncertified",
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
    let directory = crate::tests::directory(&root, "empty").unwrap();
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
        "local deployment journal, configured identities, planned Roots and selected manager reports"
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
        slot: None,
        upgrade: None,
        network_id: "a".into(),
        manager: "manager".into(),
        reported: Some(Default::default()),
        observation_error: None,
    });
    snapshot.managers[0].reported.as_mut().unwrap().worker = Some("worker".into());
    let links = ["entry:worker".into()];
    let output = render(&snapshot, &links, false, true);
    assert!(
        output
            .lines()
            .any(|line| line.starts_with("\x1b[35m") && line.contains("manager"))
    );
    assert!(output.contains("external:1 = operator"));
    assert!(output.contains("worker"));
    assert!(output.contains("entry"));
    assert!(!output.contains("ROOT"));
    assert!(!output.contains("full-module-hash"));
    assert!(!output.contains("Network identity (diagnostic):"));
    assert!(output.contains("Network #1: same URL"));
    assert!(output.contains("Network #2: same URL"));
    assert_eq!(roles::role(&snapshot, &snapshot.canisters[2], &links), "—");
    assert_eq!(roles::role(&snapshot, &snapshot.canisters[4], &links), "—");
    let plain = render(&snapshot, &links, true, false);
    assert!(!plain.contains('\x1b'));
    assert!(plain.contains("full-module-hash"));
    snapshot.canisters[1].observation_error = Some("observation failed".into());
    assert!(!roles::is_controller(&snapshot, &snapshot.canisters[0]));
    assert_eq!(
        roles::role(&snapshot, &snapshot.canisters[0], &links),
        "manager"
    );
}

#[test]
fn automatic_colours_respect_redirection_no_color_and_dumb_terminals() {
    use crate::args::ColorMode;
    assert!(ColorMode::Auto.enabled(true, false, false));
    assert!(!ColorMode::Auto.enabled(false, false, false));
    assert!(!ColorMode::Auto.enabled(true, true, false));
    assert!(!ColorMode::Auto.enabled(true, false, true));
    assert!(ColorMode::Always.enabled(false, true, true));
    assert!(!ColorMode::Never.enabled(true, false, false));
}

#[test]
fn compact_fleet_names_each_identity_once_and_keeps_details_opt_in() {
    use canic_host::{CanisterConfig, FleetConfig, Network, SelectedFleet};
    let mut snapshot = empty();
    let network = Network::Mainnet.binding_id().unwrap();
    let declarations = [
        ("entry", "bisqm-biaaa-aaaas-qhfeq-cai"),
        ("managed-worker", "rn7lo-3aaaa-aaaad-qmouq-cai"),
        ("manager", "rk6n2-wyaaa-aaaad-qmoua-cai"),
        ("shared-worker", "4pkhu-oaaaa-aaaai-ra2ta-cai"),
    ];
    let operator = "o5trf-oqyg7-cawjp-xs4pw-aomb3-iwki5-hyezf-qahfz-j3ffd-jh4fc-oqe";
    snapshot.configuration = Some(SelectedFleet {
        app: "profile".into(),
        fleet: "mainnet".into(),
        config: FleetConfig {
            receiver_policy: None,
            subnet_child_limit: None,
            instance_pool: None,
            shard_pool: None,
            auth: Vec::new(),
            network: Network::Mainnet,
            journal: None,
            canisters: declarations
                .iter()
                .map(|(name, id)| {
                    (
                        (*name).into(),
                        CanisterConfig {
                            wasm: None,
                            id: Some((*id).into()),
                            profile_manager: *name == "manager",
                            role: Some(
                                if *name == "entry" {
                                    "entry"
                                } else if *name == "manager" {
                                    "manager"
                                } else {
                                    "worker"
                                }
                                .into(),
                            ),
                            parent: (*name == "managed-worker").then(|| "manager".into()),
                        },
                    )
                })
                .collect(),
        },
    });
    for (name, id) in declarations {
        snapshot.canisters.push(FleetCanister {
            network_id: network.clone(),
            network: "ic / mainnet".into(),
            canister: id.into(),
            observed_at_ns: 0,
            module_hash: Some("full-module-hash".into()),
            controllers: Some(vec![
                if name == "managed-worker" {
                    declarations[2].1
                } else {
                    operator
                }
                .into(),
            ]),
            observation_error: None,
        });
    }
    snapshot.managers.push(FleetManager {
        slot: None,
        upgrade: None,
        network_id: network,
        manager: declarations[2].1.into(),
        reported: Some(Default::default()),
        observation_error: None,
    });
    let status = snapshot.managers[0].reported.as_mut().unwrap();
    status.phase = canic_host::ManagementPhase::Ready;
    status.worker = Some(declarations[1].1.into());
    let before = serde_json::to_value(&snapshot).unwrap();
    let plain = render(&snapshot, &[], false, false);
    println!("{plain}");
    assert!(plain.starts_with("profile / mainnet · IC mainnet\n"));
    assert!(plain.lines().count() <= 15, "{plain}");
    for (name, id) in declarations {
        assert_eq!(plain.matches(id).count(), 1);
        assert!(
            plain
                .lines()
                .any(|line| line.contains(name) && line.contains(id))
        );
    }
    assert_eq!(plain.matches(operator).count(), 1);
    let worker = plain
        .lines()
        .find(|line| line.starts_with("└─ managed-worker"))
        .unwrap();
    assert!(worker.contains("manager"));
    assert_eq!(
        roles::role(&snapshot, &snapshot.canisters[2], &[]),
        "manager"
    );
    for noisy in [
        "[",
        "Use --",
        "Snapshot:",
        "full-module-hash",
        "attempts:",
        "metadata below",
        "Controlled by:",
    ] {
        assert!(!plain.contains(noisy), "{noisy}: {plain}");
    }
    let colored = render(&snapshot, &[], false, true);
    let stripped = colored
        .replace("\x1b[1m", "")
        .replace("\x1b[35m", "")
        .replace("\x1b[0m", "");
    assert_eq!(stripped, plain);
    assert!(
        colored
            .lines()
            .any(|line| line.starts_with("\x1b[35mmanager"))
    );
    let detailed = render(&snapshot, &[], true, false);
    for expected in [
        "full-module-hash",
        "Snapshot:",
        "attempts:",
        "Network identity",
    ] {
        assert!(detailed.contains(expected));
    }
    assert_eq!(serde_json::to_value(&snapshot).unwrap(), before);
    // Equal principals on another trust binding must never borrow these names.
    assert_eq!(
        roles::name(&snapshot, "another-binding", declarations[2].1),
        None
    );
    let html = super::super::render(&snapshot, "");
    assert!(html.contains("└─ managed-worker"));
    assert!(html.contains("role: manager"));
    // A routing link does not create a parent relationship.
    let routed = render(
        &snapshot,
        &[format!("{}:{}", declarations[0].1, declarations[3].1)],
        false,
        false,
    );
    assert!(routed.lines().any(|line| line.starts_with("shared-worker")));
    snapshot.canisters[1].observation_error = Some("provider unavailable".into());
    let partial = render(&snapshot, &[], false, false);
    assert!(partial.contains("UNKNOWN"));
    assert!(partial.contains("provider unavailable"));
    let mut missing = snapshot.configuration.as_ref().unwrap().clone();
    missing.config.canisters.get_mut("manager").unwrap().id = None;
    snapshot.configuration = Some(missing);
    let orphan = render(&snapshot, &[], false, false);
    assert!(orphan.contains("unresolved parent"));
    assert!(
        orphan
            .lines()
            .any(|line| line.starts_with("managed-worker"))
    );
    assert_eq!(
        roles::role(&snapshot, &snapshot.canisters[2], &[]),
        "manager"
    );
    let config = &mut snapshot.configuration.as_mut().unwrap().config;
    let duplicate = config.canisters["managed-worker"].clone();
    config.canisters.insert("ambiguous-child".into(), duplicate);
    let ambiguous = render(&snapshot, &[], false, false);
    assert!(
        ambiguous
            .lines()
            .any(|line| line.starts_with('—') && line.contains(declarations[1].1))
    );
    assert_eq!(
        roles::name(
            &snapshot,
            &snapshot.canisters[1].network_id,
            declarations[1].1
        ),
        None,
        "an ambiguous snapshot must not select one declaration arbitrarily"
    );
}

#[test]
fn upgrade_progress_stays_uncertified_and_unknowns_do_not_hide_creation() {
    let mut snapshot = empty();
    snapshot.managers.push(FleetManager {
        slot: None,
        network_id: "mainnet".into(),
        manager: "root".into(),
        reported: Some(Default::default()),
        observation_error: None,
        upgrade: Some(canic_host::FleetUpgrade {
            operation: "upgrade<id>\u{1b}[2J".into(),
            reported: Some(Default::default()),
            observation_error: None,
        }),
    });
    snapshot.managers[0].reported.as_mut().unwrap().phase = canic_host::ManagementPhase::Ready;
    assert!(render(&snapshot, &[], false, false).contains("Upgrade: not admitted"));
    let upgrade = snapshot.managers[0].upgrade.as_mut().unwrap();
    upgrade.reported = Some(
        serde_json::from_value(serde_json::json!({
            "approval": {"manager":"root", "child":"child", "previous_module":vec![0;32],
                "module_hash":vec![1;32], "execute_at_ns":1, "expires_at_ns":2},
            "phase":"Ready", "attempts":[1,1], "requests":[null,null],
            "previous_version":1, "observed_version":2, "failure":null
        }))
        .unwrap(),
    );
    let before = serde_json::to_value(&snapshot).unwrap();
    let text = render(&snapshot, &[], true, false);
    let html = super::super::render(&snapshot, "");
    for expected in ["report (uncertified)", "creation Ready", "Upgrade: Ready"] {
        assert!(text.contains(expected), "{text}");
    }
    assert!(html.contains("Upgrade phase: <strong>Ready</strong>"));
    assert!(html.contains("upgrade&lt;id&gt;"));
    assert!(!text.contains('\u{1b}'));
    assert!(!text.contains("Receipt: retained"));
    assert_eq!(serde_json::to_value(&snapshot).unwrap(), before);
    let upgrade = snapshot.managers[0].upgrade.as_mut().unwrap();
    upgrade.reported = None;
    upgrade.observation_error = Some("<provider>\u{1b}[2J\nforged".into());
    let text = render(&snapshot, &[], false, false);
    let html = super::super::render(&snapshot, "");
    assert!(text.contains("creation Ready"));
    assert!(text.contains("Upgrade: UNKNOWN"));
    assert!(!text.contains('\u{1b}'));
    assert!(!text.contains("\nforged"));
    assert!(html.contains("Upgrade phase: <strong>UNKNOWN</strong>"));
    assert!(!html.contains("<provider>"));
}
