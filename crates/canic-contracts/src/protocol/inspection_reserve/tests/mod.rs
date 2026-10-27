//! The actual bounded native contract preserves authority and maximum cycle values.
use super::*;
use crate::dto::{
    canister::CanisterInspectionReserveResponse, error::Error,
    inspection_reserve::InspectionReserve,
};
use ic_platform::Principal;
#[test]
fn reserve_round_trip_preserves_authority_and_rejects_wrong_frames() {
    let target = Principal::from_slice(&[7; 29]);
    let bytes = ic_wire::native::encode_request(METHOD_ID, &target, REQUEST_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<Principal>(METHOD_ID, &bytes, REQUEST_LIMITS).unwrap(),
        target
    );
    let original = CanisterInspectionReserveResponse {
        caller: Principal::from_slice(&[1]).into(),
        canister_id: target.into(),
        native_cycles: u128::MAX,
        available_liquid_cycles: u128::MAX - 1,
        required_liquid_cycles: u128::MAX - 2,
    };
    let reply: Result<InspectionReserve, Error> = Ok(original.clone().into());
    let bytes = ic_wire::native::encode_response(METHOD_ID, &reply, RESPONSE_LIMITS).unwrap();
    let decoded: Result<InspectionReserve, Error> =
        ic_wire::native::decode_response(METHOD_ID, &bytes, RESPONSE_LIMITS).unwrap();
    assert_eq!(
        CanisterInspectionReserveResponse::from(decoded.unwrap()),
        original
    );
    assert_eq!(
        ic_wire::native::decode_response::<Result<InspectionReserve, Error>>(
            METHOD_ID + 1,
            &bytes,
            RESPONSE_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    );
    let mut trailing = bytes;
    trailing.push(0);
    assert_eq!(
        ic_wire::native::decode_response::<Result<InspectionReserve, Error>>(
            METHOD_ID,
            &trailing,
            RESPONSE_LIMITS
        ),
        Err(ic_wire::Error::Trailing)
    );
    assert_eq!(
        ic_wire::native::decode_request::<Principal>(
            METHOD_ID,
            &[0; REQUEST_LIMITS.bytes + 1],
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Limit)
    );
}
#[cfg(feature = "wire-schema")]
#[test]
fn reserve_schema_uses_native_principals() {
    ic_wire::typescript::native_schema::<Principal>().unwrap();
    ic_wire::typescript::native_schema::<Result<InspectionReserve, Error>>().unwrap();
}
