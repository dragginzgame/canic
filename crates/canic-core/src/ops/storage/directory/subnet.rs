//! Module: ops::storage::directory::subnet
//!
//! Responsibility: provide deterministic access to the Subnet Directory stable record.
//! Does not own: stable schema, topology workflow, or endpoint DTOs.
//! Boundary: storage ops facade used by topology workflows.

use crate::{
    InternalError,
    dto::topology::SubnetDirectoryInput,
    model::topology::TopologyDirectoryEntry,
    ops::{
        config::ConfigOps,
        prelude::*,
        storage::directory::{
            ensure_allowed_roles, ensure_required_roles, ensure_unique_roles,
            input_entries_to_records, records_to_topology_entries,
        },
    },
    storage::stable::directory::subnet::{SubnetDirectory, SubnetDirectoryData},
};

///
/// SubnetDirectoryOps
///
/// Storage-ops facade for the Subnet Directory stable record.
///

pub struct SubnetDirectoryOps;

/// Fully validated Subnet Directory replacement ready for an infallible commit.
pub struct PreparedSubnetDirectoryImport(SubnetDirectoryData);

impl SubnetDirectoryOps {
    // -------------------------------------------------------------------------
    // Getters
    // -------------------------------------------------------------------------

    #[must_use]
    pub fn get(role: &CanisterRole) -> Option<Principal> {
        // This is still an ops-level convenience, but it stays data-based
        // and does not leak DTOs.
        SubnetDirectory::export()
            .entries
            .iter()
            .find_map(|entry| (&entry.role == role).then_some(entry.pid))
    }

    // -------------------------------------------------------------------------
    // Snapshot
    // -------------------------------------------------------------------------

    #[must_use]
    pub(crate) fn data() -> SubnetDirectoryData {
        SubnetDirectory::export()
    }

    #[must_use]
    pub(crate) fn topology_entries() -> Vec<TopologyDirectoryEntry> {
        records_to_topology_entries(&SubnetDirectory::export().entries)
    }

    pub(crate) fn filter_args_for_local_config(
        args: SubnetDirectoryInput,
    ) -> Result<SubnetDirectoryInput, InternalError> {
        let allowed = ConfigOps::current_subnet_directory_roles()?;
        Ok(SubnetDirectoryInput {
            provenance: args.provenance,
            entries: args
                .entries
                .into_iter()
                .filter(|entry| allowed.contains(&entry.role))
                .collect(),
        })
    }

    #[cfg(test)]
    pub(crate) fn import_args_allow_incomplete(
        args: SubnetDirectoryInput,
    ) -> Result<(), InternalError> {
        let prepared = Self::prepare_args_allow_incomplete(args)?;
        Self::commit_prepared(prepared);

        Ok(())
    }

    pub(crate) fn prepare_args_allow_incomplete(
        args: SubnetDirectoryInput,
    ) -> Result<PreparedSubnetDirectoryImport, InternalError> {
        let data = SubnetDirectoryData {
            entries: input_entries_to_records(args.entries),
        };
        ensure_unique_roles(&data.entries, "Subnet")?;
        let allowed = ConfigOps::current_subnet_directory_roles()?;
        ensure_allowed_roles(&data.entries, "Subnet", &allowed)?;

        Ok(PreparedSubnetDirectoryImport(data))
    }

    pub(crate) fn commit_prepared(prepared: PreparedSubnetDirectoryImport) {
        SubnetDirectory::import(prepared.0);
    }

    // -------------------------------------------------------------------------
    // Import
    // -------------------------------------------------------------------------

    /// Import data into stable storage.
    pub fn import(data: SubnetDirectoryData) -> Result<(), InternalError> {
        ensure_unique_roles(&data.entries, "Subnet")?;
        let required = ConfigOps::current_subnet_directory_roles()?;
        ensure_allowed_roles(&data.entries, "Subnet", &required)?;
        ensure_required_roles(&data.entries, "Subnet", &required)?;
        SubnetDirectory::import(data);

        Ok(())
    }

    /// Import a root-built partial Directory snapshot.
    ///
    /// External/propagated DTO snapshots must use `import_args_allow_incomplete`
    /// so they are checked against the service-derived SubnetDirectory role set.
    pub(crate) fn import_trusted_partial(data: SubnetDirectoryData) -> Result<(), InternalError> {
        ensure_unique_roles(&data.entries, "Subnet")?;
        SubnetDirectory::import(data);

        Ok(())
    }
}
