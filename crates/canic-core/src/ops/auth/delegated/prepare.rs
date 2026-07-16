//! Module: ops::auth::delegated::prepare
//!
//! Responsibility: prepare delegated-token claims before issuer proof creation.
//! Does not own: issuer proof retrieval, endpoint authorization, or active proof storage.
//! Boundary: pure token construction helper used by issuer-local auth ops.

use super::{
    audience::{
        AudienceError, audience_subset, role_grants_subset, validate_audience_shape,
        validate_role_grants,
    },
    canonical::{CanonicalAuthError, cert_hash, claims_hash},
};
use crate::{
    cdk::types::Principal,
    dto::auth::{
        DelegatedRoleGrant, DelegatedToken, DelegatedTokenClaims, DelegationAudience,
        DelegationProof, IssuerProof,
    },
};
use sha2::{Digest, Sha256};
use thiserror::Error;

const TOKEN_NONCE_DOMAIN: &[u8] = b"canic-token-nonce-v1";

///
/// PrepareDelegatedTokenInput
///
/// Input for preparing delegated-token claims from an active delegation proof.
///

pub struct PrepareDelegatedTokenInput<'a> {
    pub proof: &'a DelegationProof,
    pub operation_id: [u8; 32],
    pub prepared_by: Principal,
    pub subject: Principal,
    pub audience: DelegationAudience,
    pub grants: Vec<DelegatedRoleGrant>,
    pub ttl_ns: u64,
    pub ext: Option<Vec<u8>>,
    pub now_ns: u64,
}

///
/// PreparedDelegatedToken
///
/// Prepared delegated-token claims paired with their canonical hash and proof.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedDelegatedToken {
    pub claims: DelegatedTokenClaims,
    pub claims_hash: [u8; 32],
    pub proof: DelegationProof,
}

///
/// PrepareDelegatedTokenError
///
/// Typed failure surface for delegated-token preparation.
///

