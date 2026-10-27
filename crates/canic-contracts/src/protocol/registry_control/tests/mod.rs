//! Exact read identity and the complete synchronization receipt schema.
use super::*;
#[test]
fn synchronization_read_cannot_alias_its_mutation() {
    let operation = [24_u8; 32];
    let bytes = ic_wire::native::encode_request(
        synchronization_status::METHOD_ID,
        &operation,
        REQUEST_LIMITS,
    )
    .unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<[u8; 32]>(
            synchronization_status::METHOD_ID,
            &bytes,
            REQUEST_LIMITS
        )
        .unwrap(),
        operation
    );
    assert!(matches!(
        ic_wire::native::decode_request::<[u8; 32]>(synchronize::METHOD_ID, &bytes, REQUEST_LIMITS),
        Err(ic_wire::Error::Identity)
    ));
}
#[cfg(feature = "wire-schema")]
#[test]
fn synchronization_schema_contains_full_directory_and_acknowledgement() {
    ic_wire::typescript::native_schema::<
        Result<crate::dto::registry_control::SynchronizationStatus, crate::dto::error::Error>,
    >()
    .unwrap();
}
