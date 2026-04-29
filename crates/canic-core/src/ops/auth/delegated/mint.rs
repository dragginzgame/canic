use super::{
    audience::{AudienceError, audience_subset, validate_audience_shape},
    canonical::{CanonicalAuthError, cert_hash, claims_hash},
    policy::DELEGATED_AUTH_VERSION,
};
use crate::{
    cdk::types::Principal,
    dto::auth::{DelegatedToken, DelegatedTokenClaims, DelegationAudience, DelegationProof},
};
use thiserror::Error;

pub struct MintDelegatedTokenInput<'a> {
    pub proof: &'a DelegationProof,
    pub subject: Principal,
    pub audience: DelegationAudience,
    pub scopes: Vec<String>,
    pub ttl_secs: u64,
    pub nonce: [u8; 16],
    pub now_secs: u64,
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
    #[error("delegated auth token ttl {ttl_secs}s exceeds cert max {max_ttl_secs}s")]
    TokenTtlExceeded { ttl_secs: u64, max_ttl_secs: u64 },
    #[error("delegated auth token expires after cert")]
    TokenOutlivesCert,
    #[error("delegated auth token audience is not a subset of cert audience")]
    AudienceNotSubset,
    #[error("delegated auth token scope rejected: {scope}")]
    ScopeRejected { scope: String },
    #[cfg(test)]
    #[error("delegated auth shard signature failed: {0}")]
    SignFailed(String),
    #[error(transparent)]
    Audience(#[from] AudienceError),
    #[error(transparent)]
    Canonical(#[from] CanonicalAuthError),
}

#[cfg(test)]
pub fn mint_delegated_token<F>(
    input: MintDelegatedTokenInput<'_>,
    sign_claims_hash: F,
) -> Result<DelegatedToken, MintDelegatedTokenError>
where
    F: FnOnce([u8; 32]) -> Result<Vec<u8>, String>,
{
    let prepared = prepare_delegated_token(input)?;
    let shard_sig =
        sign_claims_hash(prepared.claims_hash).map_err(MintDelegatedTokenError::SignFailed)?;
    Ok(finish_delegated_token(prepared, shard_sig))
}

/// Prepare one canonical delegated-token claims payload before shard signing.
pub fn prepare_delegated_token(
    input: MintDelegatedTokenInput<'_>,
) -> Result<PreparedDelegatedToken, MintDelegatedTokenError> {
    let cert = &input.proof.cert;

    if input.now_secs < cert.issued_at {
        return Err(MintDelegatedTokenError::CertNotYetValid);
    }
    if input.now_secs >= cert.expires_at {
        return Err(MintDelegatedTokenError::CertExpired);
    }
    if input.ttl_secs == 0 {
        return Err(MintDelegatedTokenError::TokenTtlZero);
    }
    if input.ttl_secs > cert.max_token_ttl_secs {
        return Err(MintDelegatedTokenError::TokenTtlExceeded {
            ttl_secs: input.ttl_secs,
            max_ttl_secs: cert.max_token_ttl_secs,
        });
    }

    let expires_at = input
        .now_secs
        .checked_add(input.ttl_secs)
        .ok_or(MintDelegatedTokenError::TokenExpiresAtOverflow)?;
    if expires_at > cert.expires_at {
        return Err(MintDelegatedTokenError::TokenOutlivesCert);
    }

    validate_audience_shape(&input.audience)?;
    if !audience_subset(&input.audience, &cert.aud) {
        return Err(MintDelegatedTokenError::AudienceNotSubset);
    }
    verify_scopes(&input.scopes, &cert.scopes)?;

    let claims = DelegatedTokenClaims {
        version: DELEGATED_AUTH_VERSION,
        subject: input.subject,
        issuer_shard_pid: cert.shard_pid,
        cert_hash: cert_hash(cert)?,
        issued_at: input.now_secs,
        expires_at,
        aud: input.audience,
        scopes: input.scopes,
        nonce: input.nonce,
    };
    let claims_hash = claims_hash(&claims)?;

    Ok(PreparedDelegatedToken {
        claims,
        claims_hash,
        proof: input.proof.clone(),
    })
}

/// Combine prepared token claims with their shard signature.
pub fn finish_delegated_token(
    prepared: PreparedDelegatedToken,
    shard_sig: Vec<u8>,
) -> DelegatedToken {
    DelegatedToken {
        claims: prepared.claims,
        proof: prepared.proof,
        shard_sig,
    }
}

fn verify_scopes(subset: &[String], superset: &[String]) -> Result<(), MintDelegatedTokenError> {
    for scope in subset {
        if !superset.contains(scope) {
            return Err(MintDelegatedTokenError::ScopeRejected {
                scope: scope.clone(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::auth::{
            DelegationCert, RootPublicKey, RootTrustAnchor, ShardKeyBinding, SignatureAlgorithm,
        },
        ids::CanisterRole,
        ops::auth::delegated::{
            canonical::{public_key_hash, role_hash},
            verify::{VerifyDelegatedTokenInput, verify_delegated_token},
        },
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn cert() -> DelegationCert {
        let role = CanisterRole::new("project_instance");
        let shard_public_key_sec1 = vec![1, 2, 3];
        let shard_key_hash = public_key_hash(&shard_public_key_sec1);

        DelegationCert {
            version: DELEGATED_AUTH_VERSION,
            root_pid: p(1),
            root_key_id: "root-key".to_string(),
            root_key_hash: [9; 32],
            alg: SignatureAlgorithm::EcdsaP256Sha256,
            shard_pid: p(2),
            shard_key_id: "shard-key".to_string(),
            shard_public_key_sec1,
            shard_key_hash,
            shard_key_binding: ShardKeyBinding::IcThresholdEcdsa {
                key_name_hash: [3; 32],
                derivation_path_hash: [4; 32],
            },
            issued_at: 100,
            expires_at: 500,
            max_token_ttl_secs: 120,
            scopes: vec!["read".to_string(), "write".to_string()],
            aud: DelegationAudience::Roles(vec![role.clone()]),
            verifier_role_hash: Some(role_hash(&role).unwrap()),
        }
    }

    fn proof() -> DelegationProof {
        DelegationProof {
            cert: cert(),
            root_sig: vec![10, 11, 12],
        }
    }

    fn signed_proof_for_local_root() -> (DelegationProof, RootTrustAnchor) {
        let mut proof = proof();
        let root_public_key = vec![13, 14, 15];
        proof.cert.root_key_hash = public_key_hash(&root_public_key);
        proof.root_sig = cert_hash(&proof.cert).unwrap().to_vec();

        let trust = RootTrustAnchor {
            root_pid: p(1),
            root_key: RootPublicKey {
                root_pid: p(1),
                key_id: "root-key".to_string(),
                alg: SignatureAlgorithm::EcdsaP256Sha256,
                public_key_sec1: root_public_key,
                key_hash: proof.cert.root_key_hash,
                not_before: 90,
                not_after: None,
            },
        };

        (proof, trust)
    }

    fn input(proof: &DelegationProof) -> MintDelegatedTokenInput<'_> {
        MintDelegatedTokenInput {
            proof,
            subject: p(9),
            audience: DelegationAudience::Roles(vec![CanisterRole::new("project_instance")]),
            scopes: vec!["read".to_string()],
            ttl_secs: 60,
            nonce: [7; 16],
            now_secs: 120,
        }
    }

    fn verify_hash_signature(
        _: &[u8],
        hash: [u8; 32],
        sig: &[u8],
        _: SignatureAlgorithm,
    ) -> Result<(), String> {
        if sig == hash.as_slice() {
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
            Ok(vec![20, 21, 22])
        })
        .unwrap();

        assert_eq!(token.claims.subject, p(9));
        assert_eq!(token.claims.issuer_shard_pid, proof.cert.shard_pid);
        assert_eq!(token.claims.issued_at, 120);
        assert_eq!(token.claims.expires_at, 180);
        assert_eq!(token.proof, proof);
        assert_eq!(token.shard_sig, vec![20, 21, 22]);
        assert_eq!(observed_hash, Some(claims_hash(&token.claims).unwrap()));
    }

    #[test]
    fn minted_token_feeds_the_pure_verifier() {
        let (proof, trust) = signed_proof_for_local_root();
        let token = mint_delegated_token(input(&proof), |_| Ok(vec![20, 21, 22])).unwrap();
        let role = CanisterRole::new("project_instance");
        let required_scopes = vec!["read".to_string()];

        verify_delegated_token(
            VerifyDelegatedTokenInput {
                token: &token,
                root_trust: &trust,
                local_principal: p(99),
                local_role: Some(&role),
                ttl_policy: crate::ops::auth::delegated::policy::DelegatedAuthTtlPolicy {
                    max_cert_ttl_secs: 600,
                    max_token_ttl_secs: 120,
                },
                required_scopes: &required_scopes,
                now_secs: 130,
            },
            |_, _, _, _| Ok(()),
        )
        .unwrap();
    }

    #[test]
    fn minted_tokens_with_different_nonces_verify_when_signed() {
        let (proof, trust) = signed_proof_for_local_root();
        let role = CanisterRole::new("project_instance");
        let mut left_input = input(&proof);
        left_input.nonce = [1; 16];
        let mut right_input = input(&proof);
        right_input.nonce = [2; 16];
        let left = mint_delegated_token(left_input, |hash| Ok(hash.to_vec())).unwrap();
        let right = mint_delegated_token(right_input, |hash| Ok(hash.to_vec())).unwrap();

        for token in [&left, &right] {
            verify_delegated_token(
                VerifyDelegatedTokenInput {
                    token,
                    root_trust: &trust,
                    local_principal: p(99),
                    local_role: Some(&role),
                    ttl_policy: crate::ops::auth::delegated::policy::DelegatedAuthTtlPolicy {
                        max_cert_ttl_secs: 600,
                        max_token_ttl_secs: 120,
                    },
                    required_scopes: &[],
                    now_secs: 130,
                },
                verify_hash_signature,
            )
            .unwrap();
        }
    }

    #[test]
    fn mutating_signed_scopes_fails_verifier_signature() {
        let (proof, trust) = signed_proof_for_local_root();
        let role = CanisterRole::new("project_instance");
        let mut token = mint_delegated_token(input(&proof), |hash| Ok(hash.to_vec())).unwrap();
        token.claims.scopes = vec!["write".to_string()];

        assert_eq!(
            verify_delegated_token(
                VerifyDelegatedTokenInput {
                    token: &token,
                    root_trust: &trust,
                    local_principal: p(99),
                    local_role: Some(&role),
                    ttl_policy: crate::ops::auth::delegated::policy::DelegatedAuthTtlPolicy {
                        max_cert_ttl_secs: 600,
                        max_token_ttl_secs: 120,
                    },
                    required_scopes: &[],
                    now_secs: 130,
                },
                verify_hash_signature,
            ),
            Err(
                crate::ops::auth::delegated::verify::VerifyDelegatedTokenError::ShardSignatureInvalid(
                    "hash mismatch".to_string(),
                )
            )
        );
    }

    #[test]
    fn mint_delegated_token_rejects_audience_expansion() {
        let proof = proof();
        let mut input = input(&proof);
        input.audience = DelegationAudience::Roles(vec![CanisterRole::new("project_hub")]);

        assert_eq!(
            mint_delegated_token(input, |_| Ok(vec![])),
            Err(MintDelegatedTokenError::AudienceNotSubset)
        );
    }

    #[test]
    fn mint_delegated_token_allows_role_claim_subset_of_mixed_cert_audience() {
        let mut proof = proof();
        let role = CanisterRole::new("project_instance");
        proof.cert.aud = DelegationAudience::RolesOrPrincipals {
            roles: vec![role],
            principals: vec![],
        };

        let token = mint_delegated_token(input(&proof), |hash| Ok(hash.to_vec())).unwrap();

        assert_eq!(
            token.claims.aud,
            DelegationAudience::Roles(vec![CanisterRole::new("project_instance")])
        );
    }

    #[test]
    fn mint_delegated_token_rejects_scope_expansion() {
        let proof = proof();
        let mut input = input(&proof);
        input.scopes = vec!["admin".to_string()];

        assert_eq!(
            mint_delegated_token(input, |_| Ok(vec![])),
            Err(MintDelegatedTokenError::ScopeRejected {
                scope: "admin".to_string(),
            })
        );
    }

    #[test]
    fn mint_delegated_token_accepts_ttl_equal_to_cert_limit() {
        let proof = proof();
        let mut input = input(&proof);
        input.ttl_secs = 120;

        let token = mint_delegated_token(input, |hash| Ok(hash.to_vec())).unwrap();

        assert_eq!(token.claims.issued_at, 120);
        assert_eq!(token.claims.expires_at, 240);
    }

    #[test]
    fn mint_delegated_token_rejects_token_ttl_above_cert_limit() {
        let proof = proof();
        let mut input = input(&proof);
        input.ttl_secs = 121;

        assert_eq!(
            mint_delegated_token(input, |_| Ok(vec![])),
            Err(MintDelegatedTokenError::TokenTtlExceeded {
                ttl_secs: 121,
                max_ttl_secs: 120,
            })
        );
    }

    #[test]
    fn mint_delegated_token_rejects_token_outliving_cert() {
        let proof = proof();
        let mut input = input(&proof);
        input.now_secs = 490;
        input.ttl_secs = 20;

        assert_eq!(
            mint_delegated_token(input, |_| Ok(vec![])),
            Err(MintDelegatedTokenError::TokenOutlivesCert)
        );
    }

    #[test]
    fn mint_delegated_token_rejects_signing_failure() {
        let proof = proof();

        assert_eq!(
            mint_delegated_token(input(&proof), |_| Err("sign failed".to_string())),
            Err(MintDelegatedTokenError::SignFailed(
                "sign failed".to_string(),
            ))
        );
    }
}
