pub mod auth;
pub mod guard;
pub mod rule;

use crate::ThisError;

///
/// AccessError
///

#[derive(Debug, ThisError)]
pub enum AccessError {
    #[error(transparent)]
    Auth(#[from] auth::AuthError),

    #[error(transparent)]
    Guard(#[from] guard::GuardError),

    #[error(transparent)]
    Rule(#[from] rule::RuleError),

    #[error("access denied: {0}")]
    Denied(String),
}

#[must_use]
pub fn deny(reason: impl Into<String>) -> AccessError {
    AccessError::Denied(reason.into())
}
