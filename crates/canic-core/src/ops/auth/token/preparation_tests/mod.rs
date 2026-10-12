//! Module: ops::auth::token::preparation_tests
//!
//! Responsibility: qualify Canic token projection and signed-material integration.
//! Does not own: issuance, signature creation, or production proof verification.
//! Boundary: IC Auth owns request-rule tests; Canic owns wire and authority projection.

use super::*;
use crate::{
    dto::auth::{
        DelegatedRoleGrant, DelegatedToken, DelegationAudience, DelegationCert, DelegationProof,
        IcCanisterSignatureProofV1, IssuerProof, IssuerProofAlgorithm, IssuerProofBinding,
        RootProof,
    },
    ops::auth::delegated::canonical::{claims_hash, issuer_proof_binding_hash},
};
use canic_contracts::ids::CanisterRole;

fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}

fn cert() -> DelegationCert {
    let issuer_proof_alg = IssuerProofAlgorithm::IcCanisterSignatureV1;
    let issuer_proof_binding = IssuerProofBinding::IcCanisterSignatureV1 { seed_hash: [3; 32] };
    let issuer_proof_binding_hash =
        issuer_proof_binding_hash(p(2), issuer_proof_alg, issuer_proof_binding).unwrap();

    DelegationCert {
        root_pid: p(1),
        issuer_pid: p(2),
        issuer_proof_alg,
        issuer_proof_binding_hash,
        issuer_proof_binding,
        issued_at_ns: 100,
        not_before_ns: 100,
        expires_at_ns: 500,
        max_token_ttl_ns: 120,
        aud: DelegationAudience::Fleet(crate::test::support::fleet_key(1)),
        grants: vec![
            grant("project_hub", &["session", "upload"]),
            grant("project_instance", &["read", "write"]),
        ],
    }
}

fn proof() -> DelegationProof {
    DelegationProof {
        cert: cert(),
        root_proof: root_proof(10),
    }
}

fn root_proof(byte: u8) -> RootProof {
    crate::ops::auth::test_fixtures::chain_key_root_proof(byte)
}

fn input(proof: &ic_auth_protocol_types::DelegationProof) -> PrepareDelegatedTokenInput<'_> {
    PrepareDelegatedTokenInput {
        authority_expires_at_ns: proof.cert.expires_at_ns,
        proof,
        operation_id: [4; 32],
        prepared_by: p(9),
        audience: protocol::audience(&DelegationAudience::Fleet(crate::test::support::fleet_key(
            1,
        ))),
        grants: protocol::grants(&[grant("project_instance", &["read"])]).unwrap(),
        ttl_ns: 60,
        ext: None,
        now_ns: 120,
        limits: TokenPreparationLimits {
            max_grants: MAX_DELEGATED_ROLE_GRANTS,
            max_scopes_per_grant: MAX_SCOPES_PER_ROLE_GRANT,
        },
    }
}

fn grant(role: &str, scopes: &[&str]) -> DelegatedRoleGrant {
    DelegatedRoleGrant {
        target: CanisterRole::owned(role.to_string()),
        scopes: scopes.iter().map(|scope| (*scope).to_string()).collect(),
    }
}

fn issuer_proof_for_hash(hash: [u8; 32]) -> IssuerProof {
    IssuerProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1 {
        signature_cbor: hash.to_vec(),
        public_key_der: vec![20; 4],
    })
}

#[test]
fn prepare_delegated_token_signs_claims_hash_and_embeds_proof() {
    let proof = protocol::proof(&proof()).unwrap();
    let mut observed_hash = None;

    let token = prepare_and_finish_delegated_token_for_tests(input(&proof), |hash| {
        observed_hash = Some(hash);
        issuer_proof_for_hash(hash)
    })
    .unwrap();

    assert_eq!(token.claims.subject, p(9));
    assert_eq!(token.claims.presenter, p(9));
    assert_eq!(token.claims.issuer_pid, proof.cert.issuer_pid);
    assert_eq!(token.claims.issued_at_ns, 120);
    assert_eq!(token.claims.expires_at_ns, 180);
    assert_eq!(token.claims.ext, None);
    assert_eq!(protocol::proof(&token.proof).unwrap(), proof);
    let IssuerProof::IcCanisterSignatureV1(issuer_proof) = &token.issuer_proof;
    assert_eq!(
        issuer_proof.signature_cbor,
        claims_hash(&token.claims).unwrap()
    );
    assert_eq!(observed_hash, Some(claims_hash(&token.claims).unwrap()));
}

#[test]
fn prepare_delegated_token_signs_ext_inside_claims() {
    let proof = protocol::proof(&proof()).unwrap();
    let mut input = input(&proof);
    input.ext = Some(b"opaque-app-context".to_vec());

    let token = prepare_and_finish_delegated_token_for_tests(input, issuer_proof_for_hash).unwrap();

    assert_eq!(token.claims.ext, Some(b"opaque-app-context".to_vec()));
    let IssuerProof::IcCanisterSignatureV1(issuer_proof) = &token.issuer_proof;
    assert_eq!(
        issuer_proof.signature_cbor,
        claims_hash(&token.claims).unwrap()
    );
}

#[test]
fn prepared_tokens_with_different_operation_ids_derive_different_nonces() {
    let proof = protocol::proof(&proof()).unwrap();
    let mut left_input = input(&proof);
    left_input.operation_id = [1; 32];
    let mut right_input = input(&proof);
    right_input.operation_id = [2; 32];
    let left =
        prepare_and_finish_delegated_token_for_tests(left_input, issuer_proof_for_hash).unwrap();
    let right =
        prepare_and_finish_delegated_token_for_tests(right_input, issuer_proof_for_hash).unwrap();

    assert_ne!(left.claims.nonce, right.claims.nonce);
}

#[test]
fn prepare_delegated_token_honors_local_authority_deadline_without_rewriting_certificate() {
    let proof = protocol::proof(&proof()).unwrap();
    let mut request = input(&proof);
    request.authority_expires_at_ns = 180;
    let prepared = prepare_delegated_token(request).unwrap();
    assert_eq!(prepared.claims().expires_at_ns, 180);
    assert_eq!(prepared.proof(), &proof);

    let mut request = input(&proof);
    request.authority_expires_at_ns = 179;
    assert_eq!(
        prepare_delegated_token(request),
        Err(PrepareDelegatedTokenError::TokenOutlivesCert)
    );

    let mut request = input(&proof);
    request.authority_expires_at_ns = request.now_ns;
    assert_eq!(
        prepare_delegated_token(request),
        Err(PrepareDelegatedTokenError::CertExpired)
    );
}

fn prepare_and_finish_delegated_token_for_tests<F>(
    input: PrepareDelegatedTokenInput<'_>,
    create_issuer_proof: F,
) -> Result<DelegatedToken, PrepareDelegatedTokenError>
where
    F: FnOnce([u8; 32]) -> IssuerProof,
{
    let prepared = prepare_delegated_token(input)?;
    let issuer_proof = create_issuer_proof(prepared.claims_hash());
    Ok(protocol::token_data(finish_delegated_token(
        prepared,
        issuer_proof,
    )))
}
