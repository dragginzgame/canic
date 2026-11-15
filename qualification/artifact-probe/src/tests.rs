use crate::{
    ARTIFACT_LIMITS, AdmissionError, Approval, CHECKPOINT, Fault, MAX_WASM_BYTES, Phase, Retained,
    STAGE, artifact_hash,
};

fn approval() -> Approval {
    let mut wasm = vec![255; MAX_WASM_BYTES];
    wasm[..8].copy_from_slice(b"\0asm\x01\0\0\0");
    Approval {
        target: vec![1; 29],
        sha256: artifact_hash(&wasm),
        wasm,
        execute_at_ns: 60_000_000_000,
        fault: Fault::AfterResultWrite,
    }
}

#[test]
fn maximum_artifact_fits_upload_and_checkpoint() {
    let approval = approval();
    approval.validate(0).unwrap();
    let upload = ic_wire::native::encode_request(STAGE, &approval, ARTIFACT_LIMITS).unwrap();
    let decoded: Approval =
        ic_wire::native::decode_request(STAGE, &upload, ARTIFACT_LIMITS).unwrap();
    assert_eq!(decoded, approval);
    let state = Retained {
        approval: Some(approval),
        phase: Phase::Issued,
        attempts: 1,
        upload_attempts: 1,
        upload_request_sha256: Some([255; 32]),
        chunk_hash: Some(vec![255; 32]),
        request_sha256: Some([255; 32]),
        reply_retained: false,
        observed_module: Some(vec![255; 32]),
    };
    let bytes = ic_wire::native::encode_response(CHECKPOINT, &state, ARTIFACT_LIMITS).unwrap();
    let restored: Retained =
        ic_wire::native::decode_response(CHECKPOINT, &bytes, ARTIFACT_LIMITS).unwrap();
    assert_eq!(restored, state);
    assert!(
        ic_wire::native::decode_response::<Retained>(
            CHECKPOINT,
            &bytes[..bytes.len() - 1],
            ARTIFACT_LIMITS
        )
        .is_err()
    );
    println!(
        "maximum artifact: upload_bytes={}, checkpoint_bytes={}, stable_pages={}",
        upload.len(),
        bytes.len(),
        (bytes.len() + 4).div_ceil(65_536)
    );
}

#[test]
fn admission_binds_content_target_and_time() {
    let good = approval();
    let mut changed = good.clone();
    changed.wasm.push(0);
    assert_eq!(changed.validate(0), Err(AdmissionError::Wasm));
    changed = good.clone();
    changed.wasm[0] = 255;
    assert_eq!(changed.validate(0), Err(AdmissionError::Wasm));
    changed = good.clone();
    changed.sha256[0] ^= 1;
    assert_eq!(changed.validate(0), Err(AdmissionError::Hash));
    for target in [vec![], vec![4], vec![1; 30]] {
        changed = good.clone();
        changed.target = target;
        assert_eq!(changed.validate(0), Err(AdmissionError::Target));
    }
    assert_eq!(good.validate(good.execute_at_ns), Err(AdmissionError::Time));
    changed = good;
    changed.execute_at_ns = 3_600_000_000_001;
    assert_eq!(changed.validate(0), Err(AdmissionError::Time));
}
