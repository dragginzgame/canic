use super::ensure_unique_roles;
use crate::{
    InternalError,
    ops::prelude::*,
    storage::stable::directory::subnet::{SubnetDirectory, SubnetDirectoryData},
};

///
/// SubnetDirectorySnapshot
/// Internal, operational snapshot of subnet directory.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubnetDirectorySnapshot {
    pub entries: Vec<(CanisterRole, Principal)>,
}

impl From<SubnetDirectoryData> for SubnetDirectorySnapshot {
    fn from(data: SubnetDirectoryData) -> Self {
        Self {
            entries: data.entries,
        }
    }
}

impl From<SubnetDirectorySnapshot> for SubnetDirectoryData {
    fn from(snapshot: SubnetDirectorySnapshot) -> Self {
        Self {
            entries: snapshot.entries,
        }
    }
}

///
/// SubnetDirectoryOps
///

pub struct SubnetDirectoryOps;

impl SubnetDirectoryOps {
    // -------------------------------------------------------------
    // Getters
    // -------------------------------------------------------------

    #[must_use]
    pub fn get(role: &CanisterRole) -> Option<Principal> {
        // This is still an ops-level convenience, but it stays snapshot/data-based
        // and does not leak DTOs.
        SubnetDirectory::export()
            .entries
            .iter()
            .find_map(|(r, pid)| (r == role).then_some(*pid))
    }

    // -------------------------------------------------------------
    // Snapshot
    // -------------------------------------------------------------

    #[must_use]
    pub fn snapshot() -> SubnetDirectorySnapshot {
        SubnetDirectory::export().into()
    }

    // -------------------------------------------------------------
    // Import
    // -------------------------------------------------------------

    /// Import a snapshot into stable storage.
    pub fn import(snapshot: SubnetDirectorySnapshot) -> Result<(), InternalError> {
        ensure_unique_roles(&snapshot.entries, "subnet")?;
        let data: SubnetDirectoryData = snapshot.into();
        SubnetDirectory::import(data);

        Ok(())
    }
}
