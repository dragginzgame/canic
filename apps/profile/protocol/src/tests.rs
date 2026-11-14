use crate::growth::{
    Approval, CREATION_CYCLES, Failure, LIMITS, MAX_WASM, Phase, Refusal, STAGE, STAGE_LIMITS,
    STATUS, Stage, Status, digest,
};

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
    let frame = ic_wire::native::encode_request(STAGE, &input, STAGE_LIMITS).unwrap();
    let decoded: Stage = ic_wire::native::decode_request(STAGE, &frame, STAGE_LIMITS).unwrap();
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
    let frame = ic_wire::native::encode_response(STATUS, &status, LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_response::<Status>(STATUS, &frame, LIMITS).unwrap(),
        status
    );
}
