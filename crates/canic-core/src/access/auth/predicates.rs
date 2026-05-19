use super::{
    caller_not_registered_denial, dependency_unavailable, non_root_subnet_registry_predicate_denial,
};
use crate::{
    access::AccessError,
    cdk::{
        api::{canister_self, is_controller as caller_is_controller},
        types::Principal,
    },
    ids::CanisterRole,
    ops::{
        config::ConfigOps,
        runtime::env::EnvOps,
        storage::{
            children::CanisterChildrenOps, index::app::AppIndexOps,
            registry::subnet::SubnetRegistryOps,
        },
    },
};

/// Require that the caller controls the current canister.
/// Allows controller-only maintenance calls.
#[expect(clippy::unused_async)]
pub(super) async fn is_controller(caller: Principal) -> Result<(), AccessError> {
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
#[expect(clippy::unused_async)]
pub(super) async fn is_whitelisted(caller: Principal) -> Result<(), AccessError> {
    let whitelisted = ConfigOps::is_whitelisted(&caller)
        .map_err(|_| dependency_unavailable("config not initialized"))?;
    if !whitelisted {
        return Err(AccessError::Denied(format!(
            "caller '{caller}' is not on the whitelist"
        )));
    }

    Ok(())
}

/// Require that the caller is a direct child of the current canister.
#[expect(clippy::unused_async)]
pub(super) async fn is_child(caller: Principal) -> Result<(), AccessError> {
    if CanisterChildrenOps::contains_pid(&caller) {
        Ok(())
    } else {
        Err(AccessError::Denied(format!(
            "caller '{caller}' is not a child of this canister"
        )))
    }
}

/// Require that the caller is the configured parent canister.
#[expect(clippy::unused_async)]
pub(super) async fn is_parent(caller: Principal) -> Result<(), AccessError> {
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
#[expect(clippy::unused_async)]
pub(super) async fn is_root(caller: Principal) -> Result<(), AccessError> {
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
#[expect(clippy::unused_async)]
pub(super) async fn is_same_canister(caller: Principal) -> Result<(), AccessError> {
    if caller == canister_self() {
        Ok(())
    } else {
        Err(AccessError::Denied(format!(
            "caller '{caller}' is not the current canister"
        )))
    }
}

/// Require that the caller is the canonical app canister for the expected role.
#[expect(clippy::unused_async)]
pub(super) async fn has_app_role(caller: Principal, role: CanisterRole) -> Result<(), AccessError> {
    let expected = AppIndexOps::get(&role)
        .ok_or_else(|| dependency_unavailable(&format!("app index role '{role}' unavailable")))?;

    if caller == expected {
        Ok(())
    } else {
        Err(AccessError::Denied(format!(
            "authentication error: caller '{caller}' is not app canister '{role}'"
        )))
    }
}

/// Require that the caller is registered as a canister on this subnet.
#[expect(clippy::unused_async)]
pub(super) async fn is_registered_to_subnet(caller: Principal) -> Result<(), AccessError> {
    if !EnvOps::is_root() {
        return Err(non_root_subnet_registry_predicate_denial());
    }

    if SubnetRegistryOps::is_registered(caller) {
        Ok(())
    } else {
        Err(caller_not_registered_denial(caller))
    }
}
