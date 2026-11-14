use super::*;

#[test]
fn upgrade_binds_original_child_old_and_new_code_and_time() {
    let wasm = b"\0asm\x01\0\0\0".to_vec();
    let input = Stage {
        approval: Approval {
            manager: "root".into(),
            child: "child".into(),
            previous_module: [1; 32],
            module_hash: digest(&wasm),
            execute_at_ns: 100,
            expires_at_ns: 200,
        },
        wasm,
    };
    assert_eq!(input.validate("root", "child", &[1; 32], 99), Ok(()));
    assert_eq!(
        input.validate("other", "child", &[1; 32], 99),
        Err(Refusal::Binding)
    );
    assert_eq!(
        input.validate("root", "other", &[1; 32], 99),
        Err(Refusal::Binding)
    );
    assert_eq!(
        input.validate("root", "child", &[2; 32], 99),
        Err(Refusal::Binding)
    );
    assert_eq!(
        input.validate("root", "child", &[1; 32], 101),
        Err(Refusal::Time)
    );
    let mut changed = input.clone();
    changed.approval.previous_module = changed.approval.module_hash;
    assert_eq!(
        changed.validate("root", "child", &changed.approval.previous_module, 99),
        Err(Refusal::Binding)
    );
    changed = input.clone();
    changed.wasm.push(0);
    assert_eq!(
        changed.validate("root", "child", &[1; 32], 99),
        Err(Refusal::Artifact)
    );
    let bytes = ic_wire::native::encode_request(STAGE, &input, crate::STAGE_LIMITS).unwrap();
    let decoded: Stage =
        ic_wire::native::decode_request(STAGE, &bytes, crate::STAGE_LIMITS).unwrap();
    assert_eq!(decoded.approval, input.approval);
    let status = Status {
        approval: Some(input.approval),
        phase: Phase::Ready,
        attempts: [1; 2],
        requests: [Some([3; 32]); 2],
        previous_version: Some(1),
        observed_version: Some(2),
        failure: None,
    };
    let bytes = ic_wire::native::encode_response(STATUS, &status, crate::LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_response::<Status>(STATUS, &bytes, crate::LIMITS).unwrap(),
        status
    );
}
