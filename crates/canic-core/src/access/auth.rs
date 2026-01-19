//! Cryptographic authorization helpers.
//!
//! Security invariants:
//! - Delegated tokens are only valid if their proof matches the currently stored delegation proof.
//! - Delegation rotation invalidates all previously issued delegated tokens.
//! - All temporal validation (iat/exp/now) is enforced before access is granted.
//! - This module validates cryptographic claims only; it does not authorize principals directly.

use crate::{
    access::{AccessError, AccessRuleError, AccessRuleResult, metrics::DelegationMetrics},
    cdk::{api::msg_arg_data, candid::Decode, types::Principal},
    config::Config,
    dto::auth::{DelegatedToken, DelegatedTokenClaims, DelegationProof},
    ops::{
        auth::DelegatedTokenOps, ic::IcOps, runtime::env::EnvOps, storage::auth::DelegationStateOps,
    },
};

const MAX_INGRESS_BYTES: usize = 64 * 1024; // 64 KiB

/// Verify a delegated token read from the ingress payload.
///
/// Contract:
/// - The delegated token MUST be the first candid argument.
/// - Decoding failures result in access denial.
#[must_use]
pub fn verify_delegated_token() -> AccessRuleResult {
    Box::pin(async move {
        let cfg = Config::try_get().ok_or_else(|| {
            AccessRuleError::DependencyUnavailable("config not initialized".to_string())
        })?;

        if !cfg.delegation.enabled {
            return Err(AccessError::Denied(
                "delegated token auth disabled".to_string(),
            ));
        }

        let token = delegated_token_from_args()?;

        // Enforce bounded TTL (relative)
        if let Some(max_ttl_secs) = cfg.delegation.max_ttl_secs {
            if token.claims.exp < token.claims.iat {
                return Err(AccessError::Denied(
                    "delegated token expiry precedes iat".to_string(),
                ));
            }

            let ttl_secs = token.claims.exp - token.claims.iat;
            if ttl_secs > max_ttl_secs {
                return Err(AccessError::Denied(format!(
                    "delegated token ttl exceeds max {max_ttl_secs}s"
                )));
            }
        }

        let authority_pid = EnvOps::root_pid().map_err(|_| {
            AccessRuleError::DependencyUnavailable("root pid unavailable".to_string())
        })?;

        let now_secs = IcOps::now_secs();

        verify_token(token, authority_pid, now_secs).await
    })
}

/// Verify a delegated token against the configured authority.
#[must_use]
pub fn verify_token(
    token: DelegatedToken,
    authority_pid: Principal,
    now_secs: u64,
) -> AccessRuleResult {
    Box::pin(async move {
        // Enforce absolute temporal validity
        if token.claims.iat > now_secs {
            return Err(AccessError::Denied(
                "delegated token not yet valid".to_string(),
            ));
        }

        if token.claims.exp < now_secs {
            return Err(AccessError::Denied("delegated token expired".to_string()));
        }

        let stored = DelegationStateOps::proof_dto().ok_or_else(|| {
            AccessRuleError::DependencyUnavailable("delegation proof unavailable".to_string())
        })?;

        if !token.proof.semantically_equals(&stored) {
            return Err(AccessError::Denied(
                "delegation proof does not match current proof".to_string(),
            ));
        }

        let verified = DelegatedTokenOps::verify_token(&token, authority_pid, now_secs)
            .map_err(|err| AccessError::Denied(err.to_string()))?;

        DelegationMetrics::record_authority(verified.cert.signer_pid);

        Ok(())
    })
}

/// Require that the claims include the requested scope.
#[must_use]
pub fn require_scope(
    claims: DelegatedTokenClaims,
    required_scope: &'static str,
) -> AccessRuleResult {
    Box::pin(async move {
        if claims.scopes.iter().any(|scope| scope == required_scope) {
            Ok(())
        } else {
            Err(AccessError::Denied(format!(
                "missing required scope '{required_scope}'"
            )))
        }
    })
}

/// Require that the claims target the expected audience.
#[must_use]
pub fn require_audience(
    claims: DelegatedTokenClaims,
    required_audience: &'static str,
) -> AccessRuleResult {
    Box::pin(async move {
        if claims.aud == required_audience {
            Ok(())
        } else {
            Err(AccessError::Denied(format!(
                "expected audience '{required_audience}'"
            )))
        }
    })
}

impl DelegationProof {
    /// Semantic equality for delegation proofs.
    ///
    /// This defines the security boundary for delegation rotation.
    #[must_use]
    pub fn semantically_equals(&self, other: &Self) -> bool {
        let a = &self.cert;
        let b = &other.cert;

        a.v == b.v
            && a.signer_pid == b.signer_pid
            && a.audiences == b.audiences
            && a.scopes == b.scopes
            && a.issued_at == b.issued_at
            && a.expires_at == b.expires_at
            && self.cert_sig == other.cert_sig
    }
}

fn delegated_token_from_args() -> Result<DelegatedToken, AccessError> {
    let bytes = msg_arg_data();

    if bytes.len() > MAX_INGRESS_BYTES {
        return Err(AccessError::Denied(
            "delegated token payload exceeds size limit".to_string(),
        ));
    }

    // Decode the FIRST candid argument as DelegatedToken.
    Decode!(&bytes, DelegatedToken).map_err(|err| {
        AccessError::Denied(format!(
            "failed to decode delegated token as first argument: {err}"
        ))
    })
}
