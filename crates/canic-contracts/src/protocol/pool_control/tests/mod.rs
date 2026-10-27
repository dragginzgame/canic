//! Physical import keeps exact target and typed reset outcomes under native framing.
use super::*;
use crate::dto::{error::Error, pool::PoolImportResponse, pool_control::ImportRequest};

#[test]
fn physical_import_preserves_target_and_reset_outcome() {
    let request = ImportRequest {
        canister_id: ic_platform::Principal::from_slice(&[5; 29]),
    };
    let bytes =
        ic_wire::native::encode_request(import::METHOD_ID, &request, REQUEST_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<ImportRequest>(import::METHOD_ID, &bytes, REQUEST_LIMITS)
            .unwrap(),
        request
    );
    assert!(matches!(
        ic_wire::native::decode_request::<ImportRequest>(
            handoff::METHOD_ID,
            &bytes,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
    for outcome in [
        PoolImportResponse::Imported {
            canister_id: request.canister_id,
        },
        PoolImportResponse::ResetFailed {
            canister_id: request.canister_id,
            reason: "management refusal".into(),
        },
    ] {
        let response: Result<PoolImportResponse, Error> = Ok(outcome);
        let bytes = ic_wire::native::encode_response(import::METHOD_ID, &response, RESPONSE_LIMITS)
            .unwrap();
        assert_eq!(
            ic_wire::native::decode_response::<Result<PoolImportResponse, Error>>(
                import::METHOD_ID,
                &bytes,
                RESPONSE_LIMITS
            )
            .unwrap(),
            response
        );
    }
}

#[cfg(feature = "wire-schema")]
#[test]
fn physical_import_has_complete_native_metadata() {
    ic_wire::typescript::native_schema::<ImportRequest>().unwrap();
    ic_wire::typescript::native_schema::<Result<PoolImportResponse, Error>>().unwrap();
}
