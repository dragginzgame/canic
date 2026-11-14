use super::*;
use std::process::Command;

#[test]
fn slot_summaries_expose_network_and_identity_conflicts_without_erasing_evidence() {
    use canic_host::{
        CanisterConfig, DeploymentName, FleetConfig, FleetOperation, Network, SelectedFleet,
    };
    use std::collections::BTreeMap;

    let network = Network::Local {
        url: "http://localhost:8000".into(),
        root_key: "00".repeat(133),
        routing_canister: "aaaaa-aa".into(),
    };
    let mut snapshot = Fleet {
        configuration: Some(SelectedFleet {
            app: "app".into(),
            fleet: "demo".into(),
            config: FleetConfig {
                network: network.clone(),
                journal: None,
                canisters: BTreeMap::from([("worker".into(), CanisterConfig::default())]),
            },
        }),
        started_at_ns: 0,
        finished_at_ns: 0,
        scope: "test",
        canisters: vec![],
        operations: vec![FleetOperation {
            upgrade: None,
            management: None,
            deployment: Some(DeploymentName {
                app: "app".into(),
                fleet: "demo".into(),
                canister: "worker".into(),
            }),
            operation: "original-operation".into(),
            network_id: network.binding_id().unwrap(),
            network: "local / http://localhost:8000".into(),
            canister: Some("original-principal".into()),
            approved_module: "original-module".into(),
            approved_controller: "original-controller".into(),
            effects: BTreeMap::new(),
            receipt_retained: false,
            approval_expired: true,
        }],
        issues: vec![],
        managers: vec![],
        wallet: None,
    };
    assert_eq!(
        slot_status(&snapshot, "worker"),
        "original-principal (retained deployment identity)"
    );
    // Same endpoint, app, fleet and principal: only the trust anchor changed.
    let Network::Local { root_key, .. } =
        &mut snapshot.configuration.as_mut().unwrap().config.network
    else {
        unreachable!()
    };
    *root_key = "01".repeat(133);
    for configured_id in [None, Some("original-principal".into())] {
        snapshot
            .configuration
            .as_mut()
            .unwrap()
            .config
            .canisters
            .get_mut("worker")
            .unwrap()
            .id = configured_id;
        let before = serde_json::to_value(&snapshot).unwrap();
        assert!(slot_status(&snapshot, "worker").starts_with("Network conflict"));
        for output in [
            render(&snapshot, ""),
            terminal(&snapshot, &[], false, false),
        ] {
            assert!(output.contains("Network conflict"));
            assert!(output.contains("original-principal"));
            assert!(output.contains("original-operation"));
        }
        assert_eq!(serde_json::to_value(&snapshot).unwrap(), before);
    }
    snapshot.configuration.as_mut().unwrap().config.network = network;
    assert_eq!(
        slot_status(&snapshot, "worker"),
        "original-principal (configured identity; metadata below)"
    );
    snapshot
        .configuration
        .as_mut()
        .unwrap()
        .config
        .canisters
        .get_mut("worker")
        .unwrap()
        .id = Some("replacement-principal".into());
    for output in [
        render(&snapshot, ""),
        terminal(&snapshot, &[], false, false),
    ] {
        assert!(output.contains("Identity conflict"));
        assert!(output.contains("original-principal"));
    }
    snapshot.operations[0].canister = None;
    assert!(slot_status(&snapshot, "worker").starts_with("Identity conflict"));
}

