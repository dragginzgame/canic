//! Authorization helpers for canister-to-canister and user calls.
//!
//! Compose rule futures and enforce them with [`require_all`] or
//! [`require_any`]. For ergonomics, prefer the macros [`auth_require_all!`]
//! and [`auth_require_any!`], which accept async closures or functions that
//! return [`AuthRuleResult`].

use crate::{
    Error,
    cdk::api::{canister_self, msg_caller},
    memory::{
        Env,
        directory::{AppDirectory, SubnetDirectory},
        topology::{SubnetCanisterChildren, SubnetCanisterRegistry},
    },
    types::CanisterType,
};
use candid::Principal;
use std::pin::Pin;
use thiserror::Error as ThisError;

/// Error returned by authorization rule checks.
///
/// Each variant captures the principal that failed a rule (where relevant),
/// making it easy to emit actionable diagnostics in logs.

#[derive(Debug, ThisError)]
pub enum AuthError {
    /// Guardrail for unreachable states (should not be observed in practice).
    #[error("invalid error state - this should never happen")]
    InvalidState,

    /// No rules were provided to an authorization check.
    #[error("one or more rules must be defined")]
    NoRulesDefined,

    #[error("caller '{0}' does not match the app directory's canister type '{1}'")]
    NotAppDirectoryType(Principal, CanisterType),

    #[error("caller '{0}' does not match the subnet directory's canister type '{1}'")]
    NotSubnetDirectoryType(Principal, CanisterType),

    #[error("caller '{0}' is not a child of this canister")]
    NotChild(Principal),

    #[error("caller '{0}' is not a controller of this canister")]
    NotController(Principal),

    #[error("caller '{0}' is not the parent of this canister")]
    NotParent(Principal),

    #[error("expected caller principal '{1}' got '{0}'")]
    NotPrincipal(Principal, Principal),

    #[error("caller '{0}' is not root")]
    NotRoot(Principal),

    #[error("caller '{0}' is not the current canister")]
    NotSameCanister(Principal),

    #[error("caller '{0}' is not registered on the subnet registry")]
    NotRegisteredToSubnet(Principal),

    #[error("caller '{0}' is not on the whitelist")]
    NotWhitelisted(Principal),
}

/// Callable issuing an authorization decision for a given caller.
pub type AuthRuleFn =
    Box<dyn Fn(Principal) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send>> + Send + Sync>;

/// Future produced by an [`AuthRuleFn`].
pub type AuthRuleResult = Pin<Box<dyn Future<Output = Result<(), Error>> + Send>>;

/// Require that all provided rules pass for the current caller.
///
/// Returns the first failing rule error, or [`AuthError::NoRulesDefined`] if
/// `rules` is empty.
pub async fn require_all(rules: Vec<AuthRuleFn>) -> Result<(), Error> {
    let caller = msg_caller();

    if rules.is_empty() {
        return Err(AuthError::NoRulesDefined.into());
    }

    for rule in rules {
        rule(caller).await?; // early return on failure
    }

    Ok(())
}

/// Require that any one of the provided rules passes for the current caller.
///
/// Returns the last failing rule error if none pass, or
/// [`AuthError::NoRulesDefined`] if `rules` is empty.
pub async fn require_any(rules: Vec<AuthRuleFn>) -> Result<(), Error> {
    let caller = msg_caller();

    if rules.is_empty() {
        return Err(AuthError::NoRulesDefined.into());
    }

    let mut last_error = None;
    for rule in rules {
        match rule(caller).await {
            Ok(()) => return Ok(()),
            Err(e) => last_error = Some(e),
        }
    }

    Err(last_error.unwrap_or_else(|| AuthError::InvalidState.into()))
}

/// Enforce that every supplied rule future succeeds for the current caller.
///
/// This is a convenience wrapper around [`require_all`], allowing guard
/// checks to stay in expression position within async endpoints.
#[macro_export]
macro_rules! auth_require_all {
    ($($f:expr),* $(,)?) => {{
        $crate::auth::require_all(vec![
            $( Box::new(move |caller| Box::pin($f(caller))) ),*
        ]).await
    }};
}

