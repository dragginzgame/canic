use super::*;
use crate::args::Cli;
use canic_host::placements::PlacementSlot;
use clap::Parser as _;

#[test]
fn placements_command_selects_config_only() {
    assert!(
        Cli::try_parse_from([
            "canic",
            "fleet",
            "placements",
            "--app",
            "app",
            "--fleet",
            "ic"
        ])
        .is_ok()
    );
    for forbidden in [
        "--state-dir",
        "--identity",
        "--no-config",
        "--watch",
        "--serve",
    ] {
        assert!(Cli::try_parse_from(["canic", "fleet", "placements", forbidden]).is_err());
    }
}

#[test]
fn placements_unbound_output_is_explicit_and_escaped() {
    let report = FleetPlacements {
        app: "app\u{1b}[31m".into(),
        fleet: "mainnet".into(),
        observation: None,
        slots: vec![PlacementSlot {
            name: "worker".into(),
            role: Some("worker".into()),
            parent: Some("root".into()),
            canister_id: None,
            observation_index: None,
        }],
    };
    let text = render(&report);
    assert!(!text.contains('\u{1b}'));
    assert!(text.contains("unbound") && text.contains("not attempted") && text.contains("| SLOT"));
    assert!(matches!(output(&report, false), Output::Partial(_)));
    let Output::Partial(inner) = output(&report, true) else {
        panic!("partial")
    };
    let Output::Json(value) = *inner else {
        panic!("json")
    };
    assert!(value["observation"].is_null());
    assert_eq!(value["app"], "app\u{1b}[31m");
}

#[tokio::test]
async fn placements_unbound_config_never_resolves_or_writes_journals() {
    let parent =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/placements-tests");
    std::fs::create_dir_all(&parent).unwrap();
    let root = ic_host_fs::directory::create_unique_directory(
        &parent,
        "unbound",
        0o700,
        std::num::NonZeroUsize::new(32),
    )
    .unwrap();
    let path = root.join("canic.json");
    let raw = r#"{"apps":{"app":{"canisters":{"root":{"role":"root"}},"fleets":{"mainnet":{"network":{"kind":"Mainnet"},"journal":"must-not-exist","canisters":{}}}}}}"#;
    std::fs::write(&path, raw).unwrap();
    let cli = Cli::try_parse_from([
        "canic",
        "--json",
        "fleet",
        "placements",
        "--config",
        path.to_str().unwrap(),
    ])
    .unwrap();
    let Output::Partial(inner) = crate::run(&cli).await.unwrap() else {
        panic!("partial output")
    };
    let Output::Json(value) = *inner else {
        panic!("JSON output")
    };
    assert!(value["observation"].is_null());
    assert_eq!(value["slots"][0]["name"], "root");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), raw);
    assert!(!root.join("must-not-exist").exists());
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
}

#[test]
fn placements_mixed_evidence_keeps_unknowns_out_of_observed_subnet_counts() {
    use ic_topology::placement::collection::{CanisterPlacement, PlacementCollectionReport};
    use ic_topology::placement::{PlacementError, PlacementEvidence, PlacementReport};
    let canister = "ryjl3-tyaaa-aaaaa-aaaba-cai".parse().unwrap();
    let subnet = "tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe"
        .parse()
        .unwrap();
    let read = || PlacementOutcome::Read {
        report: PlacementReport {
            schema: 1,
            canister_id: canister,
            subnet_id: subnet,
            evidence: PlacementEvidence {
                endpoint: "https://ic0.app",
                source_canister: "rwlgt-iiaaa-aaaaa-aaaaa-cai",
                source_method: "get_subnet_for_canister",
                observed_at_unix_seconds: 123,
                query_signature_verified: true,
                result_certified: false,
                registry_version: None,
                selected_lookup_complete: true,
            },
        },
    };
    let outcomes = vec![
        read(),
        read(),
        PlacementOutcome::Failed {
            error: PlacementError::RegistryRejected("raw\n\u{1b}[31m".into()),
        },
        PlacementOutcome::OverallDeadline,
        PlacementOutcome::NotStarted,
    ];
    let mut slots: Vec<_> = (0..5)
        .map(|i| PlacementSlot {
            name: format!("slot{i}"),
            role: None,
            parent: None,
            canister_id: Some(canister.to_string()),
            observation_index: Some(i),
        })
        .collect();
    slots.push(PlacementSlot {
        name: "unbound".into(),
        role: None,
        parent: None,
        canister_id: None,
        observation_index: None,
    });
    let mut report = FleetPlacements {
        app: "app".into(),
        fleet: "mainnet".into(),
        slots,
        observation: Some(PlacementCollectionReport {
            schema: 1,
            atomic_snapshot: false,
            started_at_unix_seconds: 122,
            elapsed: std::time::Duration::from_secs(60),
            collection_complete: false,
            overall_deadline_reached: true,
            canisters: outcomes
                .into_iter()
                .map(|outcome| CanisterPlacement {
                    canister_id: canister,
                    outcome,
                })
                .collect(),
        }),
    };
    let text = render(&report);
    assert!(text.contains("2 read / 5 configured | 3 unresolved | 1 unbound slots"));
    assert!(text.contains("Observed subnets: 1 (successful reads only)"));
    assert!(text.contains("collection deadline") && text.contains("not started"));
    assert!(!text.contains('\u{1b}'));
    let Output::Partial(inner) = output(&report, true) else {
        panic!("partial")
    };
    let Output::Json(value) = *inner else {
        panic!("JSON")
    };
    assert_eq!(
        value["observation"]["canisters"][2]["outcome"]["error"]["reason"],
        "raw\n\u{1b}[31m"
    );
    assert_eq!(
        value["observation"]["canisters"][0]["outcome"]["report"]["evidence"]["observed_at_unix_seconds"],
        123
    );
    assert_eq!(
        value["observation"]["canisters"][3]["outcome"]["status"],
        "overall_deadline"
    );
    report.slots.truncate(2);
    let observation = report.observation.as_mut().unwrap();
    observation.canisters.truncate(2);
    observation.collection_complete = true;
    observation.overall_deadline_reached = false;
    assert!(matches!(output(&report, true), Output::Json(_)));
    assert!(matches!(output(&report, false), Output::Text(_)));
    assert!(render(&report).contains("2 read / 2 configured | 0 unresolved | 0 unbound slots"));
}
