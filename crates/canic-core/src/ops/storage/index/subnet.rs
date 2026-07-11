//! Module: ops::storage::index::subnet
//!
//! Responsibility: provide deterministic access to the subnet index stable record.
//! Does not own: stable schema, topology workflow, or endpoint DTOs.
//! Boundary: storage ops facade used by topology workflows and queries.

use crate::{
    InternalError,
    dto::topology::SubnetIndexArgs,
    ops::{
        config::ConfigOps,
        prelude::*,
        storage::index::{
            ensure_allowed_roles, ensure_required_roles, ensure_unique_roles,
            mapper::SubnetIndexDataMapper,
        },
    },
    storage::stable::index::subnet::{SubnetIndex, SubnetIndexData},
};

///
/// SubnetIndexOps
///
/// Storage-ops facade for the subnet index stable record.
///

pub struct SubnetIndexOps;

impl SubnetIndexOps {
    // -------------------------------------------------------------------------
    // Getters
    // -------------------------------------------------------------------------

    #[must_use]
    pub fn get(role: &CanisterRole) -> Option<Principal> {
        // This is still an ops-level convenience, but it stays data-based
        // and does not leak DTOs.
        SubnetIndex::export()
            .entries
            .iter()
            .find_map(|entry| (&entry.role == role).then_some(entry.pid))
    }

    // -------------------------------------------------------------------------
    // Snapshot
    // -------------------------------------------------------------------------

    #[must_use]
    pub fn data() -> SubnetIndexData {
        SubnetIndex::export()
    }

    #[must_use]
    pub fn snapshot_args() -> SubnetIndexArgs {
        SubnetIndexDataMapper::data_to_input(SubnetIndex::export())
    }

    pub(crate) fn filter_args_for_local_config(
        args: SubnetIndexArgs,
    ) -> Result<SubnetIndexArgs, InternalError> {
        let allowed = ConfigOps::current_subnet()?.subnet_index_roles();
        Ok(SubnetIndexArgs(
            args.0
                .into_iter()
                .filter(|entry| allowed.contains(&entry.role))
                .collect(),
        ))
    }

    pub(crate) fn import_args_allow_incomplete(args: SubnetIndexArgs) -> Result<(), InternalError> {
        let data = SubnetIndexDataMapper::input_to_data(args);
        ensure_unique_roles(&data.entries, "subnet")?;
        let subnet_cfg = ConfigOps::current_subnet()?;
        ensure_allowed_roles(&data.entries, "subnet", &subnet_cfg.subnet_index_roles())?;
        SubnetIndex::import(data);

        Ok(())
    }

    // -------------------------------------------------------------------------
    // Import
    // -------------------------------------------------------------------------

    /// Import data into stable storage.
    pub fn import(data: SubnetIndexData) -> Result<(), InternalError> {
        ensure_unique_roles(&data.entries, "subnet")?;
        let subnet_cfg = ConfigOps::current_subnet()?;
        let required = subnet_cfg.subnet_index_roles();
        ensure_allowed_roles(&data.entries, "subnet", &required)?;
        ensure_required_roles(&data.entries, "subnet", &required)?;
        SubnetIndex::import(data);

        Ok(())
    }

    /// Import a root-built partial index snapshot.
    ///
    /// External/propagated DTO snapshots must use `import_args_allow_incomplete`
    /// so they are checked against the service-derived SubnetIndex role set.
    pub(crate) fn import_trusted_partial(data: SubnetIndexData) -> Result<(), InternalError> {
        ensure_unique_roles(&data.entries, "subnet")?;
        SubnetIndex::import(data);

        Ok(())
    }
}
