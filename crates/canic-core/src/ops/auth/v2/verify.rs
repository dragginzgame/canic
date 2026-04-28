use super::{
    audience::{AudienceV2Error, audience_subset, audience_uses_role, verifier_is_in_audience},
    canonical::{CanonicalAuthV2Error, cert_hash, claims_hash, role_hash},
    policy::{CertPolicyV2Error, DelegatedAuthTtlPolicyV2, validate_cert_issuance_policy},
    root_key::{RootKeyResolutionV2Error, RootKeyResolveRequestV2, resolve_root_key},
};
use crate::{
    cdk::types::Principal,
    dto::auth::{DelegatedTokenV2, RootTrustAnchorV2, SignatureAlgorithmV2},
    ids::CanisterRole,
};
use thiserror::Error;

pub struct VerifyDelegatedTokenV2Input<'a> {
    pub token: &'a DelegatedTokenV2,
    pub root_trust: &'a RootTrustAnchorV2,
    pub local_principal: Principal,
    pub local_role: Option<&'a CanisterRole>,
    pub ttl_policy: DelegatedAuthTtlPolicyV2,
    pub expected_shard_key_hash: [u8; 32],
    pub required_scopes: &'a [String],
    pub now_secs: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedDelegationV2 {
    pub subject: Principal,
    pub issuer_shard_pid: Principal,
    pub scopes: Vec<String>,
    pub cert_hash: [u8; 32],
}

