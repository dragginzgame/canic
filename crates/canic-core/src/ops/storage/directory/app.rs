use super::ensure_unique_roles;
use crate::{
    InternalError,
    dto::topology::AppDirectoryArgs,
    ops::prelude::*,
    ops::storage::directory::mapper::AppDirectoryRecordMapper,
    storage::stable::directory::app::{AppDirectory, AppDirectoryRecord},
};

///
/// AppDirectoryOps
///

pub struct AppDirectoryOps;

impl AppDirectoryOps {
    // -------------------------------------------------------------
    // Getters
    // -------------------------------------------------------------

    #[must_use]
    pub fn get(role: &CanisterRole) -> Option<Principal> {
        AppDirectory::export()
            .entries
            .iter()
            .find_map(|(r, pid)| (r == role).then_some(*pid))
    }

    // -------------------------------------------------------------
    // Canonical data access
    // -------------------------------------------------------------

    #[must_use]
    pub fn data() -> AppDirectoryRecord {
        AppDirectory::export()
    }

    #[must_use]
    pub fn snapshot_args() -> AppDirectoryArgs {
        AppDirectoryRecordMapper::record_to_view(AppDirectory::export())
    }

    pub(crate) fn import_args(args: AppDirectoryArgs) -> Result<(), InternalError> {
        let data = AppDirectoryRecordMapper::dto_to_record(args);
        Self::import(data)
    }

    pub(crate) fn import(data: AppDirectoryRecord) -> Result<(), InternalError> {
        ensure_unique_roles(&data.entries, "app")?;
        AppDirectory::import(data);

        Ok(())
    }
}
