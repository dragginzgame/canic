use crate::{
    ops::prelude::*,
    storage::stable::directory::app::{AppDirectory, AppDirectoryData},
};

///
/// AppDirectorySnapshot
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppDirectorySnapshot {
    pub entries: Vec<(CanisterRole, Principal)>,
}

impl From<AppDirectoryData> for AppDirectorySnapshot {
    fn from(data: AppDirectoryData) -> Self {
        Self {
            entries: data.entries,
        }
    }
}

impl From<AppDirectorySnapshot> for AppDirectoryData {
    fn from(snapshot: AppDirectorySnapshot) -> Self {
        Self {
            entries: snapshot.entries,
        }
    }
}

///
/// AppDirectoryOps
///

pub struct AppDirectoryOps;

impl AppDirectoryOps {
    // -------------------------------------------------------------
    // Snapshot
    // -------------------------------------------------------------

    #[must_use]
    pub fn snapshot() -> AppDirectorySnapshot {
        AppDirectory::export().into()
    }

    // -------------------------------------------------------------
    // Import
    // -------------------------------------------------------------

    pub(crate) fn import(snapshot: AppDirectorySnapshot) {
        let data: AppDirectoryData = snapshot.into();
        AppDirectory::import(data);
    }

    // -------------------------------------------------------------
    // Internal helpers (ops-only)
    // -------------------------------------------------------------

    #[must_use]
    pub fn get(role: &CanisterRole) -> Option<Principal> {
        AppDirectory::export()
            .entries
            .iter()
            .find_map(|(r, pid)| (r == role).then_some(*pid))
    }

    #[must_use]
    pub fn matches(role: &CanisterRole, caller: Principal) -> bool {
        AppDirectory::export()
            .entries
            .iter()
            .any(|(r, pid)| r == role && *pid == caller)
    }
}
