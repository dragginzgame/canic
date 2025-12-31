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
}
