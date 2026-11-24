use crate::{custody, store_maintenance as maintenance, stored, upgrade};

#[test]
fn complete_root_custody_fits_combined_budget() {
    let preservation = maintenance::Preservation {
        after_removed: false,
        catalog: ic_wasm_store_protocol::preservation::Catalog {
            quota: ic_wasm_store_protocol::Quota {
                reserved_bytes: u64::MAX,
                releases: u64::MAX,
            },
            reserved_bytes: u64::MAX,
            releases: u64::MAX,
            artifacts: u64::MAX,
            chunks: u64::MAX,
        },
        after: Some(ic_wasm_store_protocol::Release {
            name: "n".repeat(64),
            version: "v".repeat(64),
        }),
        observed: u64::MAX,
        digest: [255; 32],
        complete: true,
    };
    let operation = maintenance::Status {
        approval: maintenance::Approval {
            store: "s".repeat(63),
            previous_module: [254; 32],
            module_hash: [255; 32],
            attempt: ic_wasm_store_protocol::MaintenanceAttempt {
                sequence: u64::MAX,
                operation: [255; 32],
            },
            expires_at_ns: u64::MAX,
        },
        operator: "o".repeat(63),
        phase: maintenance::Phase::Ready,
        stop_request: Some([255; 32]),
        failure: Some(crate::Failure::Rejected(u32::MAX)),
        preservation: crate::store_maintenance::Scan::Entries(Some(preservation.clone())),
        installation: Some(maintenance::Installation {
            upload_request: [255; 32],
            request: Some([255; 32]),
            sender_version: Some(u64::MAX),
            start_request: Some([255; 32]),
            verification: crate::store_maintenance::Scan::Entries(Some(preservation)),
            activation_request: Some([255; 32]),
        }),
    };
    let source = stored::Delivery {
        reference: stored::Reference {
            store: "s".repeat(63),
            manifest: ic_wasm_store_protocol::Manifest {
                release: ic_wasm_store_protocol::Release {
                    name: "n".repeat(64),
                    version: "v".repeat(64),
                },
                hash: [255; 32],
                bytes: u64::MAX,
                chunks: vec![[255; 32]; ic_wasm_store_protocol::MAX_CHUNKS],
            },
        },
        operation: [255; 32],
        state: stored::PinState::Retained,
        cache_deadline: Some(u64::MAX),
    };
    let child = custody::Custody {
        creation: crate::Status {
            approval: Some(crate::Approval {
                manager: "r".repeat(63),
                module_hash: [255; 32],
                creation_cycles: u64::MAX,
                execute_at_ns: u64::MAX,
                expires_at_ns: u64::MAX,
            }),
            phase: crate::Phase::Ready,
            worker: Some("c".repeat(63)),
            attempts: [u8::MAX; 3],
            requests: [Some([255; 32]); 3],
            creation_refund: Some(u128::MAX),
            observed_module: Some(vec![255; 32]),
            failure: Some(crate::Failure::Rejected(u32::MAX)),
            ..Default::default()
        },
        upgrades: vec![
            upgrade::Status {
                approval: Some(upgrade::Approval {
                    manager: "r".repeat(63),
                    child: "c".repeat(63),
                    previous_module: [254; 32],
                    module_hash: [255; 32],
                    execute_at_ns: u64::MAX,
                    expires_at_ns: u64::MAX,
                }),
                phase: upgrade::Phase::Ready,
                attempts: [u8::MAX; 2],
                requests: [Some([255; 32]); 2],
                previous_version: Some(u64::MAX),
                observed_version: Some(u64::MAX),
                failure: Some(crate::Failure::Rejected(u32::MAX)),
            };
            upgrade::MAX_UPGRADES
        ],
    };
    let sources = stored::Sources {
        creation: source.clone(),
        upgrades: vec![source; upgrade::MAX_UPGRADES],
    };
    let value = custody::Root {
        stored: stored::Custody {
            children: (0..8).map(|_| child.clone()).collect(),
            sources: (0..8).map(|_| Some(sources.clone())).collect(),
        },
        store_operation: Some(operation.clone()),
    };
    let bytes = ic_wire::encode(
        &value,
        ic_wire::Encoding::Frame {
            method: custody::ROOT,
            direction: ic_wire::Direction::Response,
        },
        custody::ROOT_LIMITS,
    )
    .unwrap();
    assert_eq!(
        ic_wire::decode::<custody::Root>(
            &bytes,
            ic_wire::Encoding::Frame {
                method: custody::ROOT,
                direction: ic_wire::Direction::Response
            },
            custody::ROOT_LIMITS
        )
        .unwrap(),
        value
    );
    let history = custody::RootHistory {
        current: value,
        store_history: vec![operation; maintenance::MAX_UPGRADES - 1],
    };
    let bytes_history = ic_wire::encode(
        &history,
        ic_wire::Encoding::Payload,
        custody::ROOT_HISTORY_LIMITS,
    )
    .unwrap();
    assert_eq!(
        ic_wire::decode::<custody::RootHistory>(
            &bytes_history,
            ic_wire::Encoding::Payload,
            custody::ROOT_HISTORY_LIMITS
        )
        .unwrap(),
        history
    );
    // Same tuple shape as Root's CUJC payload, including both full child journals.
    // The existing checked journal lease must fit every admitted record at its worst field sizes.
    let child_journal = (
        &child.creation,
        child.upgrades.last().unwrap(),
        &child.upgrades[..upgrade::MAX_UPGRADES - 1],
    );
    let journal = (
        vec![(0_u8, child_journal), (1_u8, child_journal)],
        &history.current.store_operation,
        &history.store_history,
    );
    let stable_limits = ic_wire::Limits {
        bytes: 16_384 - 4,
        allocation: 2 * (1 + upgrade::MAX_UPGRADES) * crate::LIMITS.allocation
            + maintenance::MAX_UPGRADES * maintenance::LIMITS.allocation,
        nodes: 2 * (1 + upgrade::MAX_UPGRADES) * crate::LIMITS.nodes
            + maintenance::MAX_UPGRADES * maintenance::LIMITS.nodes,
        depth: crate::LIMITS.depth + 5,
    };
    let journal_bytes =
        ic_wire::encode(&journal, ic_wire::Encoding::Payload, stable_limits).unwrap();
    assert!(journal_bytes.len() + 4 <= 16_384);
    type ChildJournal = (crate::Status, upgrade::Status, Vec<upgrade::Status>);
    let decoded: (
        Vec<(u8, ChildJournal)>,
        Option<maintenance::Status>,
        Vec<maintenance::Status>,
    ) = ic_wire::decode(&journal_bytes, ic_wire::Encoding::Payload, stable_limits).unwrap();
    assert_eq!(decoded.1, history.current.store_operation);
    assert_eq!(decoded.2, history.store_history);
    assert!(
        ic_wire::decode::<stored::Custody>(
            &bytes,
            ic_wire::Encoding::Frame {
                method: stored::CUSTODY,
                direction: ic_wire::Direction::Response
            },
            stored::LIMITS
        )
        .is_err()
    );
}
