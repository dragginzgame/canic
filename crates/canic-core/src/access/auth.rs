//! Authorization helpers for canister-to-canister and user calls.
//!
//! Compose rule futures and enforce them with [`require_all`] or
//! [`require_any`]. For ergonomics, prefer the macros [`auth_require_all!`]
//! and [`auth_require_any!`], which accept async closures or functions that
//! return [`AuthRuleResult`].

use crate::{
    Error, ThisError,
    access::AccessError,
    cdk::api::{canister_self, msg_caller},
    ids::CanisterRole,
    log,
    log::Topic,
    ops::{
        runtime::env::EnvOps,
        storage::{
            children::CanisterChildrenOps,
            directory::{AppDirectoryOps, SubnetDirectoryOps},
            registry::SubnetRegistryOps,
        },
    },
};
use candid::Principal;
use std::pin::Pin;

///
/// AuthError
///
/// Each variant captures the principal that failed a rule (where relevant),
/// making it easy to emit actionable diagnostics in logs.
///

#[derive(Debug, ThisError)]
pub enum AuthError {
    /// Guardrail for unreachable states (should not be observed in practice).
    #[error("invalid error state - this should never happen")]
    InvalidState,

    /// No rules were provided to an authorization check.
    #[error("one or more rules must be defined")]
    NoRulesDefined,

    #[error("caller '{0}' does not match the app directory's canister role '{1}'")]
    NotAppDirectoryType(Principal, CanisterRole),

    #[error("caller '{0}' does not match the subnet directory's canister role '{1}'")]
    NotSubnetDirectoryType(Principal, CanisterRole),

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

impl From<AuthError> for Error {
    fn from(err: AuthError) -> Self {
        AccessError::AuthError(err).into()
    }
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
        if let Err(err) = rule(caller).await {
            let err_msg = err.to_string();
            log!(
                Topic::Auth,
                Warn,
                "auth failed (all) caller={caller}: {err_msg}"
            );

            return Err(err);
        }
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

    let err = last_error.unwrap_or_else(|| AuthError::InvalidState.into());
    let err_msg = err.to_string();
    log!(
        Topic::Auth,
        Warn,
        "auth failed (any) caller={caller}: {err_msg}"
    );

    Err(err)
}

/// Enforce that every supplied rule future succeeds for the current caller.
///
/// This is a convenience wrapper around [`require_all`], allowing guard
/// checks to stay in expression position within async endpoints.
#[macro_export]
macro_rules! auth_require_all {
    ($($f:expr),* $(,)?) => {{
        $crate::access::auth::require_all(vec![
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
        $crate::access::auth::require_any(vec![
            $( Box::new(move |caller| Box::pin($f(caller))) ),*
        ]).await
    }};
}

// -----------------------------------------------------------------------------
// Rule functions
// -----------------------------------------------------------------------------

/// Ensure the caller matches the subnet directory entry recorded for `role`.
/// Use for admin endpoints that expect specific app directory canisters.
#[must_use]
pub fn is_app_directory_role(caller: Principal, role: CanisterRole) -> AuthRuleResult {
    Box::pin(async move {
        match AppDirectoryOps::get(&role) {
            Some(pid) if pid == caller => Ok(()),
            _ => Err(AuthError::NotAppDirectoryType(caller, role).into()),
        }
    })
}

/// Ensure the caller matches the subnet directory entry recorded for `role`.
/// Use for admin endpoints that expect specific subnet directory canisters.
#[must_use]
pub fn is_subnet_directory_role(caller: Principal, role: CanisterRole) -> AuthRuleResult {
    Box::pin(async move {
        match SubnetDirectoryOps::get(&role) {
            Some(pid) if pid == caller => Ok(()),
            _ => Err(AuthError::NotSubnetDirectoryType(caller, role).into()),
        }
    })
}

/// Require that the caller is a direct child of the current canister.
/// Protects child-only endpoints (e.g., sync) from sibling/root callers.
#[must_use]
pub fn is_child(caller: Principal) -> AuthRuleResult {
    Box::pin(async move {
        CanisterChildrenOps::find_by_pid(&caller).ok_or(AuthError::NotChild(caller))?;

        Ok(())
    })
}

/// Require that the caller controls the current canister.
/// Allows controller-only maintenance calls.
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
/// Gate root-only operations (e.g., topology mutations).
#[must_use]
pub fn is_root(caller: Principal) -> AuthRuleResult {
    Box::pin(async move {
        let root_pid = EnvOps::root_pid()?;

        if caller == root_pid {
            Ok(())
        } else {
            Err(AuthError::NotRoot(caller))?
        }
    })
}

/// Require that the caller is the root or a registered parent canister.
/// Use on child sync endpoints to enforce parent-only calls.
#[must_use]
pub fn is_parent(caller: Principal) -> AuthRuleResult {
    Box::pin(async move {
        let parent_pid = EnvOps::parent_pid()?;

        if parent_pid == caller {
            Ok(())
        } else {
            Err(AuthError::NotParent(caller))?
        }
    })
}

/// Require that the caller equals the provided `expected` principal.
/// Handy for single-tenant or pre-registered callers.
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
/// For self-calls only.
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
/// Ensures only registered canisters call root orchestration endpoints.
#[must_use]
pub fn is_registered_to_subnet(caller: Principal) -> AuthRuleResult {
    Box::pin(async move {
        if SubnetRegistryOps::is_registered(caller) {
            Ok(())
        } else {
            Err(AuthError::NotRegisteredToSubnet(caller))?
        }
    })
}

/// Require that the caller appears in the active whitelist (IC deployments).
/// No-op on local builds; enforces whitelist on IC.
#[must_use]
pub fn is_whitelisted(caller: Principal) -> AuthRuleResult {
    Box::pin(async move {
        use crate::config::Config;
        let cfg = Config::get()?;

        if !cfg.is_whitelisted(&caller) {
            Err(AuthError::NotWhitelisted(caller))?;
        }

        Ok(())
    })
}
