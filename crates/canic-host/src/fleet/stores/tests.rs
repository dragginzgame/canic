use super::*;
use crate::fleet::tests::root_operation;

const STORE: &str = "ryjl3-tyaaa-aaaaa-aaaba-cai";

fn operation(network: Network) -> FleetOperation {
    let mut operation = root_operation(network);
    operation.store_root = operation.management.take().map(|a| a.manager);
    operation.canister = Some(STORE.into());
    operation
}

#[test]
fn store_selection_requires_retained_owner_and_target_and_keeps_networks_separate() {
    let first = operation(Network::Mainnet);
    let second = operation(Network::Local {
        url: "http://localhost:8000/".into(),
        root_key: "aa".repeat(133),
        routing_canister: "aaaaa-aa".into(),
    });
    let duplicate = operation(Network::Mainnet);
    let mut unknown = operation(Network::Mainnet);
    unknown.canister = None;
    let child = root_operation(Network::Mainnet);
    let chosen = selected(&[first, second, duplicate, unknown, child]).unwrap();
    assert_eq!(chosen.len(), 2);
    assert_ne!(chosen.first().unwrap().0, chosen.last().unwrap().0);
    assert!(chosen.iter().all(|(_, _, store)| store == STORE));
    let too_many: Vec<_> = (0..17)
        .map(|i| {
            let mut value = operation(Network::Mainnet);
            value.network_id = format!("trust-{i}");
            value
        })
        .collect();
    assert!(selected(&too_many).is_err());
}

#[test]
fn store_report_distinguishes_empty_from_malformed_and_foreign_attempts() {
    let report = wire::Status {
        approval: wire::Approval {
            store: STORE.into(),
            previous_module: [1; 32],
            module_hash: [2; 32],
            attempt: ic_wasm_store_protocol::MaintenanceAttempt {
                sequence: 1,
                operation: [3; 32],
            },
            expires_at_ns: 9,
        },
        operator: "2vxsx-fae".into(),
        phase: wire::Phase::Stopped,
        stop_request: Some([4; 32]),
        failure: None,
        preservation: None,
        installation: None,
    };
    let reply = ic_wire::encode(
        &Some(report.clone()),
        ic_wire::Encoding::Frame {
            method: wire::STATUS,
            direction: ic_wire::Direction::Response,
        },
        wire::LIMITS,
    )
    .unwrap();
    assert_eq!(decode(STORE, &reply).unwrap(), Some(report));
    assert!(decode("rrkah-fqaaa-aaaaa-aaaaq-cai", &reply).is_err());
    let empty = ic_wire::encode(
        &None::<wire::Status>,
        ic_wire::Encoding::Frame {
            method: wire::STATUS,
            direction: ic_wire::Direction::Response,
        },
        wire::LIMITS,
    )
    .unwrap();
    assert_eq!(decode(STORE, &empty).unwrap(), None);
    assert!(decode(STORE, b"malformed").is_err());
    let wrong_method = ic_wire::encode(
        &None::<wire::Status>,
        ic_wire::Encoding::Frame {
            method: wire::OBSERVE,
            direction: ic_wire::Direction::Response,
        },
        wire::LIMITS,
    )
    .unwrap();
    assert!(decode(STORE, &wrong_method).is_err());
}

#[tokio::test(flavor = "current_thread")]
async fn exhausted_budget_remains_unknown_without_network_or_journal_mutation() {
    let operation = operation(Network::Mainnet);
    let targets = BTreeMap::from([(
        (
            operation.network_id.clone(),
            operation.store_root.clone().unwrap(),
        ),
        Network::Mainnet,
    )]);
    let before = serde_json::to_value(&operation).unwrap();
    let reports = collect(
        std::slice::from_ref(&operation),
        &targets,
        tokio::time::Instant::now(),
    )
    .await
    .unwrap();
    assert_eq!(reports.len(), 1);
    assert!(reports[0].reported.is_none());
    assert!(
        reports[0]
            .observation_error
            .as_ref()
            .unwrap()
            .contains("budget exhausted")
    );
    assert_eq!(serde_json::to_value(operation).unwrap(), before);
}
