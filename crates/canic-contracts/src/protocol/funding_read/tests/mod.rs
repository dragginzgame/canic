//! Exact funding census identity and complete native schema.
use super::*;
#[test]
fn release_cursor_cannot_alias_pool_census() {
    let request = Some(u64::MAX);
    let bytes =
        ic_wire::native::encode_request(release::METHOD_ID, &request, REQUEST_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<Option<u64>>(release::METHOD_ID, &bytes, REQUEST_LIMITS)
            .unwrap(),
        request
    );
    assert!(matches!(
        ic_wire::native::decode_request::<Option<u64>>(
            crate::protocol::pool_census::METHOD_ID,
            &bytes,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
}
#[cfg(feature = "wire-schema")]
#[test]
fn funding_census_registers_complete_native_evidence() {
    ic_wire::typescript::native_schema::<Option<u64>>().unwrap();
    ic_wire::typescript::native_schema::<
        Result<crate::dto::funding_read::ReleasePage, crate::dto::error::Error>,
    >()
    .unwrap();
}

#[test]
fn root_status_request_is_distinct_from_release_census() {
    let bytes =
        ic_wire::native::encode_request(root_status::METHOD_ID, &(), REQUEST_LIMITS).unwrap();
    ic_wire::native::decode_request::<()>(root_status::METHOD_ID, &bytes, REQUEST_LIMITS).unwrap();
    assert!(matches!(
        ic_wire::native::decode_request::<()>(release::METHOD_ID, &bytes, REQUEST_LIMITS),
        Err(ic_wire::Error::Identity)
    ));
}

#[cfg(feature = "wire-schema")]
#[test]
fn root_status_registers_full_rotation_and_refill_evidence() {
    ic_wire::typescript::native_schema::<
        Result<crate::dto::funding_read::RootStatus, crate::dto::error::Error>,
    >()
    .unwrap();
}

#[test]
fn coordinator_funding_request_cannot_alias_root_funding() {
    let bytes = ic_wire::native::encode_request(coordinator_status::METHOD_ID, &(), REQUEST_LIMITS)
        .unwrap();
    ic_wire::native::decode_request::<()>(coordinator_status::METHOD_ID, &bytes, REQUEST_LIMITS)
        .unwrap();
    assert!(matches!(
        ic_wire::native::decode_request::<()>(root_status::METHOD_ID, &bytes, REQUEST_LIMITS),
        Err(ic_wire::Error::Identity)
    ));
}

#[cfg(feature = "wire-schema")]
#[test]
fn coordinator_funding_registers_complete_rotation_evidence() {
    ic_wire::typescript::native_schema::<
        Result<crate::dto::funding_read::CoordinatorStatus, crate::dto::error::Error>,
    >()
    .unwrap();
}

#[test]
fn operation_read_preserves_exact_id_and_rejects_another_method() {
    let operation_id = [0x42_u8; 32];
    let bytes = ic_wire::native::encode_request(
        super::rotation::METHOD_ID,
        &operation_id,
        super::REQUEST_LIMITS,
    )
    .unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<[u8; 32]>(
            super::rotation::METHOD_ID,
            &bytes,
            super::REQUEST_LIMITS
        )
        .unwrap(),
        operation_id
    );
    assert!(matches!(
        ic_wire::native::decode_request::<[u8; 32]>(
            crate::protocol::public_overview::METHOD_ID,
            &bytes,
            super::REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
}

#[cfg(feature = "wire-schema")]
#[test]
fn operation_read_registers_complete_native_evidence() {
    ic_wire::typescript::native_schema::<
        Result<crate::dto::funding_read::RotationStatus, crate::dto::error::Error>,
    >()
    .unwrap();
}
