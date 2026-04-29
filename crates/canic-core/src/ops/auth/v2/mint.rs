use super::{
    audience::{AudienceV2Error, audience_subset, validate_audience_shape},
    canonical::{CanonicalAuthV2Error, cert_hash, claims_hash},
    policy::DELEGATED_AUTH_V2_VERSION,
};
use crate::{
    cdk::types::Principal,
    dto::auth::{
        DelegatedTokenClaimsV2, DelegatedTokenV2, DelegationAudienceV2, DelegationProofV2,
    },
};
use thiserror::Error;

pub struct MintDelegatedTokenV2Input<'a> {
    pub proof: &'a DelegationProofV2,
    pub subject: Principal,
    pub audience: DelegationAudienceV2,
    pub scopes: Vec<String>,
    pub ttl_secs: u64,
    pub nonce: [u8; 16],
    pub now_secs: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedDelegatedTokenV2 {
    pub claims: DelegatedTokenClaimsV2,
    pub claims_hash: [u8; 32],
    pub proof: DelegationProofV2,
}

#[derive(Debug, Eq, Error, PartialEq)]
pub enum MintDelegatedTokenV2Error {
    #[error("delegated auth v2 cert is not yet valid")]
    CertNotYetValid,
    #[error("delegated auth v2 cert expired")]
    CertExpired,
    #[error("delegated auth v2 token ttl must be greater than zero")]
    TokenTtlZero,
    #[error("delegated auth v2 token expires_at overflow")]
    TokenExpiresAtOverflow,
    #[error("delegated auth v2 token ttl {ttl_secs}s exceeds cert max {max_ttl_secs}s")]
    TokenTtlExceeded { ttl_secs: u64, max_ttl_secs: u64 },
    #[error("delegated auth v2 token expires after cert")]
    TokenOutlivesCert,
    #[error("delegated auth v2 token audience is not a subset of cert audience")]
    AudienceNotSubset,
    #[error("delegated auth v2 token scope rejected: {scope}")]
    ScopeRejected { scope: String },
    #[error("delegated auth v2 shard signature failed: {0}")]
    SignFailed(String),
    #[error(transparent)]
    Audience(#[from] AudienceV2Error),
    #[error(transparent)]
    Canonical(#[from] CanonicalAuthV2Error),
}

pub fn mint_delegated_token_v2<F>(
    input: MintDelegatedTokenV2Input<'_>,
    sign_claims_hash: F,
) -> Result<DelegatedTokenV2, MintDelegatedTokenV2Error>
where
    F: FnOnce([u8; 32]) -> Result<Vec<u8>, String>,
{
    let prepared = prepare_delegated_token_v2(input)?;
    let shard_sig =
        sign_claims_hash(prepared.claims_hash).map_err(MintDelegatedTokenV2Error::SignFailed)?;
    Ok(finish_delegated_token_v2(prepared, shard_sig))
}

/// Prepare one canonical V2 delegated-token claims payload before shard signing.
pub fn prepare_delegated_token_v2(
    input: MintDelegatedTokenV2Input<'_>,
) -> Result<PreparedDelegatedTokenV2, MintDelegatedTokenV2Error> {
    let cert = &input.proof.cert;

    if input.now_secs < cert.issued_at {
        return Err(MintDelegatedTokenV2Error::CertNotYetValid);
    }
    if input.now_secs >= cert.expires_at {
        return Err(MintDelegatedTokenV2Error::CertExpired);
    }
    if input.ttl_secs == 0 {
        return Err(MintDelegatedTokenV2Error::TokenTtlZero);
    }
    if input.ttl_secs > cert.max_token_ttl_secs {
        return Err(MintDelegatedTokenV2Error::TokenTtlExceeded {
            ttl_secs: input.ttl_secs,
            max_ttl_secs: cert.max_token_ttl_secs,
        });
    }

    let expires_at = input
        .now_secs
        .checked_add(input.ttl_secs)
        .ok_or(MintDelegatedTokenV2Error::TokenExpiresAtOverflow)?;
    if expires_at > cert.expires_at {
        return Err(MintDelegatedTokenV2Error::TokenOutlivesCert);
    }

    validate_audience_shape(&input.audience)?;
    if !audience_subset(&input.audience, &cert.aud) {
        return Err(MintDelegatedTokenV2Error::AudienceNotSubset);
    }
    verify_scopes(&input.scopes, &cert.scopes)?;

    let claims = DelegatedTokenClaimsV2 {
        version: DELEGATED_AUTH_V2_VERSION,
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

    Ok(PreparedDelegatedTokenV2 {
        claims,
        claims_hash,
        proof: input.proof.clone(),
    })
}

/// Combine prepared V2 token claims with their shard signature.
pub fn finish_delegated_token_v2(
    prepared: PreparedDelegatedTokenV2,
    shard_sig: Vec<u8>,
) -> DelegatedTokenV2 {
    DelegatedTokenV2 {
        claims: prepared.claims,
        proof: prepared.proof,
        shard_sig,
    }
}

fn verify_scopes(subset: &[String], superset: &[String]) -> Result<(), MintDelegatedTokenV2Error> {
    for scope in subset {
        if !superset.contains(scope) {
            return Err(MintDelegatedTokenV2Error::ScopeRejected {
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
            DelegationCertV2, RootKeyCertificateV2, RootKeySetV2, RootPublicKeyV2,
            RootTrustAnchorV2, ShardKeyBindingV2, SignatureAlgorithmV2,
        },
        ids::CanisterRole,
        ops::auth::v2::{
            canonical::{public_key_hash, role_hash},
            verify::{VerifyDelegatedTokenV2Input, verify_delegated_token_v2},
        },
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn cert() -> DelegationCertV2 {
        let role = CanisterRole::new("project_instance");
        let shard_public_key_sec1 = vec![1, 2, 3];
        let shard_key_hash = public_key_hash(&shard_public_key_sec1);

        DelegationCertV2 {
            version: DELEGATED_AUTH_V2_VERSION,
            root_pid: p(1),
            root_key_id: "root-key".to_string(),
            root_key_hash: [9; 32],
            alg: SignatureAlgorithmV2::EcdsaP256Sha256,
            shard_pid: p(2),
            shard_key_id: "shard-key".to_string(),
            shard_public_key_sec1,
            shard_key_hash,
            shard_key_binding: ShardKeyBindingV2::IcThresholdEcdsa {
                key_name_hash: [3; 32],
                derivation_path_hash: [4; 32],
            },
            issued_at: 100,
            expires_at: 500,
            max_token_ttl_secs: 120,
            scopes: vec!["read".to_string(), "write".to_string()],
            aud: DelegationAudienceV2::Roles(vec![role.clone()]),
            verifier_role_hash: Some(role_hash(&role).unwrap()),
        }
    }

    fn proof() -> DelegationProofV2 {
        DelegationProofV2 {
            cert: cert(),
            root_sig: vec![10, 11, 12],
            root_public_key_sec1: Some(vec![13, 14, 15]),
            root_key_cert: Some(RootKeyCertificateV2 {
                root_pid: p(1),
                key_id: "root-key".to_string(),
                alg: SignatureAlgorithmV2::EcdsaP256Sha256,
                public_key_sec1: vec![13, 14, 15],
                key_hash: public_key_hash(&[13, 14, 15]),
                not_before: 90,
                not_after: None,
                authority_sig: vec![16, 17, 18],
            }),
        }
    }

    fn signed_proof_for_local_root() -> (DelegationProofV2, RootTrustAnchorV2) {
        let mut proof = proof();
        let root_public_key_sec1 = proof.root_public_key_sec1.clone().unwrap();
        proof.cert.root_key_hash = public_key_hash(&root_public_key_sec1);
        proof.root_sig = cert_hash(&proof.cert).unwrap().to_vec();
        proof.root_key_cert = None;

        let trust = RootTrustAnchorV2 {
            root_pid: p(1),
            trusted_root_keys: RootKeySetV2 {
                keys: vec![RootPublicKeyV2 {
                    root_pid: p(1),
                    key_id: "root-key".to_string(),
                    alg: SignatureAlgorithmV2::EcdsaP256Sha256,
                    public_key_sec1: root_public_key_sec1,
                    key_hash: proof.cert.root_key_hash,
                    not_before: 90,
                    not_after: None,
                }],
            },
            key_authority: None,
        };

        (proof, trust)
    }

    fn input(proof: &DelegationProofV2) -> MintDelegatedTokenV2Input<'_> {
        MintDelegatedTokenV2Input {
            proof,
            subject: p(9),
            audience: DelegationAudienceV2::Roles(vec![CanisterRole::new("project_instance")]),
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
        _: SignatureAlgorithmV2,
    ) -> Result<(), String> {
        if sig == hash.as_slice() {
            Ok(())
        } else {
            Err("hash mismatch".to_string())
        }
    }

    #[test]
    fn mint_delegated_token_v2_signs_claims_hash_and_embeds_proof() {
        let proof = proof();
        let mut observed_hash = None;

        let token = mint_delegated_token_v2(input(&proof), |hash| {
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
    fn minted_token_feeds_the_pure_v2_verifier() {
        let (proof, trust) = signed_proof_for_local_root();
        let token = mint_delegated_token_v2(input(&proof), |_| Ok(vec![20, 21, 22])).unwrap();
        let role = CanisterRole::new("project_instance");
        let required_scopes = vec!["read".to_string()];

        verify_delegated_token_v2(
            VerifyDelegatedTokenV2Input {
                token: &token,
                root_trust: &trust,
                local_principal: p(99),
                local_role: Some(&role),
                ttl_policy: crate::ops::auth::v2::policy::DelegatedAuthTtlPolicyV2 {
                    max_cert_ttl_secs: 600,
                    max_token_ttl_secs: 120,
                },
                expected_shard_key_hash: token.proof.cert.shard_key_hash,
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
        let left = mint_delegated_token_v2(left_input, |hash| Ok(hash.to_vec())).unwrap();
        let right = mint_delegated_token_v2(right_input, |hash| Ok(hash.to_vec())).unwrap();

        for token in [&left, &right] {
            verify_delegated_token_v2(
                VerifyDelegatedTokenV2Input {
                    token,
                    root_trust: &trust,
                    local_principal: p(99),
                    local_role: Some(&role),
                    ttl_policy: crate::ops::auth::v2::policy::DelegatedAuthTtlPolicyV2 {
                        max_cert_ttl_secs: 600,
                        max_token_ttl_secs: 120,
                    },
                    expected_shard_key_hash: token.proof.cert.shard_key_hash,
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
        let mut token = mint_delegated_token_v2(input(&proof), |hash| Ok(hash.to_vec())).unwrap();
        token.claims.scopes = vec!["write".to_string()];

        assert_eq!(
            verify_delegated_token_v2(
                VerifyDelegatedTokenV2Input {
                    token: &token,
                    root_trust: &trust,
                    local_principal: p(99),
                    local_role: Some(&role),
                    ttl_policy: crate::ops::auth::v2::policy::DelegatedAuthTtlPolicyV2 {
                        max_cert_ttl_secs: 600,
                        max_token_ttl_secs: 120,
                    },
                    expected_shard_key_hash: token.proof.cert.shard_key_hash,
                    required_scopes: &[],
                    now_secs: 130,
                },
                verify_hash_signature,
            ),
            Err(
                crate::ops::auth::v2::verify::VerifyDelegatedTokenV2Error::ShardSignatureInvalid(
                    "hash mismatch".to_string(),
                )
            )
        );
    }

    #[test]
    fn mint_delegated_token_v2_rejects_audience_expansion() {
        let proof = proof();
        let mut input = input(&proof);
        input.audience = DelegationAudienceV2::Roles(vec![CanisterRole::new("project_hub")]);

        assert_eq!(
            mint_delegated_token_v2(input, |_| Ok(vec![])),
            Err(MintDelegatedTokenV2Error::AudienceNotSubset)
        );
    }

    #[test]
    fn mint_delegated_token_v2_allows_role_claim_subset_of_mixed_cert_audience() {
        let mut proof = proof();
        let role = CanisterRole::new("project_instance");
        proof.cert.aud = DelegationAudienceV2::RolesOrPrincipals {
            roles: vec![role],
            principals: vec![],
        };

        let token = mint_delegated_token_v2(input(&proof), |hash| Ok(hash.to_vec())).unwrap();

        assert_eq!(
            token.claims.aud,
            DelegationAudienceV2::Roles(vec![CanisterRole::new("project_instance")])
        );
    }

    #[test]
    fn mint_delegated_token_v2_rejects_scope_expansion() {
        let proof = proof();
        let mut input = input(&proof);
        input.scopes = vec!["admin".to_string()];

        assert_eq!(
            mint_delegated_token_v2(input, |_| Ok(vec![])),
            Err(MintDelegatedTokenV2Error::ScopeRejected {
                scope: "admin".to_string(),
            })
        );
    }

    #[test]
    fn mint_delegated_token_v2_accepts_ttl_equal_to_cert_limit() {
        let proof = proof();
        let mut input = input(&proof);
        input.ttl_secs = 120;

        let token = mint_delegated_token_v2(input, |hash| Ok(hash.to_vec())).unwrap();

        assert_eq!(token.claims.issued_at, 120);
        assert_eq!(token.claims.expires_at, 240);
    }

    #[test]
    fn mint_delegated_token_v2_rejects_token_ttl_above_cert_limit() {
        let proof = proof();
        let mut input = input(&proof);
        input.ttl_secs = 121;

        assert_eq!(
            mint_delegated_token_v2(input, |_| Ok(vec![])),
            Err(MintDelegatedTokenV2Error::TokenTtlExceeded {
                ttl_secs: 121,
                max_ttl_secs: 120,
            })
        );
    }

    #[test]
    fn mint_delegated_token_v2_rejects_token_outliving_cert() {
        let proof = proof();
        let mut input = input(&proof);
        input.now_secs = 490;
        input.ttl_secs = 20;

        assert_eq!(
            mint_delegated_token_v2(input, |_| Ok(vec![])),
            Err(MintDelegatedTokenV2Error::TokenOutlivesCert)
        );
    }

    #[test]
    fn mint_delegated_token_v2_rejects_signing_failure() {
        let proof = proof();

        assert_eq!(
            mint_delegated_token_v2(input(&proof), |_| Err("sign failed".to_string())),
            Err(MintDelegatedTokenV2Error::SignFailed(
                "sign failed".to_string(),
            ))
        );
    }
}