#[derive(Debug, Eq, Error, PartialEq)]
pub enum VerifyDelegatedTokenV2Error {
    #[error("delegated auth v2 cert hash mismatch")]
    CertHashMismatch,
    #[error("delegated auth v2 root signature unavailable")]
    RootSignatureUnavailable,
    #[error("delegated auth v2 shard signature unavailable")]
    ShardSignatureUnavailable,
    #[error("delegated auth v2 root signature invalid: {0}")]
    RootSignatureInvalid(String),
    #[error("delegated auth v2 shard signature invalid: {0}")]
    ShardSignatureInvalid(String),
    #[error("delegated auth v2 token issuer shard pid mismatch")]
    IssuerShardPidMismatch,
    #[error("delegated auth v2 token expiry must be greater than issued_at")]
    TokenInvalidWindow,
    #[error("delegated auth v2 token ttl {ttl_secs}s exceeds cert max {max_ttl_secs}s")]
    TokenTtlExceeded { ttl_secs: u64, max_ttl_secs: u64 },
    #[error("delegated auth v2 token issued before cert")]
    TokenIssuedBeforeCert,
    #[error("delegated auth v2 token expires after cert")]
    TokenOutlivesCert,
    #[error("delegated auth v2 token is not yet valid")]
    TokenNotYetValid,
    #[error("delegated auth v2 token expired")]
    TokenExpired,
    #[error("delegated auth v2 cert is not yet valid")]
    CertNotYetValid,
    #[error("delegated auth v2 cert expired")]
    CertExpired,
    #[error("delegated auth v2 token audience is not a subset of cert audience")]
    AudienceNotSubset,
    #[error("delegated auth v2 verifier is outside token audience")]
    TokenAudienceRejected,
    #[error("delegated auth v2 verifier is outside cert audience")]
    CertAudienceRejected,
    #[error("delegated auth v2 local verifier role is required")]
    MissingLocalRole,
    #[error("delegated auth v2 local verifier role hash mismatch")]
    LocalRoleHashMismatch,
    #[error("delegated auth v2 scope rejected: {scope}")]
    ScopeRejected { scope: String },
    #[error(transparent)]
    Canonical(#[from] CanonicalAuthV2Error),
    #[error(transparent)]
    CertPolicy(#[from] CertPolicyV2Error),
    #[error(transparent)]
    RootKey(#[from] RootKeyResolutionV2Error),
    #[error(transparent)]
    Audience(#[from] AudienceV2Error),
}

pub fn verify_delegated_token_v2<F>(
    input: VerifyDelegatedTokenV2Input<'_>,
    mut verify_signature: F,
) -> Result<VerifiedDelegationV2, VerifyDelegatedTokenV2Error>
where
    F: FnMut(&[u8], [u8; 32], &[u8], SignatureAlgorithmV2) -> Result<(), String>,
{
    let cert = &input.token.proof.cert;
    let claims = &input.token.claims;

    validate_cert_issuance_policy(
        cert,
        input.ttl_policy,
        input.root_trust.root_pid,
        input.expected_shard_key_hash,
    )?;
    verify_cert_time(cert.issued_at, cert.expires_at, input.now_secs)?;

    let actual_cert_hash = cert_hash(cert)?;
    if claims.cert_hash != actual_cert_hash {
        return Err(VerifyDelegatedTokenV2Error::CertHashMismatch);
    }

    if input.token.proof.root_sig.is_empty() {
        return Err(VerifyDelegatedTokenV2Error::RootSignatureUnavailable);
    }
    if input.token.shard_sig.is_empty() {
        return Err(VerifyDelegatedTokenV2Error::ShardSignatureUnavailable);
    }

    let root_key = resolve_root_key(
        input.root_trust,
        RootKeyResolveRequestV2 {
            root_pid: cert.root_pid,
            key_id: &cert.root_key_id,
            key_hash: cert.root_key_hash,
            alg: cert.alg,
            embedded_key: input.token.proof.root_public_key_sec1.as_deref(),
            embedded_key_cert: input.token.proof.root_key_cert.as_ref(),
            now_secs: input.now_secs,
        },
        |public_key, hash, sig, alg| verify_signature(public_key, hash, sig, alg),
    )?;

    verify_signature(
        &root_key.public_key_sec1,
        actual_cert_hash,
        &input.token.proof.root_sig,
        cert.alg,
    )
    .map_err(VerifyDelegatedTokenV2Error::RootSignatureInvalid)?;

    verify_claims(&input, actual_cert_hash)?;

    let actual_claims_hash = claims_hash(claims)?;
    verify_signature(
        &cert.shard_public_key_sec1,
        actual_claims_hash,
        &input.token.shard_sig,
        cert.alg,
    )
    .map_err(VerifyDelegatedTokenV2Error::ShardSignatureInvalid)?;

    Ok(VerifiedDelegationV2 {
        subject: claims.subject,
        issuer_shard_pid: claims.issuer_shard_pid,
        scopes: claims.scopes.clone(),
        cert_hash: actual_cert_hash,
    })
}

const fn verify_cert_time(
    issued_at: u64,
    expires_at: u64,
    now_secs: u64,
) -> Result<(), VerifyDelegatedTokenV2Error> {
    if now_secs < issued_at {
        return Err(VerifyDelegatedTokenV2Error::CertNotYetValid);
    }
    if now_secs >= expires_at {
        return Err(VerifyDelegatedTokenV2Error::CertExpired);
    }
    Ok(())
}

fn verify_claims(
    input: &VerifyDelegatedTokenV2Input<'_>,
    actual_cert_hash: [u8; 32],
) -> Result<(), VerifyDelegatedTokenV2Error> {
    let cert = &input.token.proof.cert;
    let claims = &input.token.claims;

    if claims.issuer_shard_pid != cert.shard_pid {
        return Err(VerifyDelegatedTokenV2Error::IssuerShardPidMismatch);
    }
    if claims.cert_hash != actual_cert_hash {
        return Err(VerifyDelegatedTokenV2Error::CertHashMismatch);
    }

    let token_ttl_secs = claims
        .expires_at
        .checked_sub(claims.issued_at)
        .ok_or(VerifyDelegatedTokenV2Error::TokenInvalidWindow)?;
    if token_ttl_secs == 0 {
        return Err(VerifyDelegatedTokenV2Error::TokenInvalidWindow);
    }
    if token_ttl_secs > cert.max_token_ttl_secs {
        return Err(VerifyDelegatedTokenV2Error::TokenTtlExceeded {
            ttl_secs: token_ttl_secs,
            max_ttl_secs: cert.max_token_ttl_secs,
        });
    }
    if claims.issued_at < cert.issued_at {
        return Err(VerifyDelegatedTokenV2Error::TokenIssuedBeforeCert);
    }
    if claims.expires_at > cert.expires_at {
        return Err(VerifyDelegatedTokenV2Error::TokenOutlivesCert);
    }
    if input.now_secs < claims.issued_at {
        return Err(VerifyDelegatedTokenV2Error::TokenNotYetValid);
    }
    if input.now_secs >= claims.expires_at {
        return Err(VerifyDelegatedTokenV2Error::TokenExpired);
    }

    verify_audience(input)?;
    verify_scopes(&claims.scopes, &cert.scopes)?;
    verify_scopes(input.required_scopes, &claims.scopes)
}

fn verify_audience(
    input: &VerifyDelegatedTokenV2Input<'_>,
) -> Result<(), VerifyDelegatedTokenV2Error> {
    let cert_aud = &input.token.proof.cert.aud;
    let claims_aud = &input.token.claims.aud;

    if audience_uses_role(claims_aud) || audience_uses_role(cert_aud) {
        let local_role = input
            .local_role
            .ok_or(VerifyDelegatedTokenV2Error::MissingLocalRole)?;
        if input.token.proof.cert.verifier_role_hash != Some(role_hash(local_role)?) {
            return Err(VerifyDelegatedTokenV2Error::LocalRoleHashMismatch);
        }
    }

    if !audience_subset(claims_aud, cert_aud) {
        return Err(VerifyDelegatedTokenV2Error::AudienceNotSubset);
    }
    if !verifier_is_in_audience(input.local_principal, input.local_role, claims_aud) {
        return Err(VerifyDelegatedTokenV2Error::TokenAudienceRejected);
    }
    if !verifier_is_in_audience(input.local_principal, input.local_role, cert_aud) {
        return Err(VerifyDelegatedTokenV2Error::CertAudienceRejected);
    }

    Ok(())
}

fn verify_scopes(
    subset: &[String],
    superset: &[String],
) -> Result<(), VerifyDelegatedTokenV2Error> {
    for scope in subset {
        if !superset.contains(scope) {
            return Err(VerifyDelegatedTokenV2Error::ScopeRejected {
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
            DelegatedTokenClaimsV2, DelegationAudienceV2, DelegationCertV2, DelegationProofV2,
            RootKeySetV2, RootPublicKeyV2, ShardKeyBindingV2,
        },
        ops::auth::v2::{
            canonical::{public_key_hash, role_hash},
            policy::DELEGATED_AUTH_V2_VERSION,
        },
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn role() -> CanisterRole {
        CanisterRole::new("project_instance")
    }

    fn ttl_policy() -> DelegatedAuthTtlPolicyV2 {
        DelegatedAuthTtlPolicyV2 {
            max_cert_ttl_secs: 600,
            max_token_ttl_secs: 120,
        }
    }

    fn root_key() -> RootPublicKeyV2 {
        let public_key_sec1 = vec![10, 11, 12];
        RootPublicKeyV2 {
            root_pid: p(1),
            key_id: "root-key".to_string(),
            alg: SignatureAlgorithmV2::EcdsaP256Sha256,
            key_hash: public_key_hash(&public_key_sec1),
            public_key_sec1,
            not_before: 90,
            not_after: None,
        }
    }

    fn root_trust() -> RootTrustAnchorV2 {
        RootTrustAnchorV2 {
            root_pid: p(1),
            trusted_root_keys: RootKeySetV2 {
                keys: vec![root_key()],
            },
            key_authority: None,
        }
    }

    fn cert() -> DelegationCertV2 {
        let role = role();
        let shard_public_key_sec1 = vec![20, 21, 22];
        let shard_key_hash = public_key_hash(&shard_public_key_sec1);
        let root_key = root_key();

        DelegationCertV2 {
            version: DELEGATED_AUTH_V2_VERSION,
            root_pid: p(1),
            root_key_id: root_key.key_id,
            root_key_hash: root_key.key_hash,
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

    fn token() -> DelegatedTokenV2 {
        let cert = cert();
        let cert_hash = cert_hash(&cert).unwrap();
        DelegatedTokenV2 {
            claims: DelegatedTokenClaimsV2 {
                version: DELEGATED_AUTH_V2_VERSION,
                subject: p(9),
                issuer_shard_pid: cert.shard_pid,
                cert_hash,
                issued_at: 120,
                expires_at: 180,
                aud: cert.aud.clone(),
                scopes: vec!["read".to_string()],
                nonce: [7; 16],
            },
            proof: DelegationProofV2 {
                cert,
                root_sig: vec![1, 2, 3],
                root_public_key_sec1: None,
                root_key_cert: None,
            },
            shard_sig: vec![4, 5, 6],
        }
    }

    fn input<'a>(
        token: &'a DelegatedTokenV2,
        trust: &'a RootTrustAnchorV2,
        local_role: Option<&'a CanisterRole>,
        required_scopes: &'a [String],
    ) -> VerifyDelegatedTokenV2Input<'a> {
        VerifyDelegatedTokenV2Input {
            token,
            root_trust: trust,
            local_principal: p(99),
            local_role,
            ttl_policy: ttl_policy(),
            expected_shard_key_hash: token.proof.cert.shard_key_hash,
            required_scopes,
            now_secs: 150,
        }
    }

    #[test]
    fn verify_delegated_token_v2_accepts_self_validating_token_without_proof_lookup() {
        let token = token();
        let trust = root_trust();
        let role = role();
        let required_scopes = vec!["read".to_string()];
        let mut verified_hashes = Vec::new();

        let verified = verify_delegated_token_v2(
            input(&token, &trust, Some(&role), &required_scopes),
            |_, hash, sig, _| {
                verified_hashes.push((hash, sig.to_vec()));
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(verified.subject, p(9));
        assert_eq!(verified.issuer_shard_pid, p(2));
        assert_eq!(verified.scopes, vec!["read".to_string()]);
        assert_eq!(verified_hashes.len(), 2);
        assert_eq!(verified_hashes[0].1, vec![1, 2, 3]);
        assert_eq!(verified_hashes[1].1, vec![4, 5, 6]);
    }

    #[test]
    fn verify_delegated_token_v2_rejects_root_signature_failure() {
        let token = token();
        let trust = root_trust();
        let role = role();

        assert_eq!(
            verify_delegated_token_v2(input(&token, &trust, Some(&role), &[]), |_, _, sig, _| {
                if sig == [1, 2, 3] {
                    Err("bad root sig".to_string())
                } else {
                    Ok(())
                }
            }),
            Err(VerifyDelegatedTokenV2Error::RootSignatureInvalid(
                "bad root sig".to_string(),
            ))
        );
    }

    #[test]
    fn verify_delegated_token_v2_rejects_shard_signature_failure() {
        let token = token();
        let trust = root_trust();
        let role = role();

        assert_eq!(
            verify_delegated_token_v2(input(&token, &trust, Some(&role), &[]), |_, _, sig, _| {
                if sig == [4, 5, 6] {
                    Err("bad shard sig".to_string())
                } else {
                    Ok(())
                }
            }),
            Err(VerifyDelegatedTokenV2Error::ShardSignatureInvalid(
                "bad shard sig".to_string(),
            ))
        );
    }

    #[test]
    fn verify_delegated_token_v2_rejects_cert_hash_drift() {
        let mut token = token();
        token.claims.cert_hash = [0; 32];
        let trust = root_trust();
        let role = role();

        assert_eq!(
            verify_delegated_token_v2(input(&token, &trust, Some(&role), &[]), |_, _, _, _| Ok(())),
            Err(VerifyDelegatedTokenV2Error::CertHashMismatch)
        );
    }

    #[test]
    fn verify_delegated_token_v2_rejects_audience_subset_drift() {
        let mut token = token();
        token.claims.aud = DelegationAudienceV2::Roles(vec![CanisterRole::new("project_hub")]);
        let trust = root_trust();
        let role = role();

        assert_eq!(
            verify_delegated_token_v2(input(&token, &trust, Some(&role), &[]), |_, _, _, _| Ok(())),
            Err(VerifyDelegatedTokenV2Error::AudienceNotSubset)
        );
    }

    #[test]
    fn verify_delegated_token_v2_rejects_missing_local_role_for_role_audience() {
        let token = token();
        let trust = root_trust();

        assert_eq!(
            verify_delegated_token_v2(input(&token, &trust, None, &[]), |_, _, _, _| Ok(())),
            Err(VerifyDelegatedTokenV2Error::MissingLocalRole)
        );
    }

    #[test]
    fn verify_delegated_token_v2_rejects_required_scope_outside_claims() {
        let token = token();
        let trust = root_trust();
        let role = role();
        let required_scopes = vec!["admin".to_string()];

        assert_eq!(
            verify_delegated_token_v2(
                input(&token, &trust, Some(&role), &required_scopes),
                |_, _, _, _| Ok(()),
            ),
            Err(VerifyDelegatedTokenV2Error::ScopeRejected {
                scope: "admin".to_string(),
            })
        );
    }

    #[test]
    fn verify_delegated_token_v2_rejects_expired_token_at_boundary() {
        let token = token();
        let trust = root_trust();
        let role = role();
        let mut input = input(&token, &trust, Some(&role), &[]);
        input.now_secs = 180;

        assert_eq!(
            verify_delegated_token_v2(input, |_, _, _, _| Ok(())),
            Err(VerifyDelegatedTokenV2Error::TokenExpired)
        );
    }
}
