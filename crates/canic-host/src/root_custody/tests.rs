use super::*;
use canic_management_protocol::{Phase, Status};
use ic_wasm_store_protocol::{MaintenanceAttempt, Manifest, Release};

fn stored() -> stored::Custody {
    let empty = custody::Custody {
        creation: Status::default(),
        upgrades: vec![],
    };
    let mut pending = empty.clone();
    pending.creation.phase = Phase::CreateIssued;
    pending.creation.attempts = [1, 0, 0];
    pending.creation.requests = [Some([7; 32]), None, None];
    let delivery = stored::Delivery {
        reference: stored::Reference {
            store: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
            manifest: Manifest {
                release: Release {
                    name: "app".into(),
                    version: "v1".into(),
                },
                hash: [9; 32],
                bytes: 8,
                chunks: vec![[9; 32]],
            },
        },
        operation: [4; 32],
        state: stored::PinState::Issued,
        cache_deadline: Some(123),
    };
    stored::Custody {
        children: {
            let mut slots = [empty, pending].into_iter();
            std::array::from_fn(|_| {
                slots
                    .next()
                    .unwrap_or_else(|| canic_management_protocol::custody::Custody {
                        creation: Default::default(),
                        upgrades: vec![],
                    })
            })
        },
        sources: {
            let mut slots = [
                None,
                Some(stored::Sources {
                    creation: delivery,
                    upgrades: vec![],
                }),
            ]
            .into_iter();
            std::array::from_fn(|_| slots.next().flatten())
        },
    }
}

fn operation(sequence: u64) -> store_maintenance::Status {
    store_maintenance::Status {
        approval: store_maintenance::Approval {
            store: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
            previous_module: [1; 32],
            module_hash: [2; 32],
            attempt: MaintenanceAttempt {
                sequence,
                operation: [3; 32],
            },
            expires_at_ns: 123,
        },
        operator: "2vxsx-fae".into(),
        phase: store_maintenance::Phase::StopIssued,
        stop_request: Some([4; 32]),
        failure: None,
        preservation: canic_management_protocol::store_maintenance::Scan::Legacy(None),
        installation: None,
    }
}

fn frame(value: &impl serde::Serialize, method: u64) -> Vec<u8> {
    ic_wire::encode(
        value,
        ic_wire::Encoding::Frame {
            method,
            direction: ic_wire::Direction::Response,
        },
        custody::ROOT_HISTORY_LIMITS,
    )
    .unwrap()
}

#[test]
fn historical_contracts_keep_empty_and_uncertain_children_bound_to_identity() {
    let stored = stored();
    let current = custody::Root {
        stored: stored.clone(),
        store_operation: Some(operation(2)),
    };
    let history = custody::RootHistory {
        current: current.clone(),
        store_history: vec![operation(1)],
    };
    for (method, bytes, expected_operation, expected_history) in [
        (
            stored::CUSTODY,
            frame(&stored, stored::CUSTODY),
            None,
            vec![],
        ),
        (
            custody::ROOT,
            frame(&current, custody::ROOT),
            current.store_operation.clone(),
            vec![],
        ),
        (
            custody::ROOT_HISTORY,
            frame(&history, custody::ROOT_HISTORY),
            current.store_operation.clone(),
            history.store_history.clone(),
        ),
    ] {
        let snapshot = decode(method, &bytes).unwrap();
        assert_eq!(snapshot.children.len(), ChildId::ALL.len());
        for (index, id) in ChildId::ALL.into_iter().enumerate() {
            let child = snapshot.child(id).unwrap();
            assert_eq!(child.journal, stored.children[index]);
            assert_eq!(child.sources, stored.sources[index]);
        }
        assert_eq!(snapshot.store_operation, expected_operation);
        assert_eq!(snapshot.store_history, expected_history);
        assert!(decode(method, &bytes[..bytes.len() - 1]).is_err());
        for wrong in [stored::CUSTODY, custody::ROOT, custody::ROOT_HISTORY]
            .into_iter()
            .filter(|id| *id != method)
        {
            assert!(
                decode(wrong, &bytes).is_err(),
                "a different contract must not become fallback evidence"
            );
        }
    }
}

#[test]
fn wrong_direction_and_unsupported_contract_are_not_observations() {
    let input = ic_wire::encode(
        &stored(),
        ic_wire::Encoding::Frame {
            method: stored::CUSTODY,
            direction: ic_wire::Direction::Request,
        },
        stored::LIMITS,
    )
    .unwrap();
    assert!(decode(stored::CUSTODY, &input).is_err());
    assert!(decode(custody::OBSERVE, &frame(&stored(), custody::OBSERVE)).is_err());
}
