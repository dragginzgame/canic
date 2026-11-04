//! Module: ops::component_registry::allocation_continuation
//!
//! Responsibility: retain autonomous ordinary and peer allocation demand.
//! Does not own: scheduling, aggregate Component-group provisioning, or paid effects.
//! Boundary: continuation changes preserve the original reservation and capacity charge.

use crate::{
    ops::component_registry::{
        ComponentRegistryOps, allocation_record_to_view, map_allocation_commit_error,
    },
    storage::stable::component_registry::RootComponentRegistryStore,
    view::component_registry::{RootComponentAllocationProgressView, RootComponentAllocationView},
};
use canic_contracts::dto::component_registry::ComponentProvisioningOrigin;
use canic_core::control_plane_support::error::InternalError;

impl ComponentRegistryOps {
    /// Retain demand without enrolling work already owned by aggregate provisioning.
    pub(crate) fn request_allocation_continuation(
        operation_id: [u8; 32],
    ) -> Result<(), InternalError> {
        let before = RootComponentRegistryStore::allocation(operation_id)
            .ok_or_else(InternalError::unavailable)?;
        if matches!(
            before.provisioning_origin,
            ComponentProvisioningOrigin::ComponentGroup { .. }
        ) {
            return Err(InternalError::conflict());
        }
        if before.continuation_requested
            || Self::allocation_reconciliation_complete(&allocation_record_to_view(before.clone()))
        {
            return Ok(());
        }
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let mut after = before.clone();
        after.continuation_requested = true;
        RootComponentRegistryStore::replace_allocation(&current, current.clone(), &before, after)
            .map_err(map_allocation_commit_error)
    }

    /// Select the next requested operation in stable key order, wrapping after the last operation.
    #[cfg(test)]
    pub(crate) fn next_allocation_continuation(
        previous: Option<[u8; 32]>,
    ) -> Result<Option<[u8; 32]>, InternalError> {
        use crate::ops::continuation_retry::{self, Selection};
        use canic_contracts::dto::operation_retry::Operation;
        Ok(
            match continuation_retry::select(
                Self::allocation_continuation_operations()?,
                previous.map(|operation_id| Operation::RootAllocation { operation_id }),
                0,
            )? {
                Selection::Ready(Operation::RootAllocation { operation_id }) => Some(operation_id),
                _ => None,
            },
        )
    }

    pub(crate) fn allocation_reconciliation_complete(
        allocation: &RootComponentAllocationView,
    ) -> bool {
        match &allocation.progress {
            RootComponentAllocationProgressView::Committed { commitment, .. } => {
                commitment.directory_prepared
                    && commitment.runtime_activated
                    && commitment
                        .membership
                        .as_ref()
                        .is_some_and(|membership| membership.directory_synchronized)
            }
            RootComponentAllocationProgressView::Removed { .. } => true,
            _ => false,
        }
    }
}
