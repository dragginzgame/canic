//! Access predicate composition and evaluation.
//!
//! External semantics:
//! - Access failures are mapped to `ErrorCode::Unauthorized` at the API boundary.
//! - Access denial metrics are emitted by the endpoint macro, not by predicate helpers.

/// Access-layer errors returned by user-defined access predicates.
///
/// These errors are framework-agnostic and are converted into InternalError
/// immediately at the framework boundary.
pub(crate) mod auth;
pub(crate) mod env;
pub mod expr;
pub(crate) mod guard;
pub mod metrics;
pub(crate) mod rule;

use thiserror::Error as ThisError;

///
/// AccessError
///

#[derive(Debug, ThisError)]
pub enum AccessError {
    #[error("access denied: {0}")]
    Denied(String),
}