/// Enforce that at least one supplied rule future succeeds for the current
/// caller.
///
/// See [`auth_require_all!`] for details on accepted rule shapes.
#[macro_export]
macro_rules! auth_require_any {
    ($($f:expr),* $(,)?) => {{
        $crate::auth::require_any(vec![
            $( Box::new(move |caller| Box::pin($f(caller))) ),*
        ]).await
    }};
}

// -----------------------------------------------------------------------------
// Rule functions
// -----------------------------------------------------------------------------

/// Ensure the caller matches the subnet directory entry recorded for `ty`.
#[must_use]
pub fn is_app_directory_type(caller: Principal, ty: CanisterType) -> AuthRuleResult {
    Box::pin(async move {
        let canister_pid = AppDirectory::try_get(&ty)
            .map_err(|_| AuthError::NotAppDirectoryType(caller, ty.clone()))?;

        if canister_pid == caller {
            Ok(())
        } else {
            Err(AuthError::NotAppDirectoryType(caller, ty.clone()))?
        }
    })
}

/// Ensure the caller matches the subnet directory entry recorded for `ty`.
#[must_use]
pub fn is_subnet_directory_type(caller: Principal, ty: CanisterType) -> AuthRuleResult {
    Box::pin(async move {
        let canister_pid = SubnetDirectory::try_get(&ty)
            .map_err(|_| AuthError::NotSubnetDirectoryType(caller, ty.clone()))?;

        if canister_pid == caller {
            Ok(())
        } else {
            Err(AuthError::NotSubnetDirectoryType(caller, ty.clone()))?
        }
    })
}

/// Require that the caller is a direct child of the current canister.
#[must_use]
pub fn is_child(caller: Principal) -> AuthRuleResult {
    Box::pin(async move {
        SubnetCanisterChildren::find_by_pid(&caller).ok_or(AuthError::NotChild(caller))?;

        Ok(())
    })
}

/// Require that the caller controls the current canister.
#[must_use]
pub fn is_controller(caller: Principal) -> AuthRuleResult {
    Box::pin(async move {
        if crate::cdk::api::is_controller(&caller) {
            Ok(())
        } else {
            Err(AuthError::NotController(caller).into())
        }
    })
}

/// Require that the caller equals the configured root canister.
#[must_use]
pub fn is_root(caller: Principal) -> AuthRuleResult {
    Box::pin(async move {
        let root_pid = Env::try_get_root_pid()?;

        if caller == root_pid {
            Ok(())
        } else {
            Err(AuthError::NotRoot(caller))?
        }
    })
}

/// Require that the caller is the root or a registered parent canister.
#[must_use]
pub fn is_parent(caller: Principal) -> AuthRuleResult {
    Box::pin(async move {
        if Some(caller) == Env::get_parent_pid() {
            Ok(())
        } else {
            Err(AuthError::NotParent(caller))?
        }
    })
}

/// Require that the caller equals the provided `expected` principal.
#[must_use]
pub fn is_principal(caller: Principal, expected: Principal) -> AuthRuleResult {
    Box::pin(async move {
        if caller == expected {
            Ok(())
        } else {
            Err(AuthError::NotPrincipal(caller, expected))?
        }
    })
}

/// Require that the caller is the currently executing canister.
#[must_use]
pub fn is_same_canister(caller: Principal) -> AuthRuleResult {
    Box::pin(async move {
        if caller == canister_self() {
            Ok(())
        } else {
            Err(AuthError::NotSameCanister(caller))?
        }
    })
}

/// Require that the caller is registered as an canister on this
/// subnet
/// *** ONLY ON ROOT FOR NOW ***
#[must_use]
pub fn is_registered_to_subnet(caller: Principal) -> AuthRuleResult {
    Box::pin(async move {
        match SubnetCanisterRegistry::get(caller) {
            Some(_) => Ok(()),
            None => Err(AuthError::NotRegisteredToSubnet(caller))?,
        }
    })
}

/// Require that the caller appears in the active whitelist (IC deployments).
#[must_use]
#[allow(unused_variables)]
pub fn is_whitelisted(caller: Principal) -> AuthRuleResult {
    Box::pin(async move {
        #[cfg(feature = "ic")]
        {
            use crate::config::Config;
            let cfg = Config::get();

            if !cfg.is_whitelisted(&caller) {
                Err(AuthError::NotWhitelisted(caller))?;
            }
        }

        Ok(())
    })
}
