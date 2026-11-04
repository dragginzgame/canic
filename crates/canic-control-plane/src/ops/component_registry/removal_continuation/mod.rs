//! Module: ops::component_registry::removal_continuation
//!
//! Responsibility: retain and select explicitly requested retirement work.
//! Does not own: timers, callback execution, or parent-driven retirement orchestration.
//! Boundary: custody changes preserve exact journals, authority and capacity charges.

#[cfg(test)]
use crate::view::component_registry::{
    RootComponentRemovalContinuationView, RootComponentRemovalKind,
};

use crate::{
    ops::component_registry::{ComponentRegistryOps, map_allocation_commit_error},
    storage::stable::component_registry::RootComponentRegistryStore,
    view::component_registry::{
        RootComponentDeletionProgressView, RootComponentSubtreeRemovalProgressView,
    },
};
use canic_contracts::{
    dto::component_registry::ComponentLifecycleStatus, ids::ComponentInstanceId,
};
use canic_core::control_plane_support::error::InternalError;

impl ComponentRegistryOps {
    /// Enroll an accepted standalone Component removal without taking Root's nested work.
    pub(crate) fn request_component_removal_continuation(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
    ) -> Result<(), InternalError> {
        let view = Self::component_draining(component)?.ok_or_else(InternalError::unavailable)?;
        if view.operation_id != operation_id {
            return Err(InternalError::conflict());
        }
        let before = RootComponentRegistryStore::component_draining(component)
            .ok_or_else(InternalError::unavailable)?;
        if before.continuation_requested
            || matches!(
                view.deletion,
                Some(RootComponentDeletionProgressView::MembershipRemoved(_))
            )
        {
            return Ok(());
        }
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        if current.root_draining.is_some() {
            return Err(InternalError::conflict());
        }
        RootComponentRegistryStore::request_component_removal_continuation(&before)
            .map_err(map_allocation_commit_error)
    }

    /// Enroll a standalone subtree; a draining Component retains its own nested cursor.
    pub(crate) fn request_subtree_removal_continuation(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
    ) -> Result<(), InternalError> {
        let view = Self::subtree_removal(component, operation_id)?
            .ok_or_else(InternalError::unavailable)?;
        let before = RootComponentRegistryStore::subtree_removal(component, operation_id)
            .ok_or_else(InternalError::unavailable)?;
        if before.continuation_requested
            || matches!(
                view.progress,
                RootComponentSubtreeRemovalProgressView::Completed(_)
            )
        {
            return Ok(());
        }
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
        if partition.status != ComponentLifecycleStatus::Active || current.root_draining.is_some() {
            return Err(InternalError::conflict());
        }
        let mut after = before.clone();
        after.continuation_requested = true;
        if RootComponentRegistryStore::subtree_removal_entry_bytes(&after)
            != RootComponentRegistryStore::subtree_removal_entry_bytes(&before)
        {
            return Err(InternalError::invariant());
        }
        RootComponentRegistryStore::replace_subtree_removal(
            &current,
            current.clone(),
            &partition,
            partition.clone(),
            &before,
            after,
        )
        .map_err(map_allocation_commit_error)
    }

    /// Select one eligible operation in stable order, wrapping for fair progress.
    #[cfg(test)]
    pub(crate) fn next_removal_continuation(
        previous: Option<RootComponentRemovalContinuationView>,
    ) -> Result<Option<RootComponentRemovalContinuationView>, InternalError> {
        use crate::ops::continuation_retry::{self, Selection};
        use canic_contracts::dto::operation_retry::Operation;
        let previous = previous.map(|item| match item.kind {
            RootComponentRemovalKind::Component => Operation::ComponentRemoval {
                component: item.component,
                operation_id: item.operation_id,
            },
            RootComponentRemovalKind::Subtree => Operation::SubtreeRemoval {
                component: item.component,
                operation_id: item.operation_id,
            },
        });
        Ok(
            match continuation_retry::select(Self::removal_continuation_operations()?, previous, 0)?
            {
                Selection::Ready(Operation::ComponentRemoval {
                    component,
                    operation_id,
                }) => Some(RootComponentRemovalContinuationView {
                    component,
                    operation_id,
                    kind: RootComponentRemovalKind::Component,
                }),
                Selection::Ready(Operation::SubtreeRemoval {
                    component,
                    operation_id,
                }) => Some(RootComponentRemovalContinuationView {
                    component,
                    operation_id,
                    kind: RootComponentRemovalKind::Subtree,
                }),
                _ => None,
            },
        )
    }
}
