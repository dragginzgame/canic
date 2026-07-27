//! Module: workflow::placement::index::config
//!
//! Responsibility: resolve configured index pool definitions.
//! Does not own: configuration storage, index mutation, or endpoint defaults.
//! Boundary: maps missing index configuration into workflow errors.

use crate::{
    InternalError,
    config::schema::IndexPool,
    ops::config::ConfigOps,
    workflow::placement::index::{
        PlacementIndexWorkflow,
        state::{PlacementIndexWorkflowError, available_pool_names},
    },
};

impl PlacementIndexWorkflow {
    // Resolve the configured pool definition for the current index-bearing parent.
    pub(super) fn get_index_pool_cfg(pool: &str) -> Result<IndexPool, InternalError> {
        let index =
            ConfigOps::current_index_config()?.ok_or(PlacementIndexWorkflowError::IndexDisabled)?;
        let available = available_pool_names(&index);

        index
            .pools
            .get(pool)
            .cloned()
            .ok_or_else(|| PlacementIndexWorkflowError::UnknownPool {
                requested: pool.to_string(),
                available,
            })
            .map_err(InternalError::from)
    }
}
