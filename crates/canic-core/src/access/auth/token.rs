//! Module: access::auth::token
//!
//! Responsibility: decode and verify delegated tokens for endpoint access.
//! Does not own: replay receipts, endpoint payload decoding, or public response mapping.
//! Boundary: `access::auth` calls this after resolving the authenticated subject.

use super::dependency_unavailable;
use crate::{
    InternalError,
    access::AccessError,
    cdk::{
        api::msg_arg_data,
        candid::de::{DecoderConfig, IDLDeserialize},
        types::Principal,
    },
    dto::{auth::DelegatedToken, error::ErrorCode},
    ops::{
        auth::{AuthOps, VerifyDelegatedTokenRuntimeInput},
        config::ConfigOps,
        ic::IcOps,
    },
};

const DELEGATED_TOKEN_DECODING_QUOTA: usize = 256 * 1024;
const DELEGATED_TOKEN_MAX_TYPE_LEN: usize = 16 * 1024;
const DEFAULT_DELEGATED_AUTH_MAX_TTL_SECS: u64 = 24 * 60 * 60;
const NS_PER_SEC: u64 = 1_000_000_000;

pub(super) fn delegated_token_verified(
    authenticated_subject: Principal,
    required_scope: Option<&str>,
) -> Result<Principal, AccessError> {
    let token = delegated_token_from_args()?;

    let now_ns = IcOps::now_nanos();

    verify_token(token, authenticated_subject, now_ns, required_scope)
}

// Verify a delegated token; endpoint-local binding and scope checks still run
// after any positive cryptographic verification cache hit.
fn verify_token(
    token: DelegatedToken,
    caller: Principal,
    now_ns: u64,
    required_scope: Option<&str>,
) -> Result<Principal, AccessError> {
    let max_ttl_ns = delegated_token_max_ttl_ns()?;
    let required_scopes = required_scope
        .map(|scope| vec![scope.to_string()])
        .unwrap_or_default();
    let verified = AuthOps::verify_token(VerifyDelegatedTokenRuntimeInput {
        token: &token,
        caller,
        max_cert_ttl_ns: max_ttl_ns,
        max_token_ttl_ns: max_ttl_ns,
        required_scopes: &required_scopes,
        now_ns,
    })
    .map_err(access_error_from_verification)?;

    enforce_subject_binding(verified.subject, caller)?;
    enforce_required_scope(required_scope, &verified.scopes)?;

    Ok(verified.issuer_pid)
}

fn access_error_from_verification(err: InternalError) -> AccessError {
    match err.public_error().map(|error| error.code) {
        Some(ErrorCode::AuthProofExpired) => AccessError::DelegatedAuthCertExpired,
        Some(ErrorCode::AuthTokenExpired) => AccessError::DelegatedAuthTokenExpired,
        _ => AccessError::Denied(err.to_string()),
    }
}

pub(super) fn enforce_subject_binding(
    sub: Principal,
    caller: Principal,
) -> Result<(), AccessError> {
    if sub == caller {
        Ok(())
    } else {
        Err(AccessError::Denied(format!(
            "delegated token subject '{sub}' does not match caller '{caller}'"
        )))
    }
}

pub(super) fn enforce_required_scope(
    required_scope: Option<&str>,
    token_scopes: &[String],
) -> Result<(), AccessError> {
    let Some(required_scope) = required_scope else {
        return Ok(());
    };

    if token_scopes.iter().any(|scope| scope == required_scope) {
        Ok(())
    } else {
        Err(AccessError::Denied(format!(
            "delegated token missing required scope '{required_scope}'"
        )))
    }
}

fn delegated_token_from_args() -> Result<DelegatedToken, AccessError> {
    let bytes = msg_arg_data();
    delegated_token_from_ingress_bytes(&bytes)
}

// Decode and size-check only the delegated token, not later endpoint payloads.
fn delegated_token_from_ingress_bytes(bytes: &[u8]) -> Result<DelegatedToken, AccessError> {
    delegated_token_from_bytes(bytes).map_err(|err| {
        AccessError::Denied(format!(
            "failed to decode DelegatedToken as first argument: {err}"
        ))
    })
}

// Decode the first ingress argument as a delegated token.
fn delegated_token_from_bytes(bytes: &[u8]) -> Result<DelegatedToken, String> {
    let mut config = DecoderConfig::new();
    config
        .set_decoding_quota(DELEGATED_TOKEN_DECODING_QUOTA)
        .set_max_type_len(DELEGATED_TOKEN_MAX_TYPE_LEN)
        .set_full_error_message(false);
    let mut decoder = IDLDeserialize::new_with_config(bytes, &config)
        .map_err(|err| format!("failed to decode ingress arguments: {err}"))?;
    decoder
        .get_value::<DelegatedToken>()
        .map_err(|err| err.to_string())
}

