use super::{VerifiedAccessToken, dependency_unavailable};
use crate::{
    access::AccessError,
    cdk::{
        api::msg_arg_data,
        candid::de::{DecoderConfig, IDLDeserialize},
        types::Principal,
    },
    dto::auth::DelegatedToken,
    ids::EndpointCallKind,
    ops::{
        auth::{AuthOps, VerifyDelegatedTokenRuntimeInput},
        config::ConfigOps,
        ic::IcOps,
    },
};

const DELEGATED_TOKEN_DECODING_QUOTA: usize = 256 * 1024;
const DELEGATED_TOKEN_MAX_TYPE_LEN: usize = 16 * 1024;
const DEFAULT_DELEGATED_AUTH_MAX_TTL_SECS: u64 = 24 * 60 * 60;

pub(super) fn delegated_token_verified(
    authenticated_subject: Principal,
    required_scope: Option<&str>,
    call_kind: EndpointCallKind,
) -> Result<VerifiedAccessToken, AccessError> {
    let token = delegated_token_from_args()?;

    let now_secs = IcOps::now_secs();

    verify_token(
        token,
        authenticated_subject,
        now_secs,
        required_scope,
        call_kind,
    )
}

// Verify a delegated token without local proof-cache lookup.
fn verify_token(
    token: DelegatedToken,
    caller: Principal,
    now_secs: u64,
    required_scope: Option<&str>,
    call_kind: EndpointCallKind,
) -> Result<VerifiedAccessToken, AccessError> {
    let max_ttl_secs = delegated_token_max_ttl_secs()?;
    let required_scopes = required_scope
        .map(|scope| vec![scope.to_string()])
        .unwrap_or_default();
    let verified = AuthOps::verify_token(VerifyDelegatedTokenRuntimeInput {
        token: &token,
        max_cert_ttl_secs: max_ttl_secs,
        max_token_ttl_secs: max_ttl_secs,
        required_scopes: &required_scopes,
        now_secs,
    })
    .map_err(|err| AccessError::Denied(err.to_string()))?;

    enforce_subject_binding(verified.subject, caller)?;
    enforce_required_scope(required_scope, &verified.scopes)?;
    consume_update_token_once(&token, now_secs, call_kind)?;

    Ok(VerifiedAccessToken {
        issuer_shard_pid: verified.issuer_shard_pid,
    })
}

fn consume_update_token_once(
    token: &DelegatedToken,
    now_secs: u64,
    call_kind: EndpointCallKind,
) -> Result<(), AccessError> {
    if !matches!(call_kind, EndpointCallKind::Update) {
        return Ok(());
    }

    AuthOps::consume_delegated_token_use(token, now_secs)
        .map_err(|err| AccessError::Denied(err.to_string()))
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
fn delegated_token_max_ttl_secs() -> Result<u64, AccessError> {
    let cfg = ConfigOps::delegated_tokens_config()
        .map_err(|_| dependency_unavailable("delegated token config unavailable"))?;
    if !cfg.enabled {
        return Err(AccessError::Denied(
            "delegated token auth disabled; set auth.delegated_tokens.enabled=true in canic.toml"
                .to_string(),
        ));
    }

    Ok(cfg
        .max_ttl_secs
        .unwrap_or(DEFAULT_DELEGATED_AUTH_MAX_TTL_SECS))
}

#[cfg(test)]
mod tests {
    use super::{consume_update_token_once, delegated_token_from_ingress_bytes};
    use crate::{
        cdk::{
            candid::{Principal, encode_args},
            types,
        },
        dto::auth::{
            DelegatedToken, DelegatedTokenClaims, DelegationAudience, DelegationCert,
            DelegationProof, ShardKeyBinding, SignatureAlgorithm,
        },
        ids::EndpointCallKind,
    };

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

        assert!(err.to_string().contains("failed to decode DelegatedToken"));
    }

    // Reject a second update use of the same delegated token nonce while active.
    #[test]
    fn update_token_consume_rejects_active_replay() {
        let mut token = token_with_scopes(vec!["transfer".to_string()]);
        token.claims.nonce = [44; 16];

        consume_update_token_once(&token, 10, EndpointCallKind::Update)
            .expect("first update token use should be consumed");
        let err = consume_update_token_once(&token, 11, EndpointCallKind::Update)
            .expect_err("second update token use should reject");

        assert!(err.to_string().contains("delegated token replay rejected"));
    }

    // Query calls do not get durable replay protection, so they must not consume tokens.
    #[test]
    fn query_token_consume_is_stateless() {
        let mut token = token_with_scopes(vec!["read".to_string()]);
        token.claims.nonce = [45; 16];

        consume_update_token_once(&token, 10, EndpointCallKind::Query)
            .expect("query token use should not consume");
        consume_update_token_once(&token, 11, EndpointCallKind::Query)
            .expect("query token use should remain stateless");
    }

    // Build one structurally complete delegated token for access decode tests.
    fn token_with_scopes(scopes: Vec<String>) -> DelegatedToken {
        DelegatedToken {
            claims: DelegatedTokenClaims {
                version: 1,
                subject: p(1),
                issuer_shard_pid: p(2),
                cert_hash: [3; 32],
                issued_at: 10,
                expires_at: 20,
                aud: DelegationAudience::Principals(vec![p(4)]),
                scopes: scopes.clone(),
                nonce: [5; 16],
            },
            proof: DelegationProof {
                cert: DelegationCert {
                    version: 1,
                    root_pid: p(6),
                    root_key_id: "root-key".to_string(),
                    root_key_hash: [7; 32],
                    alg: SignatureAlgorithm::EcdsaP256Sha256,
                    shard_pid: p(2),
                    shard_key_id: "shard-key".to_string(),
                    shard_public_key_sec1: vec![8; 33],
                    shard_key_hash: [9; 32],
                    shard_key_binding: ShardKeyBinding::IcThresholdEcdsa {
                        key_name_hash: [10; 32],
                        derivation_path_hash: [11; 32],
                    },
                    issued_at: 10,
                    expires_at: 20,
                    max_token_ttl_secs: 10,
                    scopes,
                    aud: DelegationAudience::Principals(vec![p(4)]),
                    verifier_role_hash: None,
                },
                root_sig: vec![12; 64],
            },
            shard_sig: vec![13; 64],
        }
    }

    // Produce deterministic non-management principals for token fixtures.
    fn p(id: u8) -> Principal {
        types::Principal::from_slice(&[id; 29])
    }
}
