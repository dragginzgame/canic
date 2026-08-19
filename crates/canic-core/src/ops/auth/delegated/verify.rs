//! Module: ops::auth::delegated::verify
//!
//! Responsibility: verify delegated-token proofs, claims, audience, and scopes.
//! Does not own: runtime config, positive cache storage, or endpoint authorization.
//! Boundary: pure verifier helper called by auth ops after runtime context is resolved.

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
    dto::auth::{DelegatedToken, DelegationCert, IssuerProof, RootProof},
    ids::{CanisterRole, FleetKey},
    model::auth::application_authorization::{
        ApplicationAuthorityModelError, ApplicationScope, CanonicalApplicationScopes,
        VerifiedApplicationAuthority,
    },
    ops::auth::AUTH_TIME_SKEW_ALLOWANCE_NS,
};
use thiserror::Error;

///
/// VerifyDelegatedTokenInput
///
/// Input for local delegated-token semantic and proof verification.
///

pub struct VerifyDelegatedTokenInput<'a> {
    pub token: &'a DelegatedToken,
    pub expected_presenter: Principal,
    pub local_fleet: FleetKey,
    pub local_role: Option<&'a CanisterRole>,
    pub ttl_limits: DelegatedAuthTtlLimits,
    pub required_scopes: &'a [String],
    pub now_ns: u64,
}

///
/// VerifyDelegatedTokenError
///
/// Typed failure surface for delegated-token verification.
///

