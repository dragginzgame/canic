//! Module: access::auth::predicates
//!
//! Responsibility: enforce caller and topology auth predicates.
//! Does not own: delegated token verification, app mode, or environment predicates.
//! Boundary: `access::auth` exposes these checks to access expressions.

use super::dependency_unavailable;
use crate::{
    access::AccessError,
    cdk::types::Principal,
    ops::{runtime::env::EnvOps, storage::children::CanisterChildrenOps},
    workflow::runtime_whitelist::RuntimeWhitelistWorkflow,
};
use ic_cdk::api::{canister_self, is_controller as caller_is_controller};

/// Require that the caller controls the current canister.
/// Allows controller-only maintenance calls.
#[expect(clippy::unused_async)]
pub(super) async fn is_controller(caller: Principal) -> Result<(), AccessError> {
    if caller_is_controller(&caller) {
        Ok(())
    } else {
        Err(AccessError::ControllerRequired)
    }
}

/// Require that the caller appears in the canonical runtime whitelist.
/// Missing or unavailable stable authority fails closed.
#[expect(clippy::unused_async)]
pub(super) async fn is_whitelisted(caller: Principal) -> Result<(), AccessError> {
    let whitelisted = RuntimeWhitelistWorkflow::contains(caller).map_err(dependency_unavailable)?;
    if !whitelisted {
        return Err(AccessError::WhitelistRequired);
    }

    Ok(())
}

/// Require controller authority first, then the exact stable Root binding.
pub(super) async fn is_controller_or_root(caller: Principal) -> Result<(), AccessError> {
    if is_controller(caller).await.is_ok() {
        return Ok(());
    }
    is_root(caller).await
}

/// Require that the caller is a direct child of the current canister.
#[expect(clippy::unused_async)]
pub(super) async fn is_child(caller: Principal) -> Result<(), AccessError> {
    if CanisterChildrenOps::contains_pid(&caller) {
        Ok(())
    } else {
        Err(AccessError::DirectChildRequired)
    }
}

/// Require that the caller is the configured parent canister.
#[expect(clippy::unused_async)]
pub(super) async fn is_parent(caller: Principal) -> Result<(), AccessError> {
    let parent_pid = EnvOps::parent_pid().map_err(dependency_unavailable)?;

    if parent_pid == caller {
        Ok(())
    } else {
        Err(AccessError::ParentRequired)
    }
}

/// Require that the caller equals the configured root canister.
#[expect(clippy::unused_async)]
pub(super) async fn is_root(caller: Principal) -> Result<(), AccessError> {
    let root_pid = EnvOps::root_pid().map_err(dependency_unavailable)?;

    if caller == root_pid {
        Ok(())
    } else {
        Err(AccessError::RootRequired)
    }
}

/// Require that the caller is the currently executing canister.
#[expect(clippy::unused_async)]
pub(super) async fn is_same_canister(caller: Principal) -> Result<(), AccessError> {
    if caller == canister_self() {
        Ok(())
    } else {
        Err(AccessError::SelfRequired)
    }
}
