use super::*;
use crate::dto::{
    cycle_drain::{DrainRequest, DrainResponse},
    error::Error,
};

#[test]
fn original_drain_authority_and_full_range_amount_round_trip() {
    let request = DrainRequest {
        operation_id: "original-operation".into(),
        destination: ic_platform_types::Principal::from_slice(&[9; 29]),
        cycles: u128::MAX,
    };
    let bytes = ic_wire::native::encode_request(METHOD_ID, &request, REQUEST_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<DrainRequest>(METHOD_ID, &bytes, REQUEST_LIMITS).unwrap(),
        request
    );
    for response in [
        DrainResponse::Accepted {
            transferred_cycles: u128::MAX,
        },
        DrainResponse::Replayed {
            transferred_cycles: u128::MAX,
        },
    ] {
        let reply: Result<DrainResponse, Error> = Ok(response);
        let bytes = ic_wire::native::encode_response(METHOD_ID, &reply, RESPONSE_LIMITS).unwrap();
        assert_eq!(
            ic_wire::native::decode_response::<Result<DrainResponse, Error>>(
                METHOD_ID,
                &bytes,
                RESPONSE_LIMITS
            )
            .unwrap(),
            reply
        );
        assert_eq!(
            ic_wire::native::decode_response::<Result<DrainResponse, Error>>(
                METHOD_ID + 1,
                &bytes,
                RESPONSE_LIMITS
            ),
            Err(ic_wire::Error::Identity)
        );
    }
}
