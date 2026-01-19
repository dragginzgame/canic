use crate::{
    InternalError,
    dto::topology::SubnetDirectoryArgs,
    ops::storage::directory::mapper::SubnetDirectoryRecordMapper,
    ops::{prelude::*, storage::directory::ensure_unique_roles},
    storage::stable::directory::subnet::{SubnetDirectory, SubnetDirectoryRecord},
};

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
    pub fn data() -> SubnetDirectoryRecord {
        SubnetDirectory::export()
    }

    #[must_use]
    pub fn snapshot_args() -> SubnetDirectoryArgs {
        SubnetDirectoryRecordMapper::record_to_view(SubnetDirectory::export())
    }

    pub(crate) fn import_args(args: SubnetDirectoryArgs) -> Result<(), InternalError> {
        let data = SubnetDirectoryRecordMapper::dto_to_record(args);
        Self::import(data)
    }

    // -------------------------------------------------------------
    // Import
    // -------------------------------------------------------------

    /// Import data into stable storage.
    pub fn import(data: SubnetDirectoryRecord) -> Result<(), InternalError> {
        ensure_unique_roles(&data.entries, "subnet")?;
        SubnetDirectory::import(data);

        Ok(())
    }
}
