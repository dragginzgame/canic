use crate::{
    InternalError,
    dto::topology::SubnetIndexArgs,
    ops::storage::index::mapper::SubnetIndexRecordMapper,
    ops::storage::index::{ensure_required_roles, ensure_unique_roles},
    ops::{config::ConfigOps, prelude::*},
    storage::stable::index::subnet::{SubnetIndex, SubnetIndexRecord},
};

///
/// SubnetIndexOps
///

pub struct SubnetIndexOps;

impl SubnetIndexOps {
    // -------------------------------------------------------------
    // Getters
    // -------------------------------------------------------------

    #[must_use]
    pub fn get(role: &CanisterRole) -> Option<Principal> {
        // This is still an ops-level convenience, but it stays data-based
        // and does not leak DTOs.
        SubnetIndex::export()
            .entries
            .iter()
            .find_map(|(r, pid)| (r == role).then_some(*pid))
    }

    // -------------------------------------------------------------
    // Snapshot
    // -------------------------------------------------------------

    #[must_use]
    pub fn data() -> SubnetIndexRecord {
        SubnetIndex::export()
    }

    #[must_use]
    pub fn snapshot_args() -> SubnetIndexArgs {
        SubnetIndexRecordMapper::record_to_view(SubnetIndex::export())
    }

    pub(crate) fn import_args_allow_incomplete(args: SubnetIndexArgs) -> Result<(), InternalError> {
        let data = SubnetIndexRecordMapper::dto_to_record(args);
        Self::import_allow_incomplete(data)
    }

    // -------------------------------------------------------------
    // Import
    // -------------------------------------------------------------

    /// Import data into stable storage.
    pub fn import(data: SubnetIndexRecord) -> Result<(), InternalError> {
        ensure_unique_roles(&data.entries, "subnet")?;
        let subnet_cfg = ConfigOps::current_subnet()?;
        ensure_required_roles(&data.entries, "subnet", &subnet_cfg.subnet_index)?;
        SubnetIndex::import(data);

        Ok(())
    }

    pub(crate) fn import_allow_incomplete(data: SubnetIndexRecord) -> Result<(), InternalError> {
        ensure_unique_roles(&data.entries, "subnet")?;
        SubnetIndex::import(data);

        Ok(())
    }
}
