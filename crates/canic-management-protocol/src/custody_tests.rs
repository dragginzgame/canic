use crate::{custody, store_maintenance as maintenance, stored, upgrade};

#[test]
fn complete_root_custody_fits_combined_budget() {
    let preservation = maintenance::Preservation {
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
        preservation: Some(preservation.clone()),
        installation: Some(maintenance::Installation {
            upload_request: [255; 32],
            request: Some([255; 32]),
            sender_version: Some(u64::MAX),
            start_request: Some([255; 32]),
            verification: Some(preservation),
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
            children: [child.clone(), child],
            sources: [Some(sources.clone()), Some(sources)],
        },
        store_operation: Some(operation),
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
