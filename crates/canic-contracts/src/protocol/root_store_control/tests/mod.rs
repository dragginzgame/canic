//! Exact native Store observation identities and complete schema graphs.
use super::*;

#[test]
fn store_observations_preserve_operation_ids_and_reject_other_methods() {
    let operation_id = [21_u8; 32];
    for (method, other) in [
        (adoption::METHOD_ID, bootstrap_status::METHOD_ID),
        (bootstrap_status::METHOD_ID, adoption::METHOD_ID),
    ] {
        let bytes = ic_wire::native::encode_request(method, &operation_id, REQUEST_LIMITS).unwrap();
        assert_eq!(
            ic_wire::native::decode_request::<[u8; 32]>(method, &bytes, REQUEST_LIMITS).unwrap(),
            operation_id
        );
        assert!(matches!(
            ic_wire::native::decode_request::<[u8; 32]>(other, &bytes, REQUEST_LIMITS),
            Err(ic_wire::Error::Identity)
        ));
    }
}

#[cfg(feature = "wire-schema")]
#[test]
fn store_observation_schemas_include_authority_catalog_and_fixtures() {
    ic_wire::typescript::native_schema::<
        Result<crate::dto::root_store_control::Adoption, crate::dto::error::Error>,
    >()
    .unwrap();
    ic_wire::typescript::native_schema::<
        Result<crate::dto::root_store_control::Bootstrap, crate::dto::error::Error>,
    >()
    .unwrap();
}
