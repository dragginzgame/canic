//! Exact native method identity and controller reserve evidence are separate from management.
use super::*;
use crate::dto::{
    error::Error,
    inspection::{Request, Response},
    inspection_reserve::InspectionReserve,
};

#[test]
fn target_and_typed_refusal_round_trip_under_inspection_identity() {
    let target = ic_platform::Principal::from_slice(&[7; 29]);
    let request = Request {
        canister_id: target,
    };
    let frame = ic_wire::native::encode_request(METHOD_ID, &request, REQUEST_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<Request>(METHOD_ID, &frame, REQUEST_LIMITS).unwrap(),
        request
    );
    assert!(matches!(
        ic_wire::native::decode_request::<Request>(
            crate::protocol::inspection_history::METHOD_ID,
            &frame,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
    let response: Result<Response, Error> =
        Ok(Response::InspectionReserveRequired(InspectionReserve {
            caller: ic_platform::Principal::from_slice(&[8; 29]),
            canister_id: target,
            native_cycles: u128::MAX,
            available_liquid_cycles: 7,
            required_liquid_cycles: 8,
        }));
    let frame = ic_wire::native::encode_response(METHOD_ID, &response, RESPONSE_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_response::<Result<Response, Error>>(
            METHOD_ID,
            &frame,
            RESPONSE_LIMITS
        )
        .unwrap(),
        response
    );
    assert!(matches!(
        ic_wire::native::decode_request::<Request>(METHOD_ID, &frame, REQUEST_LIMITS),
        Err(ic_wire::Error::Identity)
    ));
}
