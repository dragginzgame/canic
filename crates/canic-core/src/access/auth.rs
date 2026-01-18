//! Cryptographic authorization helpers.
//!
//! This module verifies tokens and claims.
//! It does not inspect topology or configuration.

use crate::{
    access::{AccessError, AccessRuleResult},
    cdk::types::Principal,
    dto::auth::{DelegatedToken, DelegatedTokenClaims},
    ops::auth::DelegatedTokenOps,
};

/// Verify a delegated token against the configured authority.
#[must_use]
pub fn verify_token(
    token: DelegatedToken,
    authority_pid: Principal,
    now_secs: u64,
) -> AccessRuleResult {
    Box::pin(async move {
        DelegatedTokenOps::verify_token(&token, authority_pid, now_secs)
            .map(|_| ())
            .map_err(|err| AccessError::Denied(err.to_string()))
    })
}

/// Require that the claims include the requested scope.
#[must_use]
pub fn require_scope(claims: DelegatedTokenClaims, required_scope: String) -> AccessRuleResult {
    Box::pin(async move {
        if claims.scopes.iter().any(|scope| scope == &required_scope) {
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
    required_audience: String,
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

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{require_audience, require_scope, verify_token};
    use crate::{
        access::AccessError,
        cdk::types::Principal,
        dto::auth::{DelegatedToken, DelegatedTokenClaims, DelegationCert, DelegationProof},
    };
    use futures::executor::block_on;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn base_claims() -> DelegatedTokenClaims {
        DelegatedTokenClaims {
            sub: p(1),
            aud: "canic:user-api".to_string(),
            scopes: vec!["user:read".to_string(), "user:write".to_string()],
            iat: 0,
            exp: 10,
            nonce: None,
        }
    }

    #[test]
    fn require_scope_allows_matching_scope() {
        let claims = base_claims();
        let result = block_on(require_scope(claims, "user:read".to_string()));

        assert!(result.is_ok());
    }

    #[test]
    fn require_scope_denies_missing_scope() {
        let claims = base_claims();
        let result = block_on(require_scope(claims, "admin:write".to_string()));

        match result {
            Err(AccessError::Denied(msg)) => {
                assert!(msg.contains("missing required scope"));
            }
            other => panic!("expected denied, got {other:?}"),
        }
    }

    #[test]
    fn require_audience_allows_matching_audience() {
        let claims = base_claims();
        let result = block_on(require_audience(claims, "canic:user-api".to_string()));

        assert!(result.is_ok());
    }

    #[test]
    fn require_audience_denies_mismatch() {
        let claims = base_claims();
        let result = block_on(require_audience(claims, "canic:admin".to_string()));

        match result {
            Err(AccessError::Denied(msg)) => {
                assert!(msg.contains("expected audience"));
            }
            other => panic!("expected denied, got {other:?}"),
        }
    }

    #[test]
    fn verify_token_denies_missing_cert_signature() {
        let cert = DelegationCert {
            v: 1,
            signer_pid: p(2),
            audiences: vec!["canic:user-api".to_string()],
            scopes: vec!["user:read".to_string()],
            issued_at: 0,
            expires_at: 10,
        };

        let proof = DelegationProof {
            cert,
            cert_sig: Vec::new(),
        };

        let claims = DelegatedTokenClaims {
            sub: p(1),
            aud: "canic:user-api".to_string(),
            scopes: vec!["user:read".to_string()],
            iat: 0,
            exp: 10,
            nonce: None,
        };

        let token = DelegatedToken {
            v: 1,
            claims,
            proof,
            token_sig: Vec::new(),
        };

        let result = block_on(verify_token(token, p(9), 1));

        match result {
            Err(AccessError::Denied(msg)) => {
                assert!(msg.contains("delegation cert signature unavailable"));
            }
            other => panic!("expected denied, got {other:?}"),
        }
    }
}
