//! Module: fleet_ensure::dto
//!
//! Responsibility: parse the current desired-Fleet document at the host boundary.
//! Does not own: policy, persistence, observations, or effects.
//! Boundary: returns passive model input plus the canonical source digest.

use crate::fleet_ensure::model::DesiredFleet;
use canic_core::cdk::utils::hash::sha256_hex;
use std::{fs, io, path::Path};
use thiserror::Error as ThisError;

/// Parsed current desired state and its exact source identity.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadedDesiredFleet {
    pub desired: DesiredFleet,
    pub sha256: String,
}

/// Typed desired-document read or parse failure.

#[derive(Debug, ThisError)]
pub enum DesiredFleetLoadError {
    #[error("failed to read desired Fleet document {}: {source}", path.display())]
    Read {
        path: std::path::PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to parse desired Fleet document {}: {source}", path.display())]
    Parse {
        path: std::path::PathBuf,
        #[source]
        source: toml::de::Error,
    },
}

/// Read one complete current-generation desired Fleet document.
pub fn load_desired_fleet(path: &Path) -> Result<LoadedDesiredFleet, DesiredFleetLoadError> {
    let bytes = fs::read(path).map_err(|source| DesiredFleetLoadError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let desired = toml::from_slice::<DesiredFleet>(&bytes).map_err(|source| {
        DesiredFleetLoadError::Parse {
            path: path.to_path_buf(),
            source,
        }
    })?;
    Ok(LoadedDesiredFleet {
        desired,
        sha256: sha256_hex(&bytes),
    })
}
