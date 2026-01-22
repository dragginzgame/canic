use super::{ensure_required_roles, ensure_unique_roles};
use crate::{
    InternalError,
    dto::topology::AppDirectoryArgs,
    ops::config::ConfigOps,
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

    pub(crate) fn import_args_allow_incomplete(
        args: AppDirectoryArgs,
    ) -> Result<(), InternalError> {
        let data = AppDirectoryRecordMapper::dto_to_record(args);
        Self::import_allow_incomplete(data)
    }

    pub(crate) fn import(data: AppDirectoryRecord) -> Result<(), InternalError> {
        ensure_unique_roles(&data.entries, "app")?;
        let required = ConfigOps::get()?.app_directory.clone();
        ensure_required_roles(&data.entries, "app", &required)?;
        AppDirectory::import(data);

        Ok(())
    }

    pub(crate) fn import_allow_incomplete(data: AppDirectoryRecord) -> Result<(), InternalError> {
        ensure_unique_roles(&data.entries, "app")?;
        AppDirectory::import(data);

        Ok(())
    }
}
