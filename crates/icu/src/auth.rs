//! Authorization helpers for canister-to-canister and user calls.
//!
//! Compose rule futures and enforce them with `require_all` or `require_any`.
//! For ergonomics, prefer the macros `auth_require_all!` and `auth_require_any!`.

use crate::{
    Error,
    cdk::api::{canister_self, msg_caller},
    memory::{CanisterChildren, CanisterDirectory, CanisterRegistry, CanisterState},
    types::CanisterType,
};
use candid::Principal;
use std::pin::Pin;
use thiserror::Error as ThisError;

/// Errors returned by authorization rule checks.
#[derive(Debug, ThisError)]
pub enum AuthError {
    /// Guardrail for unreachable states (should not be observed in practice).
    #[error("invalid error state - this should never happen")]
    InvalidState,

    /// No rules were provided to an authorization check.
    #[error("one or more rules must be defined")]
    NoRulesDefined,

    /// Caller is not an application canister registered on this subnet.
    #[error("caller '{0}' is not an application canister on this subnet")]
    NotApp(Principal),

    /// Caller does not match the expected canister type.
    #[error("caller '{0}' does not match canister type '{1}'")]
    NotCanisterType(Principal, CanisterType),

    /// Caller is not a child of the current canister.
    #[error("caller '{0}' is not a child of this canister")]
    NotChild(Principal),

    /// Caller is not a controller of the current canister.
    #[error("caller '{0}' is not a controller of this canister")]
    NotController(Principal),

    /// Caller is not the parent of the current canister.
    #[error("caller '{0}' is not the parent of this canister")]
    NotParent(Principal),

    /// Caller principal does not equal the expected principal.
    #[error("expected caller principal '{1}' got '{0}'")]
    NotPrincipal(Principal, Principal),

    /// Caller is not the root canister.
    #[error("caller '{0}' is not root")]
    NotRoot(Principal),

    /// Caller is not the current canister (self).
    #[error("caller '{0}' is not the current canister")]
    NotSameCanister(Principal),

    /// Caller is not present in the active whitelist.
    #[error("caller '{0}' is not on the whitelist")]
    NotWhitelisted(Principal),
}

///
/// Rule
///

pub type AuthRuleFn =
    Box<dyn Fn(Principal) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send>> + Send + Sync>;

pub type AuthRuleResult = Pin<Box<dyn Future<Output = Result<(), Error>> + Send>>;

///
/// Auth Functions
///

/// Require that all provided rules pass for the current caller.
///
/// Returns the first failing rule error, or `AuthError::NoRulesDefined` if `rules` is empty.
///
/// Example (no_run):
/// ```no_run
/// use icu::auth;
/// # async fn demo() -> Result<(), icu::IcuError> {
/// let _ = auth::require_all(vec![]).await; // will error: NoRulesDefined
/// # Ok(()) }
/// ```
// require_all
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
/// Returns the last failing rule error if none pass, or `AuthError::NoRulesDefined` if empty.
///
/// Example (no_run):
/// ```no_run
/// use icu::auth;
/// # async fn demo() -> Result<(), icu::IcuError> {
/// let _ = auth::require_any(vec![]).await; // will error: NoRulesDefined
/// # Ok(()) }
/// ```
// require_any
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

///
/// RULE MACROS
///

#[macro_export]
macro_rules! auth_require_all {
    ($($f:expr),* $(,)?) => {{
        $crate::auth::require_all(vec![
            $( Box::new(move |caller| Box::pin($f(caller))) ),*
        ]).await
    }};
}

#[macro_export]
macro_rules! auth_require_any {
    ($($f:expr),* $(,)?) => {{
        $crate::auth::require_any(vec![
            $( Box::new(move |caller| Box::pin($f(caller))) ),*
        ]).await
    }};
}

///
/// RULE FUNCTIONS
///

// is_app
#[must_use]
pub fn is_app(caller: Principal) -> AuthRuleResult {
    Box::pin(async move {
        match CanisterRegistry::get(caller) {
            Some(_) => Ok(()),
            None => Err(AuthError::NotApp(caller))?,
        }
    })
}

// is_canister_type
// check caller against the id of a specific canister path
#[must_use]
pub fn is_canister_type(caller: Principal, ty: CanisterType) -> AuthRuleResult {
    Box::pin(async move {
        CanisterDirectory::try_get(&ty)
            .map_err(|_| AuthError::NotCanisterType(caller, ty.clone()))?;

        Ok(())
    })
}

// is_child
#[must_use]
pub fn is_child(caller: Principal) -> AuthRuleResult {
    Box::pin(async move {
        CanisterChildren::get(&caller).ok_or(AuthError::NotChild(caller))?;

        Ok(())
    })
}

// is_controller
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

// is_root
#[must_use]
pub fn is_root(caller: Principal) -> AuthRuleResult {
    Box::pin(async move {
        let root_pid = CanisterState::get_root_pid();

        if caller == root_pid {
            Ok(())
        } else {
            Err(AuthError::NotRoot(caller))?
        }
    })
}

// is_parent
#[must_use]
pub fn is_parent(caller: Principal) -> AuthRuleResult {
    Box::pin(async move {
        if CanisterState::has_parent_pid(&caller) {
            Ok(())
        } else {
            Err(AuthError::NotParent(caller))?
        }
    })
}

// is_principal
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

// is_same_canister
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

// is_whitelisted
// only on mainnet - only if the whitelist is active
#[must_use]
#[allow(unused_variables)]
pub fn is_whitelisted(caller: Principal) -> AuthRuleResult {
    Box::pin(async move {
        #[cfg(feature = "ic")]
        {
            use crate::config::Config;
            if !Config::is_whitelisted(&caller)? {
                Err(AuthError::NotWhitelisted(caller))?;
            }
        }

        Ok(())
    })
}
