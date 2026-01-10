pub mod app;
pub mod subnet;

use crate::{
    Error, ThisError, cdk::types::Principal, ids::CanisterRole, ops::storage::StorageOpsError,
};
use std::collections::BTreeSet;

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
}

impl From<DirectoryOpsError> for Error {
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
