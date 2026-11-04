//! Module: ops::component_registry::child_continuation
//!
//! Responsibility: retain and project autonomous child allocation demand.
//! Does not own: timers, retry decisions, or paid effects.
//! Boundary: demand changes preserve reservation authority and capacity accounting.

use crate::{
    ops::component_registry::{
        ComponentRegistryOps, child_allocation_is_terminal, child_failure,
        map_allocation_commit_error, validate_child_allocation_record,
    },
    storage::stable::component_registry::RootComponentRegistryStore,
    view::component_registry::RootComponentChildContinuationView,
};
use canic_contracts::ids::ComponentInstanceId;
use canic_core::control_plane_support::error::InternalError;

impl ComponentRegistryOps {
    /// Retain an accepted request for continuation independently of heap registrations.
    pub(crate) fn request_child_continuation(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
    ) -> Result<(), InternalError> {
        let before = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(InternalError::unavailable)?;
        validate_child_allocation_record(&before)?;
        if before.continuation_requested || child_allocation_is_terminal(&before) {
            return Ok(());
        }
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
        let mut after = before.clone();
        after.continuation_requested = true;
        RootComponentRegistryStore::replace_child_allocation(
            &current,
            current.clone(),
            &partition,
            partition.clone(),
            &before,
            after,
        )
        .map_err(map_allocation_commit_error)
    }

    /// Discover requested, unfinished work from the existing capacity-bounded registry.
    pub(crate) fn child_continuations()
    -> Result<Vec<RootComponentChildContinuationView>, InternalError> {
        let mut pending = Vec::new();
        for record in RootComponentRegistryStore::registry_components()
            .into_iter()
            .flat_map(RootComponentRegistryStore::child_allocations)
        {
            if !record.continuation_requested {
                continue;
            }
            validate_child_allocation_record(&record)?;
            if !child_allocation_is_terminal(&record) {
                pending.push(RootComponentChildContinuationView {
                    component: record.component,
                    operation_id: record.operation_id,
                    last_failure: record.last_failure.map(child_failure::view),
                });
            }
        }
        Ok(pending)
    }
}
