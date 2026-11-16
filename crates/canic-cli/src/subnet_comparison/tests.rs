use super::*;
use crate::args::Cli;
use clap::Parser as _;
use ic_topology::subnet::{
    compare::{Candidate, DistributionOutcome, SubnetComparisonReport},
    distribution::DistributionError,
    metrics::{MetricsOutcome, SubnetMetricsError},
};
use std::time::Duration;

fn fixture() -> FleetSubnetComparison {
    let candidate = |id: &str| Candidate {
        subnet_id: id.parse().unwrap(),
        distribution: DistributionOutcome::Failed {
            error: DistributionError::InitialDeadline,
        },
        metrics: MetricsOutcome::Failed {
            error: SubnetMetricsError::Read(ic_topology::ReadError::Deadline),
        },
    };
    FleetSubnetComparison {
        app: "app\u{1b}[31m".into(),
        fleet: "mainnet".into(),
        observation: SubnetComparisonReport {
            schema: 1,
            left: candidate("aaaaa-aa"),
            right: candidate("2vxsx-fae"),
            started_at_unix_seconds: 1,
            elapsed: Duration::from_secs(5),
            collection_complete: false,
            atomic_snapshot: false,
            registry_versions_agree: None,
        },
    }
}

#[test]
fn subnet_comparison_cli_scopes_explicit_candidates() {
    assert!(
        Cli::try_parse_from([
            "canic",
            "fleet",
            "compare-subnets",
            "aaaaa-aa",
            "2vxsx-fae",
            "--app",
            "app",
            "--fleet",
            "ic",
            "--json"
        ])
        .is_ok()
    );
    assert!(Cli::try_parse_from(["canic", "fleet", "compare-subnets", "aaaaa-aa"]).is_err());
    for forbidden in [
        "--state-dir",
        "--identity",
        "--no-config",
        "--watch",
        "--serve",
    ] {
        assert!(
            Cli::try_parse_from([
                "canic",
                "fleet",
                "compare-subnets",
                "aaaaa-aa",
                "2vxsx-fae",
                forbidden
            ])
            .is_err()
        );
    }
}

#[test]
fn subnet_comparison_missing_and_zero_metrics_remain_distinct() {
    use ic_topology::subnet::metrics::{SubnetMetricsEvidence, SubnetMetricsReport};
    let mut report = fixture();
    assert!(render(&report).contains("Infrastructure overlap unavailable"));
    report.observation.left.metrics = MetricsOutcome::Read {
        report: SubnetMetricsReport {
            schema: 1,
            subnet_id: report.observation.left.subnet_id,
            num_canisters: 0,
            canister_state_bytes: u64::MAX,
            consumed_cycles_total: u128::MAX,
            update_transactions_total: 0,
            evidence: SubnetMetricsEvidence {
                endpoint: "https://ic0.app",
                source_path: "left".into(),
                state_tree_path: "metrics".into(),
                observed_at_unix_seconds: 123,
                state_certificate_verified: true,
            },
        },
    };
    let text = render(&report);
    assert!(text.contains(&u64::MAX.to_string()));
    assert!(text.contains("unavailable") && text.contains("| OBSERVATION"));
    assert!(!text.contains('\u{1b}'));
    assert!(matches!(output(&report, false), Output::Partial(_)));
    let Output::Partial(inner) = output(&report, true) else {
        panic!("partial")
    };
    let Output::Json(value) = *inner else {
        panic!("JSON")
    };
    assert_eq!(
        value["observation"]["left"]["metrics"]["report"]["num_canisters"],
        0
    );
    assert_eq!(
        value["observation"]["left"]["metrics"]["report"]["consumed_cycles_total"],
        u128::MAX.to_string()
    );
    assert_eq!(value["observation"]["right"]["metrics"]["status"], "failed");
    assert!(value["observation"]["infrastructure_overlap"].is_null());
    assert_eq!(value["app"], report.app);
}