// Resolve the verifier-side TTL policy from delegated-token config.
fn delegated_token_max_ttl_ns() -> Result<u64, AccessError> {
    let cfg = ConfigOps::delegated_tokens_config()
        .map_err(|_| dependency_unavailable("delegated token config unavailable"))?;
    if !cfg.enabled {
        return Err(AccessError::Denied(
            "delegated token auth disabled; set auth.delegated_tokens.enabled=true in canic.toml"
                .to_string(),
        ));
    }

    let max_ttl_secs = cfg
        .max_ttl_secs
        .unwrap_or(DEFAULT_DELEGATED_AUTH_MAX_TTL_SECS);
    max_ttl_secs.checked_mul(NS_PER_SEC).ok_or_else(|| {
        AccessError::Denied("auth.delegated_tokens.max_ttl_secs overflows nanoseconds".to_string())
    })
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{access_error_from_verification, delegated_token_from_ingress_bytes};
    use crate::{
        InternalError, InternalErrorOrigin,
        access::AccessError,
        cdk::{
            candid::{Principal, encode_args},
            types,
        },
        dto::auth::{
            DelegatedRoleGrant, DelegatedToken, DelegatedTokenClaims, DelegationAudience,
            DelegationCert, DelegationProof, IcCanisterSignatureProofV1, IssuerProof,
            IssuerProofAlgorithm, IssuerProofBinding,
        },
    };

    #[test]
    fn verification_expiry_codes_map_to_typed_access_denials() {
        let token = access_error_from_verification(InternalError::auth_token_expired("expired"));
        assert!(matches!(token, AccessError::DelegatedAuthTokenExpired));

        let cert = access_error_from_verification(InternalError::auth_proof_expired("expired"));
        assert!(matches!(cert, AccessError::DelegatedAuthCertExpired));

        let other = access_error_from_verification(InternalError::ops(
            InternalErrorOrigin::Ops,
            "verification failed",
        ));
        assert!(matches!(other, AccessError::Denied(_)));
    }

    // Decode auth calls with large non-token arguments after the token.
    #[test]
    fn delegated_token_decode_allows_large_trailing_endpoint_payload() {
        let token = token_with_scopes(vec!["upload:image".to_string()]);
        let chunk = vec![7_u8; 128 * 1024];
        let bytes = encode_args((token.clone(), chunk)).expect("encode auth call");

        let decoded =
            delegated_token_from_ingress_bytes(&bytes).expect("large trailing payload must pass");

        assert_eq!(decoded, token);
    }

    // Reject genuinely oversized delegated tokens after decoding the first arg.
    #[test]
    fn delegated_token_decode_rejects_oversized_token() {
        let token = token_with_scopes(vec!["x".repeat(300 * 1024)]);
        let bytes = encode_args((token, Vec::<u8>::new())).expect("encode auth call");

        let err =
            delegated_token_from_ingress_bytes(&bytes).expect_err("oversized token must fail");

        assert!(matches!(err, crate::access::AccessError::Denied(_)));
    }

    #[test]
    fn delegated_auth_guard_has_no_verifier_local_use_store() {
        let source = include_str!("token.rs");
        let storage_fn = ["consume_delegated", "_token_use"].concat();
        let access_fn = ["consume_update", "_token_once"].concat();

        assert!(!source.contains(&storage_fn));
        assert!(!source.contains(&access_fn));
    }

    #[test]
    fn delegated_auth_guard_preserves_verify_bind_scope_order() {
        let source = include_str!("token.rs");
        let start = source
            .find("fn verify_token(")
            .expect("verify_token exists");
        let end = source[start..]
            .find("pub(super) fn enforce_subject_binding")
            .map_or(source.len(), |offset| start + offset);
        let body = &source[start..end];

        let verify = body
            .find("AuthOps::verify_token")
            .expect("verifier call exists");
        let bind = body
            .find("enforce_subject_binding")
            .expect("subject binding exists");
        let scope = body
            .find("enforce_required_scope")
            .expect("scope check exists");

        assert!(verify < bind);
        assert!(bind < scope);
    }

    #[test]
    fn delegated_auth_guard_uses_nanosecond_now_for_verification() {
        let source = include_str!("token.rs");
        let start = source
            .find("pub(super) fn delegated_token_verified")
            .expect("delegated_token_verified exists");
        let end = source[start..]
            .find("// Verify a delegated token")
            .map_or(source.len(), |offset| start + offset);
        let body = &source[start..end];

        assert!(body.contains("IcOps::now_nanos()"));
        assert!(!body.contains("IcOps::now_secs()"));
    }

    // Build one structurally complete delegated token for access decode tests.
    fn token_with_scopes(scopes: Vec<String>) -> DelegatedToken {
        let issuer_proof_alg = IssuerProofAlgorithm::IcCanisterSignatureV1;
        let issuer_proof_binding = IssuerProofBinding::IcCanisterSignatureV1 {
            seed_hash: [10; 32],
        };

        DelegatedToken {
            claims: DelegatedTokenClaims {
                subject: p(1),
                issuer_pid: p(2),
                cert_hash: [3; 32],
                issued_at_ns: 10,
                expires_at_ns: 20,
                aud: DelegationAudience::Project("test".to_string()),
                grants: vec![grant("project_instance", &scopes)],
                nonce: [5; 16],
                ext: None,
            },
            proof: DelegationProof {
                cert: DelegationCert {
                    root_pid: p(6),
                    issuer_pid: p(2),
                    issuer_proof_alg,
                    issuer_proof_binding_hash: [11; 32],
                    issuer_proof_binding,
                    issued_at_ns: 10,
                    not_before_ns: 10,
                    expires_at_ns: 20,
                    max_token_ttl_ns: 10,
                    aud: DelegationAudience::Project("test".to_string()),
                    grants: vec![grant("project_instance", &scopes)],
                },
                root_proof: crate::ops::auth::test_fixtures::chain_key_root_proof(12),
            },
            issuer_proof: IssuerProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1 {
                signature_cbor: vec![13; 64],
                public_key_der: vec![14; 32],
            }),
        }
    }

    fn grant(role: &str, scopes: &[String]) -> DelegatedRoleGrant {
        DelegatedRoleGrant {
            target: crate::ids::CanisterRole::owned(role.to_string()),
            scopes: scopes.to_vec(),
        }
    }

    // Produce deterministic non-management principals for token fixtures.
    fn p(id: u8) -> Principal {
        types::Principal::from_slice(&[id; 29])
    }
}
