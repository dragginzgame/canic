//! Native retained refill identity and complete evidence.
use super::*;

#[test]
fn status_read_preserves_exact_id_and_cannot_be_an_update() {
    let operation = [23_u8; 32];
    let bytes =
        ic_wire::native::encode_request(status::METHOD_ID, &operation, REQUEST_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<[u8; 32]>(status::METHOD_ID, &bytes, REQUEST_LIMITS)
            .unwrap(),
        operation
    );
    assert!(matches!(
        ic_wire::native::decode_request::<[u8; 32]>(execute::METHOD_ID, &bytes, REQUEST_LIMITS),
        Err(ic_wire::Error::Identity)
    ));
}

#[cfg(feature = "wire-schema")]
#[test]
fn status_schema_retains_complete_evidence() {
    ic_wire::typescript::native_schema::<
        Result<crate::dto::funding_read::RefillResult, crate::dto::error::Error>,
    >()
    .unwrap();
}
