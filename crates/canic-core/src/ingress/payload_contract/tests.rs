use super::*;

#[test]
fn declaration_carries_current_inspector_policy_without_altering_service() {
    let service = "service : { query_method : () -> () query; update_method : () -> (); };";
    let annotated = annotate_candid(service.into());
    let (body, encoded) = annotated
        .split_once(CANDID_PAYLOAD_CONTRACT_PREFIX)
        .unwrap();
    assert_eq!(body.trim(), service);
    let bytes = crate::cdk::utils::hash::decode_hex(encoded.trim()).unwrap();
    let contract: PayloadContract = serde_cbor::from_reader(bytes.as_slice()).unwrap();
    assert_eq!(contract.schema_version, 1);
    assert_eq!(
        contract.default_update_ingress_max_bytes,
        super::super::payload::DEFAULT_UPDATE_INGRESS_MAX_BYTES as u64
    );
    assert_eq!(
        contract.update_overrides,
        super::super::payload::declaration_limits()
    );
}
