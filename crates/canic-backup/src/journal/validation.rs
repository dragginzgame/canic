//! Module: journal::validation
//!
//! Responsibility: validate durable artifact journal records before resume.
//! Does not own: journal persistence, download execution, or reporting.
//! Boundary: returns typed validation errors for persisted journal input.

use crate::journal::{ArtifactJournalEntry, ArtifactState, DownloadJournal};

use std::{
    collections::BTreeSet,
    path::{Component, PathBuf},
    str::FromStr,
};

use candid::Principal;
use thiserror::Error as ThisError;

const SUPPORTED_JOURNAL_VERSION: u16 = 1;
const SHA256_ALGORITHM: &str = "sha256";

impl DownloadJournal {
    /// Validate resumable artifact state for one backup run.
    pub fn validate(&self) -> Result<(), JournalValidationError> {
        validate_journal_version(self.journal_version)?;
        validate_nonempty("backup_id", &self.backup_id)?;
        validate_hash("discovery_topology_hash", &self.discovery_topology_hash)?;
        validate_hash(
            "pre_snapshot_topology_hash",
            &self.pre_snapshot_topology_hash,
        )?;

        if self.artifacts.is_empty() {
            return Err(JournalValidationError::EmptyCollection("artifacts"));
        }

        let mut keys = BTreeSet::new();
        for artifact in &self.artifacts {
            artifact.validate()?;
            let key = (artifact.canister_id.clone(), artifact.snapshot_id.clone());
            if !keys.insert(key) {
                return Err(JournalValidationError::DuplicateArtifact {
                    canister_id: artifact.canister_id.clone(),
                    snapshot_id: artifact.snapshot_id.clone(),
                });
            }
        }

        Ok(())
    }
}

impl ArtifactJournalEntry {
    /// Validate one artifact's resumable state.
    fn validate(&self) -> Result<(), JournalValidationError> {
        validate_principal("artifacts[].canister_id", &self.canister_id)?;
        validate_nonempty("artifacts[].snapshot_id", &self.snapshot_id)?;
        validate_nonempty("artifacts[].artifact_path", &self.artifact_path)?;
        validate_relative_artifact_path("artifacts[].artifact_path", &self.artifact_path)?;
        validate_nonempty("artifacts[].checksum_algorithm", &self.checksum_algorithm)?;
        validate_nonempty("artifacts[].updated_at", &self.updated_at)?;

        if self.checksum_algorithm != SHA256_ALGORITHM {
            return Err(JournalValidationError::UnsupportedHashAlgorithm(
                self.checksum_algorithm.clone(),
            ));
        }

        if matches!(
            self.state,
            ArtifactState::Downloaded | ArtifactState::ChecksumVerified
        ) {
            validate_required_option("artifacts[].temp_path", self.temp_path.as_deref())?;
        }

        if matches!(
            self.state,
            ArtifactState::ChecksumVerified | ArtifactState::Durable
        ) {
            validate_required_hash("artifacts[].checksum", self.checksum.as_deref())?;
        }

        Ok(())
    }
}

///
/// JournalValidationError
///
/// Typed validation failure for durable artifact journals.
/// Owned by backup journaling and returned before unsafe resume.
///

#[derive(Debug, ThisError)]
pub enum JournalValidationError {
    #[error("duplicate artifact entry for canister {canister_id} snapshot {snapshot_id}")]
    DuplicateArtifact {
        canister_id: String,
        snapshot_id: String,
    },

    #[error("collection {0} must not be empty")]
    EmptyCollection(&'static str),

    #[error("field {0} must not be empty")]
    EmptyField(&'static str),

    #[error("field {field} must be a relative artifact path under the backup root: {value}")]
    InvalidArtifactPath { field: &'static str, value: String },

    #[error("field {0} must be a non-empty sha256 hex string")]
    InvalidHash(&'static str),

    #[error("field {field} must be a valid principal: {value}")]
    InvalidPrincipal { field: &'static str, value: String },

    #[error("invalid journal transition from {from:?} to {to:?}")]
    InvalidStateTransition {
        from: ArtifactState,
        to: ArtifactState,
    },

    #[error("unsupported hash algorithm {0}")]
    UnsupportedHashAlgorithm(String),

    #[error("unsupported journal version {0}")]
    UnsupportedJournalVersion(u16),
}

const fn validate_journal_version(version: u16) -> Result<(), JournalValidationError> {
    if version == SUPPORTED_JOURNAL_VERSION {
        Ok(())
    } else {
        Err(JournalValidationError::UnsupportedJournalVersion(version))
    }
}

fn validate_nonempty(field: &'static str, value: &str) -> Result<(), JournalValidationError> {
    if value.trim().is_empty() {
        Err(JournalValidationError::EmptyField(field))
    } else {
        Ok(())
    }
}

fn validate_relative_artifact_path(
    field: &'static str,
    value: &str,
) -> Result<(), JournalValidationError> {
    let path = PathBuf::from(value);
    if path.is_absolute()
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
    {
        return Err(JournalValidationError::InvalidArtifactPath {
            field,
            value: value.to_string(),
        });
    }
    Ok(())
}

fn validate_required_option(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), JournalValidationError> {
    match value {
        Some(value) => validate_nonempty(field, value),
        None => Err(JournalValidationError::EmptyField(field)),
    }
}

fn validate_principal(field: &'static str, value: &str) -> Result<(), JournalValidationError> {
    validate_nonempty(field, value)?;
    Principal::from_str(value)
        .map(|_| ())
        .map_err(|_| JournalValidationError::InvalidPrincipal {
            field,
            value: value.to_string(),
        })
}

fn validate_required_hash(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), JournalValidationError> {
    match value {
        Some(value) => validate_hash(field, value),
        None => Err(JournalValidationError::EmptyField(field)),
    }
}

fn validate_hash(field: &'static str, value: &str) -> Result<(), JournalValidationError> {
    const SHA256_HEX_LEN: usize = 64;
    validate_nonempty(field, value)?;
    if value.len() == SHA256_HEX_LEN && value.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(JournalValidationError::InvalidHash(field))
    }
}
