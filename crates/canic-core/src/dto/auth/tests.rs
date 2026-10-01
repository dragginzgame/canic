use super::{DelegatedTokenClaims, DelegatedTokenPrepareRequest};
use candid::CandidType;

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
