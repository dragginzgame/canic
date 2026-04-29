use super::{VerifiedAccessToken, dependency_unavailable};
use crate::{
    access::AccessError,
    cdk::{api::msg_arg_data, candid::de::IDLDeserialize, types::Principal},
    dto::auth::DelegatedTokenV2,
    ops::{
        auth::{DelegatedTokenOps, VerifyDelegatedTokenV2RuntimeInput},
        config::ConfigOps,
        ic::IcOps,
    },
};

const MAX_INGRESS_BYTES: usize = 64 * 1024; // 64 KiB
const DEFAULT_DELEGATED_AUTH_V2_MAX_TTL_SECS: u64 = 24 * 60 * 60;

pub(super) async fn delegated_token_verified(
    authenticated_subject: Principal,
    required_scope: Option<&str>,
) -> Result<VerifiedAccessToken, AccessError> {
    let token = delegated_token_from_args()?;

    let now_secs = IcOps::now_secs();

    verify_token_v2(token, authenticated_subject, now_secs, required_scope).await
}

// Verify a delegated token without local proof-cache lookup.
async fn verify_token_v2(
    token: DelegatedTokenV2,
    caller: Principal,
    now_secs: u64,
    required_scope: Option<&str>,
) -> Result<VerifiedAccessToken, AccessError> {
    DelegatedTokenOps::ensure_v2_root_public_key_cached(&token)
        .await
        .map_err(|err| AccessError::Denied(err.to_string()))?;

    let max_ttl_secs = delegated_auth_v2_max_ttl_secs()?;
    let required_scopes = required_scope
        .map(|scope| vec![scope.to_string()])
        .unwrap_or_default();
    let verified = DelegatedTokenOps::verify_token_v2(VerifyDelegatedTokenV2RuntimeInput {
        token: &token,
        max_cert_ttl_secs: max_ttl_secs,
        max_token_ttl_secs: max_ttl_secs,
        required_scopes: &required_scopes,
        now_secs,
    })
    .map_err(|err| AccessError::Denied(err.to_string()))?;

    enforce_subject_binding(verified.subject, caller)?;
    enforce_required_scope(required_scope, &verified.scopes)?;

    Ok(VerifiedAccessToken {
        issuer_shard_pid: verified.issuer_shard_pid,
    })
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

fn delegated_token_from_args() -> Result<DelegatedTokenV2, AccessError> {
    let bytes = msg_arg_data();

    if bytes.len() > MAX_INGRESS_BYTES {
        return Err(AccessError::Denied(
            "delegated token payload exceeds size limit".to_string(),
        ));
    }

    delegated_token_v2_from_bytes(&bytes).map_err(|err| {
        AccessError::Denied(format!(
            "failed to decode DelegatedTokenV2 as first argument: {err}"
        ))
    })
}

// Decode the first ingress argument as a delegated token.
fn delegated_token_v2_from_bytes(bytes: &[u8]) -> Result<DelegatedTokenV2, String> {
    let mut decoder = IDLDeserialize::new(bytes)
        .map_err(|err| format!("failed to decode ingress arguments: {err}"))?;
    decoder
        .get_value::<DelegatedTokenV2>()
        .map_err(|err| err.to_string())
}

// Resolve the verifier-side V2 TTL policy from delegated-token config.
fn delegated_auth_v2_max_ttl_secs() -> Result<u64, AccessError> {
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
        .unwrap_or(DEFAULT_DELEGATED_AUTH_V2_MAX_TTL_SECS))
}
