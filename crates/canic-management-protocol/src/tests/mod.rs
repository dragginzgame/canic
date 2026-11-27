mod application;
mod custody;

use crate::{Approval, CREATION_CYCLES, Failure, LIMITS, Phase, Refusal, STATUS, Status};

#[test]
fn creation_admission_uses_original_expiry_not_earliest_dispatch() {
    let mut approval = Approval {
        manager: "root".into(),
        module_hash: [1; 32],
        creation_cycles: CREATION_CYCLES,
        execute_at_ns: 100,
        expires_at_ns: 200,
    };
    for now in [99, 100, 101, 199] {
        assert_eq!(approval.validate("root", now), Ok(()));
    }
    for now in [200, 201, u64::MAX] {
        assert_eq!(approval.validate("root", now), Err(Refusal::Time));
    }
    approval.expires_at_ns = 100;
    assert_eq!(approval.validate("root", 99), Err(Refusal::Time));
    approval.expires_at_ns = 600_000_000_101;
    assert_eq!(approval.validate("root", 101), Err(Refusal::Time));
    approval.execute_at_ns = 3_600_000_000_100;
    approval.expires_at_ns = approval.execute_at_ns + 1;
    assert_eq!(approval.validate("root", 99), Err(Refusal::Time));
}

#[test]
fn publication_envelope_carries_a_maximum_chunk_and_bounded_status() {
    use crate::publication::{self, Command, Request};
    use ic_wasm_store_protocol as store;
    let release = store::Release {
        name: "a".repeat(64),
        version: "v".repeat(64),
    };
    let request = Request {
        store: "a".repeat(63),
        expires_at_ns: u64::MAX,
        command: Command::Put(store::Put {
            release: release.clone(),
            index: (store::MAX_CHUNKS - 1) as u32,
            bytes: vec![42; store::CHUNK_BYTES],
        }),
    };
    let frame = ic_wire::encode(
        &request,
        ic_wire::Encoding::Frame {
            method: publication::EXECUTE,
            direction: ic_wire::Direction::Request,
        },
        publication::REQUEST_LIMITS,
    )
    .unwrap();
    let decoded: Request = ic_wire::decode(
        &frame,
        ic_wire::Encoding::Frame {
            method: publication::EXECUTE,
            direction: ic_wire::Direction::Request,
        },
        publication::REQUEST_LIMITS,
    )
    .unwrap();
    assert!(
        matches!(decoded.command, Command::Put(chunk) if chunk.bytes == vec![42; store::CHUNK_BYTES])
    );
    assert!(
        ic_wire::decode::<Request>(
            &frame,
            ic_wire::Encoding::Frame {
                method: publication::EXECUTE,
                direction: ic_wire::Direction::Request
            },
            publication::RESPONSE_LIMITS
        )
        .is_err()
    );
    let status = store::Status {
        manifest: store::Manifest {
            release,
            hash: [255; 32],
            bytes: (store::CHUNK_BYTES * store::MAX_CHUNKS) as u64,
            chunks: vec![[255; 32]; store::MAX_CHUNKS],
        },
        published: false,
        missing: (0..store::MAX_CHUNKS as u32).collect(),
    };
    let reply: publication::Reply = Ok(status);
    let frame = ic_wire::encode(
        &reply,
        ic_wire::Encoding::Frame {
            method: publication::EXECUTE,
            direction: ic_wire::Direction::Response,
        },
        publication::RESPONSE_LIMITS,
    )
    .unwrap();
    assert_eq!(
        ic_wire::decode::<publication::Reply>(
            &frame,
            ic_wire::Encoding::Frame {
                method: publication::EXECUTE,
                direction: ic_wire::Direction::Response
            },
            publication::RESPONSE_LIMITS
        )
        .unwrap(),
        reply
    );
}

