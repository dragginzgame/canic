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
use thiserror::Error;

pub struct MintDelegatedTokenInput<'a> {
    pub proof: &'a DelegationProof,
    pub subject: Principal,
    pub audience: DelegationAudience,
    pub grants: Vec<DelegatedRoleGrant>,
    pub ttl_ns: u64,
    pub nonce: [u8; 16],
    pub ext: Option<Vec<u8>>,
    pub now_ns: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedDelegatedToken {
    pub claims: DelegatedTokenClaims,
    pub claims_hash: [u8; 32],
    pub proof: DelegationProof,
}

#[derive(Debug, Eq, Error, PartialEq)]
pub enum MintDelegatedTokenError {
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
pub fn mint_delegated_token<F>(
    input: MintDelegatedTokenInput<'_>,
    create_issuer_proof: F,
) -> Result<DelegatedToken, MintDelegatedTokenError>
where
    F: FnOnce([u8; 32]) -> Result<IssuerProof, String>,
{
    let prepared = prepare_delegated_token(input)?;
    let issuer_proof = create_issuer_proof(prepared.claims_hash)
        .map_err(MintDelegatedTokenError::IssuerProofFailed)?;
    Ok(finish_delegated_token(prepared, issuer_proof))
}

/// Prepare one canonical delegated-token claims payload before issuer proof creation.
pub fn prepare_delegated_token(
    input: MintDelegatedTokenInput<'_>,
) -> Result<PreparedDelegatedToken, MintDelegatedTokenError> {
    let cert = &input.proof.cert;

    if input.now_ns < cert.not_before_ns {
        return Err(MintDelegatedTokenError::CertNotYetValid);
    }
    if input.now_ns >= cert.expires_at_ns {
        return Err(MintDelegatedTokenError::CertExpired);
    }
    if input.ttl_ns == 0 {
        return Err(MintDelegatedTokenError::TokenTtlZero);
    }
    if input.ttl_ns > cert.max_token_ttl_ns {
        return Err(MintDelegatedTokenError::TokenTtlExceeded {
            ttl_ns: input.ttl_ns,
            max_ttl_ns: cert.max_token_ttl_ns,
        });
    }

    let expires_at = input
        .now_ns
        .checked_add(input.ttl_ns)
        .ok_or(MintDelegatedTokenError::TokenExpiresAtOverflow)?;
    if expires_at > cert.expires_at_ns {
        return Err(MintDelegatedTokenError::TokenOutlivesCert);
    }

    validate_audience_shape(&input.audience)?;
    validate_role_grants(&input.grants)?;
    if !audience_subset(&input.audience, &cert.aud) {
        return Err(MintDelegatedTokenError::AudienceNotSubset);
    }
    if !role_grants_subset(&input.grants, &cert.grants) {
        return Err(MintDelegatedTokenError::GrantsNotSubset);
    }

    let claims = DelegatedTokenClaims {
        subject: input.subject,
        issuer_pid: cert.issuer_pid,
        cert_hash: cert_hash(cert)?,
        issued_at_ns: input.now_ns,
        expires_at_ns: expires_at,
        aud: input.audience,
        grants: input.grants,
        nonce: input.nonce,
        ext: input.ext,
    };
    let claims_hash = claims_hash(&claims)?;

    Ok(PreparedDelegatedToken {
        claims,
        claims_hash,
        proof: input.proof.clone(),
    })
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
        let issuer_signer_generation = None;
        let issuer_proof_binding_hash = issuer_proof_binding_hash(
            p(2),
            issuer_proof_alg,
            issuer_proof_binding,
            issuer_signer_generation,
        );

        DelegationCert {
            root_pid: p(1),
            issuer_pid: p(2),
            issuer_proof_alg,
            issuer_proof_binding_hash,
            issuer_proof_binding,
            issuer_signer_generation,
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
        RootProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1 {
            signature_cbor: vec![byte; 8],
            public_key_der: vec![byte; 4],
        })
    }

    fn input(proof: &DelegationProof) -> MintDelegatedTokenInput<'_> {
        MintDelegatedTokenInput {
            proof,
            subject: p(9),
            audience: DelegationAudience::Project("test".to_string()),
            grants: vec![grant("project_instance", &["read"])],
            ttl_ns: 60,
            nonce: [7; 16],
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

    fn verify_hash_signature(
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
    fn mint_delegated_token_signs_claims_hash_and_embeds_proof() {
        let proof = proof();
        let mut observed_hash = None;

        let token = mint_delegated_token(input(&proof), |hash| {
            observed_hash = Some(hash);
            Ok(issuer_proof_for_hash(hash))
        })
        .unwrap();

        assert_eq!(token.claims.subject, p(9));
        assert_eq!(token.claims.issuer_pid, proof.cert.issuer_pid);
        assert_eq!(token.claims.issued_at_ns, 120);
        assert_eq!(token.claims.expires_at_ns, 180);
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
    fn mint_delegated_token_signs_ext_inside_claims() {
        let proof = proof();
        let mut input = input(&proof);
        input.ext = Some(b"opaque-app-context".to_vec());

        let token = mint_delegated_token(input, |hash| Ok(issuer_proof_for_hash(hash))).unwrap();

        assert_eq!(token.claims.ext, Some(b"opaque-app-context".to_vec()));
        let IssuerProof::IcCanisterSignatureV1(issuer_proof) = &token.issuer_proof;
        assert_eq!(
            issuer_proof.signature_cbor,
            claims_hash(&token.claims).unwrap()
        );
    }

    #[test]
    fn minted_token_feeds_the_pure_verifier() {
        let proof = proof();
        let token =
            mint_delegated_token(input(&proof), |hash| Ok(issuer_proof_for_hash(hash))).unwrap();
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
            |_, _, _| Ok(()),
            |_, _, _| Ok(()),
        )
        .unwrap();
    }

    #[test]
    fn minted_tokens_with_different_nonces_verify_when_signed() {
        let proof = proof();
        let role = CanisterRole::new("project_instance");
        let mut left_input = input(&proof);
        left_input.nonce = [1; 16];
        let mut right_input = input(&proof);
        right_input.nonce = [2; 16];
        let left =
            mint_delegated_token(left_input, |hash| Ok(issuer_proof_for_hash(hash))).unwrap();
        let right =
            mint_delegated_token(right_input, |hash| Ok(issuer_proof_for_hash(hash))).unwrap();

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
                |_, _, _| Ok(()),
                verify_hash_signature,
            )
            .unwrap();
        }
    }

