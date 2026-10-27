//! Native envelope admission preserves replay identity, caller-owned amounts and typed funding outcomes.
use super::*;
use crate::dto::{capability::*, rpc::*};

#[test]
fn native_request_preserves_original_replay_metadata_and_full_cycle_amount() {
    let request = NonrootCyclesCapabilityEnvelopeV1 {
        service: CapabilityService::Root,
        capability_version: CAPABILITY_VERSION_V1,
        proof: CapabilityProof::Structural,
        metadata: CapabilityRequestMetadata {
            request_id: [3; 32],
            issued_at_ns: 12,
            ttl_ns: 34,
        },
        capability: CyclesRequest {
            cycles: u128::MAX,
            metadata: Some(RootRequestMetadata {
                request_id: [4; 32],
                ttl_ns: 56,
            }),
        },
    };
    let frame = ic_wire::native::encode_request(METHOD_ID, &request, REQUEST_LIMITS).unwrap();
    let decoded: NonrootCyclesCapabilityEnvelopeV1 =
        ic_wire::native::decode_request(METHOD_ID, &frame, REQUEST_LIMITS).unwrap();
    assert_eq!(decoded.service, request.service);
    assert_eq!(decoded.capability_version, request.capability_version);
    assert_eq!(decoded.proof, request.proof);
    assert_eq!(decoded.metadata, request.metadata);
    assert_eq!(decoded.capability.cycles, request.capability.cycles);
    assert_eq!(decoded.capability.metadata, request.capability.metadata);
    assert!(matches!(
        ic_wire::native::decode_request::<NonrootCyclesCapabilityEnvelopeV1>(
            METHOD_ID + 1,
            &frame,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
    let mut trailing = frame;
    trailing.push(0);
    assert!(matches!(
        ic_wire::native::decode_request::<NonrootCyclesCapabilityEnvelopeV1>(
            METHOD_ID,
            &trailing,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Trailing)
    ));
}

#[test]
fn native_reply_distinguishes_transfers_and_all_preflight_refusals() {
    for response in [
        CyclesResponse::Transferred {
            cycles_transferred: u128::MAX,
        },
        CyclesResponse::PreflightRejected(CyclesFundingPreflightResponse::ChildBudgetExhausted {
            remaining_child_budget: 10,
            max_per_child: 20,
        }),
        CyclesResponse::PreflightRejected(CyclesFundingPreflightResponse::CooldownActive {
            retry_after_secs: u64::MAX,
        }),
        CyclesResponse::PreflightRejected(
            CyclesFundingPreflightResponse::ParentFundingUnavailable {
                approved_cycles: 30,
            },
        ),
    ] {
        let frame = ic_wire::native::encode_response(
            METHOD_ID,
            &Ok::<_, crate::dto::error::Error>(NonrootCyclesCapabilityResponseV1 { response }),
            RESPONSE_LIMITS,
        )
        .unwrap();
        let decoded: Result<NonrootCyclesCapabilityResponseV1, crate::dto::error::Error> =
            ic_wire::native::decode_response(METHOD_ID, &frame, RESPONSE_LIMITS).unwrap();
        assert_eq!(decoded.unwrap().response, response);
    }
}

#[cfg(feature = "wire-schema")]
#[test]
fn request_and_reply_have_complete_native_metadata() {
    ic_wire::typescript::native_schema::<NonrootCyclesCapabilityEnvelopeV1>().unwrap();
    ic_wire::typescript::native_schema::<
        Result<NonrootCyclesCapabilityResponseV1, crate::dto::error::Error>,
    >()
    .unwrap();
}