#[test]
fn creation_evidence_fits_the_response_budget() {
    let approval = Approval {
        manager: "a".repeat(63),
        module_hash: [255; 32],
        creation_cycles: CREATION_CYCLES,
        execute_at_ns: 10,
        expires_at_ns: 20,
    };
    let status = Status {
        approval: Some(approval),
        phase: Phase::Ready,
        worker: Some("a".repeat(63)),
        attempts: [1; 3],
        requests: [Some([255; 32]); 3],
        creation_refund: Some(u128::MAX),
        failure: Some(Failure::Rejected(u32::MAX)),
        observed_module: Some(vec![255; 32]),
        stable_pages: 64,
        maintenance_candidate: true,
    };
    let frame = ic_wire::encode(
        &status,
        ic_wire::Encoding::Frame {
            method: STATUS,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap();
    assert_eq!(
        ic_wire::decode::<Status>(
            &frame,
            ic_wire::Encoding::Frame {
                method: STATUS,
                direction: ic_wire::Direction::Response
            },
            LIMITS
        )
        .unwrap(),
        status
    );
}

#[test]
fn store_readiness_frame_is_separate_from_publication_and_carries_bounded_info() {
    use crate::publication;
    use ic_wasm_store_protocol as store;
    let input = publication::InfoRequest {
        store: "a".repeat(63),
        expires_at_ns: u64::MAX,
    };
    let frame = ic_wire::encode(
        &input,
        ic_wire::Encoding::Frame {
            method: publication::INFO,
            direction: ic_wire::Direction::Request,
        },
        LIMITS,
    )
    .unwrap();
    let decoded: publication::InfoRequest = ic_wire::decode(
        &frame,
        ic_wire::Encoding::Frame {
            method: publication::INFO,
            direction: ic_wire::Direction::Request,
        },
        LIMITS,
    )
    .unwrap();
    assert_eq!(decoded.store, input.store);
    assert!(
        ic_wire::decode::<publication::InfoRequest>(
            &frame,
            ic_wire::Encoding::Frame {
                method: publication::EXECUTE,
                direction: ic_wire::Direction::Request
            },
            LIMITS
        )
        .is_err()
    );
    let expected = store::Info {
        quota: store::Quota {
            reserved_bytes: u64::MAX,
            releases: u64::MAX,
        },
        reserved_bytes: u64::MAX,
        releases: u64::MAX,
        artifacts: u64::MAX,
        chunks: u64::MAX,
        maintenance_candidate: true,
    };
    let reply: publication::InfoReply = Ok(expected.clone());
    let frame = ic_wire::encode(
        &reply,
        ic_wire::Encoding::Frame {
            method: publication::INFO,
            direction: ic_wire::Direction::Response,
        },
        publication::RESPONSE_LIMITS,
    )
    .unwrap();
    let decoded: publication::InfoReply = ic_wire::decode(
        &frame,
        ic_wire::Encoding::Frame {
            method: publication::INFO,
            direction: ic_wire::Direction::Response,
        },
        publication::RESPONSE_LIMITS,
    )
    .unwrap();
    assert_eq!(decoded.unwrap(), expected);
}

#[test]
fn removal_envelope_binds_full_manifest_and_cannot_be_publication() {
    use crate::publication::{self, Removal, RemovalRequest};
    use ic_wasm_store_protocol as store;
    let manifest = store::Manifest {
        release: store::Release {
            name: "a".repeat(64),
            version: "v".repeat(64),
        },
        hash: [255; 32],
        bytes: (store::CHUNK_BYTES * store::MAX_CHUNKS) as u64,
        chunks: vec![[255; 32]; store::MAX_CHUNKS],
    };
    for action in [Removal::Delete, Removal::Cancel] {
        let input = RemovalRequest {
            store: "a".repeat(63),
            expires_at_ns: u64::MAX,
            action,
            manifest: manifest.clone(),
        };
        let encoding = ic_wire::Encoding::Frame {
            method: publication::REMOVE,
            direction: ic_wire::Direction::Request,
        };
        let bytes = ic_wire::encode(&input, encoding, publication::REMOVAL_LIMITS).unwrap();
        assert_eq!(
            ic_wire::decode::<RemovalRequest>(&bytes, encoding, publication::REMOVAL_LIMITS)
                .unwrap(),
            input
        );
        assert!(
            ic_wire::decode::<publication::Request>(
                &bytes,
                ic_wire::Encoding::Frame {
                    method: publication::EXECUTE,
                    direction: ic_wire::Direction::Request,
                },
                publication::REQUEST_LIMITS
            )
            .is_err()
        );
        let reply: publication::RemovalReply = Ok(manifest.clone());
        let encoding = ic_wire::Encoding::Frame {
            method: publication::REMOVE,
            direction: ic_wire::Direction::Response,
        };
        let bytes = ic_wire::encode(&reply, encoding, publication::RESPONSE_LIMITS).unwrap();
        assert_eq!(
            ic_wire::decode::<publication::RemovalReply>(
                &bytes,
                encoding,
                publication::RESPONSE_LIMITS
            )
            .unwrap(),
            reply
        );
    }
}
