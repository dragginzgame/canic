/// Access-layer errors returned by user-defined auth, guard, and rule hooks.
///
/// These errors are framework-agnostic and are converted into InternalError
/// immediately at the framework boundary.
pub mod auth;
pub mod env;
pub mod guard;
pub mod metrics;
pub mod rule;

use thiserror::Error as ThisError;

///
/// AccessError
///

#[derive(Debug, ThisError)]
pub enum AccessError {
    #[error(transparent)]
    Auth(#[from] auth::AuthAccessError),

    #[error(transparent)]
    Env(#[from] env::EnvAccessError),

    #[error(transparent)]
    Guard(#[from] guard::GuardAccessError),

    #[error(transparent)]
    Rule(#[from] rule::RuleAccessError),

    #[error("access denied: {0}")]
    Denied(String),
}

/// Use this to return a custom access failure from endpoint-specific rules.
#[must_use]
pub fn deny(reason: impl Into<String>) -> AccessError {
    AccessError::Denied(reason.into())
}
