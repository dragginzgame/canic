//! Module: access
//!
//! Responsibility: compose endpoint access predicates and normalize access denial errors.
//! Does not own: endpoint response mapping, workflow authorization, or runtime metrics storage.
//! Boundary: endpoint macros call access predicates before delegating to workflow.

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
/// Framework-agnostic access-layer error returned by endpoint access predicates.
/// Endpoint boundaries convert this into the public unauthorized error shape.
///

#[derive(Debug, ThisError)]
pub enum AccessError {
    #[error("access denied: {0}")]
    Denied(String),
}
