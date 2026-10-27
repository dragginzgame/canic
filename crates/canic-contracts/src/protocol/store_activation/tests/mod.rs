//! Credential and activation frames bind exact operation evidence and method identity.
use super::*;
use crate::dto::{
    error::Error,
    fleet_activation::{
        FleetActivationRequest, FleetCredentialGenerationRef, FleetCredentialGenerationRequest,
    },
    role::OperationReceipt,
};

#[test]
fn activation_preserves_generation_and_evidence() {
    let credential = FleetCredentialGenerationRef {
        generation: 7,
        manifest_hash: [2; 32],
    };
    let preparation = FleetCredentialGenerationRequest {
        operation_id: [1; 32],
        credential,
    };
    let frame =
        ic_wire::native::encode_request(prepare::METHOD_ID, &preparation, REQUEST_LIMITS).unwrap();
    let decoded: FleetCredentialGenerationRequest =
        ic_wire::native::decode_request(prepare::METHOD_ID, &frame, REQUEST_LIMITS).unwrap();
    assert_eq!(decoded, preparation);
    assert!(matches!(
        ic_wire::native::decode_request::<FleetCredentialGenerationRequest>(
            activate::METHOD_ID,
            &frame,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
    let request = FleetActivationRequest {
        operation_id: preparation.operation_id,
        credential,
        activation_evidence_hash: [3; 32],
    };
    let mut frame =
        ic_wire::native::encode_request(activate::METHOD_ID, &request, REQUEST_LIMITS).unwrap();
    let decoded: FleetActivationRequest =
        ic_wire::native::decode_request(activate::METHOD_ID, &frame, REQUEST_LIMITS).unwrap();
    assert_eq!(decoded, request);
    frame.push(0);
    assert!(matches!(
        ic_wire::native::decode_request::<FleetActivationRequest>(
            activate::METHOD_ID,
            &frame,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Trailing)
    ));
}

#[test]
fn receipt_retains_original_operation() {
    for id in [prepare::METHOD_ID, activate::METHOD_ID] {
        let reply: Result<OperationReceipt, Error> = Ok(OperationReceipt {
            operation_id: [9; 32],
        });
        let frame = ic_wire::native::encode_response(id, &reply, RESPONSE_LIMITS).unwrap();
        let decoded: Result<OperationReceipt, Error> =
            ic_wire::native::decode_response(id, &frame, RESPONSE_LIMITS).unwrap();
        assert_eq!(decoded.unwrap().operation_id, [9; 32]);
    }
}

#[cfg(feature = "wire-schema")]
#[test]
fn activation_graphs_have_native_metadata() {
    ic_wire::typescript::native_schema::<FleetCredentialGenerationRequest>().unwrap();
    ic_wire::typescript::native_schema::<FleetActivationRequest>().unwrap();
    ic_wire::typescript::native_schema::<Result<OperationReceipt, Error>>().unwrap();
}
