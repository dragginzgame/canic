use super::*;
use crate::FleetConfig;
use ic_topology::subnet::{
    compare::{Candidate, DistributionOutcome},
    distribution::DistributionError,
    metrics::{MetricsOutcome, SubnetMetricsError},
};
use std::{cell::Cell, collections::BTreeMap, time::Duration};

fn selected() -> SelectedFleet {
    SelectedFleet {
        app: "app".into(),
        fleet: "mainnet".into(),
        config: FleetConfig {
            auth: None,
            network: Network::Mainnet,
            journal: Some("must-not-be-used".into()),
            canisters: BTreeMap::new(),
        },
    }
}
fn candidate(subnet_id: Principal) -> Candidate {
    Candidate {
        subnet_id,
        distribution: DistributionOutcome::Failed {
            error: DistributionError::InitialDeadline,
        },
        metrics: MetricsOutcome::Failed {
            error: SubnetMetricsError::Read(ic_topology::ReadError::Deadline),
        },
    }
}

#[tokio::test]
async fn subnet_comparison_admits_network_and_ids_before_reading() {
    let mut local = selected();
    local.config.network = Network::Local {
        url: "http://localhost:4943".into(),
        root_key: "00".into(),
        root_subnet: None,
        routing_canister: "aaaaa-aa".into(),
    };
    for (selection, left, right) in [
        (local, "aaaaa-aa", "2vxsx-fae"),
        (selected(), "bad", "2vxsx-fae"),
        (selected(), "aaaaa-aa", "bad"),
    ] {
        let called = Cell::new(false);
        let result = collect(&selection, left, right, |_, _| {
            called.set(true);
            async { unreachable!() }
        })
        .await;
        assert!(matches!(result, Err(Error::Invalid(_))));
        assert!(!called.get());
    }
    let error = observe(&selected(), "aaaaa-aa", "aaaaa-aa")
        .await
        .unwrap_err();
    let Error::Supplier(error) = error else {
        panic!("retain supplier cause")
    };
    assert!(matches!(
        error.downcast_ref::<ComparisonError>(),
        Some(ComparisonError::SameSubnet)
    ));
}

#[tokio::test]
async fn subnet_comparison_retains_candidate_order_and_all_failed_report() {
    let result = collect(
        &selected(),
        "aaaaa-aa",
        "2vxsx-fae",
        |left, right| async move {
            assert_eq!(left.to_text(), "aaaaa-aa");
            assert_eq!(right.to_text(), "2vxsx-fae");
            Ok(SubnetComparisonReport {
                schema: 1,
                left: candidate(left),
                right: candidate(right),
                started_at_unix_seconds: 123,
                elapsed: Duration::from_secs(9),
                collection_complete: false,
                atomic_snapshot: false,
                registry_versions_agree: None,
            })
        },
    )
    .await
    .unwrap();
    assert_eq!(result.app, "app");
    assert!(!result.observation.collection_complete);
    let json = serde_json::to_value(result).unwrap();
    assert_eq!(json["observation"]["schema"], 1);
    assert_eq!(json["observation"]["left"]["subnet_id"], "aaaaa-aa");
    assert_eq!(json["observation"]["right"]["subnet_id"], "2vxsx-fae");
    assert!(json["observation"]["infrastructure_overlap"].is_null());
    assert_eq!(
        json["observation"]["left"]["distribution"]["status"],
        "failed"
    );
    assert_eq!(json["observation"]["right"]["metrics"]["status"], "failed");
    assert_eq!(json["observation"]["started_at_unix_seconds"], 123);
}
