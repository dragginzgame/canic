use super::{
    audience::{
        AudienceError, audience_accepted, audience_subset, role_grants_subset, scopes_for_role,
    },
    canonical::{CanonicalAuthError, cert_hash, claims_hash},
    cert_rules::{
        CertRuleError, DELEGATED_AUTH_VERSION, DelegatedAuthTtlLimits, validate_cert_issuance_rules,
    },
    root_key::{RootKeyResolutionError, RootKeyResolveRequest, resolve_root_key},
};
use crate::{
    cdk::types::Principal,
    dto::auth::{DelegatedToken, RootTrustAnchor, SignatureAlgorithm},
    ids::CanisterRole,
};
use thiserror::Error;

pub struct VerifyDelegatedTokenInput<'a> {
    pub token: &'a DelegatedToken,
    pub root_trust: &'a RootTrustAnchor,
    pub local_role: Option<&'a CanisterRole>,
    pub local_project: Option<&'a str>,
    pub ttl_limits: DelegatedAuthTtlLimits,
    pub required_scopes: &'a [String],
    pub now_secs: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedDelegatedToken {
    pub subject: Principal,
    pub issuer_shard_pid: Principal,
    pub scopes: Vec<String>,
    pub cert_hash: [u8; 32],
}

#[derive(Debug, Eq, Error, PartialEq)]
pub enum VerifyDelegatedTokenError {
    #[error("delegated auth cert hash mismatch")]
    CertHashMismatch,
    #[error("delegated auth root signature unavailable")]
    RootSignatureUnavailable,
    #[error("delegated auth shard signature unavailable")]
    ShardSignatureUnavailable,
    #[error("delegated auth root signature invalid: {0}")]
    RootSignatureInvalid(String),
    #[error("delegated auth shard signature invalid: {0}")]
    ShardSignatureInvalid(String),
    #[error("delegated auth token issuer shard pid mismatch")]
    IssuerShardPidMismatch,
    #[error("delegated auth token claims version mismatch (expected {expected}, found {found})")]
    TokenVersionMismatch { expected: u16, found: u16 },
    #[error("delegated auth token expiry must be greater than issued_at")]
    TokenInvalidWindow,
    #[error("delegated auth token ttl {ttl_secs}s exceeds cert max {max_ttl_secs}s")]
    TokenTtlExceeded { ttl_secs: u64, max_ttl_secs: u64 },
    #[error("delegated auth token issued before cert")]
    TokenIssuedBeforeCert,
    #[error("delegated auth token expires after cert")]
    TokenOutlivesCert,
    #[error("delegated auth token is not yet valid")]
    TokenNotYetValid,
    #[error("delegated auth token expired")]
    TokenExpired,
    #[error("delegated auth cert is not yet valid")]
    CertNotYetValid,
    #[error("delegated auth cert expired")]
    CertExpired,
    #[error("delegated auth token audience is not a subset of cert audience")]
    AudienceNotSubset,
    #[error("delegated auth verifier is outside token audience")]
    TokenAudienceRejected,
    #[error("delegated auth verifier is outside cert audience")]
    CertAudienceRejected,
    #[error("delegated auth token grants are not a subset of cert grants")]
    GrantsNotSubset,
    #[error("delegated auth local verifier role is outside token grants")]
    TokenGrantRejected,
    #[error("delegated auth local verifier role is required")]
    MissingLocalRole,
    #[error("delegated auth scope rejected: {scope}")]
    ScopeRejected { scope: String },
    #[error(transparent)]
    Canonical(#[from] CanonicalAuthError),
    #[error(transparent)]
    CertRules(#[from] CertRuleError),
    #[error(transparent)]
    RootKey(#[from] RootKeyResolutionError),
    #[error(transparent)]
    Audience(#[from] AudienceError),
}

pub fn verify_delegated_token<F>(
    input: VerifyDelegatedTokenInput<'_>,
    mut verify_signature: F,
) -> Result<VerifiedDelegatedToken, VerifyDelegatedTokenError>
where
    F: FnMut(&[u8], [u8; 32], &[u8], SignatureAlgorithm) -> Result<(), String>,
{
    let cert = &input.token.proof.cert;
    let claims = &input.token.claims;

    validate_cert_issuance_rules(cert, input.ttl_limits, input.root_trust.root_pid)?;
    verify_cert_time(cert.issued_at, cert.expires_at, input.now_secs)?;

    let actual_cert_hash = cert_hash(cert)?;
    if claims.cert_hash != actual_cert_hash {
        return Err(VerifyDelegatedTokenError::CertHashMismatch);
    }

    if input.token.proof.root_sig.is_empty() {
        return Err(VerifyDelegatedTokenError::RootSignatureUnavailable);
    }
    if input.token.shard_sig.is_empty() {
        return Err(VerifyDelegatedTokenError::ShardSignatureUnavailable);
    }

    let root_key = resolve_root_key(
        input.root_trust,
        RootKeyResolveRequest {
            root_pid: cert.root_pid,
            key_id: &cert.root_key_id,
            key_hash: cert.root_key_hash,
            alg: cert.alg,
            now_secs: input.now_secs,
        },
    )?;

    verify_signature(
        &root_key.public_key_sec1,
        actual_cert_hash,
        &input.token.proof.root_sig,
        cert.alg,
    )
    .map_err(VerifyDelegatedTokenError::RootSignatureInvalid)?;

    let local_scopes = verify_claims(&input, actual_cert_hash)?;

    let actual_claims_hash = claims_hash(claims)?;
    verify_signature(
        &cert.shard_public_key_sec1,
        actual_claims_hash,
        &input.token.shard_sig,
        cert.alg,
    )
    .map_err(VerifyDelegatedTokenError::ShardSignatureInvalid)?;

    Ok(VerifiedDelegatedToken {
        subject: claims.subject,
        issuer_shard_pid: claims.issuer_shard_pid,
        scopes: local_scopes,
        cert_hash: actual_cert_hash,
    })
}

const fn verify_cert_time(
    issued_at: u64,
    expires_at: u64,
    now_secs: u64,
) -> Result<(), VerifyDelegatedTokenError> {
    if now_secs < issued_at {
        return Err(VerifyDelegatedTokenError::CertNotYetValid);
    }
    if now_secs >= expires_at {
        return Err(VerifyDelegatedTokenError::CertExpired);
    }
    Ok(())
}

fn verify_claims(
    input: &VerifyDelegatedTokenInput<'_>,
    actual_cert_hash: [u8; 32],
) -> Result<Vec<String>, VerifyDelegatedTokenError> {
    let cert = &input.token.proof.cert;
    let claims = &input.token.claims;

    if claims.version != DELEGATED_AUTH_VERSION {
        return Err(VerifyDelegatedTokenError::TokenVersionMismatch {
            expected: DELEGATED_AUTH_VERSION,
            found: claims.version,
        });
    }
    if claims.issuer_shard_pid != cert.shard_pid {
        return Err(VerifyDelegatedTokenError::IssuerShardPidMismatch);
    }
    if claims.cert_hash != actual_cert_hash {
        return Err(VerifyDelegatedTokenError::CertHashMismatch);
    }

    let token_ttl_secs = claims
        .expires_at
        .checked_sub(claims.issued_at)
        .ok_or(VerifyDelegatedTokenError::TokenInvalidWindow)?;
    if token_ttl_secs == 0 {
        return Err(VerifyDelegatedTokenError::TokenInvalidWindow);
    }
    if token_ttl_secs > cert.max_token_ttl_secs {
        return Err(VerifyDelegatedTokenError::TokenTtlExceeded {
            ttl_secs: token_ttl_secs,
            max_ttl_secs: cert.max_token_ttl_secs,
        });
    }
    if claims.issued_at < cert.issued_at {
        return Err(VerifyDelegatedTokenError::TokenIssuedBeforeCert);
    }
    if claims.expires_at > cert.expires_at {
        return Err(VerifyDelegatedTokenError::TokenOutlivesCert);
    }
    if input.now_secs < claims.issued_at {
        return Err(VerifyDelegatedTokenError::TokenNotYetValid);
    }
    if input.now_secs >= claims.expires_at {
        return Err(VerifyDelegatedTokenError::TokenExpired);
    }

    let local_scopes = verify_audience_and_grants(input)?;
    verify_scopes(input.required_scopes, &local_scopes)?;
    Ok(local_scopes)
}

fn verify_audience_and_grants(
    input: &VerifyDelegatedTokenInput<'_>,
) -> Result<Vec<String>, VerifyDelegatedTokenError> {
    let cert_aud = &input.token.proof.cert.aud;
    let claims_aud = &input.token.claims.aud;
    let local_role = input
        .local_role
        .ok_or(VerifyDelegatedTokenError::MissingLocalRole)?;

    if !audience_subset(claims_aud, cert_aud) {
        return Err(VerifyDelegatedTokenError::AudienceNotSubset);
    }
    if !audience_accepted(input.local_project, claims_aud) {
        return Err(VerifyDelegatedTokenError::TokenAudienceRejected);
    }
    if !audience_accepted(input.local_project, cert_aud) {
        return Err(VerifyDelegatedTokenError::CertAudienceRejected);
    }

    if !role_grants_subset(&input.token.claims.grants, &input.token.proof.cert.grants) {
        return Err(VerifyDelegatedTokenError::GrantsNotSubset);
    }

    scopes_for_role(&input.token.claims.grants, local_role)
        .ok_or(VerifyDelegatedTokenError::TokenGrantRejected)
}

fn verify_scopes(subset: &[String], superset: &[String]) -> Result<(), VerifyDelegatedTokenError> {
    for scope in subset {
        if !superset.contains(scope) {
            return Err(VerifyDelegatedTokenError::ScopeRejected {
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
            DelegatedRoleGrant, DelegatedTokenClaims, DelegationAudience, DelegationCert,
            DelegationProof, RootPublicKey, ShardKeyBinding,
        },
        ops::auth::delegated::{canonical::public_key_hash, cert_rules::DELEGATED_AUTH_VERSION},
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn role() -> CanisterRole {
        CanisterRole::new("project_instance")
    }

    fn ttl_limits() -> DelegatedAuthTtlLimits {
        DelegatedAuthTtlLimits {
            max_cert_ttl_secs: 600,
            max_token_ttl_secs: 120,
        }
    }

    fn root_key() -> RootPublicKey {
        let public_key_sec1 = vec![10, 11, 12];
        RootPublicKey {
            root_pid: p(1),
            key_id: "root-key".to_string(),
            alg: SignatureAlgorithm::EcdsaP256Sha256,
            key_hash: public_key_hash(&public_key_sec1),
            public_key_sec1,
            not_before: 90,
            not_after: None,
        }
    }

    fn root_trust() -> RootTrustAnchor {
        RootTrustAnchor {
            root_pid: p(1),
            root_key: root_key(),
        }
    }

    fn cert() -> DelegationCert {
        let shard_public_key_sec1 = vec![20, 21, 22];
        let shard_key_hash = public_key_hash(&shard_public_key_sec1);
        let root_key = root_key();

        DelegationCert {
            version: DELEGATED_AUTH_VERSION,
            root_pid: p(1),
            root_key_id: root_key.key_id,
            root_key_hash: root_key.key_hash,
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
            aud: DelegationAudience::Project("test".to_string()),
            grants: vec![
                grant("project_hub", &["session", "upload"]),
                grant("project_instance", &["read", "write"]),
                grant("user_shard", &["session"]),
            ],
        }
    }

    fn grant(role: &str, scopes: &[&str]) -> DelegatedRoleGrant {
        DelegatedRoleGrant {
            target: CanisterRole::owned(role.to_string()),
            scopes: scopes.iter().map(|scope| (*scope).to_string()).collect(),
        }
    }

    fn token() -> DelegatedToken {
        let cert = cert();
        let cert_hash = cert_hash(&cert).unwrap();
        DelegatedToken {
            claims: DelegatedTokenClaims {
                version: DELEGATED_AUTH_VERSION,
                subject: p(9),
                issuer_shard_pid: cert.shard_pid,
                cert_hash,
                issued_at: 120,
                expires_at: 180,
                aud: cert.aud.clone(),
                grants: vec![
                    grant("project_hub", &["upload"]),
                    grant("project_instance", &["read"]),
                    grant("user_shard", &["session"]),
                ],
                nonce: [7; 16],
            },
            proof: DelegationProof {
                cert,
                root_sig: vec![1, 2, 3],
            },
            shard_sig: vec![4, 5, 6],
        }
    }

    fn input<'a>(
        token: &'a DelegatedToken,
        trust: &'a RootTrustAnchor,
        local_role: Option<&'a CanisterRole>,
        required_scopes: &'a [String],
    ) -> VerifyDelegatedTokenInput<'a> {
        VerifyDelegatedTokenInput {
            token,
            root_trust: trust,
            local_role,
            local_project: Some("test"),
            ttl_limits: ttl_limits(),
            required_scopes,
            now_secs: 150,
        }
    }

    #[test]
    fn verify_delegated_token_accepts_self_validating_token_without_proof_lookup() {
        let token = token();
        let trust = root_trust();
        let role = role();
        let required_scopes = vec!["read".to_string()];
        let mut verified_hashes = Vec::new();

        let verified = verify_delegated_token(
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
    fn verify_delegated_token_rejects_root_signature_failure() {
        let token = token();
        let trust = root_trust();
        let role = role();

        assert_eq!(
            verify_delegated_token(input(&token, &trust, Some(&role), &[]), |_, _, sig, _| {
                if sig == [1, 2, 3] {
                    Err("bad root sig".to_string())
                } else {
                    Ok(())
                }
            }),
            Err(VerifyDelegatedTokenError::RootSignatureInvalid(
                "bad root sig".to_string(),
            ))
        );
    }

    #[test]
    fn verify_delegated_token_rejects_shard_signature_failure() {
        let token = token();
        let trust = root_trust();
        let role = role();

        assert_eq!(
            verify_delegated_token(input(&token, &trust, Some(&role), &[]), |_, _, sig, _| {
                if sig == [4, 5, 6] {
                    Err("bad shard sig".to_string())
                } else {
                    Ok(())
                }
            }),
            Err(VerifyDelegatedTokenError::ShardSignatureInvalid(
                "bad shard sig".to_string(),
            ))
        );
    }

    #[test]
    fn verify_delegated_token_rejects_cert_hash_drift() {
        let mut token = token();
        token.claims.cert_hash = [0; 32];
        let trust = root_trust();
        let role = role();

        assert_eq!(
            verify_delegated_token(input(&token, &trust, Some(&role), &[]), |_, _, _, _| Ok(())),
            Err(VerifyDelegatedTokenError::CertHashMismatch)
        );
    }

    #[test]
    fn verify_delegated_token_rejects_claims_version_mismatch() {
        let mut token = token();
        token.claims.version = DELEGATED_AUTH_VERSION - 1;
        let trust = root_trust();
        let role = role();

        assert_eq!(
            verify_delegated_token(input(&token, &trust, Some(&role), &[]), |_, _, _, _| Ok(())),
            Err(VerifyDelegatedTokenError::TokenVersionMismatch {
                expected: DELEGATED_AUTH_VERSION,
                found: DELEGATED_AUTH_VERSION - 1,
            })
        );
    }

    #[test]
    fn verify_delegated_token_rejects_noncanonical_cert_grants() {
        let mut token = token();
        token.proof.cert.grants = vec![
            grant("project_instance", &["read"]),
            grant("project_hub", &["upload"]),
        ];
        let trust = root_trust();
        let role = role();

        assert_eq!(
            verify_delegated_token(input(&token, &trust, Some(&role), &[]), |_, _, _, _| Ok(())),
            Err(VerifyDelegatedTokenError::CertRules(
                CertRuleError::Audience(AudienceError::NonCanonicalGrants)
            ))
        );
    }

    #[test]
    fn verify_delegated_token_rejects_noncanonical_claim_grants() {
        let mut token = token();
        token.claims.grants = vec![
            grant("project_instance", &["read"]),
            grant("project_hub", &["upload"]),
        ];
        let trust = root_trust();
        let role = role();

        assert_eq!(
            verify_delegated_token(input(&token, &trust, Some(&role), &[]), |_, _, _, _| Ok(())),
            Err(VerifyDelegatedTokenError::Canonical(
                CanonicalAuthError::NonCanonicalRoles
            ))
        );
    }

    #[test]
    fn verify_delegated_token_rejects_audience_subset_drift() {
        let mut token = token();
        token.claims.aud = DelegationAudience::Canic;
        let trust = root_trust();
        let role = role();

        assert_eq!(
            verify_delegated_token(input(&token, &trust, Some(&role), &[]), |_, _, _, _| Ok(())),
            Err(VerifyDelegatedTokenError::AudienceNotSubset)
        );
    }

    #[test]
    fn verify_delegated_token_rejects_non_matching_project_audience() {
        let mut token = token();
        token.proof.cert.aud = DelegationAudience::Project("other".to_string());
        token.claims.aud = DelegationAudience::Project("other".to_string());
        token.claims.cert_hash = cert_hash(&token.proof.cert).unwrap();
        let trust = root_trust();
        let role = role();

        assert_eq!(
            verify_delegated_token(input(&token, &trust, Some(&role), &[]), |_, _, _, _| Ok(())),
            Err(VerifyDelegatedTokenError::TokenAudienceRejected)
        );
    }

    #[test]
    fn verify_delegated_token_rejects_missing_local_role_for_grant_lookup() {
        let token = token();
        let trust = root_trust();

        assert_eq!(
            verify_delegated_token(input(&token, &trust, None, &[]), |_, _, _, _| Ok(())),
            Err(VerifyDelegatedTokenError::MissingLocalRole)
        );
    }

    #[test]
    fn verify_delegated_token_rejects_local_role_outside_token_grants() {
        let token = token();
        let trust = root_trust();
        let role = CanisterRole::new("admin");

        assert_eq!(
            verify_delegated_token(input(&token, &trust, Some(&role), &[]), |_, _, _, _| Ok(())),
            Err(VerifyDelegatedTokenError::TokenGrantRejected)
        );
    }

    #[test]
    fn verify_delegated_token_rejects_claim_grant_expansion() {
        let mut token = token();
        token.claims.grants = vec![grant("project_instance", &["admin"])];
        let trust = root_trust();
        let role = role();

        assert_eq!(
            verify_delegated_token(input(&token, &trust, Some(&role), &[]), |_, _, _, _| Ok(())),
            Err(VerifyDelegatedTokenError::GrantsNotSubset)
        );
    }

    #[test]
    fn verify_delegated_token_rejects_required_scope_outside_local_role_grant() {
        let token = token();
        let trust = root_trust();
        let role = role();
        let required_scopes = vec!["admin".to_string()];

        assert_eq!(
            verify_delegated_token(
                input(&token, &trust, Some(&role), &required_scopes),
                |_, _, _, _| Ok(()),
            ),
            Err(VerifyDelegatedTokenError::ScopeRejected {
                scope: "admin".to_string(),
            })
        );
    }

    #[test]
    fn verify_delegated_token_rejects_expired_token_at_boundary() {
        let token = token();
        let trust = root_trust();
        let role = role();
        let mut input = input(&token, &trust, Some(&role), &[]);
        input.now_secs = 180;

        assert_eq!(
            verify_delegated_token(input, |_, _, _, _| Ok(())),
            Err(VerifyDelegatedTokenError::TokenExpired)
        );
    }
}
