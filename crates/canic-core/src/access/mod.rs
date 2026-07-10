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
    #[error("access denied: delegated auth cert expired")]
    DelegatedAuthCertExpired,

    #[error("access denied: delegated auth token expired")]
    DelegatedAuthTokenExpired,

    #[error("access denied: {0}")]
    Denied(String),
}

impl AccessError {
    #[must_use]
    pub const fn kind(&self) -> AccessErrorKind {
        match self {
            Self::DelegatedAuthCertExpired => AccessErrorKind::DelegatedAuthCertExpired,
            Self::DelegatedAuthTokenExpired => AccessErrorKind::DelegatedAuthTokenExpired,
            Self::Denied(_) => AccessErrorKind::Denied,
        }
    }
}

///
/// AccessErrorKind
///
/// Machine-readable access denial category for endpoint error adapters.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessErrorKind {
    DelegatedAuthCertExpired,
    DelegatedAuthTokenExpired,
    Denied,
}
