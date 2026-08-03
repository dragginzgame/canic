//! Module: ops::storage::directory::fleet
//!
//! Responsibility: provide deterministic access to the Fleet Directory stable record.
//! Does not own: stable schema, topology workflow, or endpoint DTOs.
//! Boundary: storage ops facade used by topology workflows.

use crate::{
    InternalError,
    dto::topology::FleetDirectoryInput,
    ops::{
        config::ConfigOps,
        prelude::*,
        storage::directory::{
            ensure_allowed_roles, ensure_required_roles, ensure_unique_roles,
            input_entries_to_records,
        },
    },
    storage::stable::directory::fleet::{FleetDirectory, FleetDirectoryData},
};

///
/// FleetDirectoryOps
///
/// Storage-ops facade for the Fleet Directory stable record.
///

pub struct FleetDirectoryOps;

/// Fully validated Fleet Directory replacement ready for an infallible commit.
pub struct PreparedFleetDirectoryImport(FleetDirectoryData);

impl FleetDirectoryOps {
    // -------------------------------------------------------------------------
    // Getters
    // -------------------------------------------------------------------------

    #[must_use]
    pub fn get(role: &CanisterRole) -> Option<Principal> {
        FleetDirectory::export()
            .entries
            .iter()
            .find_map(|entry| (&entry.role == role).then_some(entry.pid))
    }

    // -------------------------------------------------------------------------
    // Canonical data access
    // -------------------------------------------------------------------------

    #[must_use]
    pub(crate) fn data() -> FleetDirectoryData {
        FleetDirectory::export()
    }

    pub(crate) fn filter_args_for_local_config(
        args: FleetDirectoryInput,
    ) -> Result<FleetDirectoryInput, InternalError> {
        let allowed = ConfigOps::get()?.fleet_directory_roles();
        Ok(FleetDirectoryInput {
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
        args: FleetDirectoryInput,
    ) -> Result<(), InternalError> {
        let prepared = Self::prepare_args_allow_incomplete(args)?;
        Self::commit_prepared(prepared);

        Ok(())
    }

    pub(crate) fn prepare_args_allow_incomplete(
        args: FleetDirectoryInput,
    ) -> Result<PreparedFleetDirectoryImport, InternalError> {
        let data = FleetDirectoryData {
            entries: input_entries_to_records(args.entries),
        };
        ensure_unique_roles(&data.entries, "Fleet")?;
        let allowed = ConfigOps::get()?.fleet_directory_roles();
        ensure_allowed_roles(&data.entries, "Fleet", &allowed)?;

        Ok(PreparedFleetDirectoryImport(data))
    }

    pub(crate) fn commit_prepared(prepared: PreparedFleetDirectoryImport) {
        FleetDirectory::import(prepared.0);
    }

    pub(crate) fn import(data: FleetDirectoryData) -> Result<(), InternalError> {
        ensure_unique_roles(&data.entries, "Fleet")?;
        let required = ConfigOps::get()?.fleet_directory_roles();
        ensure_allowed_roles(&data.entries, "Fleet", &required)?;
        ensure_required_roles(&data.entries, "Fleet", &required)?;
        FleetDirectory::import(data);

        Ok(())
    }

    /// Import a root-built partial Directory snapshot.
    ///
    /// External/propagated DTO snapshots must use `import_args_allow_incomplete`
    /// so they are checked against the configured FleetDirectory role set.
    pub(crate) fn import_trusted_partial(data: FleetDirectoryData) -> Result<(), InternalError> {
        ensure_unique_roles(&data.entries, "Fleet")?;
        FleetDirectory::import(data);

        Ok(())
    }
}
