//! Exact operation routing and preservation of capability authority across native frames.
use super::*;
use crate::dto::capability::{CapabilityProof, CapabilityRequestMetadata, CapabilityService};

fn envelope(capability: stored::Request) -> RootCapabilityEnvelopeV1 {
    RootCapabilityEnvelopeV1 {
        service: CapabilityService::Root,
        capability_version: 1,
        capability,
        proof: CapabilityProof::Structural,
        metadata: CapabilityRequestMetadata {
            request_id: [7; 32],
            issued_at_ns: 42,
            ttl_ns: 99,
        },
    }
}

#[test]
fn cycles_frame_preserves_amount_and_both_replay_identities() {
    let metadata = Some(stored::RootRequestMetadata {
        request_id: [8; 32],
        ttl_ns: 88,
    });
    let call = encode_request(envelope(stored::Request::Cycles(stored::CyclesRequest {
        cycles: u128::MAX,
        metadata,
    })))
    .unwrap();
    assert_eq!(call.method, wire::cycles::METHOD);
    let decoded: native::CapabilityRequest<stored::CyclesRequest> =
        ic_wire::native::decode_request(call.method_id, &call.bytes, wire::REQUEST_LIMITS).unwrap();
    assert_eq!(decoded.capability.cycles, u128::MAX);
    assert_eq!(decoded.capability.metadata, metadata);
    assert_eq!(decoded.metadata.request_id, [7; 32]);
    assert_eq!(decoded.metadata.issued_at_ns, 42);
    assert_eq!(decoded.metadata.ttl_ns, 99);
    assert_eq!(decoded.proof, CapabilityProof::Structural);
    assert_eq!(decoded.service, CapabilityService::Root);
    assert_eq!(decoded.capability_version, 1);
}

#[test]
fn acknowledgement_and_recycle_responses_cannot_be_substituted() {
    let frame = ic_wire::native::encode_response(
        wire::acknowledge::METHOD_ID,
        &Ok::<(), Error>(()),
        wire::RESPONSE_LIMITS,
    )
    .unwrap();
    let response = decode_response(wire::acknowledge::METHOD_ID, &frame)
        .unwrap()
        .unwrap();
    assert!(acknowledge_response(response.clone()).is_ok());
    assert_eq!(
        recycle_response(response),
        Err(Error::from_registered(
            crate::diagnostics::codes::STATE_INVALID
        ))
    );
    assert!(matches!(
        decode_response(wire::recycle::METHOD_ID, &frame),
        Err(ic_wire::Error::Identity)
    ));
    let mut trailing = frame;
    trailing.push(0);
    assert!(matches!(
        decode_response(wire::acknowledge::METHOD_ID, &trailing),
        Err(ic_wire::Error::Trailing)
    ));
}

#[cfg(feature = "wire-schema")]
#[test]
fn every_operation_has_native_request_and_response_metadata() {
    ic_wire::typescript::native_schema::<
        native::CapabilityRequest<stored::AcknowledgePlacementReceiptRequest>,
    >()
    .unwrap();
    ic_wire::typescript::native_schema::<native::CapabilityRequest<native::CreateCanisterRequest>>(
    )
    .unwrap();
    ic_wire::typescript::native_schema::<native::CapabilityRequest<native::RecycleCanisterRequest>>().unwrap();
    ic_wire::typescript::native_schema::<native::CapabilityRequest<stored::CyclesRequest>>()
        .unwrap();
    ic_wire::typescript::native_schema::<Result<(), Error>>().unwrap();
    ic_wire::typescript::native_schema::<Result<native::CreateCanisterResponse, Error>>().unwrap();
    ic_wire::typescript::native_schema::<Result<stored::CyclesResponse, Error>>().unwrap();
}

#[test]
fn provision_and_recycle_frames_preserve_original_payloads() {
    let pid = ic_platform::Principal::from_slice(&[1, 2, 3]);
    let metadata = Some(stored::RootRequestMetadata {
        request_id: [9; 32],
        ttl_ns: 123,
    });
    let create = stored::CreateCanisterRequest {
        canister_role: crate::ids::CanisterRole::owned("worker".to_owned()),
        parent: stored::CreateCanisterParent::Canister(pid.into()),
        extra_arg: Some(vec![0, 255, 1]),
        metadata,
    };
    for (request, method, id) in [
        (
            stored::Request::AllocatePlacementChild(create.clone()),
            wire::allocate::METHOD,
            wire::allocate::METHOD_ID,
        ),
        (
            stored::Request::CreateCanister(create),
            wire::create::METHOD,
            wire::create::METHOD_ID,
        ),
    ] {
        let call = encode_request(envelope(request)).unwrap();
        assert_eq!((call.method, call.method_id), (method, id));
        let decoded: native::CapabilityRequest<native::CreateCanisterRequest> =
            ic_wire::native::decode_request(id, &call.bytes, wire::REQUEST_LIMITS).unwrap();
        assert!(
            matches!(decoded.capability.parent, native::CreateCanisterParent::Canister(value) if value == pid)
        );
        assert_eq!(decoded.capability.extra_arg, Some(vec![0, 255, 1]));
        assert_eq!(decoded.capability.metadata, metadata);
    }
    let call = encode_request(envelope(stored::Request::RecycleCanister(
        stored::RecycleCanisterRequest {
            allocation_operation_id: [6; 32],
            canister_pid: pid.into(),
            metadata,
        },
    )))
    .unwrap();
    assert_eq!(call.method, wire::recycle::METHOD);
    let decoded: native::CapabilityRequest<native::RecycleCanisterRequest> =
        ic_wire::native::decode_request(call.method_id, &call.bytes, wire::REQUEST_LIMITS).unwrap();
    assert_eq!(decoded.capability.allocation_operation_id, [6; 32]);
    assert_eq!(decoded.capability.canister_pid, pid);
    assert_eq!(decoded.capability.metadata, metadata);
    let call = encode_request(envelope(stored::Request::AcknowledgePlacementReceipt(
        stored::AcknowledgePlacementReceiptRequest {
            operation_id: [6; 32],
            metadata,
        },
    )))
    .unwrap();
    assert_eq!(call.method, wire::acknowledge::METHOD);
    let decoded: native::CapabilityRequest<stored::AcknowledgePlacementReceiptRequest> =
        ic_wire::native::decode_request(call.method_id, &call.bytes, wire::REQUEST_LIMITS).unwrap();
    assert_eq!(decoded.capability.operation_id, [6; 32]);
    assert_eq!(decoded.capability.metadata, metadata);
}
