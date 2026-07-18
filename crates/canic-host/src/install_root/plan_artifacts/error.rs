//! Module: install_root::plan_artifacts::error
//!
//! Responsibility: classify supplied deployment-plan artifact failures.
//! Does not own: error projection, deployment policy, or recovery decisions.
//! Boundary: preserves admission and revalidation causes through install phases.

use std::{io, path::PathBuf};

use thiserror::Error as ThisError;

///
/// PlanArtifactError
///
/// Typed failure while admitting or revalidating supplied deployment-plan
/// artifact bytes.
///

#[derive(Debug, ThisError)]
pub(in crate::install_root) enum PlanArtifactError {
    #[error("deployment plan role {role} has conflicting {kind} digests: {first} and {second}")]
    ConflictingDigest {
        role: String,
        kind: &'static str,
        first: String,
        second: String,
    },

    #[error(
        "deployment plan role {role} {kind} digest mismatch: expected {expected}, found {found}"
    )]
    DigestMismatch {
        role: String,
        kind: &'static str,
        expected: String,
        found: String,
    },

    #[error("deployment plan contains duplicate artifact role: {role}")]
    DuplicateRole { role: String },

    #[error(
        "deployment plan environment mismatch: install environment {install}, plan environment {plan}"
    )]
    EnvironmentMismatch { install: String, plan: String },

    #[error("deployment plan environment name is invalid: {name}")]
    InvalidEnvironment { name: String },

    #[error("deployment plan role {role} artifact is not a valid gzip stream")]
    InvalidGzip {
        role: String,
        #[source]
        source: io::Error,
    },

    #[error("deployment plan produced an invalid root release-set manifest at {path}: {reason}")]
    InvalidManifest { path: PathBuf, reason: String },

    #[error("deployment plan role is invalid: {role}: {reason}")]
    InvalidRole { role: String, reason: String },

    #[error("deployment plan role {role} artifact does not contain a Wasm module")]
    InvalidWasm { role: String },

    #[error("deployment plan role {role} {kind} artifact has no matching digest pin")]
    MissingDigestPin { role: String, kind: &'static str },

    #[error("deployment plan ID must not be empty")]
    MissingPlanId,

    #[error("deployment plan is missing a root role artifact")]
    MissingRoot,

    #[error("deployment plan role {role} has no artifact path")]
    MissingSource { role: String },

    #[error("deployment plan artifact IO failed for {path}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("deployment plan role {role} raw and gzip artifacts contain different Wasm bytes")]
    RepresentationMismatch { role: String },

    #[error("deployment plan schema mismatch: expected {expected}, found {found}")]
    SchemaVersionMismatch { expected: u32, found: u32 },

    #[error("deployment plan manifest serialization failed for {path}")]
    Serialization {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("deployment plan artifact path is unsafe for role {role}: {path}")]
    UnsafePath { role: String, path: PathBuf },
}
