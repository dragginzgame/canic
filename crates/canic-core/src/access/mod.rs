pub mod auth;
pub mod env;
pub mod guard;
pub mod rule;

use crate::ThisError;

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

#[must_use]
pub fn deny(reason: impl Into<String>) -> AccessError {
    AccessError::Denied(reason.into())
}
