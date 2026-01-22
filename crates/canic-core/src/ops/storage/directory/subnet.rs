use crate::{
    InternalError,
    dto::topology::SubnetDirectoryArgs,
    ops::storage::directory::mapper::SubnetDirectoryRecordMapper,
    ops::storage::directory::{ensure_required_roles, ensure_unique_roles},
    ops::{config::ConfigOps, prelude::*},
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

    pub(crate) fn import_args_allow_incomplete(
        args: SubnetDirectoryArgs,
    ) -> Result<(), InternalError> {
        let data = SubnetDirectoryRecordMapper::dto_to_record(args);
        Self::import_allow_incomplete(data)
    }

    // -------------------------------------------------------------
    // Import
    // -------------------------------------------------------------

    /// Import data into stable storage.
    pub fn import(data: SubnetDirectoryRecord) -> Result<(), InternalError> {
        ensure_unique_roles(&data.entries, "subnet")?;
        let subnet_cfg = ConfigOps::current_subnet()?;
        ensure_required_roles(&data.entries, "subnet", &subnet_cfg.subnet_directory)?;
        SubnetDirectory::import(data);

        Ok(())
    }

    pub(crate) fn import_allow_incomplete(
        data: SubnetDirectoryRecord,
    ) -> Result<(), InternalError> {
        ensure_unique_roles(&data.entries, "subnet")?;
        SubnetDirectory::import(data);

        Ok(())
    }
}
