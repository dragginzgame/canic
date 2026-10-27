//! Native retirement reports preserve exact destinations, amounts and frame correlation.
use super::*;
use crate::dto::{
    store_control::ReclamationResponse,
    template::{
        WasmStoreDeletionCycleReclamationRequest, WasmStoreDeletionCycleReclamationResponse,
    },
};

#[test]
fn reclamation_retains_all_observed_cycle_evidence() {
    let original = WasmStoreDeletionCycleReclamationResponse {
        destination: ic_platform::principal::ExternalPrincipal::from_slice(&[8; 29]),
        cycles_before: u128::MAX,
        retained_cycles_target: 500,
        cycles_transferred: u128::MAX - 500,
        cycles_after: 499,
    };
    let native = ReclamationResponse::from(original);
    let frame =
        ic_wire::native::encode_response(reclaim::METHOD_ID, &native, RESPONSE_LIMITS).unwrap();
    let decoded: ReclamationResponse =
        ic_wire::native::decode_response(reclaim::METHOD_ID, &frame, RESPONSE_LIMITS).unwrap();
    assert_eq!(
        WasmStoreDeletionCycleReclamationResponse::from(decoded),
        original
    );
    assert!(matches!(
        ic_wire::native::decode_response::<ReclamationResponse>(
            grant::METHOD_ID,
            &frame,
            RESPONSE_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
}

#[test]
fn reclamation_frame_keeps_retained_target_and_rejects_trailing_data() {
    let request = WasmStoreDeletionCycleReclamationRequest {
        retained_cycles_target: u128::MAX,
    };
    let mut frame =
        ic_wire::native::encode_request(reclaim::METHOD_ID, &request, REQUEST_LIMITS).unwrap();
    let decoded: WasmStoreDeletionCycleReclamationRequest =
        ic_wire::native::decode_request(reclaim::METHOD_ID, &frame, REQUEST_LIMITS).unwrap();
    assert_eq!(decoded, request);
    frame.push(0);
    assert!(matches!(
        ic_wire::native::decode_request::<WasmStoreDeletionCycleReclamationRequest>(
            reclaim::METHOD_ID,
            &frame,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Trailing)
    ));
}

#[cfg(feature = "wire-schema")]
#[test]
fn control_graphs_have_native_metadata() {
    ic_wire::typescript::native_schema::<crate::dto::store_control::FixtureGrantRequest>().unwrap();
    ic_wire::typescript::native_schema::<
        Result<
            Result<
                crate::dto::runtime_reporting::FixtureGrant,
                crate::dto::fixture_provisioning::FixtureStoreError,
            >,
            crate::dto::error::Error,
        >,
    >()
    .unwrap();
    ic_wire::typescript::native_schema::<WasmStoreDeletionCycleReclamationRequest>().unwrap();
    ic_wire::typescript::native_schema::<Result<ReclamationResponse, crate::dto::error::Error>>()
        .unwrap();
}
