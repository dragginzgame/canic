use super::{
    DelegatedRoleGrant, DelegatedTokenClaims, DelegatedTokenPrepareRequest, DelegationAudience,
};
use crate::cdk::types::Principal;
use candid::CandidType;

#[derive(CandidType, candid::Deserialize)]
struct PresenterlessDelegatedTokenClaims {
    subject: Principal,
    issuer_pid: Principal,
    cert_hash: [u8; 32],
    issued_at_ns: u64,
    expires_at_ns: u64,
    aud: DelegationAudience,
    grants: Vec<DelegatedRoleGrant>,
    nonce: [u8; 16],
    ext: Option<Vec<u8>>,
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
fn delegated_token_candid_hard_cuts_presenter_and_request_subject() {
    let claims = DelegatedTokenClaims::_ty().to_string();
    assert!(claims.contains("presenter : principal"));
    assert!(claims.contains("subject : principal"));

    let prepare = DelegatedTokenPrepareRequest::_ty().to_string();
    assert!(!prepare.contains("presenter : principal"));
    assert!(!prepare.contains("subject : principal"));

    let presenterless = PresenterlessDelegatedTokenClaims {
        subject: Principal::anonymous(),
        issuer_pid: Principal::management_canister(),
        cert_hash: [1; 32],
        issued_at_ns: 10,
        expires_at_ns: 20,
        aud: DelegationAudience::Fleet(crate::test::support::fleet_key(1)),
        grants: Vec::new(),
        nonce: [2; 16],
        ext: None,
    };
    let bytes = candid::encode_one(presenterless).expect("encode presenter-less predecessor");
    assert!(candid::decode_one::<DelegatedTokenClaims>(&bytes).is_err());
}
