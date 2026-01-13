//! Authorization helpers for canister-to-canister and user calls.
//!
//! Compose rule futures and enforce them with [`require_all`] or
//! [`require_any`]. For ergonomics, prefer the facade macros
//! `canic::auth_require_all!` and `canic::auth_require_any!`, which accept
//! async closures or functions that return [`AuthRuleResult`].

use crate::{
    InternalError, ThisError,
    access::AccessError,
    cdk::{
        api::{canister_self, msg_caller},
        types::Principal,
    },
    ids::CanisterRole,
    log,
    log::Topic,
    ops::{
        runtime::env::EnvOps,
        storage::{
            children::CanisterChildrenOps,
            directory::{app::AppDirectoryOps, subnet::SubnetDirectoryOps},
            registry::subnet::SubnetRegistryOps,
        },
    },
};
use std::pin::Pin;

///
/// AuthAccessError
///
/// Each variant captures the principal that failed a rule (where relevant),
/// making it easy to emit actionable diagnostics in logs.
///

#[derive(Debug, ThisError)]
pub enum AuthAccessError {
    /// Guardrail for unreachable states (should not be observed in practice).
    #[error("invalid error state - this should never happen")]
    InvalidState,

    /// No rules were provided to an authorization check.
    #[error("one or more rules must be defined")]
    NoRulesDefined,

    #[error("access dependency unavailable: {0}")]
    DependencyUnavailable(String),

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

impl From<AuthAccessError> for InternalError {
    fn from(err: AuthAccessError) -> Self {
        AccessError::Auth(err).into()
    }
}

/// Callable issuing an authorization decision for a given caller.
pub type AuthRuleFn = Box<
    dyn Fn(Principal) -> Pin<Box<dyn Future<Output = Result<(), AccessError>> + Send>>
        + Send
        + Sync,
>;

/// Future produced by an [`AuthRuleFn`].
pub type AuthRuleResult = Pin<Box<dyn Future<Output = Result<(), AccessError>> + Send>>;

/// Require that all provided rules pass for the current caller.
///
/// Returns the first failing rule error, or [`AuthError::NoRulesDefined`] if
/// `rules` is empty.
pub async fn require_all(rules: Vec<AuthRuleFn>) -> Result<(), AccessError> {
    let caller = msg_caller();

    if rules.is_empty() {
        return Err(AuthAccessError::NoRulesDefined.into());
    }

    for rule in rules {
        if let Err(err) = rule(caller).await {
            log!(
                Topic::Auth,
                Warn,
                "auth failed (all) caller={caller}: {err}",
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
pub async fn require_any(rules: Vec<AuthRuleFn>) -> Result<(), AccessError> {
    let caller = msg_caller();

    if rules.is_empty() {
        return Err(AuthAccessError::NoRulesDefined.into());
    }

    let mut last_error = None;
    for rule in rules {
        match rule(caller).await {
            Ok(()) => return Ok(()),
            Err(e) => last_error = Some(e),
        }
    }

    let err = last_error.unwrap_or_else(|| AuthAccessError::InvalidState.into());
    log!(
        Topic::Auth,
        Warn,
        "auth failed (any) caller={caller}: {err}",
    );

    Err(err)
}

// -----------------------------------------------------------------------------
// Rule functions
// -----------------------------------------------------------------------------

/// Ensure the caller matches the subnet directory entry recorded for `role`.
/// Use for admin endpoints that expect specific app directory canisters.
#[must_use]
pub fn is_app_directory_role(caller: Principal, role: CanisterRole) -> AuthRuleResult {
    Box::pin(async move {
        if AppDirectoryOps::matches(&role, caller) {
            Ok(())
        } else {
            Err(AuthAccessError::NotAppDirectoryType(caller, role).into())
        }
    })
}

/// Require that the caller is a direct child of the current canister.
/// Protects child-only endpoints (e.g., sync) from sibling/root callers.
#[must_use]
pub fn is_child(caller: Principal) -> AuthRuleResult {
    Box::pin(async move {
        if CanisterChildrenOps::contains_pid(&caller) {
            Ok(())
        } else {
            Err(AuthAccessError::NotChild(caller).into())
        }
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
            Err(AuthAccessError::NotController(caller).into())
        }
    })
}

/// Require that the caller is the configured parent canister.
/// Use on child sync endpoints to enforce parent-only calls.
#[must_use]
pub fn is_parent(caller: Principal) -> AuthRuleResult {
    Box::pin(async move {
        let parent_pid = EnvOps::parent_pid().map_err(to_access)?;

        if parent_pid == caller {
            Ok(())
        } else {
            Err(AuthAccessError::NotParent(caller).into())
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
            Err(AuthAccessError::NotPrincipal(caller, expected).into())
        }
    })
}

/// Require that the caller is registered as a canister on this subnet.
///
/// NOTE: Currently enforced only on the root canister.
#[must_use]
pub fn is_registered_to_subnet(caller: Principal) -> AuthRuleResult {
    Box::pin(async move {
        if SubnetRegistryOps::is_registered(caller) {
            Ok(())
        } else {
            Err(AuthAccessError::NotRegisteredToSubnet(caller).into())
        }
    })
}

/// Require that the caller equals the configured root canister.
/// Gate root-only operations (e.g., topology mutations).
#[must_use]
pub fn is_root(caller: Principal) -> AuthRuleResult {
    Box::pin(async move {
        let root_pid = EnvOps::root_pid().map_err(to_access)?;

        if caller == root_pid {
            Ok(())
        } else {
            Err(AuthAccessError::NotRoot(caller).into())
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
            Err(AuthAccessError::NotSameCanister(caller).into())
        }
    })
}

/// Ensure the caller matches the subnet directory entry recorded for `role`.
/// Use for admin endpoints that expect specific app directory canisters.
#[must_use]
pub fn is_subnet_directory_role(caller: Principal, role: CanisterRole) -> AuthRuleResult {
    Box::pin(async move {
        match SubnetDirectoryOps::get(&role) {
            Some(pid) if pid == caller => Ok(()),
            _ => Err(AuthAccessError::NotSubnetDirectoryType(caller, role).into()),
        }
    })
}

/// Require that the caller appears in the active whitelist (IC deployments).
/// No-op on local builds; enforces whitelist on IC.
#[must_use]
pub fn is_whitelisted(caller: Principal) -> AuthRuleResult {
    Box::pin(async move {
        use crate::config::Config;
        let cfg = Config::get().map_err(to_access)?;

        if !cfg.is_whitelisted(&caller) {
            return Err(AuthAccessError::NotWhitelisted(caller).into());
        }

        Ok(())
    })
}

/// to_access
/// helper function
fn to_access(err: InternalError) -> AccessError {
    AuthAccessError::DependencyUnavailable(err.to_string()).into()
}