#[test]
fn report_escapes_untrusted_text_and_labels_partial_inventory() {
    let mut snapshot = Fleet {
        configuration: None,
        started_at_ns: 0,
        finished_at_ns: 0,
        scope: "local deployment journal only",
        canisters: vec![],
        operations: vec![],
        issues: vec!["<script>alert('unsafe')</script> & \"quoted\"".into()],
        managers: vec![],
        wallet: None,
    };
    snapshot.managers.push(canic_host::FleetManager {
        upgrade: None,
        network_id: "local".into(),
        manager: "manager<script>".into(),
        reported: Some(Default::default()),
        observation_error: Some("<bad provider>".into()),
    });
    let status = snapshot.managers[0].reported.as_mut().unwrap();
    status.phase = canic_host::ManagementPhase::CreateIssued;
    status.attempts = [1, 0, 0];
    snapshot.wallet = Some(canic_host::Wallet {
        principal: "owner<script>".into(),
        icp_address: "address<unsafe>".into(),
        network: "ic / mainnet",
        certified: false,
        icp: canic_host::WalletBalance {
            ledger: "icp",
            amount: Some("510925125".into()),
            display: Some("5.10925125 ICP".into()),
            observed_at_ns: 0,
            observation_error: None,
        },
        cycles: canic_host::WalletBalance {
            ledger: "cycles",
            amount: None,
            display: None,
            observed_at_ns: 0,
            observation_error: Some("<failed ledger>".into()),
        },
    });
    let html = render(&snapshot, "");
    assert!(!html.contains("<script>alert"));
    assert!(html.contains("&lt;script&gt;alert(&#39;unsafe&#39;)"));
    assert!(html.contains("Partial inventory"));
    assert!(html.contains("0 canisters"));
    assert!(html.contains("Regenerate the report to refresh"));
    assert!(html.contains("manager&lt;script&gt;"));
    assert!(html.contains("&lt;bad provider&gt;"));
    assert!(html.contains("No reported principal"));
    assert!(html.contains("No second creation is authorized"));
    assert!(html.contains("Manager report (uncertified)"));
    assert!(html.contains("Uncertified balance queries"));
    assert!(html.contains("owner&lt;script&gt;"));
    assert!(html.contains("address&lt;unsafe&gt;"));
    assert!(html.contains("5.10925125 ICP"));
    assert!(html.contains("<strong>Unknown</strong>"));
    assert!(html.contains("&lt;failed ledger&gt;"));
    assert!(!html.contains("0.000000000000 T cycles"));
}

#[test]
fn declared_routes_require_unambiguous_known_network_and_escape_text() {
    let mut snapshot = Fleet {
        configuration: None,
        started_at_ns: 0,
        finished_at_ns: 0,
        scope: "local deployment journal only",
        canisters: vec![],
        operations: vec![],
        issues: vec![],
        managers: vec![],
        wallet: None,
    };
    let link = "entry<script>:worker".to_owned();
    assert!(declared_routes(&snapshot, std::slice::from_ref(&link)).is_err());
    for canister in ["entry<script>", "worker"] {
        snapshot.canisters.push(canic_host::FleetCanister {
            network_id: "network-a".into(),
            network: "local".into(),
            canister: canister.into(),
            observed_at_ns: 0,
            module_hash: None,
            controllers: None,
            observation_error: Some("unavailable".into()),
        });
    }
    let html = declared_routes(&snapshot, std::slice::from_ref(&link)).unwrap();
    assert!(html.contains("Declared · not verified"));
    assert!(html.contains("Source canister"));
    assert!(html.contains("Destination canister"));
    assert!(!html.contains("Profile"));
    assert!(!html.contains("owns private data"));
    assert!(html.contains("entry&lt;script&gt;"));
    assert!(!html.contains("entry<script>"));
    snapshot.canisters[1].network_id = "network-b".into();
    assert!(declared_routes(&snapshot, std::slice::from_ref(&link)).is_err());
    snapshot.canisters[1].network_id = "network-a".into();
    for canister in ["entry<script>", "worker"] {
        snapshot.canisters.push(canic_host::FleetCanister {
            network_id: "network-b".into(),
            network: "local".into(),
            canister: canister.into(),
            observed_at_ns: 0,
            module_hash: None,
            controllers: None,
            observation_error: None,
        });
    }
    assert!(declared_routes(&snapshot, &[link]).is_err());
}

#[test]
fn browser_handoff_uses_localhost_and_preserves_background_lifetime() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .join("target/fleet-open-tests");
    durable::prepare_private_parents(&root).unwrap();
    let dir = ic_host_fs::directory::create_unique_directory(
        &root,
        "launch",
        0o700,
        NonZeroUsize::new(32),
    )
    .unwrap();
    let report = dir.join("- report ' $(literal).html");
    std::fs::write(&report, "report retained").unwrap();
    let marker = dir.join("opened");
    let mut opener = Command::new("sh");
    opener
        .arg("-c")
        .arg("sleep 0.1; printf '%s' \"$1\" > \"$0\"")
        .arg(&marker);
    let expected = "http://localhost:8787/";
    crate::view::launch(expected, opener).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        if std::fs::read_to_string(&marker).is_ok_and(|value| value == expected) {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "detached opener did not receive the exact report path"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(crate::view::launch(expected, Command::new(dir.join("missing-opener"))).is_err());
    assert_eq!(std::fs::read_to_string(report).unwrap(), "report retained");
}
