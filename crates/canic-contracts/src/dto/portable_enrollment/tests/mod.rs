//! Bounded enrollment frames, exact method identity and native schema generation.

use super::*;
use crate::protocol::portable_enrollment as wire;

fn snapshot() -> PortableEnrollmentSnapshot {
    PortableEnrollmentSnapshot {
        authority: Principal::from_slice(&[1]),
        receiver: Principal::from_slice(&[2]),
        fleet: FleetKey {
            canonical_network_id: crate::ids::CanonicalNetworkId::ic_mainnet(),
            fleet_id: crate::ids::FleetId::from_generated_bytes([1; 32]),
        },
        boot_challenge: [9; 32],
        revision: 42,
        generation: 7,
        issued_at_ns: 10,
        expires_at_ns: 100,
        entries: (0..wire::MAX_ENTRIES)
            .map(|id| PortableIssuerEnrollment {
                issuer: Principal::from_slice(&[3]),
                key: PortableIssuerKey::Ed25519 {
                    public_key: [u8::try_from(id).unwrap(); 32],
                },
                not_before_ns: 1,
                expires_at_ns: 100,
                grants: vec![PortableEnrollmentGrant {
                    role: "receiver".into(),
                    scopes: vec!["read".into()],
                }],
                deployment_approval_digest: [8; 32],
                deployment_approval_expires_at_ns: 100,
            })
            .collect(),
        retired_keys: (0..wire::MAX_RETIRED_KEYS)
            .map(|id| {
                let mut key = [255; 32];
                key[30..].copy_from_slice(&u16::try_from(id).unwrap().to_be_bytes());
                key
            })
            .collect(),
    }
}

#[test]
fn portable_enrollment_request_and_complete_response_round_trip_under_owner_limits() {
    let request = PortableEnrollmentRequest {
        boot_challenge: [9; 32],
    };
    let bytes =
        ic_wire::native::encode_request(wire::METHOD_ID, &request, wire::REQUEST_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<PortableEnrollmentRequest>(
            wire::METHOD_ID,
            &bytes,
            wire::REQUEST_LIMITS
        )
        .unwrap(),
        request
    );
    assert!(
        ic_wire::native::decode_request::<PortableEnrollmentRequest>(
            wire::METHOD_ID + 1,
            &bytes,
            wire::REQUEST_LIMITS
        )
        .is_err()
    );
    let value = snapshot();
    let response: Result<_, &crate::dto::error::Error> = Ok(&value);
    let bytes = ic_wire::native::encode_response(wire::METHOD_ID, &response, wire::RESPONSE_LIMITS)
        .unwrap();
    let decoded: Result<PortableEnrollmentSnapshot, crate::dto::error::Error> =
        ic_wire::native::decode_response(wire::METHOD_ID, &bytes, wire::RESPONSE_LIMITS).unwrap();
    assert_eq!(decoded.unwrap(), value);
    let exact = ic_wire::Limits {
        bytes: bytes.len(),
        ..wire::RESPONSE_LIMITS
    };
    assert!(
        ic_wire::native::decode_response::<
            Result<PortableEnrollmentSnapshot, crate::dto::error::Error>,
        >(wire::METHOD_ID, &bytes, exact)
        .is_ok()
    );
    let too_small = ic_wire::Limits {
        bytes: bytes.len() - 1,
        ..wire::RESPONSE_LIMITS
    };
    assert!(
        ic_wire::native::decode_response::<
            Result<PortableEnrollmentSnapshot, crate::dto::error::Error>,
        >(wire::METHOD_ID, &bytes, too_small)
        .is_err()
    );
    let mut trailing = bytes;
    trailing.push(0);
    assert!(matches!(
        ic_wire::native::decode_response::<
            Result<PortableEnrollmentSnapshot, crate::dto::error::Error>,
        >(wire::METHOD_ID, &trailing, wire::RESPONSE_LIMITS),
        Err(ic_wire::Error::Trailing)
    ));
}

#[cfg(feature = "wire-schema")]
#[test]
fn portable_enrollment_native_schema_covers_both_directions() {
    ic_wire::typescript::native_schema::<PortableEnrollmentRequest>().unwrap();
    ic_wire::typescript::native_schema::<
        Result<PortableEnrollmentSnapshot, crate::dto::error::Error>,
    >()
    .unwrap();
}
