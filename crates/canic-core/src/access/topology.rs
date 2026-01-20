use crate::{
    access::{AccessRuleError, AccessRuleResult},
    cdk::{api::canister_self, types::Principal},
    ops::{runtime::env::EnvOps, storage::children::CanisterChildrenOps},
};

///
/// Topology
///

/// Require that the caller is a direct child of the current canister.
/// Protects child-only endpoints (e.g., sync) from sibling/root callers.
#[must_use]
pub fn is_child(caller: Principal) -> AccessRuleResult {
    Box::pin(async move {
        if CanisterChildrenOps::contains_pid(&caller) {
            Ok(())
        } else {
            Err(AccessRuleError::NotChild(caller).into())
        }
    })
}

/// Require that the caller is the configured parent canister.
/// Use on child sync endpoints to enforce parent-only calls.
#[must_use]
pub fn is_parent(caller: Principal) -> AccessRuleResult {
    Box::pin(async move {
        let snapshot = EnvOps::snapshot();
        let parent_pid = snapshot.parent_pid.ok_or_else(|| {
            AccessRuleError::DependencyUnavailable("parent pid unavailable".to_string())
        })?;

        if parent_pid == caller {
            Ok(())
        } else {
            Err(AccessRuleError::NotParent(caller).into())
        }
    })
}

/// Require that the caller equals the configured root canister.
/// Gate root-only operations (e.g., topology mutations).
#[must_use]
pub fn caller_is_root(caller: Principal) -> AccessRuleResult {
    Box::pin(async move {
        let root_pid = EnvOps::root_pid().map_err(|_| {
            AccessRuleError::DependencyUnavailable("root pid unavailable".to_string())
        })?;

        if caller == root_pid {
            Ok(())
        } else {
            Err(AccessRuleError::NotRoot(caller).into())
        }
    })
}

/// Require that the caller is the currently executing canister.
/// For self-calls only.
#[must_use]
pub fn is_same_canister(caller: Principal) -> AccessRuleResult {
    Box::pin(async move {
        if caller == canister_self() {
            Ok(())
        } else {
            Err(AccessRuleError::NotSameCanister(caller).into())
        }
    })
}
