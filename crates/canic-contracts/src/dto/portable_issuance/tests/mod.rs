//! Bounded issuer frames and upstream credential schema propagation.

use super::*;
use crate::protocol::portable_issuance as wire;
use ic_auth_protocol_types::limits;

#[test]
fn maximum_canonical_grant_request_fits_the_issuer_frame() {
    let request = IssuePortableCredential {
        epoch: 1,
        operation_id: [1; 32],
        login_epoch: 1,
        login_id: [2; 32],
        browser_public_key: [3; 32],
        grants: (0..limits::MAX_GRANTS)
            .map(|role| PortableEnrollmentGrant {
                role: format!("role{role:02}"),
                scopes: (0..limits::MAX_SCOPES_PER_GRANT)
                    .map(|scope| {
                        format!("scope{scope:02}{}", "x".repeat(limits::MAX_LABEL_BYTES - 7))
                    })
                    .collect(),
            })
            .collect(),
    };
    let bytes = ic_wire::native::encode_request(1, &request, wire::REQUEST_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<IssuePortableCredential>(1, &bytes, wire::REQUEST_LIMITS)
            .unwrap(),
        request
    );
    assert!(
        ic_wire::native::decode_request::<IssuePortableCredential>(2, &bytes, wire::REQUEST_LIMITS)
            .is_err()
    );
}

#[test]
fn upstream_maximum_credential_envelope_fits_original_result_and_lookup() {
    let credential: ic_auth_wire::model::PortableCredential =
        ic_auth_protocol_types::PortableCredential {
            claims: vec![0; limits::MAX_CLAIMS_BYTES],
            proof: ic_auth_protocol_types::PortableProof::Ed25519 {
                signature: vec![0; 64],
            },
        }
        .try_into()
        .unwrap();
    let result = PortableCredentialIssuanceStatus::Issued(IssuedPortableCredential {
        credential,
        admission_deadline_ns: 123,
    });
    let bytes = ic_wire::native::encode_response(1, &result, wire::RESPONSE_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_response::<PortableCredentialIssuanceStatus>(
            1,
            &bytes,
            wire::RESPONSE_LIMITS
        )
        .unwrap(),
        result
    );
    let mut trailing = bytes;
    trailing.push(0);
    assert!(matches!(
        ic_wire::native::decode_response::<PortableCredentialIssuanceStatus>(
            1,
            &trailing,
            wire::RESPONSE_LIMITS
        ),
        Err(ic_wire::Error::Trailing)
    ));
}

#[cfg(feature = "wire-schema")]
#[test]
fn issuer_schema_uses_upstream_credential_and_both_typed_results() {
    ic_wire::typescript::native_schema::<PortableIssuanceContext>().unwrap();
    ic_wire::typescript::native_schema::<PortableIssuanceMaintenance>().unwrap();
    ic_wire::typescript::native_schema::<IssuePortableCredential>().unwrap();
    ic_wire::typescript::native_schema::<ReconcilePortableCredential>().unwrap();
    ic_wire::typescript::native_schema::<
        Result<IssuedPortableCredential, PortableCredentialIssuanceRefusal>,
    >()
    .unwrap();
    ic_wire::typescript::native_schema::<
        Result<PortableCredentialIssuanceStatus, PortableCredentialIssuanceRefusal>,
    >()
    .unwrap();
}
