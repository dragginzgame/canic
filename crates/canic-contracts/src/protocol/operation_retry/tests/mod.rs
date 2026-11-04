//! Exact resume frames preserve operation and generation without accepting status frames.
use super::*;
use crate::{
    dto::operation_retry::{Operation, ResumeRequest, State, Status},
    ids::ComponentInstanceId,
};

#[test]
fn retry_contract_binds_operation_generation_and_method() {
    let operation = Operation::SubtreeRemoval {
        component: ComponentInstanceId::from_generated_bytes([7; 32]),
        operation_id: [8; 32],
    };
    let request = ResumeRequest {
        operation,
        expected_generation: u64::MAX,
    };
    let bytes =
        ic_wire::native::encode_request(resume::METHOD_ID, &request, REQUEST_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<ResumeRequest>(resume::METHOD_ID, &bytes, REQUEST_LIMITS)
            .unwrap(),
        request
    );
    assert!(
        ic_wire::native::decode_request::<ResumeRequest>(status::METHOD_ID, &bytes, REQUEST_LIMITS)
            .is_err()
    );
    let status = Status {
        operation,
        generation: u64::MAX,
        attempts_without_progress: u32::MAX,
        maximum_attempts_without_progress: u32::MAX,
        retry_at_ns: u64::MAX,
        diagnostic_code: Some(u16::MAX),
        state: State::ReviewRequired,
    };
    let bytes =
        ic_wire::native::encode_response(status::METHOD_ID, &status, RESPONSE_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_response::<Status>(status::METHOD_ID, &bytes, RESPONSE_LIMITS)
            .unwrap(),
        status
    );
}
