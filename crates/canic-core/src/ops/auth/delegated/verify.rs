use super::{
    audience::{
        AudienceAcceptanceContext, AudienceError, audience_accepted, audience_subset,
        role_grants_subset, scopes_for_role,
    },
    canonical::{CanonicalAuthError, cert_hash, claims_hash},
    cert_rules::{CertRuleError, DelegatedAuthTtlLimits, validate_cert_issuance_rules},
};
use crate::{
    cdk::types::Principal,
    dto::auth::{DelegatedToken, IssuerProof, RootProof},
    ids::CanisterRole,
    ops::auth::AUTH_TIME_SKEW_ALLOWANCE_NS,
};
use thiserror::Error;

pub struct VerifyDelegatedTokenInput<'a> {
    pub token: &'a DelegatedToken,
    pub local_canister: Principal,
    pub local_canic_subnet: Option<Principal>,
    pub local_role: Option<&'a CanisterRole>,
    pub local_project: Option<&'a str>,
    pub ttl_limits: DelegatedAuthTtlLimits,
    pub required_scopes: &'a [String],
    pub now_ns: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedDelegatedToken {
    pub subject: Principal,
    pub issuer_pid: Principal,
    pub scopes: Vec<String>,
    pub cert_hash: [u8; 32],
}

#[derive(Debug, Eq, Error, PartialEq)]
pub enum VerifyDelegatedTokenError {
    #[error("delegated auth cert hash mismatch")]
    CertHashMismatch,
    #[error("delegated auth issuer proof unavailable")]
    IssuerProofUnavailable,
    #[error("delegated auth root signature invalid: {0}")]
    RootSignatureInvalid(String),
    #[error("delegated auth issuer proof invalid: {0}")]
    IssuerProofInvalid(String),
    #[error("delegated auth token issuer pid mismatch")]
    IssuerPidMismatch,
    #[error("delegated auth token expiry must be greater than issued_at")]
    TokenInvalidWindow,
    #[error("delegated auth token ttl {ttl_ns}ns exceeds cert max {max_ttl_ns}ns")]
    TokenTtlExceeded { ttl_ns: u64, max_ttl_ns: u64 },
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
    Audience(#[from] AudienceError),
}

pub fn verify_delegated_token<R, S>(
    input: VerifyDelegatedTokenInput<'_>,
    mut verify_root_proof: R,
    mut verify_issuer_proof: S,
) -> Result<VerifiedDelegatedToken, VerifyDelegatedTokenError>
where
    R: FnMut([u8; 32], &RootProof, Principal) -> Result<(), String>,
    S: FnMut([u8; 32], &IssuerProof, Principal) -> Result<(), String>,
{
    let material = verify_delegated_token_material(&input, true)?;

    verify_root_proof(
        material.cert_hash,
        &input.token.proof.root_proof,
        input.token.proof.cert.root_pid,
    )
    .map_err(VerifyDelegatedTokenError::RootSignatureInvalid)?;

    verify_issuer_proof(
        material.claims_hash,
        &input.token.issuer_proof,
        input.token.proof.cert.issuer_pid,
    )
    .map_err(VerifyDelegatedTokenError::IssuerProofInvalid)?;

    Ok(material.verified)
}

pub fn verify_delegated_token_without_signatures(
    input: VerifyDelegatedTokenInput<'_>,
) -> Result<VerifiedDelegatedToken, VerifyDelegatedTokenError> {
    verify_delegated_token_material(&input, false).map(|material| material.verified)
}

struct VerifiedDelegatedTokenMaterial {
    verified: VerifiedDelegatedToken,
    cert_hash: [u8; 32],
    claims_hash: [u8; 32],
}

fn verify_delegated_token_material(
    input: &VerifyDelegatedTokenInput<'_>,
    require_issuer_proof_bytes: bool,
) -> Result<VerifiedDelegatedTokenMaterial, VerifyDelegatedTokenError> {
    let cert = &input.token.proof.cert;
    let claims = &input.token.claims;

    validate_cert_issuance_rules(cert, input.ttl_limits, cert.root_pid)?;
    verify_cert_time(cert.not_before_ns, cert.expires_at_ns, input.now_ns)?;

    let actual_cert_hash = cert_hash(cert)?;
    if claims.cert_hash != actual_cert_hash {
        return Err(VerifyDelegatedTokenError::CertHashMismatch);
    }

    let local_scopes = verify_claims(input, actual_cert_hash)?;
    let actual_claims_hash = claims_hash(claims)?;
    let IssuerProof::IcCanisterSignatureV1(issuer_proof) = &input.token.issuer_proof;
    if require_issuer_proof_bytes
        && (issuer_proof.signature_cbor.is_empty() || issuer_proof.public_key_der.is_empty())
    {
        return Err(VerifyDelegatedTokenError::IssuerProofUnavailable);
    }

    Ok(VerifiedDelegatedTokenMaterial {
        verified: VerifiedDelegatedToken {
            subject: claims.subject,
            issuer_pid: claims.issuer_pid,
            scopes: local_scopes,
            cert_hash: actual_cert_hash,
        },
        cert_hash: actual_cert_hash,
        claims_hash: actual_claims_hash,
    })
}

const fn verify_cert_time(
    not_before_ns: u64,
    expires_at_ns: u64,
    now_ns: u64,
) -> Result<(), VerifyDelegatedTokenError> {
    if not_before_ns > now_ns.saturating_add(AUTH_TIME_SKEW_ALLOWANCE_NS) {
        return Err(VerifyDelegatedTokenError::CertNotYetValid);
    }
    if now_ns >= expires_at_ns {
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

    if claims.issuer_pid != cert.issuer_pid {
        return Err(VerifyDelegatedTokenError::IssuerPidMismatch);
    }
    if claims.cert_hash != actual_cert_hash {
        return Err(VerifyDelegatedTokenError::CertHashMismatch);
    }

    let token_ttl_ns = claims
        .expires_at_ns
        .checked_sub(claims.issued_at_ns)
        .ok_or(VerifyDelegatedTokenError::TokenInvalidWindow)?;
    if token_ttl_ns == 0 {
        return Err(VerifyDelegatedTokenError::TokenInvalidWindow);
    }
    if token_ttl_ns > cert.max_token_ttl_ns {
        return Err(VerifyDelegatedTokenError::TokenTtlExceeded {
            ttl_ns: token_ttl_ns,
            max_ttl_ns: cert.max_token_ttl_ns,
        });
    }
    if claims.issued_at_ns < cert.not_before_ns {
        return Err(VerifyDelegatedTokenError::TokenIssuedBeforeCert);
    }
    if claims.expires_at_ns > cert.expires_at_ns {
        return Err(VerifyDelegatedTokenError::TokenOutlivesCert);
    }
    if claims.issued_at_ns > input.now_ns.saturating_add(AUTH_TIME_SKEW_ALLOWANCE_NS) {
        return Err(VerifyDelegatedTokenError::TokenNotYetValid);
    }
    if input.now_ns >= claims.expires_at_ns {
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
    let audience_ctx = AudienceAcceptanceContext {
        local_canister: input.local_canister,
        local_canic_subnet: input.local_canic_subnet,
        local_project: input.local_project,
    };
    if !audience_accepted(audience_ctx, claims_aud) {
        return Err(VerifyDelegatedTokenError::TokenAudienceRejected);
    }
    if !audience_accepted(audience_ctx, cert_aud) {
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
            DelegationProof, IcCanisterSignatureProofV1, IssuerProof, IssuerProofAlgorithm,
            IssuerProofBinding, RootProof,
        },
        ops::auth::delegated::canonical::{claims_hash, issuer_proof_binding_hash},
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn role() -> CanisterRole {
        CanisterRole::new("project_instance")
    }

    fn ttl_limits() -> DelegatedAuthTtlLimits {
        DelegatedAuthTtlLimits {
            max_cert_ttl_ns: 600,
            max_token_ttl_ns: 120,
        }
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
        let claims = DelegatedTokenClaims {
            subject: p(9),
            issuer_pid: cert.issuer_pid,
            cert_hash,
            issued_at_ns: 120,
            expires_at_ns: 180,
            aud: cert.aud.clone(),
            grants: vec![
                grant("project_hub", &["upload"]),
                grant("project_instance", &["read"]),
                grant("user_shard", &["session"]),
            ],
            nonce: [7; 16],
            ext: None,
        };
        let issuer_proof = issuer_proof_for_claims(&claims);

        DelegatedToken {
            claims,
            proof: DelegationProof {
                cert,
                root_proof: root_proof(1),
            },
            issuer_proof,
        }
    }

    fn input<'a>(
        token: &'a DelegatedToken,
        local_role: Option<&'a CanisterRole>,
        required_scopes: &'a [String],
    ) -> VerifyDelegatedTokenInput<'a> {
        VerifyDelegatedTokenInput {
            token,
            local_canister: p(20),
            local_canic_subnet: Some(p(21)),
            local_role,
            local_project: Some("test"),
            ttl_limits: ttl_limits(),
            required_scopes,
            now_ns: 150,
        }
    }

    fn input_at<'a>(
        token: &'a DelegatedToken,
        local_role: Option<&'a CanisterRole>,
        required_scopes: &'a [String],
        now_ns: u64,
    ) -> VerifyDelegatedTokenInput<'a> {
        let mut input = input(token, local_role, required_scopes);
        input.now_ns = now_ns;
        input
    }

    fn future_token(now_ns: u64, offset_ns: u64) -> DelegatedToken {
        let mut token = token();
        let issued_at_ns = now_ns + offset_ns;
        token.proof.cert.issued_at_ns = issued_at_ns;
        token.proof.cert.not_before_ns = issued_at_ns;
        token.proof.cert.expires_at_ns = issued_at_ns + 120;
        token.claims.issued_at_ns = issued_at_ns;
        token.claims.expires_at_ns = issued_at_ns + 60;
        token.claims.cert_hash = cert_hash(&token.proof.cert).unwrap();
        token.issuer_proof = issuer_proof_for_claims(&token.claims);
        token
    }

    fn root_proof(byte: u8) -> RootProof {
        RootProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1 {
            signature_cbor: vec![byte; 8],
            public_key_der: vec![byte; 4],
        })
    }

    fn verify_root_ok(
        expected_cert_hash: [u8; 32],
    ) -> impl FnMut([u8; 32], &RootProof, Principal) -> Result<(), String> {
        move |actual_cert_hash, proof, root_pid| {
            if actual_cert_hash != expected_cert_hash {
                return Err("cert hash mismatch".to_string());
            }
            if root_pid != p(1) {
                return Err("root pid mismatch".to_string());
            }
            match proof {
                RootProof::IcCanisterSignatureV1(proof)
                    if !proof.signature_cbor.is_empty() && !proof.public_key_der.is_empty() =>
                {
                    Ok(())
                }
                RootProof::IcCanisterSignatureV1(_) => Err("root proof missing".to_string()),
            }
        }
    }

    fn issuer_proof_for_claims(claims: &DelegatedTokenClaims) -> IssuerProof {
        IssuerProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1 {
            signature_cbor: claims_hash(claims).unwrap().to_vec(),
            public_key_der: vec![9; 4],
        })
    }

    fn verify_issuer_ok(
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

    fn verify_root_and_issuer(
        token: &DelegatedToken,
        local_role: Option<&CanisterRole>,
        required_scopes: &[String],
    ) -> Result<VerifiedDelegatedToken, VerifyDelegatedTokenError> {
        verify_delegated_token(
            input(token, local_role, required_scopes),
            verify_root_ok(cert_hash(&token.proof.cert).unwrap()),
            verify_issuer_ok,
        )
    }

    #[test]
    fn verify_delegated_token_accepts_self_validating_token_without_proof_lookup() {
        let token = token();
        let role = role();
        let required_scopes = vec!["read".to_string()];

        let verified = verify_root_and_issuer(&token, Some(&role), &required_scopes).unwrap();

        assert_eq!(verified.subject, p(9));
        assert_eq!(verified.issuer_pid, p(2));
        assert_eq!(verified.scopes, vec!["read".to_string()]);
    }

    #[test]
    fn verify_delegated_token_without_signatures_accepts_cached_exact_token_identity() {
        let mut token = token();
        token.proof.root_proof = RootProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1 {
            signature_cbor: Vec::new(),
            public_key_der: Vec::new(),
        });
        token.issuer_proof = IssuerProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1 {
            signature_cbor: Vec::new(),
            public_key_der: Vec::new(),
        });
        let role = role();
        let required_scopes = vec!["read".to_string()];

        let verified =
            verify_delegated_token_without_signatures(input(&token, Some(&role), &required_scopes))
                .expect("cache-hit local checks should not re-run cryptographic verification");

        assert_eq!(verified.subject, p(9));
        assert_eq!(verified.issuer_pid, p(2));
        assert_eq!(verified.scopes, vec!["read".to_string()]);
    }

    #[test]
    fn verify_delegated_token_accepts_issuer_clock_within_future_skew() {
        let now_ns = 1_000_000_000_000;
        let token = future_token(now_ns, 30_000_000_000);
        let role = role();
        let required_scopes = vec!["read".to_string()];

        let verified = verify_delegated_token_without_signatures(input_at(
            &token,
            Some(&role),
            &required_scopes,
            now_ns,
        ))
        .expect("issuer clock within skew allowance should verify");

        assert_eq!(verified.subject, p(9));
    }

    #[test]
    fn verify_delegated_token_rejects_cert_farther_in_future_than_skew() {
        let now_ns = 1_000_000_000_000;
        let token = future_token(now_ns, AUTH_TIME_SKEW_ALLOWANCE_NS + 1);
        let role = role();
        let required_scopes = vec!["read".to_string()];

        let err = verify_delegated_token_without_signatures(input_at(
            &token,
            Some(&role),
            &required_scopes,
            now_ns,
        ))
        .expect_err("cert beyond skew allowance must reject");

        assert_eq!(err, VerifyDelegatedTokenError::CertNotYetValid);
    }

    #[test]
    fn verify_delegated_token_rejects_claims_farther_in_future_than_skew() {
        let now_ns = 1_000_000_000_000;
        let mut token = token();
        token.proof.cert.issued_at_ns = now_ns;
        token.proof.cert.not_before_ns = now_ns;
        token.proof.cert.expires_at_ns = now_ns + AUTH_TIME_SKEW_ALLOWANCE_NS + 500;
        token.proof.cert.max_token_ttl_ns = 120;
        token.claims.issued_at_ns = now_ns + AUTH_TIME_SKEW_ALLOWANCE_NS + 1;
        token.claims.expires_at_ns = token.claims.issued_at_ns + 60;
        token.claims.cert_hash = cert_hash(&token.proof.cert).unwrap();
        token.issuer_proof = issuer_proof_for_claims(&token.claims);

        let role = role();
        let required_scopes = vec!["read".to_string()];
        let mut input = input_at(&token, Some(&role), &required_scopes, now_ns);
        input.ttl_limits.max_cert_ttl_ns = AUTH_TIME_SKEW_ALLOWANCE_NS + 1_000;

        let err = verify_delegated_token_without_signatures(input)
            .expect_err("claims beyond skew allowance must reject");

        assert_eq!(err, VerifyDelegatedTokenError::TokenNotYetValid);
    }

    #[test]
    fn verify_delegated_token_rejects_root_signature_failure() {
        let token = token();
        let role = role();

        assert_eq!(
            verify_delegated_token(
                input(&token, Some(&role), &[]),
                |_, _, _| Err("bad root sig".to_string()),
                verify_issuer_ok,
            ),
            Err(VerifyDelegatedTokenError::RootSignatureInvalid(
                "bad root sig".to_string(),
            ))
        );
    }

    #[test]
    fn verify_delegated_token_rejects_issuer_proof_failure() {
        let token = token();
        let role = role();

        assert_eq!(
            verify_delegated_token(
                input(&token, Some(&role), &[]),
                verify_root_ok(cert_hash(&token.proof.cert).unwrap()),
                |_, _, _| Err("bad issuer proof".to_string()),
            ),
            Err(VerifyDelegatedTokenError::IssuerProofInvalid(
                "bad issuer proof".to_string(),
            ))
        );
    }

    #[test]
    fn verify_delegated_token_rejects_cert_hash_drift() {
        let mut token = token();
        token.claims.cert_hash = [0; 32];
        let role = role();

        assert_eq!(
            verify_root_and_issuer(&token, Some(&role), &[]),
            Err(VerifyDelegatedTokenError::CertHashMismatch)
        );
    }

    #[test]
    fn verify_delegated_token_rejects_noncanonical_cert_grants() {
        let mut token = token();
        token.proof.cert.grants = vec![
            grant("project_instance", &["read"]),
            grant("project_hub", &["upload"]),
        ];
        let role = role();

        assert_eq!(
            verify_delegated_token(
                input(&token, Some(&role), &[]),
                |_, _, _| Ok(()),
                verify_issuer_ok
            ),
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
        let role = role();

        assert_eq!(
            verify_root_and_issuer(&token, Some(&role), &[]),
            Err(VerifyDelegatedTokenError::Canonical(
                CanonicalAuthError::NonCanonicalRoles
            ))
        );
    }

    #[test]
    fn verify_delegated_token_rejects_audience_subset_drift() {
        let mut token = token();
        token.claims.aud = DelegationAudience::Canister(p(20));
        let role = role();

        assert_eq!(
            verify_root_and_issuer(&token, Some(&role), &[]),
            Err(VerifyDelegatedTokenError::AudienceNotSubset)
        );
    }

    #[test]
    fn verify_delegated_token_rejects_non_matching_project_audience() {
        let mut token = token();
        token.proof.cert.aud = DelegationAudience::Project("other".to_string());
        token.claims.aud = DelegationAudience::Project("other".to_string());
        token.claims.cert_hash = cert_hash(&token.proof.cert).unwrap();
        let role = role();

        assert_eq!(
            verify_root_and_issuer(&token, Some(&role), &[]),
            Err(VerifyDelegatedTokenError::TokenAudienceRejected)
        );
    }

    #[test]
    fn verify_delegated_token_rejects_missing_local_role_for_grant_lookup() {
        let token = token();

        assert_eq!(
            verify_root_and_issuer(&token, None, &[]),
            Err(VerifyDelegatedTokenError::MissingLocalRole)
        );
    }

    #[test]
    fn verify_delegated_token_rejects_local_role_outside_token_grants() {
        let token = token();
        let role = CanisterRole::new("admin");

        assert_eq!(
            verify_root_and_issuer(&token, Some(&role), &[]),
            Err(VerifyDelegatedTokenError::TokenGrantRejected)
        );
    }

    #[test]
    fn verify_delegated_token_rejects_claim_grant_expansion() {
        let mut token = token();
        token.claims.grants = vec![grant("project_instance", &["admin"])];
        let role = role();

        assert_eq!(
            verify_root_and_issuer(&token, Some(&role), &[]),
            Err(VerifyDelegatedTokenError::GrantsNotSubset)
        );
    }

    #[test]
    fn verify_delegated_token_rejects_required_scope_outside_local_role_grant() {
        let token = token();
        let role = role();
        let required_scopes = vec!["admin".to_string()];

        assert_eq!(
            verify_root_and_issuer(&token, Some(&role), &required_scopes),
            Err(VerifyDelegatedTokenError::ScopeRejected {
                scope: "admin".to_string(),
            })
        );
    }

    #[test]
    fn verify_delegated_token_rejects_expired_token_at_boundary() {
        let token = token();
        let role = role();
        let mut input = input(&token, Some(&role), &[]);
        input.now_ns = 180;

        assert_eq!(
            verify_delegated_token(
                input,
                verify_root_ok(cert_hash(&token.proof.cert).unwrap()),
                verify_issuer_ok,
            ),
            Err(VerifyDelegatedTokenError::TokenExpired)
        );
    }
}
