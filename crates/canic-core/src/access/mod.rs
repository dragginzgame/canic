//! Access rule composition and evaluation.
//!
//! This module defines generic rule combinators used to gate access.
//! It does not define authorization, topology, or environment logic.

/// Access-layer errors returned by user-defined auth, guard, and rule hooks.
///
/// These errors are framework-agnostic and are converted into InternalError
/// immediately at the framework boundary.
pub mod auth;
pub mod env;
pub mod guard;
pub mod metrics;
pub mod rule;
pub mod topology;

use crate::{
    cdk::{api::msg_caller, types::Principal},
    ids::{BuildNetwork, CanisterRole},
    log,
    log::Topic,
};
use std::pin::Pin;
use thiserror::Error as ThisError;

///
/// AccessRuleError
///
/// Each variant captures the principal that failed a rule (where relevant),
/// making it easy to emit actionable diagnostics in logs.
///

#[derive(Debug, ThisError)]
pub enum AccessRuleError {
    #[error("access dependency unavailable: {0}")]
    DependencyUnavailable(String),

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

///
/// RuleAccessError
///

#[derive(Debug, ThisError)]
pub enum RuleAccessError {
    #[error("this endpoint requires a build-time network (DFX_NETWORK) of either 'ic' or 'local'")]
    BuildNetworkUnknown,

    #[error(
        "this endpoint is only available when built for '{expected}' (DFX_NETWORK), but was built for '{actual}'"
    )]
    BuildNetworkMismatch {
        expected: BuildNetwork,
        actual: BuildNetwork,
    },
}

///
/// AccessError
///

#[derive(Debug, ThisError)]
pub enum AccessError {
    #[error(transparent)]
    Auth(#[from] AccessRuleError),

    #[error(transparent)]
    Env(#[from] env::EnvAccessError),

    #[error(transparent)]
    Guard(#[from] guard::GuardAccessError),

    #[error(transparent)]
    Rule(#[from] RuleAccessError),

    #[error("access denied: {0}")]
    Denied(String),
}

/// Callable issuing an authorization decision for a given caller.
pub type AccessRuleFn = Box<
    dyn Fn(Principal) -> Pin<Box<dyn Future<Output = Result<(), AccessError>> + Send>>
        + Send
        + Sync,
>;

/// Future produced by an [`AccessRuleFn`].
pub type AccessRuleResult = Pin<Box<dyn Future<Output = Result<(), AccessError>> + Send>>;

/// Require that all provided rules pass for the current caller.
///
/// Returns the first failing rule error, or [`AccessRuleError::NoRulesDefined`] if
/// `rules` is empty.
pub async fn require_all(rules: Vec<AccessRuleFn>) -> Result<(), AccessError> {
    let caller = msg_caller();

    if rules.is_empty() {
        return Err(AccessRuleError::NoRulesDefined.into());
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
/// [`AccessRuleError::NoRulesDefined`] if `rules` is empty.
pub async fn require_any(rules: Vec<AccessRuleFn>) -> Result<(), AccessError> {
    let caller = msg_caller();

    if rules.is_empty() {
        return Err(AccessRuleError::NoRulesDefined.into());
    }

    let mut last_error = None;
    for rule in rules {
        match rule(caller).await {
            Ok(()) => return Ok(()),
            Err(e) => last_error = Some(e),
        }
    }

    let err = last_error.unwrap_or_else(|| AccessRuleError::InvalidState.into());
    log!(
        Topic::Auth,
        Warn,
        "auth failed (any) caller={caller}: {err}",
    );

    Err(err)
}

/// Use this to return a custom access failure from endpoint-specific rules.
#[must_use]
pub fn deny(reason: impl Into<String>) -> AccessError {
    AccessError::Denied(reason.into())
}
