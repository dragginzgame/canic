use crate::{
    Approval, CREATION_CYCLES, Failure, LIMITS, MAX_WASM, Phase, Refusal, STAGE, STAGE_LIMITS,
    STATUS, Stage, Status, digest,
};

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
    use canic_wasm_store_protocol as store;
    let release = store::Release {
        name: "a".repeat(64),
        version: "v".repeat(64),
    };
    let request = Request {
        store: "a".repeat(63),
        expires_at_ns: u64::MAX,
        command: Command::Put(store::Put {
            release: release.clone(),
            index: 31,
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
fn growth_frames_cover_declared_artifact_and_retained_evidence() {
    let mut wasm = vec![0; MAX_WASM];
    wasm[..8].copy_from_slice(b"\0asm\x01\0\0\0");
    let mut input = Stage {
        approval: Approval {
            manager: "a".repeat(63),
            module_hash: digest(&wasm),
            creation_cycles: CREATION_CYCLES,
            execute_at_ns: 10,
            expires_at_ns: 20,
        },
        wasm,
    };
    assert_eq!(input.validate(&input.approval.manager, 0), Ok(()));
    let frame = ic_wire::encode(
        &input,
        ic_wire::Encoding::Frame {
            method: STAGE,
            direction: ic_wire::Direction::Request,
        },
        STAGE_LIMITS,
    )
    .unwrap();
    let decoded: Stage = ic_wire::decode(
        &frame,
        ic_wire::Encoding::Frame {
            method: STAGE,
            direction: ic_wire::Direction::Request,
        },
        STAGE_LIMITS,
    )
    .unwrap();
    assert_eq!(decoded.wasm, input.wasm);
    assert_eq!(decoded.approval, input.approval);
    input.wasm.push(0);
    input.approval.module_hash = digest(&input.wasm);
    assert_eq!(
        input.validate(&input.approval.manager, 0),
        Err(Refusal::Artifact)
    );

    let status = Status {
        approval: Some(input.approval),
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
    use canic_wasm_store_protocol as store;
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
