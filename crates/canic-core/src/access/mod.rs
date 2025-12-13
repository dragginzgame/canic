pub mod auth;
pub mod guard;
pub mod policy;

use crate::ThisError;

///
/// AccessError
///

#[derive(Debug, ThisError)]
pub enum AccessError {
    #[error(transparent)]
    AuthError(#[from] auth::AuthError),

    #[error(transparent)]
    GuardError(#[from] guard::GuardError),

    #[error(transparent)]
    PolicyError(#[from] policy::PolicyError),
}