    #[test]
    fn mutating_signed_grants_fails_verifier_signature() {
        let proof = proof();
        let role = CanisterRole::new("project_instance");
        let mut token =
            mint_delegated_token(input(&proof), |hash| Ok(issuer_proof_for_hash(hash))).unwrap();
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
                |_, _, _| Ok(()),
                verify_hash_signature,
            ),
            Err(
                crate::ops::auth::delegated::verify::VerifyDelegatedTokenError::IssuerProofInvalid(
                    "hash mismatch".to_string(),
                )
            )
        );
    }

    #[test]
    fn mutating_signed_ext_fails_verifier_signature() {
        let proof = proof();
        let role = CanisterRole::new("project_instance");
        let mut input = input(&proof);
        input.ext = Some(b"left".to_vec());
        let mut token =
            mint_delegated_token(input, |hash| Ok(issuer_proof_for_hash(hash))).unwrap();
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
                |_, _, _| Ok(()),
                verify_hash_signature,
            ),
            Err(
                crate::ops::auth::delegated::verify::VerifyDelegatedTokenError::IssuerProofInvalid(
                    "hash mismatch".to_string(),
                )
            )
        );
    }

    #[test]
    fn mint_delegated_token_rejects_oversized_ext() {
        let proof = proof();
        let mut input = input(&proof);
        input.ext = Some(vec![
            1;
            crate::ops::auth::delegated::canonical::MAX_TOKEN_EXT_BYTES
                + 1
        ]);

        assert_eq!(
            mint_delegated_token(input, |_| Ok(issuer_proof_for_hash([0; 32]))),
            Err(MintDelegatedTokenError::Canonical(
                CanonicalAuthError::TokenExtTooLarge {
                    len: crate::ops::auth::delegated::canonical::MAX_TOKEN_EXT_BYTES + 1,
                    max: crate::ops::auth::delegated::canonical::MAX_TOKEN_EXT_BYTES,
                }
            ))
        );
    }

    #[test]
    fn mint_delegated_token_rejects_audience_expansion() {
        let proof = proof();
        let mut input = input(&proof);
        input.audience = DelegationAudience::Project("other".to_string());

        assert_eq!(
            mint_delegated_token(input, |_| Ok(issuer_proof_for_hash([0; 32]))),
            Err(MintDelegatedTokenError::AudienceNotSubset)
        );
    }

    #[test]
    fn mint_delegated_token_rejects_grant_expansion() {
        let proof = proof();
        let mut input = input(&proof);
        input.grants = vec![grant("project_instance", &["admin"])];

        assert_eq!(
            mint_delegated_token(input, |_| Ok(issuer_proof_for_hash([0; 32]))),
            Err(MintDelegatedTokenError::GrantsNotSubset)
        );
    }

    #[test]
    fn mint_delegated_token_accepts_ttl_equal_to_cert_limit() {
        let proof = proof();
        let mut input = input(&proof);
        input.ttl_ns = 120;

        let token = mint_delegated_token(input, |hash| Ok(issuer_proof_for_hash(hash))).unwrap();

        assert_eq!(token.claims.issued_at_ns, 120);
        assert_eq!(token.claims.expires_at_ns, 240);
    }

    #[test]
    fn mint_delegated_token_rejects_token_ttl_above_cert_limit() {
        let proof = proof();
        let mut input = input(&proof);
        input.ttl_ns = 121;

        assert_eq!(
            mint_delegated_token(input, |_| Ok(issuer_proof_for_hash([0; 32]))),
            Err(MintDelegatedTokenError::TokenTtlExceeded {
                ttl_ns: 121,
                max_ttl_ns: 120,
            })
        );
    }

    #[test]
    fn mint_delegated_token_rejects_token_outliving_cert() {
        let proof = proof();
        let mut input = input(&proof);
        input.now_ns = 490;
        input.ttl_ns = 20;

        assert_eq!(
            mint_delegated_token(input, |_| Ok(issuer_proof_for_hash([0; 32]))),
            Err(MintDelegatedTokenError::TokenOutlivesCert)
        );
    }

    #[test]
    fn mint_delegated_token_rejects_issuer_proof_failure() {
        let proof = proof();

        assert_eq!(
            mint_delegated_token(input(&proof), |_| Err("sign failed".to_string())),
            Err(MintDelegatedTokenError::IssuerProofFailed(
                "sign failed".to_string(),
            ))
        );
    }
}
