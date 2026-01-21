//! Access predicate composition and evaluation.
//!
//! External semantics:
//! - Access failures are mapped to `ErrorCode::Unauthorized` at the API boundary.
//! - Access denial metrics are emitted by the endpoint macro, not by predicate helpers.

/// Access-layer errors returned by user-defined access predicates.
///
/// These errors are framework-agnostic and are converted into InternalError
/// immediately at the framework boundary.
pub mod app;
pub mod auth;
pub mod env;
#[doc(hidden)]
pub mod expr;
pub mod metrics;

use thiserror::Error as ThisError;

///
/// AccessError
///

#[derive(Debug, ThisError)]
pub enum AccessError {
    #[error("access denied: {0}")]
    Denied(String),
}
