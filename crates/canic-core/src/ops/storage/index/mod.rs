pub mod app;
pub mod mapper;
pub mod subnet;

use crate::{
    InternalError, cdk::types::Principal, ids::CanisterRole, ops::storage::StorageOpsError,
};
use std::collections::BTreeSet;
use thiserror::Error as ThisError;

///
/// IndexOpsError
///

#[derive(Debug, ThisError)]
pub enum IndexOpsError {
    #[error("{index} index role {role} appears more than once")]
    DuplicateRole {
        index: &'static str,
        role: CanisterRole,
    },

    #[error("{index} index missing required roles: {roles}")]
    MissingRoles { index: &'static str, roles: String },
}

impl From<IndexOpsError> for InternalError {
    fn from(err: IndexOpsError) -> Self {
        StorageOpsError::from(err).into()
    }
}

pub(super) fn ensure_unique_roles(
    entries: &[(CanisterRole, Principal)],
    index: &'static str,
) -> Result<(), IndexOpsError> {
    let mut seen = BTreeSet::new();
    for (role, _) in entries {
        if !seen.insert(role.clone()) {
            return Err(IndexOpsError::DuplicateRole {
                index,
                role: role.clone(),
            });
        }
    }

    Ok(())
}

pub(super) fn ensure_required_roles(
    entries: &[(CanisterRole, Principal)],
    index: &'static str,
    required: &BTreeSet<CanisterRole>,
) -> Result<(), IndexOpsError> {
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
        Err(IndexOpsError::MissingRoles {
            index,
            roles: missing.join(", "),
        })
    }
}
