use super::ensure_unique_roles;
pub use crate::storage::stable::directory::subnet::SubnetDirectoryData;
use crate::{InternalError, ops::prelude::*, storage::stable::directory::subnet::SubnetDirectory};

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
        // This is still an ops-level convenience, but it stays data-based
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
    pub fn data() -> SubnetDirectoryData {
        SubnetDirectory::export()
    }

    // -------------------------------------------------------------
    // Import
    // -------------------------------------------------------------

    /// Import data into stable storage.
    pub fn import(data: SubnetDirectoryData) -> Result<(), InternalError> {
        ensure_unique_roles(&data.entries, "subnet")?;
        SubnetDirectory::import(data);

        Ok(())
    }
}
