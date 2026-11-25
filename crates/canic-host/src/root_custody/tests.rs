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
            (0..8)
                .map(|_| {
                    slots
                        .next()
                        .unwrap_or_else(|| canic_management_protocol::custody::Custody {
                            creation: Default::default(),
                            upgrades: vec![],
                        })
                })
                .collect()
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
            (0..8).map(|_| slots.next().flatten()).collect()
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
    let value = application::Custody {
        applications: vec![None; stored.children.len()],
        root: history.clone(),
    };
    for method in [application::CUSTODY, application::CHILD_EVIDENCE] {
        let bytes = frame(&value, method);
        let decoded = decode(method, &bytes).unwrap();
        assert_eq!(decoded, value);
        let slots = children(decoded.root.current.stored).unwrap();
        for (index, id) in ChildId::HISTORICAL_SERVICES.into_iter().enumerate() {
            assert_eq!(slots[&id].journal, stored.children[index]);
            assert_eq!(slots[&id].sources, stored.sources[index]);
        }
        assert!(decode(method, &bytes[..bytes.len() - 1]).is_err());
    }
    for (old, bytes) in [
        (stored::CUSTODY, frame(&stored, stored::CUSTODY)),
        (custody::ROOT, frame(&current, custody::ROOT)),
        (
            custody::ROOT_HISTORY,
            frame(&history, custody::ROOT_HISTORY),
        ),
    ] {
        assert!(decode(old, &bytes).is_err());
        assert!(decode(application::CUSTODY, &bytes).is_err());
    }
    assert!(
        decode(
            application::CUSTODY,
            &frame(&value, application::CHILD_EVIDENCE)
        )
        .is_err()
    );
    let mut invalid = value;
    invalid.applications.pop();
    assert!(decode(application::CUSTODY, &frame(&invalid, application::CUSTODY)).is_err());
}
