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

    #[error("field {field} must be a valid principal: {value}")]
    InvalidPrincipal { field: &'static str, value: String },

    #[error("mapping is missing source canister {0}")]
    MissingMappingSource(String),

    #[error("restore plan contains a parent cycle or unresolved dependency")]
    RestoreOrderCycle,

    #[error("mapping references unknown source canister {0}")]
    UnknownMappingSource(String),
}
