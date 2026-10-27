//! Native transport must preserve signed payloads and explicit caller/audience bindings.
use super::*;
use crate::{dto::auth, protocol::role_attestation as wire};

#[test]
fn signed_attestation_roundtrip_preserves_canonical_payload_and_certificate_bytes() {
    let original = auth::SignedRoleAttestation {
        payload: auth::RoleAttestation {
            subject: ic_platform::principal::ExternalPrincipal::from_slice(&[1]),
            role: crate::ids::CanisterRole::owned("worker".to_owned()),
            subnet_id: Some(ic_platform::principal::ExternalPrincipal::from_slice(&[2])),
            audience: ic_platform::principal::ExternalPrincipal::from_slice(&[3]),
            issued_at_ns: 4,
            expires_at_ns: 5,
            epoch: 6,
        },
        root_proof: auth::RoleAttestationRootProof::IcCanisterSignatureV1(
            auth::IcCanisterSignatureProofV1 {
                signature_cbor: vec![0, 255, 1],
                public_key_der: vec![3, 0, 2],
            },
        ),
    };
    let canonical = crate::protocol::authority::encode(&original).unwrap();
    let native = native::SignedRoleAttestation::from(original.clone());
    let frame =
        ic_wire::native::encode_response(wire::get::METHOD_ID, &native, wire::RESPONSE_LIMITS)
            .unwrap();
    let decoded: native::SignedRoleAttestation =
        ic_wire::native::decode_response(wire::get::METHOD_ID, &frame, wire::RESPONSE_LIMITS)
            .unwrap();
    let restored = auth::SignedRoleAttestation::from(decoded);
    assert_eq!(restored, original);
    assert_eq!(
        crate::protocol::authority::encode(restored).unwrap(),
        canonical
    );
    assert!(matches!(
        ic_wire::native::decode_response::<native::SignedRoleAttestation>(
            wire::prepare::METHOD_ID,
            &frame,
            wire::RESPONSE_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
}

#[cfg(feature = "wire-schema")]
#[test]
fn attestation_method_graphs_are_complete() {
    ic_wire::typescript::native_schema::<native::RoleAttestationRequest>().unwrap();
    ic_wire::typescript::native_schema::<auth::RoleAttestationGetRequest>().unwrap();
    ic_wire::typescript::native_schema::<
        Result<native::RoleAttestationPrepareResponse, crate::dto::error::Error>,
    >()
    .unwrap();
    ic_wire::typescript::native_schema::<
        Result<native::SignedRoleAttestation, crate::dto::error::Error>,
    >()
    .unwrap();
}