#[tokio::test]
async fn subnet_comparison_duplicate_is_refused_without_journal_access() {
    let parent = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/subnet-comparison-tests");
    std::fs::create_dir_all(&parent).unwrap();
    let root = crate::tests::directory(&parent, "duplicate").unwrap();
    let path = root.join("canic.json");
    let raw = r#"{"apps":{"app":{"canisters":{"root":{"role":"root"}},"fleets":{"mainnet":{"network":{"kind":"Mainnet"},"journal":"must-not-exist","canisters":{}}}}}}"#;
    std::fs::write(&path, raw).unwrap();
    let cli = Cli::try_parse_from([
        "canic",
        "fleet",
        "compare-subnets",
        "aaaaa-aa",
        "aaaaa-aa",
        "--config",
        path.to_str().unwrap(),
    ])
    .unwrap();
    assert!(crate::run(&cli).await.is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), raw);
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
}

#[test]
fn subnet_comparison_overlap_keeps_unknown_coverage_and_independent_versions() {
    use ic_topology::subnet::{
        configuration::{SubnetConfigurationEvidence, SubnetConfigurationReport},
        distribution::{DistributionSummary, GroupCount, SubnetDistributionReport},
    };
    let distribution = |side: &Candidate, version| SubnetDistributionReport {
        schema: 1,
        endpoint: "https://ic0.app",
        source_canister: "rwlgt-iiaaa-aaaaa-aaaaa-cai",
        version_method: "get_latest_version",
        requested_registry_version: version,
        version_observed_at_unix_seconds: 100,
        started_at_unix_seconds: 99,
        elapsed: Duration::from_secs(1),
        query_signature_verified: true,
        result_certified: false,
        overall_deadline_reached: false,
        complete: false,
        configuration: SubnetConfigurationReport {
            schema: 1,
            subnet_id: side.subnet_id,
            registered_node_ids: vec![
                "ryjl3-tyaaa-aaaaa-aaaba-cai".parse().unwrap(),
                "rwlgt-iiaaa-aaaaa-aaaaa-cai".parse().unwrap(),
            ],
            replica_version_id: "version\n\u{1b}[31m".into(),
            registered_subnet_type: 1,
            is_halted: false,
            halt_at_cup_height: false,
            evidence: SubnetConfigurationEvidence {
                endpoint: "https://ic0.app",
                source_canister: "rwlgt-iiaaa-aaaaa-aaaaa-cai",
                source_method: "get_value",
                source_key: format!("subnet_record_{}", side.subnet_id),
                observed_at_unix_seconds: 100,
                record_version: version,
                record_timestamp_nanoseconds: 0,
                query_signature_verified: true,
                result_certified: false,
                membership_complete: true,
            },
        },
        nodes: vec![],
        operators: vec![],
        data_centers: vec![],
        relationships: vec![],
        summary: DistributionSummary {
            selected_nodes: 2,
            provider_resolved: 1,
            provider_unknown: 1,
            data_center_references_resolved: 0,
            location_resolved: 0,
            location_unknown: 2,
            providers: vec![GroupCount {
                value: "ryjl3-tyaaa-aaaaa-aaaba-cai".parse().unwrap(),
                nodes: 1,
            }],
            data_centers: vec![],
            regions: vec![],
            region_unknown: 2,
            owners: vec![],
            owner_unknown: 2,
        },
    };
    let mut report = fixture();
    report.observation.left.distribution = DistributionOutcome::Read {
        report: Box::new(distribution(&report.observation.left, 10)),
    };
    report.observation.right.distribution = DistributionOutcome::Read {
        report: Box::new(distribution(&report.observation.right, 11)),
    };
    report.observation.registry_versions_agree = Some(false);
    let text = render(&report);
    assert!(
        text.contains("Observed infrastructure overlap")
            && text.contains("Providers")
            && text.contains("Data centers")
    );
    assert!(text.contains("1 known / 1 unknown") && text.contains("0 known / 2 unknown"));
    assert!(!text.contains('\u{1b}'));
    let Output::Partial(inner) = output(&report, true) else {
        panic!("partial")
    };
    let Output::Json(value) = *inner else {
        panic!("JSON")
    };
    let overlap = &value["observation"]["infrastructure_overlap"];
    assert_eq!(overlap["providers"]["shared"].as_array().unwrap().len(), 1);
    assert_eq!(overlap["providers"]["coverage_complete"], false);
    assert_eq!(overlap["data_centers"]["left_unknown_nodes"], 2);
    assert_eq!(value["observation"]["registry_versions_agree"], false);
}
