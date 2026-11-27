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
        children: vec![empty, pending],
        sources: vec![
            None,
            Some(stored::Sources {
                creation: delivery,
                upgrades: vec![],
            }),
        ],
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
        preservation: None,
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
fn custody_binds_empty_and_uncertain_children_to_ordered_service_ids() {
    let stored = stored();
    let current = custody::Root {
        stored: stored.clone(),
        store_operation: Some(operation(2)),
    };
    let history = custody::RootHistory {
        current: current.clone(),
        store_history: vec![operation(1)],
    };
    let value = fixture(history.clone(), vec![None; stored.children.len()]);
    for method in [application::CUSTODY, application::CHILD_EVIDENCE] {
        let bytes = frame(&value, method);
        let decoded = decode(method, &bytes).unwrap();
        assert_eq!(decoded, value);
        for index in 0..stored.children.len() {
            let id = ChildId::service(index).unwrap();
            assert_eq!(child(&decoded, id).unwrap().journal, stored.children[index]);
            assert_eq!(child(&decoded, id).unwrap().sources, stored.sources[index]);
        }
        assert!(decode(method, &bytes[..bytes.len() - 1]).is_err());
    }
    for (old, bytes) in [
        (stored::CUSTODY, frame(&stored, stored::CUSTODY)),
        (custody::ROOT, frame(&current, custody::ROOT)),
        (58, frame(&history, 58)),
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
    for mutate in 0..3 {
        let mut invalid = value.clone();
        match mutate {
            0 => invalid.children.swap(0, 1),
            1 => invalid.children[1].slot = invalid.children[0].slot,
            _ => {
                invalid.children.remove(0);
            }
        }
        assert!(decode(application::CUSTODY, &frame(&invalid, application::CUSTODY)).is_err());
    }
    for old in [
        "canic.root.observe_child_deployments.v1",
        "canic.root.observe_root_custody.v1",
    ] {
        let old = ic_wire::method_id(old);
        assert!(decode(application::CUSTODY, &frame(&history, old)).is_err());
    }
}
