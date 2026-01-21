//! Auth bucket access checks.
//!
//! This module is a mixed bucket that includes:
//! - caller identity checks (controller/whitelist)
//! - topology checks (parent/child/root/same canister)
//! - registry checks
//! - delegated token verification
//!
//! Security invariants for delegated tokens:
//! - Delegated tokens are only valid if their proof matches the currently stored delegation proof.
//! - Delegation rotation invalidates all previously issued delegated tokens.
//! - All temporal validation (iat/exp/now) is enforced before access is granted.

use crate::{
    access::{AccessError, metrics::DelegationMetrics},
    cdk::{
        api::{canister_self, is_controller as caller_is_controller, msg_arg_data},
        candid::Decode,
        types::Principal,
    },
    config::Config,
    dto::auth::DelegatedToken,
    ops::{
        auth::DelegatedTokenOps,
        ic::IcOps,
        runtime::env::EnvOps,
        storage::{children::CanisterChildrenOps, registry::subnet::SubnetRegistryOps},
    },
};

const MAX_INGRESS_BYTES: usize = 64 * 1024; // 64 KiB

/// Verify a delegated token read from the ingress payload.
///
/// Contract:
/// - The delegated token MUST be the first candid argument.
/// - Decoding failures result in access denial.
pub async fn verify_delegated_token() -> Result<(), AccessError> {
    let token = delegated_token_from_args()?;

    let authority_pid =
        EnvOps::root_pid().map_err(|_| dependency_unavailable("root pid unavailable"))?;

    let now_secs = IcOps::now_secs();

    verify_token(token, authority_pid, now_secs).await
}

/// Verify a delegated token against the configured authority.
#[allow(clippy::unused_async)]
async fn verify_token(
    token: DelegatedToken,
    authority_pid: Principal,
    now_secs: u64,
) -> Result<(), AccessError> {
    let verified = DelegatedTokenOps::verify_token(&token, authority_pid, now_secs)
        .map_err(|err| AccessError::Denied(err.to_string()))?;

    DelegationMetrics::record_authority(verified.cert.signer_pid);

    Ok(())
}

// -----------------------------------------------------------------------------
// Caller & topology predicates
// -----------------------------------------------------------------------------

/// Require that the caller controls the current canister.
/// Allows controller-only maintenance calls.
#[allow(clippy::unused_async)]
pub async fn is_controller(caller: Principal) -> Result<(), AccessError> {
    if caller_is_controller(&caller) {
        Ok(())
    } else {
        Err(AccessError::Denied(format!(
            "caller '{caller}' is not a controller of this canister"
        )))
    }
}

/// Require that the caller appears in the active whitelist (IC deployments).
/// No-op on local builds; enforces whitelist on IC.
#[allow(clippy::unused_async)]
pub async fn is_whitelisted(caller: Principal) -> Result<(), AccessError> {
    let cfg = Config::try_get().ok_or_else(|| dependency_unavailable("config not initialized"))?;

    if !cfg.is_whitelisted(&caller) {
        return Err(AccessError::Denied(format!(
            "caller '{caller}' is not on the whitelist"
        )));
    }

    Ok(())
}

/// Require that the caller is a direct child of the current canister.
#[allow(clippy::unused_async)]
pub async fn is_child(caller: Principal) -> Result<(), AccessError> {
    if CanisterChildrenOps::contains_pid(&caller) {
        Ok(())
    } else {
        Err(AccessError::Denied(format!(
            "caller '{caller}' is not a child of this canister"
        )))
    }
}

/// Require that the caller is the configured parent canister.
#[allow(clippy::unused_async)]
pub async fn is_parent(caller: Principal) -> Result<(), AccessError> {
    let snapshot = EnvOps::snapshot();
    let parent_pid = snapshot
        .parent_pid
        .ok_or_else(|| dependency_unavailable("parent pid unavailable"))?;

    if parent_pid == caller {
        Ok(())
    } else {
        Err(AccessError::Denied(format!(
            "caller '{caller}' is not the parent of this canister"
        )))
    }
}

/// Require that the caller equals the configured root canister.
#[allow(clippy::unused_async)]
pub async fn caller_is_root(caller: Principal) -> Result<(), AccessError> {
    let root_pid =
        EnvOps::root_pid().map_err(|_| dependency_unavailable("root pid unavailable"))?;

    if caller == root_pid {
        Ok(())
    } else {
        Err(AccessError::Denied(format!(
            "caller '{caller}' is not root"
        )))
    }
}

/// Require that the caller is the currently executing canister.
#[allow(clippy::unused_async)]
pub async fn is_same_canister(caller: Principal) -> Result<(), AccessError> {
    if caller == canister_self() {
        Ok(())
    } else {
        Err(AccessError::Denied(format!(
            "caller '{caller}' is not the current canister"
        )))
    }
}

// -----------------------------------------------------------------------------
// Registry predicates
// -----------------------------------------------------------------------------

/// Ensure the caller matches the app directory entry recorded for `role`.
/// Require that the caller is registered as a canister on this subnet.
#[allow(clippy::unused_async)]
pub async fn is_registered_to_subnet(caller: Principal) -> Result<(), AccessError> {
    if SubnetRegistryOps::is_registered(caller) {
        Ok(())
    } else {
        Err(AccessError::Denied(format!(
            "caller '{caller}' is not registered on the subnet registry"
        )))
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

fn dependency_unavailable(detail: &str) -> AccessError {
    AccessError::Denied(format!("access dependency unavailable: {detail}"))
}
