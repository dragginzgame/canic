//! Native administration preserves issuer identity and exact grant/lease policy.
use super::*;

#[test]
fn configure_roundtrip_preserves_current_authority_policy() {
    let original = stored::RootIssuerConfigureRequest {
        issuer_pid: ic_platform::principal::ExternalPrincipal::from_slice(&[7]),
        enabled: true,
        aud: stored::DelegationAudience::Fleet(crate::ids::FleetKey {
            canonical_network_id: crate::ids::CanonicalNetworkId::ic_mainnet(),
            fleet_id: crate::ids::FleetId::from_generated_bytes([8; 32]),
        }),
        grants: vec![stored::DelegatedRoleGrant {
            target: crate::ids::CanisterRole::owned("receiver".to_owned()),
            scopes: vec!["read".to_owned()],
        }],
        cert_ttl_ns: 123,
        refresh_after_ratio_bps: 456,
    };
    let request = native::RootIssuerConfigureRequest::from(original.clone());
    let protocol = crate::protocol::issuer_control::configure::METHOD_ID;
    let limits = crate::protocol::issuer_control::REQUEST_LIMITS;
    let frame = ic_wire::native::encode_request(protocol, &request, limits).unwrap();
    let decoded: native::RootIssuerConfigureRequest =
        ic_wire::native::decode_request(protocol, &frame, limits).unwrap();
    assert_eq!(stored::RootIssuerConfigureRequest::from(decoded), original);
    let mut trailing = frame;
    trailing.push(0);
    assert!(matches!(
        ic_wire::native::decode_request::<native::RootIssuerConfigureRequest>(
            protocol, &trailing, limits
        ),
        Err(ic_wire::Error::Trailing)
    ));
}

#[cfg(feature = "wire-schema")]
#[test]
fn issuer_control_method_graphs_are_complete() {
    ic_wire::typescript::native_schema::<native::RootIssuerConfigureRequest>().unwrap();
    ic_wire::typescript::native_schema::<stored::RootChainKeyPublicKeyRequest>().unwrap();
    ic_wire::typescript::native_schema::<
        Result<native::RootIssuerConfigureResponse, crate::dto::error::Error>,
    >()
    .unwrap();
    ic_wire::typescript::native_schema::<
        Result<native::RootDelegationProofBatchProof, crate::dto::error::Error>,
    >()
    .unwrap();
}
