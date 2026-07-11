//! Module: ops::storage::index::app
//!
//! Responsibility: provide deterministic access to the app index stable record.
//! Does not own: stable schema, topology workflow, or endpoint DTOs.
//! Boundary: storage ops facade used by topology workflows and queries.

use crate::{
    InternalError,
    dto::topology::AppIndexArgs,
    ops::{
        config::ConfigOps,
        prelude::*,
        storage::index::{
            ensure_allowed_roles, ensure_required_roles, ensure_unique_roles,
            mapper::AppIndexDataMapper,
        },
    },
    storage::stable::index::app::{AppIndex, AppIndexData},
};

///
/// AppIndexOps
///
/// Storage-ops facade for the app index stable record.
///

pub struct AppIndexOps;

impl AppIndexOps {
    // -------------------------------------------------------------------------
    // Getters
    // -------------------------------------------------------------------------

    #[must_use]
    pub fn get(role: &CanisterRole) -> Option<Principal> {
        AppIndex::export()
            .entries
            .iter()
            .find_map(|entry| (&entry.role == role).then_some(entry.pid))
    }

    // -------------------------------------------------------------------------
    // Canonical data access
    // -------------------------------------------------------------------------

    #[must_use]
    pub fn data() -> AppIndexData {
        AppIndex::export()
    }

    #[must_use]
    pub fn snapshot_args() -> AppIndexArgs {
        AppIndexDataMapper::data_to_input(AppIndex::export())
    }

    pub(crate) fn filter_args_for_local_config(
        args: AppIndexArgs,
    ) -> Result<AppIndexArgs, InternalError> {
        let allowed = ConfigOps::get()?.app_index.clone();
        Ok(AppIndexArgs(
            args.0
                .into_iter()
                .filter(|entry| allowed.contains(&entry.role))
                .collect(),
        ))
    }

    pub(crate) fn import_args_allow_incomplete(args: AppIndexArgs) -> Result<(), InternalError> {
        let data = AppIndexDataMapper::input_to_data(args);
        ensure_unique_roles(&data.entries, "app")?;
        let allowed = ConfigOps::get()?.app_index.clone();
        ensure_allowed_roles(&data.entries, "app", &allowed)?;
        AppIndex::import(data);

        Ok(())
    }

    pub(crate) fn import(data: AppIndexData) -> Result<(), InternalError> {
        ensure_unique_roles(&data.entries, "app")?;
        let required = ConfigOps::get()?.app_index.clone();
        ensure_allowed_roles(&data.entries, "app", &required)?;
        ensure_required_roles(&data.entries, "app", &required)?;
        AppIndex::import(data);

        Ok(())
    }

    /// Import a root-built partial index snapshot.
    ///
    /// External/propagated DTO snapshots must use `import_args_allow_incomplete`
    /// so they are checked against the configured AppIndex role set.
    pub(crate) fn import_trusted_partial(data: AppIndexData) -> Result<(), InternalError> {
        ensure_unique_roles(&data.entries, "app")?;
        AppIndex::import(data);

        Ok(())
    }
}
