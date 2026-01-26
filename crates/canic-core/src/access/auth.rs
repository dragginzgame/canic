//! Auth access checks.
//!
//! This bucket includes:
//! - caller identity checks (controller/whitelist)
//! - topology checks (parent/child/root/same canister)
//! - registry-based role checks
//! - delegated token verification
//!
//! Security invariants for delegated tokens:
//! - Delegated tokens are only valid if their proof matches the currently stored delegation proof.
//! - Delegation rotation invalidates all previously issued delegated tokens.
//! - All temporal validation (iat/exp/now) is enforced before access is granted.

use crate::{
    access::AccessError,
    cdk::{
        api::{canister_self, is_controller as caller_is_controller, msg_arg_data},
        candid::Decode,
        types::Principal,
    },
    config::Config,
    dto::{auth::DelegatedToken, rpc::AuthenticatedRequest},
    ids::CanisterRole,
    ops::{
        auth::{DelegatedTokenOps, VerifiedDelegatedToken},
        ic::IcOps,
        runtime::env::EnvOps,
        storage::{children::CanisterChildrenOps, registry::subnet::SubnetRegistryOps},
    },
};

const MAX_INGRESS_BYTES: usize = 64 * 1024; // 64 KiB

pub type Role = CanisterRole;

/// Verify a delegated token read from the ingress payload.
///
/// Contract:
/// - The delegated token MUST be the first candid argument, or embedded in an
///   `AuthenticatedRequest` as the single argument.
/// - Decoding failures result in access denial.
/// - The caller argument is accepted for composability and is not inspected.
pub async fn authenticated(_caller: Principal) -> Result<(), AccessError> {
    let _ = delegated_token_verified(_caller).await?;
    Ok(())
}

pub(crate) async fn delegated_token_verified(
    _caller: Principal,
) -> Result<VerifiedDelegatedToken, AccessError> {
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
) -> Result<VerifiedDelegatedToken, AccessError> {
    let verified = DelegatedTokenOps::verify_token(&token, authority_pid, now_secs)
        .map_err(|err| AccessError::Denied(err.to_string()))?;
    Ok(verified)
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
pub async fn is_root(caller: Principal) -> Result<(), AccessError> {
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

/// Require that the caller is registered with the expected canister role.
#[allow(clippy::unused_async)]
pub async fn has_role(caller: Principal, role: Role) -> Result<(), AccessError> {
    let record = SubnetRegistryOps::get(caller).ok_or_else(|| {
        AccessError::Denied(format!(
            "caller '{caller}' is not registered on the subnet registry"
        ))
    })?;

    if record.role == role {
        Ok(())
    } else {
        Err(AccessError::Denied(format!(
            "caller '{caller}' does not have role '{role}'"
        )))
    }
}

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
    if let Ok(token) = Decode!(&bytes, DelegatedToken) {
        return Ok(token);
    }

    let envelope = Decode!(&bytes, AuthenticatedRequest).map_err(|err| {
        AccessError::Denied(format!(
            "failed to decode delegated token as first argument: {err}"
        ))
    })?;

    Ok(envelope.delegated_token)
}

fn dependency_unavailable(detail: &str) -> AccessError {
    AccessError::Denied(format!("access dependency unavailable: {detail}"))
}
