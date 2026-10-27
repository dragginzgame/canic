//! Retry transport preserves exact identity and rejects frames for another operation.
use super::*;
use crate::dto::{error::Error, pool::PoolAdminCommand, pool_retry::*};
use ic_platform::Principal;

#[test]
fn reset_selection_preserves_exact_canister() {
    let original = ResetRequest {
        canister_id: Principal::from_slice(&[7; 29]),
    };
    let bytes =
        ic_wire::native::encode_request(reset::METHOD_ID, &original, REQUEST_LIMITS).unwrap();
    let decoded: ResetRequest =
        ic_wire::native::decode_request(reset::METHOD_ID, &bytes, REQUEST_LIMITS).unwrap();
    assert_eq!(decoded, original);
    let PoolAdminCommand::RetryReset { canister_id } = decoded.into() else {
        panic!("reset selection must produce a reset command");
    };
    let response = ResetResponse::from(canister_id);
    assert_eq!(response.canister_id, original.canister_id);
    assert!(matches!(
        ic_wire::native::decode_request::<ResetRequest>(refill::METHOD_ID, &bytes, REQUEST_LIMITS),
        Err(ic_wire::Error::Identity)
    ));
    let mut trailing = bytes;
    trailing.push(0);
    assert!(matches!(
        ic_wire::native::decode_request::<ResetRequest>(
            reset::METHOD_ID,
            &trailing,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Trailing)
    ));
}

#[test]
fn refill_retains_original_operation_receipt() {
    let request = ic_wire::native::encode_request(refill::METHOD_ID, &(), REQUEST_LIMITS).unwrap();
    ic_wire::native::decode_request::<()>(refill::METHOD_ID, &request, REQUEST_LIMITS).unwrap();
    let bytes = ic_wire::native::encode_response(
        refill::METHOD_ID,
        &Ok::<_, Error>(RefillResponse {
            previous_operation_id: [19; 32],
        }),
        RESPONSE_LIMITS,
    )
    .unwrap();
    let response: Result<RefillResponse, Error> =
        ic_wire::native::decode_response(refill::METHOD_ID, &bytes, RESPONSE_LIMITS).unwrap();
    assert_eq!(response.unwrap().previous_operation_id, [19; 32]);
}

#[cfg(feature = "wire-schema")]
#[test]
fn retries_have_complete_native_metadata() {
    ic_wire::typescript::native_schema::<ResetRequest>().unwrap();
    ic_wire::typescript::native_schema::<Result<ResetResponse, Error>>().unwrap();
    ic_wire::typescript::native_schema::<Result<RefillResponse, Error>>().unwrap();
}
