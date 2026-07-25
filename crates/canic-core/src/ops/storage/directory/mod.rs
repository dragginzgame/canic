//! Module: ops::storage::directory
//!
//! Responsibility: validate Fleet/Subnet Directory data before stable replacement.
//! Does not own: stable Directory schemas, workflow orchestration, or DTO policy.
//! Boundary: storage ops between canonical snapshots and stable Directory records.

pub mod fleet;
pub mod mapper;
pub mod subnet;

use crate::{
    InternalError,
    cdk::types::Principal,
    dto::topology::DirectoryProvenance,
    ids::{CanisterRole, FleetBinding},
    ops::storage::StorageOpsError,
    storage::stable::directory::DirectoryEntryRecord,
};
use std::collections::BTreeSet;
use thiserror::Error as ThisError;

///
/// DirectoryOpsError
///
/// Typed storage failure for Fleet and Subnet Directory validation.
///

#[derive(Debug, ThisError)]
pub enum DirectoryOpsError {
    #[error("{directory} Directory role {role} appears more than once")]
    DuplicateRole {
        directory: &'static str,
        role: CanisterRole,
    },

    #[error("{directory} Directory missing required roles: {roles}")]
    MissingRoles {
        directory: &'static str,
        roles: String,
    },

    #[error("{directory} Directory contains unexpected roles: {roles}")]
    UnexpectedRoles {
        directory: &'static str,
        roles: String,
    },

    #[error(
        "Directory Fleet provenance does not match protected Fleet binding (expected {expected:?}, received {received:?})"
    )]
    FleetBindingMismatch {
        expected: Box<FleetBinding>,
        received: Box<FleetBinding>,
    },

    #[error(
        "Directory source root does not match protected Fleet root (expected {expected}, received {received})"
    )]
    SourceRootMismatch {
        expected: Principal,
        received: Principal,
    },
}

pub fn ensure_provenance(
    provenance: &DirectoryProvenance,
    expected_fleet: &FleetBinding,
    expected_source_root: Principal,
) -> Result<(), DirectoryOpsError> {
    if &provenance.fleet != expected_fleet {
        return Err(DirectoryOpsError::FleetBindingMismatch {
            expected: Box::new(expected_fleet.clone()),
            received: Box::new(provenance.fleet.clone()),
        });
    }
    if provenance.source_root != expected_source_root {
        return Err(DirectoryOpsError::SourceRootMismatch {
            expected: expected_source_root,
            received: provenance.source_root,
        });
    }
    Ok(())
}

impl From<DirectoryOpsError> for InternalError {
    fn from(err: DirectoryOpsError) -> Self {
        StorageOpsError::from(err).into()
    }
}

pub(super) fn ensure_unique_roles(
    entries: &[DirectoryEntryRecord],
    directory: &'static str,
) -> Result<(), DirectoryOpsError> {
    let mut seen = BTreeSet::new();
    for entry in entries {
        if !seen.insert(entry.role.clone()) {
            return Err(DirectoryOpsError::DuplicateRole {
                directory,
                role: entry.role.clone(),
            });
        }
    }

    Ok(())
}

pub(super) fn ensure_required_roles(
    entries: &[DirectoryEntryRecord],
    directory: &'static str,
    required: &BTreeSet<CanisterRole>,
) -> Result<(), DirectoryOpsError> {
    if required.is_empty() {
        return Ok(());
    }

    let mut missing = Vec::new();
    for role in required {
        if !entries.iter().any(|entry| &entry.role == role) {
            missing.push(role.to_string());
        }
    }

    if missing.is_empty() {
        Ok(())
    } else {
        Err(DirectoryOpsError::MissingRoles {
            directory,
            roles: missing.join(", "),
        })
    }
}

pub(super) fn ensure_allowed_roles(
    entries: &[DirectoryEntryRecord],
    directory: &'static str,
    allowed: &BTreeSet<CanisterRole>,
) -> Result<(), DirectoryOpsError> {
    let mut unexpected = Vec::new();
    for entry in entries {
        if !allowed.contains(&entry.role) {
            unexpected.push(entry.role.to_string());
        }
    }

    unexpected.sort();
    unexpected.dedup();

    if unexpected.is_empty() {
        Ok(())
    } else {
        Err(DirectoryOpsError::UnexpectedRoles {
            directory,
            roles: unexpected.join(", "),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{AppId, CanonicalNetworkId, FleetId, FleetKey};

    fn fleet(byte: u8) -> FleetBinding {
        FleetBinding {
            fleet: FleetKey {
                canonical_network_id: CanonicalNetworkId::public_ic(),
                fleet_id: FleetId::from_generated_bytes([byte; 32]),
            },
            app: AppId::from("app"),
        }
    }

    #[test]
    fn directory_provenance_requires_exact_protected_fleet_and_root() {
        let source_root = Principal::from_slice(&[1; 29]);
        let expected_fleet = fleet(2);
        let provenance = DirectoryProvenance {
            fleet: expected_fleet.clone(),
            source_root,
        };

        assert!(ensure_provenance(&provenance, &expected_fleet, source_root).is_ok());
        assert!(matches!(
            ensure_provenance(&provenance, &fleet(3), source_root),
            Err(DirectoryOpsError::FleetBindingMismatch { .. })
        ));
        assert!(matches!(
            ensure_provenance(
                &provenance,
                &expected_fleet,
                Principal::from_slice(&[4; 29]),
            ),
            Err(DirectoryOpsError::SourceRootMismatch { .. })
        ));
    }
}
