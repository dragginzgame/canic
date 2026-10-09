use super::{
    AuthRequestMetadata, DelegatedRoleGrant, DelegatedTokenClaims, DelegatedTokenGetRequest,
    DelegatedTokenPrepareRequest, DelegatedTokenPrepareResponse, DelegationAudience,
};
use crate::{cdk::types::Principal, ids::CanisterRole, test::support::fleet_key};
use candid::CandidType;
use ic_auth_protocol_types as protocol;

#[derive(CandidType, serde::Serialize)]
struct MalformedAudience {
    canonical_network_id: String,
    fleet_id: String,
}

#[test]
fn auth_dtos_remain_passive_boundary_types() {
    let production_source = concat!(
        include_str!("attestation.rs"),
        include_str!("common.rs"),
        include_str!("proof.rs"),
        include_str!("renewal.rs"),
        include_str!("token.rs"),
    );

    for marker in [
        "impl DelegatedToken",
        "impl DelegatedTokenClaims",
        "impl RoleAttestation",
        "impl SignedRoleAttestation",
        "fn verify",
        "fn sign",
        "fn resolve",
        "fn replay",
        "fn consume",
        "fn policy",
        "fn validate",
    ] {
        assert!(
            !production_source.contains(marker),
            "auth DTOs must stay passive; found marker `{marker}`"
        );
    }
}

#[test]
fn delegated_token_candid_binds_claim_identities_and_derives_request_identities() {
    let claims = DelegatedTokenClaims::_ty().to_string();
    assert!(claims.contains("presenter : principal"));
    assert!(claims.contains("subject : principal"));

    let prepare = DelegatedTokenPrepareRequest::_ty().to_string();
    assert!(!prepare.contains("presenter : principal"));
    assert!(!prepare.contains("subject : principal"));
}

#[test]
fn prepare_envelopes_share_ic_auth_wire_bytes_in_both_directions() {
    let fleet = fleet_key(7);
    for metadata in [
        None,
        Some(AuthRequestMetadata {
            request_id: [0xff; 32],
            ttl_ns: u64::MAX,
        }),
    ] {
        for ext in [None, Some(vec![]), Some(vec![0, 128, 255])] {
            let request = DelegatedTokenPrepareRequest {
                metadata,
                aud: DelegationAudience::Fleet(fleet),
                grants: vec![DelegatedRoleGrant {
                    target: CanisterRole::new("project_instance"),
                    scopes: vec!["assets:read".to_owned(), "assets:write".to_owned()],
                }],
                ttl_ns: u64::MAX,
                ext,
            };
            let canic_bytes = candid::encode_one(&request).unwrap();
            let shared =
                candid::decode_one::<protocol::DelegatedTokenPrepareRequest>(&canic_bytes).unwrap();
            let protocol::DelegationAudience::Fleet(audience) = shared.aud;
            assert_eq!(
                audience.canonical_network_id.as_bytes(),
                fleet.canonical_network_id.as_bytes()
            );
            assert_eq!(audience.fleet_id.as_bytes(), fleet.fleet_id.as_bytes());
            assert_eq!(shared.metadata, request.metadata);
            assert_eq!(
                shared.grants[0].target.as_str(),
                request.grants[0].target.as_str()
            );
            assert_eq!(shared.grants[0].scopes, request.grants[0].scopes);
            assert_eq!(shared.ttl_ns, request.ttl_ns);
            assert_eq!(shared.ext, request.ext);
            let shared_bytes = candid::encode_one(&shared).unwrap();
            assert_eq!(shared_bytes, canic_bytes);
            assert_eq!(
                candid::decode_one::<DelegatedTokenPrepareRequest>(&shared_bytes).unwrap(),
                request
            );
        }
    }
}

#[test]
fn claims_response_and_retrieval_share_ic_auth_wire_bytes_in_both_directions() {
    for ext in [None, Some(vec![]), Some(vec![0, 255])] {
        let claims = DelegatedTokenClaims {
            presenter: Principal::from_slice(&[1; 29]),
            subject: Principal::from_slice(&[2; 29]),
            issuer_pid: Principal::from_slice(&[3; 29]),
            cert_hash: [0xff; 32],
            issued_at_ns: 0,
            expires_at_ns: u64::MAX,
            aud: DelegationAudience::Fleet(fleet_key(8)),
            grants: vec![DelegatedRoleGrant {
                target: CanisterRole::new("project_instance"),
                scopes: vec!["assets:read".to_owned()],
            }],
            nonce: [0xff; 16],
            ext,
        };
        let response = DelegatedTokenPrepareResponse {
            claims: claims.clone(),
            claims_hash: [0x80; 32],
            retrieval_expires_at_ns: u64::MAX,
        };
        let bytes = candid::encode_one(&response).unwrap();
        let shared = candid::decode_one::<protocol::DelegatedTokenPrepareResponse>(&bytes).unwrap();
        assert_eq!(shared.claims.presenter, claims.presenter);
        assert_eq!(shared.claims.subject, claims.subject);
        assert_eq!(shared.claims.issuer_pid, claims.issuer_pid);
        assert_eq!(shared.claims.cert_hash, claims.cert_hash);
        assert_eq!(shared.claims.issued_at_ns, claims.issued_at_ns);
        assert_eq!(shared.claims.expires_at_ns, claims.expires_at_ns);
        assert_eq!(shared.claims.nonce, claims.nonce);
        assert_eq!(shared.claims.ext, claims.ext);
        assert_eq!(candid::encode_one(&shared).unwrap(), bytes);
        assert_eq!(
            candid::decode_one::<DelegatedTokenPrepareResponse>(
                &candid::encode_one(shared).unwrap()
            )
            .unwrap(),
            response
        );
    }
    let request = DelegatedTokenGetRequest {
        claims_hash: [0xff; 32],
    };
    let bytes = candid::encode_one(&request).unwrap();
    let shared = candid::decode_one::<protocol::DelegatedTokenGetRequest>(&bytes).unwrap();
    assert_eq!(shared.claims_hash, request.claims_hash);
    let shared_bytes = candid::encode_one(&shared).unwrap();
    assert_eq!(shared_bytes, bytes);
    assert_eq!(
        candid::decode_one::<DelegatedTokenGetRequest>(&shared_bytes).unwrap(),
        request
    );
}

#[test]
fn ic_auth_wire_decoding_rejects_invalid_roles_and_audience_ids() {
    let request = DelegatedTokenPrepareRequest {
        metadata: None,
        aud: DelegationAudience::Fleet(fleet_key(9)),
        grants: vec![DelegatedRoleGrant {
            target: CanisterRole::new("UPPER"),
            scopes: vec!["read".to_owned()],
        }],
        ttl_ns: 1,
        ext: None,
    };
    assert!(
        candid::decode_one::<protocol::DelegatedTokenPrepareRequest>(
            &candid::encode_one(request).unwrap()
        )
        .is_err()
    );

    let bytes = candid::encode_one(MalformedAudience {
        canonical_network_id: "aa".to_owned(),
        fleet_id: "00".repeat(32),
    })
    .unwrap();
    assert!(candid::decode_one::<protocol::AudienceId>(&bytes).is_err());
}
