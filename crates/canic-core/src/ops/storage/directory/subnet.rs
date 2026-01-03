use crate::{
    cdk::types::Principal,
    ids::CanisterRole,
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
    // Snapshot
    // -------------------------------------------------------------

    #[must_use]
    pub fn snapshot() -> SubnetDirectorySnapshot {
        SubnetDirectory::export().into()
    }

    // -------------------------------------------------------------
    // Import
    // -------------------------------------------------------------

    pub(crate) fn import(snapshot: SubnetDirectorySnapshot) {
        let data: SubnetDirectoryData = snapshot.into();
        SubnetDirectory::import(data);
    }

    // -------------------------------------------------------------
    // Internal helpers (ops-only)
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
}
