use super::ensure_unique_roles;
pub use crate::storage::stable::directory::app::AppDirectoryData;
use crate::{InternalError, ops::prelude::*, storage::stable::directory::app::AppDirectory};

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

    #[must_use]
    pub fn matches(role: &CanisterRole, caller: Principal) -> bool {
        AppDirectory::export()
            .entries
            .iter()
            .any(|(r, pid)| r == role && *pid == caller)
    }

    // -------------------------------------------------------------
    // Canonical data access
    // -------------------------------------------------------------

    #[must_use]
    pub fn data() -> AppDirectoryData {
        AppDirectory::export()
    }

    pub(crate) fn import(data: AppDirectoryData) -> Result<(), InternalError> {
        ensure_unique_roles(&data.entries, "app")?;
        AppDirectory::import(data);

        Ok(())
    }
}
