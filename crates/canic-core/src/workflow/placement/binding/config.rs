//! Module: workflow::placement::binding::config
//!
//! Responsibility: resolve configured binding pool definitions.
//! Does not own: configuration storage, binding mutation, or endpoint defaults.
//! Boundary: maps missing binding configuration into workflow errors.

use crate::{
    InternalError,
    config::schema::BindingPool,
    ops::config::ConfigOps,
    workflow::placement::binding::{
        PlacementBindingWorkflow,
        state::{PlacementBindingWorkflowError, available_pool_names},
    },
};

impl PlacementBindingWorkflow {
    // Resolve the configured pool definition for the current binding-bearing parent.
    pub(super) fn get_binding_pool_cfg(pool: &str) -> Result<BindingPool, InternalError> {
        let binding = ConfigOps::current_binding_config()?
            .ok_or(PlacementBindingWorkflowError::BindingDisabled)?;
        let available = available_pool_names(&binding);

        binding
            .pools
            .get(pool)
            .cloned()
            .ok_or_else(|| PlacementBindingWorkflowError::UnknownPool {
                requested: pool.to_string(),
                available,
            })
            .map_err(InternalError::from)
    }
}