#[derive(Debug, Eq, Error, PartialEq)]
pub enum VerifyDelegatedTokenError<RootProofError = String, IssuerProofError = String> {
    #[error(transparent)]
    ApplicationAuthority(#[from] ApplicationAuthorityModelError),
    #[error("delegated auth token presenter does not match the current caller")]
    PresenterCallerMismatch,
    #[error("delegated auth token presenter and subject differ")]
    PresenterSubjectMismatch,
    #[error("delegated auth cert hash mismatch")]
    CertHashMismatch,
    #[error("delegated auth issuer proof unavailable")]
    IssuerProofUnavailable,
    #[error("delegated auth root proof invalid: {0}")]
    RootProofInvalid(RootProofError),
    #[error("delegated auth issuer proof invalid: {0}")]
    IssuerProofInvalid(IssuerProofError),
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

pub fn verify_delegated_token<R, S, RootProofError, IssuerProofError>(
    input: VerifyDelegatedTokenInput<'_>,
    mut verify_root_proof: R,
    mut verify_issuer_proof: S,
) -> Result<VerifiedApplicationAuthority, VerifyDelegatedTokenError<RootProofError, IssuerProofError>>
where
    R: FnMut(&DelegationCert, &RootProof) -> Result<(), RootProofError>,
    S: FnMut([u8; 32], &IssuerProof, Principal) -> Result<(), IssuerProofError>,
{
    let material = verify_delegated_token_material(&input, true)?;

    verify_root_proof(&input.token.proof.cert, &input.token.proof.root_proof)
        .map_err(VerifyDelegatedTokenError::RootProofInvalid)?;

    verify_issuer_proof(
        material.claims_hash,
        &input.token.issuer_proof,
        input.token.proof.cert.issuer_pid,
    )
    .map_err(VerifyDelegatedTokenError::IssuerProofInvalid)?;

    Ok(material.verified)
}

pub fn verify_delegated_token_cached_proof_identity(
    input: VerifyDelegatedTokenInput<'_>,
) -> Result<VerifiedApplicationAuthority, VerifyDelegatedTokenError> {
    verify_delegated_token_material(&input, false).map(|material| material.verified)
}

struct VerifiedApplicationAuthorityMaterial {
    verified: VerifiedApplicationAuthority,
    claims_hash: [u8; 32],
}

fn verify_delegated_token_material<RootProofError, IssuerProofError>(
    input: &VerifyDelegatedTokenInput<'_>,
    require_issuer_proof_bytes: bool,
) -> Result<
    VerifiedApplicationAuthorityMaterial,
    VerifyDelegatedTokenError<RootProofError, IssuerProofError>,
> {
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

    let local_role = input
        .local_role
        .ok_or(VerifyDelegatedTokenError::MissingLocalRole)?;
    let verified = VerifiedApplicationAuthority::new(
        claims.presenter,
        claims.subject,
        claims.issuer_pid,
        input.local_fleet,
        local_role.clone(),
        local_scopes,
        claims.issued_at_ns,
        cert.not_before_ns.max(claims.issued_at_ns),
        claims.expires_at_ns,
        actual_claims_hash,
    )?;

    Ok(VerifiedApplicationAuthorityMaterial {
        verified,
        claims_hash: actual_claims_hash,
    })
}

const fn verify_cert_time<RootProofError, IssuerProofError>(
    not_before_ns: u64,
    expires_at_ns: u64,
    now_ns: u64,
) -> Result<(), VerifyDelegatedTokenError<RootProofError, IssuerProofError>> {
    if not_before_ns > now_ns.saturating_add(AUTH_TIME_SKEW_ALLOWANCE_NS) {
        return Err(VerifyDelegatedTokenError::CertNotYetValid);
    }
    if now_ns >= expires_at_ns {
        return Err(VerifyDelegatedTokenError::CertExpired);
    }
    Ok(())
}

fn verify_claims<RootProofError, IssuerProofError>(
    input: &VerifyDelegatedTokenInput<'_>,
    actual_cert_hash: [u8; 32],
) -> Result<CanonicalApplicationScopes, VerifyDelegatedTokenError<RootProofError, IssuerProofError>>
{
    let cert = &input.token.proof.cert;
    let claims = &input.token.claims;

    if claims.presenter != input.expected_presenter {
        return Err(VerifyDelegatedTokenError::PresenterCallerMismatch);
    }
    if claims.presenter != claims.subject {
        return Err(VerifyDelegatedTokenError::PresenterSubjectMismatch);
    }
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
    let scopes = local_scopes
        .into_iter()
        .map(ApplicationScope::parse)
        .collect::<Result<Vec<_>, _>>()
        .map_err(ApplicationAuthorityModelError::from)?;
    CanonicalApplicationScopes::for_verified_grant(scopes)
        .map_err(ApplicationAuthorityModelError::from)
        .map_err(Into::into)
}

fn verify_audience_and_grants<RootProofError, IssuerProofError>(
    input: &VerifyDelegatedTokenInput<'_>,
) -> Result<Vec<String>, VerifyDelegatedTokenError<RootProofError, IssuerProofError>> {
    let cert_aud = &input.token.proof.cert.aud;
    let claims_aud = &input.token.claims.aud;
    let local_role = input
        .local_role
        .ok_or(VerifyDelegatedTokenError::MissingLocalRole)?;

    if !audience_subset(claims_aud, cert_aud) {
        return Err(VerifyDelegatedTokenError::AudienceNotSubset);
    }
    let audience_ctx = AudienceAcceptanceContext {
        local_fleet: input.local_fleet,
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

fn verify_scopes<RootProofError, IssuerProofError>(
    subset: &[String],
    superset: &[String],
) -> Result<(), VerifyDelegatedTokenError<RootProofError, IssuerProofError>> {
    for scope in subset {
        if !superset.contains(scope) {
            return Err(VerifyDelegatedTokenError::ScopeRejected {
                scope: scope.clone(),
            });
        }
    }
    Ok(())
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

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
            aud: DelegationAudience::Fleet(crate::test::support::fleet_key(1)),
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
            presenter: p(9),
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
            expected_presenter: token.claims.presenter,
            local_fleet: crate::test::support::fleet_key(1),
            local_role,
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
        crate::ops::auth::test_fixtures::chain_key_root_proof(byte)
    }

    fn verify_root_ok() -> impl FnMut(&DelegationCert, &RootProof) -> Result<(), String> {
        |cert, proof| {
            if cert.root_pid != p(1) {
                return Err("root pid mismatch".to_string());
            }
            let RootProof::IcChainKeyBatchSignatureV1(_) = proof;
            Ok(())
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
    ) -> Result<VerifiedApplicationAuthority, VerifyDelegatedTokenError> {
        verify_delegated_token(
            input(token, local_role, required_scopes),
            verify_root_ok(),
            verify_issuer_ok,
        )
    }

    #[test]
    fn verify_delegated_token_accepts_self_validating_token_without_proof_lookup() {
        let token = token();
        let role = role();
        let required_scopes = vec!["read".to_string()];

        let verified = verify_root_and_issuer(&token, Some(&role), &required_scopes).unwrap();

        assert_eq!(verified.presenter(), p(9));
        assert_eq!(verified.subject(), p(9));
        assert_eq!(verified.issuer(), p(2));
        assert_eq!(verified.fleet(), crate::test::support::fleet_key(1));
        assert_eq!(verified.role(), &role);
        assert_eq!(
            verified.proof_fingerprint(),
            claims_hash(&token.claims).unwrap()
        );
        assert!(verified.scopes().contains(
            crate::model::auth::application_authorization::ApplicationScopeRef::from_static("read")
        ));
    }

    #[test]
    fn verify_delegated_token_rejects_presenter_that_differs_from_current_caller() {
        let token = token();
        let role = role();
        let mut input = input(&token, Some(&role), &[]);
        input.expected_presenter = p(8);

        assert_eq!(
            verify_delegated_token_cached_proof_identity(input),
            Err(VerifyDelegatedTokenError::PresenterCallerMismatch)
        );
    }

    #[test]
    fn verify_delegated_token_rejects_subject_that_differs_from_presenter() {
        let mut token = token();
        token.claims.subject = p(8);
        token.issuer_proof = issuer_proof_for_claims(&token.claims);
        let role = role();

        assert_eq!(
            verify_delegated_token_cached_proof_identity(input(&token, Some(&role), &[])),
            Err(VerifyDelegatedTokenError::PresenterSubjectMismatch)
        );
    }

    #[test]
    fn verify_delegated_token_cached_proof_identity_accepts_cached_exact_token_identity() {
        let mut token = token();
        token.proof.root_proof = crate::ops::auth::test_fixtures::chain_key_root_proof(0);
        token.issuer_proof = IssuerProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1 {
            signature_cbor: Vec::new(),
            public_key_der: Vec::new(),
        });
        let role = role();
        let required_scopes = vec!["read".to_string()];

        let verified = verify_delegated_token_cached_proof_identity(input(
            &token,
            Some(&role),
            &required_scopes,
        ))
        .expect("cache-hit local checks should not re-run cryptographic verification");

        assert_eq!(verified.subject(), p(9));
        assert_eq!(verified.issuer(), p(2));
        assert!(verified.scopes().contains(
            crate::model::auth::application_authorization::ApplicationScopeRef::from_static("read")
        ));
    }

    #[test]
    fn verify_delegated_token_accepts_issuer_clock_within_future_skew() {
        let now_ns = 1_000_000_000_000;
        let token = future_token(now_ns, 30_000_000_000);
        let role = role();
        let required_scopes = vec!["read".to_string()];

        let verified = verify_delegated_token_cached_proof_identity(input_at(
            &token,
            Some(&role),
            &required_scopes,
            now_ns,
        ))
        .expect("issuer clock within skew allowance should verify");

        assert_eq!(verified.subject(), p(9));
    }

    #[test]
    fn verify_delegated_token_rejects_cert_farther_in_future_than_skew() {
        let now_ns = 1_000_000_000_000;
        let token = future_token(now_ns, AUTH_TIME_SKEW_ALLOWANCE_NS + 1);
        let role = role();
        let required_scopes = vec!["read".to_string()];

        let err = verify_delegated_token_cached_proof_identity(input_at(
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

        let err = verify_delegated_token_cached_proof_identity(input)
            .expect_err("claims beyond skew allowance must reject");

        assert_eq!(err, VerifyDelegatedTokenError::TokenNotYetValid);
    }

    #[test]
    fn verify_delegated_token_rejects_root_proof_failure() {
        let token = token();
        let role = role();

        assert_eq!(
            verify_delegated_token(
                input(&token, Some(&role), &[]),
                |_, _| Err("bad root proof".to_string()),
                verify_issuer_ok,
            ),
            Err(VerifyDelegatedTokenError::RootProofInvalid(
                "bad root proof".to_string(),
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
                verify_root_ok(),
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
                |_, _| Ok::<(), String>(()),
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
        token.claims.aud = DelegationAudience::Fleet(crate::test::support::fleet_key(2));
        let role = role();

        assert_eq!(
            verify_root_and_issuer(&token, Some(&role), &[]),
            Err(VerifyDelegatedTokenError::AudienceNotSubset)
        );
    }

    #[test]
    fn verify_delegated_token_rejects_non_matching_fleet_audience() {
        let mut token = token();
        token.proof.cert.aud = DelegationAudience::Fleet(crate::test::support::fleet_key(2));
        token.claims.aud = DelegationAudience::Fleet(crate::test::support::fleet_key(2));
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
            verify_delegated_token(input, verify_root_ok(), verify_issuer_ok,),
            Err(VerifyDelegatedTokenError::TokenExpired)
        );
    }
}
