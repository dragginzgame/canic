use super::*;
use crate::{CanisterConfig, FleetConfig};
use ic_query_topology::placement::{
    PlacementError,
    collection::{CanisterPlacement, PlacementCollectionReport},
};
use std::{cell::Cell, collections::BTreeMap, time::Duration};

fn selected() -> SelectedFleet {
    SelectedFleet {
        app: "app".into(),
        fleet: "mainnet".into(),
        config: FleetConfig {
            receiver_policy: None,
            shard_pool: None,
            auth: Vec::new(),
            network: Network::Mainnet,
            journal: None,
            canisters: BTreeMap::from([
                (
                    "root".into(),
                    CanisterConfig {
                        id: Some("ryjl3-tyaaa-aaaaa-aaaba-cai".into()),
                        role: Some("root".into()),
                        ..Default::default()
                    },
                ),
                (
                    "worker".into(),
                    CanisterConfig {
                        parent: Some("root".into()),
                        ..Default::default()
                    },
                ),
            ]),
        },
    }
}

#[tokio::test]
async fn placements_admission_makes_no_reads() {
    let mut local = selected();
    local.config.network = Network::Local {
        url: "http://localhost:4943".into(),
        root_key: "00".into(),
        root_subnet: None,
        routing_canister: "aaaaa-aa".into(),
    };
    let mut bad = selected();
    bad.config.canisters.get_mut("root").unwrap().id = Some("bad-id".into());
    let mut duplicate = selected();
    duplicate.config.canisters.get_mut("worker").unwrap().id =
        duplicate.config.canisters["root"].id.clone();
    let mut excess = selected();
    excess.config.canisters = (0..33)
        .map(|n| {
            (
                format!("slot{n:02}"),
                CanisterConfig {
                    id: Some(Principal::from_slice(&[n, 1]).to_text()),
                    ..Default::default()
                },
            )
        })
        .collect();
    for config in [local, bad, duplicate, excess] {
        let called = Cell::new(false);
        let result = collect(&config, |_| {
            called.set(true);
            async { unreachable!() }
        })
        .await;
        assert!(matches!(result, Err(Error::Invalid(_))));
        assert!(!called.get());
    }
}

#[tokio::test]
async fn placements_all_unbound_is_not_a_completed_collection() {
    let mut config = selected();
    config.config.canisters.get_mut("root").unwrap().id = None;
    let report = collect(&config, |_| async { panic!("no unbound lookup") })
        .await
        .unwrap();
    assert!(report.observation.is_none());
    assert!(!report.collection_complete());
    assert_eq!(report.slots.len(), 2);
    assert!(report.slots.iter().all(|s| s.observation_index.is_none()));
}

#[tokio::test]
async fn placements_keep_partial_evidence_and_slot_declarations() {
    let config = selected();
    let report = collect(&config, |ids| async move {
        assert_eq!(ids.len(), 1);
        Ok(PlacementCollectionReport {
            schema: 1,
            atomic_snapshot: false,
            started_at_unix_seconds: 10,
            elapsed: Duration::from_secs(1),
            collection_complete: false,
            overall_deadline_reached: false,
            canisters: vec![CanisterPlacement {
                canister_id: ids[0],
                outcome: PlacementOutcome::Failed {
                    error: PlacementError::MissingSubnet,
                },
            }],
        })
    })
    .await
    .unwrap();
    assert!(!report.collection_complete());
    assert_eq!(report.slots[0].name, "root");
    assert_eq!(report.slots[0].observation_index, Some(0));
    assert_eq!(report.slots[1].parent.as_deref(), Some("root"));
    assert!(report.slots[1].observation_index.is_none());
    let json = serde_json::to_value(report).unwrap();
    assert_eq!(json["observation"]["schema"], 1);
    assert_eq!(
        json["observation"]["canisters"][0]["canister_id"],
        "ryjl3-tyaaa-aaaaa-aaaba-cai"
    );
    assert_eq!(
        json["observation"]["canisters"][0]["outcome"]["error"]["kind"],
        "missing_subnet"
    );
}
