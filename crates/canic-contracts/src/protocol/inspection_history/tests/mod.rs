//! Native transport preserves the exact external management bytes without reinterpreting them.
use super::*;
use crate::dto::{canister::CanisterHistoryResponse, error::Error, inspection_history::Request};

#[test]
fn history_frame_preserves_external_evidence_and_target() {
    let request = Request {
        canister_id: ic_platform::Principal::from_slice(&[3; 29]),
    };
    let bytes = ic_wire::native::encode_request(METHOD_ID, &request, REQUEST_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<Request>(METHOD_ID, &bytes, REQUEST_LIMITS).unwrap(),
        request
    );
    let response: Result<CanisterHistoryResponse, Error> = Ok(CanisterHistoryResponse {
        canister_id: request.canister_id,
        history_candid: vec![0, 255, 68, 73, 68, 76],
    });
    let bytes = ic_wire::native::encode_response(METHOD_ID, &response, RESPONSE_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_response::<Result<CanisterHistoryResponse, Error>>(
            METHOD_ID,
            &bytes,
            RESPONSE_LIMITS
        )
        .unwrap(),
        response
    );
    assert!(matches!(
        ic_wire::native::decode_request::<Request>(METHOD_ID, &bytes, REQUEST_LIMITS),
        Err(ic_wire::Error::Identity)
    ));
}

#[cfg(feature = "wire-schema")]
#[test]
fn history_has_complete_native_metadata() {
    ic_wire::typescript::native_schema::<Request>().unwrap();
    ic_wire::typescript::native_schema::<Result<CanisterHistoryResponse, Error>>().unwrap();
}
