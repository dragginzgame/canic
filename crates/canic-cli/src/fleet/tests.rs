use super::*;
use std::process::Command;

#[test]
fn report_escapes_untrusted_text_and_labels_partial_inventory() {
    let mut snapshot = Fleet {
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
        network_id: "local".into(),
        manager: "manager<script>".into(),
        reported: Some(Default::default()),
        observation_error: Some("<bad provider>".into()),
    });
    let status = snapshot.managers[0].reported.as_mut().unwrap();
    status.phase = canic_host::ProfileGrowthPhase::CreateIssued;
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
    assert!(html.contains("No retained principal"));
    assert!(html.contains("No second creation is authorized"));
    assert!(html.contains("Uncertified manager report"));
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
