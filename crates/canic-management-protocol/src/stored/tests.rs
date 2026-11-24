use super::*;
use crate::{Approval, CREATION_CYCLES, Status, custody, upgrade};
use ic_wasm_store_protocol::{CHUNK_BYTES, MAX_CHUNKS, Release};

#[test]
fn complete_store_custody_fits_and_operation_identity_binds_source_slot_and_action() {
    let manifest = Manifest {
        release: Release {
            name: "a".repeat(64),
            version: "v".repeat(64),
        },
        hash: [255; 32],
        bytes: (CHUNK_BYTES * MAX_CHUNKS) as u64,
        chunks: vec![[255; 32]; MAX_CHUNKS],
    };
    let stage = Stage {
        approval: Approval {
            manager: "a".repeat(63),
            module_hash: manifest.hash,
            creation_cycles: CREATION_CYCLES,
            execute_at_ns: u64::MAX - 1,
            expires_at_ns: u64::MAX,
        },
        reference: Reference {
            store: "b".repeat(63),
            manifest,
        },
    };
    let operation = stage.pin(STAGE, children::ChildId::FIRST).operation;
    assert_ne!(
        operation,
        stage.pin(STAGE, children::ChildId::SECOND).operation
    );
    assert_ne!(
        operation,
        stage.pin(STAGE_UPGRADE, children::ChildId::FIRST).operation
    );
    let mut changed = stage.clone();
    changed.reference.store.push('c');
    assert_ne!(
        operation,
        changed.pin(STAGE, children::ChildId::FIRST).operation
    );
    changed = stage.clone();
    changed.reference.manifest.release.version.push('x');
    assert_ne!(
        operation,
        changed.pin(STAGE, children::ChildId::FIRST).operation
    );
    let delivery = Delivery {
        reference: stage.reference.clone(),
        operation,
        state: PinState::Retained,
        cache_deadline: Some(u64::MAX),
    };
    let sources = Sources {
        creation: delivery.clone(),
        upgrades: vec![delivery; upgrade::MAX_UPGRADES],
    };
    let journal = custody::Custody {
        creation: Status {
            approval: Some(stage.approval),
            worker: Some("c".repeat(63)),
            attempts: [1; 3],
            requests: [Some([255; 32]); 3],
            creation_refund: Some(u128::MAX),
            observed_module: Some(vec![255; 32]),
            ..Default::default()
        },
        upgrades: vec![
            upgrade::Status {
                approval: Some(upgrade::Approval {
                    manager: "a".repeat(63),
                    child: "c".repeat(63),
                    previous_module: [255; 32],
                    module_hash: [254; 32],
                    execute_at_ns: u64::MAX - 1,
                    expires_at_ns: u64::MAX
                }),
                attempts: [1; 2],
                requests: [Some([255; 32]); 2],
                previous_version: Some(u64::MAX - 1),
                observed_version: Some(u64::MAX),
                ..Default::default()
            };
            upgrade::MAX_UPGRADES
        ],
    };
    let custody = Custody {
        children: (0..8).map(|_| journal.clone()).collect(),
        sources: (0..8).map(|_| Some(sources.clone())).collect(),
    };
    let bytes = ic_wire::encode(
        &custody,
        ic_wire::Encoding::Frame {
            method: CUSTODY,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap();
    assert_eq!(
        ic_wire::decode::<Custody>(
            &bytes,
            ic_wire::Encoding::Frame {
                method: CUSTODY,
                direction: ic_wire::Direction::Response
            },
            LIMITS
        )
        .unwrap(),
        custody
    );
}
