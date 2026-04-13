use super::{ensure_required_roles, ensure_unique_roles};
use crate::{
    InternalError,
    dto::topology::AppIndexArgs,
    ops::config::ConfigOps,
    ops::prelude::*,
    ops::storage::index::mapper::AppIndexRecordMapper,
    storage::stable::index::app::{AppIndex, AppIndexRecord},
};

///
/// AppIndexOps
///

pub struct AppIndexOps;

impl AppIndexOps {
    // -------------------------------------------------------------
    // Getters
    // -------------------------------------------------------------

    #[must_use]
    pub fn get(role: &CanisterRole) -> Option<Principal> {
        AppIndex::export()
            .entries
            .iter()
            .find_map(|(r, pid)| (r == role).then_some(*pid))
    }

    // -------------------------------------------------------------
    // Canonical data access
    // -------------------------------------------------------------

    #[must_use]
    pub fn data() -> AppIndexRecord {
        AppIndex::export()
    }

    #[must_use]
    pub fn snapshot_args() -> AppIndexArgs {
        AppIndexRecordMapper::record_to_view(AppIndex::export())
    }

    pub(crate) fn import_args_allow_incomplete(args: AppIndexArgs) -> Result<(), InternalError> {
        let data = AppIndexRecordMapper::dto_to_record(args);
        Self::import_allow_incomplete(data)
    }

    pub(crate) fn import(data: AppIndexRecord) -> Result<(), InternalError> {
        ensure_unique_roles(&data.entries, "app")?;
        let required = ConfigOps::get()?.app_index.clone();
        ensure_required_roles(&data.entries, "app", &required)?;
        AppIndex::import(data);

        Ok(())
    }

    pub(crate) fn import_allow_incomplete(data: AppIndexRecord) -> Result<(), InternalError> {
        ensure_unique_roles(&data.entries, "app")?;
        AppIndex::import(data);

        Ok(())
    }
}
