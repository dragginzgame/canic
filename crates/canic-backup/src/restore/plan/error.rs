//! Module: restore::plan::error
//!
//! Responsibility: define typed restore planning failures.
//! Does not own: manifest validation, artifact validation, or command execution.
//! Boundary: shared error contract for restore plan construction.

use crate::manifest::ManifestValidationError;

use thiserror::Error as ThisError;

///
/// RestorePlanError
///
/// Typed failure returned while building restore plans.
/// Owned by restore planning and used before restore apply or runner state exists.
///

#[derive(Debug, ThisError)]
pub enum RestorePlanError {
    #[error("unsupported restore plan version {0}")]
    UnsupportedVersion(u16),

    #[error("field {0} must not be empty")]
    EmptyField(&'static str),

    #[error("field {field} must be a valid principal: {value}")]
    InvalidPrincipal { field: &'static str, value: String },

    #[error("field {field} must be a 64-character hex sha256 value: {value}")]
    InvalidHash { field: &'static str, value: String },

    #[error("restore plan member_count is {actual}, expected {expected}")]
    MemberCountMismatch { expected: usize, actual: usize },

    #[error("restore plan projection {0} does not match its concrete members")]
    ProjectionMismatch(&'static str),

    #[error("restore plan contains duplicate source canister {0}")]
    DuplicatePlanSource(String),

    #[error("mapping contains duplicate source canister {0}")]
    DuplicateMappingSource(String),

    #[error("mapping contains duplicate target canister {0}")]
    DuplicateMappingTarget(String),

    #[error("restore plan contains duplicate target canister {0}")]
    DuplicatePlanTarget(String),

    #[error("fixed-identity member {source_canister} cannot be mapped to {target_canister}")]
    FixedIdentityRemap {
        source_canister: String,
        target_canister: String,
    },

    #[error(transparent)]
    InvalidManifest(#[from] ManifestValidationError),

    #[error("mapping is missing source canister {0}")]
    MissingMappingSource(String),

    #[error("restore plan contains a parent cycle or unresolved dependency")]
    RestoreOrderCycle,

    #[error("mapping references unknown source canister {0}")]
    UnknownMappingSource(String),
}