#[derive(Debug, Eq, Error, PartialEq)]
pub enum PrepareDelegatedTokenError {
    #[error("delegated auth cert is not yet valid")]
    CertNotYetValid,
    #[error("delegated auth cert expired")]
    CertExpired,
    #[error("delegated auth token ttl must be greater than zero")]
    TokenTtlZero,
    #[error("delegated auth token expires_at overflow")]
    TokenExpiresAtOverflow,
    #[error("delegated auth token ttl {ttl_ns}ns exceeds cert max {max_ttl_ns}ns")]
    TokenTtlExceeded { ttl_ns: u64, max_ttl_ns: u64 },
    #[error("delegated auth token expires after cert")]
    TokenOutlivesCert,
    #[error("delegated auth token audience is not a subset of cert audience")]
    AudienceNotSubset,
    #[error("delegated auth token grants are not a subset of cert grants")]
    GrantsNotSubset,
    #[cfg(test)]
    #[error("delegated auth issuer proof failed: {0}")]
    IssuerProofFailed(String),
    #[error(transparent)]
    Audience(#[from] AudienceError),
    #[error(transparent)]
    Canonical(#[from] CanonicalAuthError),
}

#[cfg(test)]
pub fn prepare_and_finish_delegated_token_for_tests<F>(
    input: PrepareDelegatedTokenInput<'_>,
    create_issuer_proof: F,
) -> Result<DelegatedToken, PrepareDelegatedTokenError>
where
    F: FnOnce([u8; 32]) -> Result<IssuerProof, String>,
{
    let prepared = prepare_delegated_token(input)?;
    let issuer_proof = create_issuer_proof(prepared.claims_hash)
        .map_err(PrepareDelegatedTokenError::IssuerProofFailed)?;
    Ok(finish_delegated_token(prepared, issuer_proof))
}

/// Prepare one canonical delegated-token claims payload before issuer proof creation.
pub fn prepare_delegated_token(
    input: PrepareDelegatedTokenInput<'_>,
) -> Result<PreparedDelegatedToken, PrepareDelegatedTokenError> {
    let cert = &input.proof.cert;

    if input.now_ns < cert.not_before_ns {
        return Err(PrepareDelegatedTokenError::CertNotYetValid);
    }
    if input.now_ns >= cert.expires_at_ns {
        return Err(PrepareDelegatedTokenError::CertExpired);
    }
    if input.ttl_ns == 0 {
        return Err(PrepareDelegatedTokenError::TokenTtlZero);
    }
    if input.ttl_ns > cert.max_token_ttl_ns {
        return Err(PrepareDelegatedTokenError::TokenTtlExceeded {
            ttl_ns: input.ttl_ns,
            max_ttl_ns: cert.max_token_ttl_ns,
        });
    }

    let expires_at = input
        .now_ns
        .checked_add(input.ttl_ns)
        .ok_or(PrepareDelegatedTokenError::TokenExpiresAtOverflow)?;
    if expires_at > cert.expires_at_ns {
        return Err(PrepareDelegatedTokenError::TokenOutlivesCert);
    }

    validate_audience_shape(&input.audience)?;
    validate_role_grants(&input.grants)?;
    if !audience_subset(&input.audience, &cert.aud) {
        return Err(PrepareDelegatedTokenError::AudienceNotSubset);
    }
    if !role_grants_subset(&input.grants, &cert.grants) {
        return Err(PrepareDelegatedTokenError::GrantsNotSubset);
    }

    let cert_hash = cert_hash(cert)?;
    let nonce = delegated_token_nonce(
        input.prepared_by,
        input.operation_id,
        input.subject,
        cert.issuer_pid,
        cert_hash,
    );

    let claims = DelegatedTokenClaims {
        subject: input.subject,
        issuer_pid: cert.issuer_pid,
        cert_hash,
        issued_at_ns: input.now_ns,
        expires_at_ns: expires_at,
        aud: input.audience,
        grants: input.grants,
        nonce,
        ext: input.ext,
    };
    let claims_hash = claims_hash(&claims)?;

    Ok(PreparedDelegatedToken {
        claims,
        claims_hash,
        proof: input.proof.clone(),
    })
}

pub fn delegated_token_nonce(
    prepared_by: Principal,
    operation_id: [u8; 32],
    subject: Principal,
    issuer_pid: Principal,
    cert_hash: [u8; 32],
) -> [u8; 16] {
    let mut hasher = Sha256::new();
    hasher.update(TOKEN_NONCE_DOMAIN);
    hasher.update(prepared_by.as_slice());
    hasher.update(operation_id);
    hasher.update(subject.as_slice());
    hasher.update(issuer_pid.as_slice());
    hasher.update(cert_hash);
    let digest: [u8; 32] = hasher.finalize().into();
    let mut nonce = [0u8; 16];
    nonce.copy_from_slice(&digest[..16]);
    nonce
}

/// Combine prepared token claims with their issuer canister-signature proof.
pub fn finish_delegated_token(
    prepared: PreparedDelegatedToken,
    issuer_proof: IssuerProof,
) -> DelegatedToken {
    DelegatedToken {
        claims: prepared.claims,
        proof: prepared.proof,
        issuer_proof,
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::auth::{
            DelegationCert, IcCanisterSignatureProofV1, IssuerProof, IssuerProofAlgorithm,
            IssuerProofBinding, RootProof,
        },
        ids::CanisterRole,
        ops::auth::delegated::{
            canonical::issuer_proof_binding_hash,
            verify::{VerifyDelegatedTokenInput, verify_delegated_token},
        },
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn cert() -> DelegationCert {
        let issuer_proof_alg = IssuerProofAlgorithm::IcCanisterSignatureV1;
        let issuer_proof_binding = IssuerProofBinding::IcCanisterSignatureV1 { seed_hash: [3; 32] };
        let issuer_proof_binding_hash =
            issuer_proof_binding_hash(p(2), issuer_proof_alg, issuer_proof_binding);

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
            aud: DelegationAudience::Project("test".to_string()),
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

    fn input(proof: &DelegationProof) -> PrepareDelegatedTokenInput<'_> {
        PrepareDelegatedTokenInput {
            proof,
            operation_id: [4; 32],
            prepared_by: p(9),
            subject: p(9),
            audience: DelegationAudience::Project("test".to_string()),
            grants: vec![grant("project_instance", &["read"])],
            ttl_ns: 60,
            ext: None,
            now_ns: 120,
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

    fn verify_issuer_proof_hash(
        hash: [u8; 32],
        proof: &IssuerProof,
        issuer_pid: Principal,
    ) -> Result<(), String> {
        let IssuerProof::IcCanisterSignatureV1(proof) = proof;
        if issuer_pid == p(2) && proof.signature_cbor == hash {
            Ok(())
        } else {
            Err("hash mismatch".to_string())
        }
    }

    #[test]
    fn prepare_delegated_token_signs_claims_hash_and_embeds_proof() {
        let proof = proof();
        let mut observed_hash = None;

        let token = prepare_and_finish_delegated_token_for_tests(input(&proof), |hash| {
            observed_hash = Some(hash);
            Ok(issuer_proof_for_hash(hash))
        })
        .unwrap();

        assert_eq!(token.claims.subject, p(9));
        assert_eq!(token.claims.issuer_pid, proof.cert.issuer_pid);
        assert_eq!(token.claims.issued_at_ns, 120);
        assert_eq!(token.claims.expires_at_ns, 180);
        assert_eq!(
            token.claims.nonce,
            delegated_token_nonce(
                p(9),
                [4; 32],
                p(9),
                proof.cert.issuer_pid,
                cert_hash(&proof.cert).unwrap()
            )
        );
        assert_eq!(token.claims.ext, None);
        assert_eq!(token.proof, proof);
        let IssuerProof::IcCanisterSignatureV1(issuer_proof) = &token.issuer_proof;
        assert_eq!(
            issuer_proof.signature_cbor,
            claims_hash(&token.claims).unwrap()
        );
        assert_eq!(observed_hash, Some(claims_hash(&token.claims).unwrap()));
    }

    #[test]
    fn prepare_delegated_token_signs_ext_inside_claims() {
        let proof = proof();
        let mut input = input(&proof);
        input.ext = Some(b"opaque-app-context".to_vec());

        let token = prepare_and_finish_delegated_token_for_tests(input, |hash| {
            Ok(issuer_proof_for_hash(hash))
        })
        .unwrap();

        assert_eq!(token.claims.ext, Some(b"opaque-app-context".to_vec()));
        let IssuerProof::IcCanisterSignatureV1(issuer_proof) = &token.issuer_proof;
        assert_eq!(
            issuer_proof.signature_cbor,
            claims_hash(&token.claims).unwrap()
        );
    }

    #[test]
    fn prepared_token_feeds_the_pure_verifier() {
        let proof = proof();
        let token = prepare_and_finish_delegated_token_for_tests(input(&proof), |hash| {
            Ok(issuer_proof_for_hash(hash))
        })
        .unwrap();
        let role = CanisterRole::new("project_instance");
        let required_scopes = vec!["read".to_string()];

        verify_delegated_token(
            VerifyDelegatedTokenInput {
                token: &token,
                local_canister: p(20),
                local_canic_subnet: Some(p(21)),
                local_role: Some(&role),
                local_project: Some("test"),
                ttl_limits: crate::ops::auth::delegated::cert_rules::DelegatedAuthTtlLimits {
                    max_cert_ttl_ns: 600,
                    max_token_ttl_ns: 120,
                },
                required_scopes: &required_scopes,
                now_ns: 130,
            },
            |_, _| Ok::<(), String>(()),
            |_, _, _| Ok::<(), String>(()),
        )
        .unwrap();
    }

    #[test]
    fn prepared_tokens_with_different_operation_ids_derive_different_nonces() {
        let proof = proof();
        let role = CanisterRole::new("project_instance");
        let mut left_input = input(&proof);
        left_input.operation_id = [1; 32];
        let mut right_input = input(&proof);
        right_input.operation_id = [2; 32];
        let left = prepare_and_finish_delegated_token_for_tests(left_input, |hash| {
            Ok(issuer_proof_for_hash(hash))
        })
        .unwrap();
        let right = prepare_and_finish_delegated_token_for_tests(right_input, |hash| {
            Ok(issuer_proof_for_hash(hash))
        })
        .unwrap();

        assert_ne!(left.claims.nonce, right.claims.nonce);

        for token in [&left, &right] {
            verify_delegated_token(
                VerifyDelegatedTokenInput {
                    token,
                    local_canister: p(20),
                    local_canic_subnet: Some(p(21)),
                    local_role: Some(&role),
                    local_project: Some("test"),
                    ttl_limits: crate::ops::auth::delegated::cert_rules::DelegatedAuthTtlLimits {
                        max_cert_ttl_ns: 600,
                        max_token_ttl_ns: 120,
                    },
                    required_scopes: &[],
                    now_ns: 130,
                },
                |_, _| Ok::<(), String>(()),
                verify_issuer_proof_hash,
            )
            .unwrap();
        }
    }

    #[test]
    fn mutating_signed_grants_fails_issuer_proof_verification() {
        let proof = proof();
        let role = CanisterRole::new("project_instance");
        let mut token = prepare_and_finish_delegated_token_for_tests(input(&proof), |hash| {
            Ok(issuer_proof_for_hash(hash))
        })
        .unwrap();
        token.claims.grants = vec![grant("project_instance", &["write"])];

        assert_eq!(
            verify_delegated_token(
                VerifyDelegatedTokenInput {
                    token: &token,
                    local_canister: p(20),
                    local_canic_subnet: Some(p(21)),
                    local_role: Some(&role),
                    local_project: Some("test"),
                    ttl_limits: crate::ops::auth::delegated::cert_rules::DelegatedAuthTtlLimits {
                        max_cert_ttl_ns: 600,
                        max_token_ttl_ns: 120,
                    },
                    required_scopes: &[],
                    now_ns: 130,
                },
                |_, _| Ok::<(), String>(()),
                verify_issuer_proof_hash,
            ),
            Err(
                crate::ops::auth::delegated::verify::VerifyDelegatedTokenError::IssuerProofInvalid(
                    "hash mismatch".to_string(),
                )
            )
        );
    }

    #[test]
    fn mutating_signed_ext_fails_issuer_proof_verification() {
        let proof = proof();
        let role = CanisterRole::new("project_instance");
        let mut input = input(&proof);
        input.ext = Some(b"left".to_vec());
        let mut token = prepare_and_finish_delegated_token_for_tests(input, |hash| {
            Ok(issuer_proof_for_hash(hash))
        })
        .unwrap();
        token.claims.ext = Some(b"right".to_vec());

        assert_eq!(
            verify_delegated_token(
                VerifyDelegatedTokenInput {
                    token: &token,
                    local_canister: p(20),
                    local_canic_subnet: Some(p(21)),
                    local_role: Some(&role),
                    local_project: Some("test"),
                    ttl_limits: crate::ops::auth::delegated::cert_rules::DelegatedAuthTtlLimits {
                        max_cert_ttl_ns: 600,
                        max_token_ttl_ns: 120,
                    },
                    required_scopes: &[],
                    now_ns: 130,
                },
                |_, _| Ok::<(), String>(()),
                verify_issuer_proof_hash,
            ),
            Err(
                crate::ops::auth::delegated::verify::VerifyDelegatedTokenError::IssuerProofInvalid(
                    "hash mismatch".to_string(),
                )
            )
        );
    }

    #[test]
    fn prepare_delegated_token_rejects_oversized_ext() {
        let proof = proof();
        let mut input = input(&proof);
        input.ext = Some(vec![
            1;
            crate::ops::auth::delegated::canonical::MAX_TOKEN_EXT_BYTES
                + 1
        ]);

        assert_eq!(
            prepare_and_finish_delegated_token_for_tests(input, |_| Ok(issuer_proof_for_hash(
                [0; 32]
            ))),
            Err(PrepareDelegatedTokenError::Canonical(
                CanonicalAuthError::TokenExtTooLarge {
                    len: crate::ops::auth::delegated::canonical::MAX_TOKEN_EXT_BYTES + 1,
                    max: crate::ops::auth::delegated::canonical::MAX_TOKEN_EXT_BYTES,
                }
            ))
        );
    }

    #[test]
    fn prepare_delegated_token_rejects_audience_expansion() {
        let proof = proof();
        let mut input = input(&proof);
        input.audience = DelegationAudience::Project("other".to_string());

        assert_eq!(
            prepare_and_finish_delegated_token_for_tests(input, |_| Ok(issuer_proof_for_hash(
                [0; 32]
            ))),
            Err(PrepareDelegatedTokenError::AudienceNotSubset)
        );
    }

    #[test]
    fn prepare_delegated_token_rejects_grant_expansion() {
        let proof = proof();
        let mut input = input(&proof);
        input.grants = vec![grant("project_instance", &["admin"])];

        assert_eq!(
            prepare_and_finish_delegated_token_for_tests(input, |_| Ok(issuer_proof_for_hash(
                [0; 32]
            ))),
            Err(PrepareDelegatedTokenError::GrantsNotSubset)
        );
    }

    #[test]
    fn prepare_delegated_token_accepts_ttl_equal_to_cert_limit() {
        let proof = proof();
        let mut input = input(&proof);
        input.ttl_ns = 120;

        let token = prepare_and_finish_delegated_token_for_tests(input, |hash| {
            Ok(issuer_proof_for_hash(hash))
        })
        .unwrap();

        assert_eq!(token.claims.issued_at_ns, 120);
        assert_eq!(token.claims.expires_at_ns, 240);
    }

    #[test]
    fn prepare_delegated_token_rejects_token_ttl_above_cert_limit() {
        let proof = proof();
        let mut input = input(&proof);
        input.ttl_ns = 121;

        assert_eq!(
            prepare_and_finish_delegated_token_for_tests(input, |_| Ok(issuer_proof_for_hash(
                [0; 32]
            ))),
            Err(PrepareDelegatedTokenError::TokenTtlExceeded {
                ttl_ns: 121,
                max_ttl_ns: 120,
            })
        );
    }

    #[test]
    fn prepare_delegated_token_rejects_token_outliving_cert() {
        let proof = proof();
        let mut input = input(&proof);
        input.now_ns = 490;
        input.ttl_ns = 20;

        assert_eq!(
            prepare_and_finish_delegated_token_for_tests(input, |_| Ok(issuer_proof_for_hash(
                [0; 32]
            ))),
            Err(PrepareDelegatedTokenError::TokenOutlivesCert)
        );
    }

    #[test]
    fn prepare_delegated_token_rejects_issuer_proof_failure() {
        let proof = proof();

        assert_eq!(
            prepare_and_finish_delegated_token_for_tests(input(&proof), |_| Err(
                "sign failed".to_string()
            )),
            Err(PrepareDelegatedTokenError::IssuerProofFailed(
                "sign failed".to_string(),
            ))
        );
    }
}
