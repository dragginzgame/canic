pub mod app;
pub mod mapper;
pub mod subnet;

use crate::{
    InternalError, cdk::types::Principal, ids::CanisterRole, ops::storage::StorageOpsError,
};
use std::collections::BTreeSet;
use thiserror::Error as ThisError;

///
/// DirectoryOpsError
///

#[derive(Debug, ThisError)]
pub enum DirectoryOpsError {
    #[error("{directory} directory role {role} appears more than once")]
    DuplicateRole {
        directory: &'static str,
        role: CanisterRole,
    },

    #[error("{directory} directory missing required roles: {roles}")]
    MissingRoles {
        directory: &'static str,
        roles: String,
    },
}

impl From<DirectoryOpsError> for InternalError {
    fn from(err: DirectoryOpsError) -> Self {
        StorageOpsError::from(err).into()
    }
}

pub(super) fn ensure_unique_roles(
    entries: &[(CanisterRole, Principal)],
    directory: &'static str,
) -> Result<(), DirectoryOpsError> {
    let mut seen = BTreeSet::new();
    for (role, _) in entries {
        if !seen.insert(role.clone()) {
            return Err(DirectoryOpsError::DuplicateRole {
                directory,
                role: role.clone(),
            });
        }
    }

    Ok(())
}

pub(super) fn ensure_required_roles(
    entries: &[(CanisterRole, Principal)],
    directory: &'static str,
    required: &BTreeSet<CanisterRole>,
) -> Result<(), DirectoryOpsError> {
    if required.is_empty() {
        return Ok(());
    }

    let mut missing = Vec::new();
    for role in required {
        if !entries.iter().any(|(entry_role, _)| entry_role == role) {
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
