//! Activation retries retain the exact operation and credential identity.
use super::*;
use crate::dto::{
    fleet_activation::{FleetActivationResumeRequest, FleetCredentialGenerationRef},
    role::OperationReceipt,
};

#[test]
fn resume_preserves_identity_and_rejects_prepare_frames() {
    let request = FleetActivationResumeRequest {
        operation_id: [9; 32],
        credential: FleetCredentialGenerationRef {
            generation: u64::MAX,
            manifest_hash: [7; 32],
        },
    };
    let bytes =
        ic_wire::native::encode_request(resume::METHOD_ID, &request, REQUEST_LIMITS).unwrap();
    let decoded = ic_wire::native::decode_request::<FleetActivationResumeRequest>(
        resume::METHOD_ID,
        &bytes,
        REQUEST_LIMITS,
    )
    .unwrap();
    assert_eq!(decoded.operation_id, request.operation_id);
    assert_eq!(decoded.credential, request.credential);
    assert!(matches!(
        ic_wire::native::decode_request::<FleetActivationResumeRequest>(
            prepare::METHOD_ID,
            &bytes,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
    let receipt = OperationReceipt {
        operation_id: decoded.operation_id,
    };
    let reply =
        ic_wire::native::encode_response(resume::METHOD_ID, &receipt, RESPONSE_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_response::<OperationReceipt>(
            resume::METHOD_ID,
            &reply,
            RESPONSE_LIMITS
        )
        .unwrap()
        .operation_id,
        request.operation_id
    );
}

#[cfg(feature = "wire-schema")]
#[test]
fn activation_request_and_result_have_native_metadata() {
    ic_wire::typescript::native_schema::<()>().unwrap();
    ic_wire::typescript::native_schema::<FleetActivationResumeRequest>().unwrap();
    ic_wire::typescript::native_schema::<Result<OperationReceipt, crate::dto::error::Error>>()
        .unwrap();
}

#[test]
fn status_read_preserves_exact_id_and_cannot_be_an_update() {
    let operation = [23_u8; 32];
    let bytes =
        ic_wire::native::encode_request(status::METHOD_ID, &operation, REQUEST_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<[u8; 32]>(status::METHOD_ID, &bytes, REQUEST_LIMITS)
            .unwrap(),
        operation
    );
    assert!(matches!(
        ic_wire::native::decode_request::<[u8; 32]>(prepare::METHOD_ID, &bytes, REQUEST_LIMITS),
        Err(ic_wire::Error::Identity)
    ));
}

#[cfg(feature = "wire-schema")]
#[test]
fn status_schema_retains_complete_evidence() {
    ic_wire::typescript::native_schema::<
        Result<
            crate::dto::runtime_reporting::FleetActivationStatusResponse,
            crate::dto::error::Error,
        >,
    >()
    .unwrap();
}
