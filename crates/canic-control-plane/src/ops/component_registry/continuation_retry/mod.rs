//! Retry windows remain in the original allocation or retirement record.
use super::*;
use crate::{
    ops::continuation_retry::Progress, storage::stable::continuation_retry::ContinuationRetryRecord,
};
use canic_contracts::dto::operation_retry::Operation;

impl ComponentRegistryOps {
    pub(crate) fn continuation_retry(
        operation: Operation,
    ) -> Result<(ContinuationRetryRecord, Progress), InternalError> {
        match operation {
            Operation::RootAllocation { operation_id } => {
                let view = Self::allocation(operation_id).ok_or_else(InternalError::unavailable)?;
                let record = RootComponentRegistryStore::allocation(operation_id)
                    .ok_or_else(InternalError::unavailable)?;
                if !record.continuation_requested {
                    return Err(InternalError::conflict());
                }
                Ok((record.retry, Progress::Allocation(Box::new(view))))
            }
            Operation::ComponentRemoval {
                component,
                operation_id,
            } => {
                let view =
                    Self::component_draining(component)?.ok_or_else(InternalError::unavailable)?;
                let record = RootComponentRegistryStore::component_draining(component)
                    .ok_or_else(InternalError::unavailable)?;
                if record.operation_id != operation_id || !record.continuation_requested {
                    return Err(InternalError::conflict());
                }
                Ok((record.retry, Progress::ComponentRemoval(Box::new(view))))
            }
            Operation::SubtreeRemoval {
                component,
                operation_id,
            } => {
                let view = Self::subtree_removal(component, operation_id)?
                    .ok_or_else(InternalError::unavailable)?;
                let record = RootComponentRegistryStore::subtree_removal(component, operation_id)
                    .ok_or_else(InternalError::unavailable)?;
                if !record.continuation_requested {
                    return Err(InternalError::conflict());
                }
                Ok((record.retry, Progress::SubtreeRemoval(Box::new(view))))
            }
            _ => Err(InternalError::invalid_input()),
        }
    }

    pub(crate) fn update_continuation_retry(
        operation: Operation,
        mutate: impl FnOnce(&mut ContinuationRetryRecord) -> Result<(), InternalError>,
    ) -> Result<(), InternalError> {
        Self::continuation_retry(operation)?;
        match operation {
            Operation::RootAllocation { operation_id } => {
                let before = RootComponentRegistryStore::allocation(operation_id)
                    .ok_or_else(InternalError::unavailable)?;
                let mut after = before.clone();
                mutate(&mut after.retry)?;
                let current =
                    RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
                RootComponentRegistryStore::replace_allocation(
                    &current,
                    current.clone(),
                    &before,
                    after,
                )
                .map_err(map_allocation_commit_error)
            }
            Operation::ComponentRemoval { component, .. } => {
                let before = RootComponentRegistryStore::component_draining(component)
                    .ok_or_else(InternalError::unavailable)?;
                let mut retry = before.retry.clone();
                mutate(&mut retry)?;
                RootComponentRegistryStore::replace_component_retry(&before, retry)
                    .map_err(map_allocation_commit_error)
            }
            Operation::SubtreeRemoval {
                component,
                operation_id,
            } => {
                let before = RootComponentRegistryStore::subtree_removal(component, operation_id)
                    .ok_or_else(InternalError::unavailable)?;
                let mut after = before.clone();
                mutate(&mut after.retry)?;
                let current =
                    RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
                let partition = RootComponentRegistryStore::partition(component)
                    .ok_or_else(InternalError::unavailable)?;
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
            _ => Err(InternalError::invalid_input()),
        }
    }

    pub(crate) fn allocation_continuation_operations() -> Result<Vec<Operation>, InternalError> {
        let mut operations = Vec::new();
        for record in RootComponentRegistryStore::allocations() {
            if !record.continuation_requested {
                continue;
            }
            if matches!(
                record.provisioning_origin,
                canic_contracts::dto::component_registry::ComponentProvisioningOrigin::ComponentGroup { .. }
            ) {
                return Err(InternalError::invariant());
            }
            let view =
                Self::allocation(record.operation_id).ok_or_else(InternalError::invariant)?;
            if !Self::allocation_reconciliation_complete(&view) {
                operations.push(Operation::RootAllocation {
                    operation_id: record.operation_id,
                });
            }
        }
        Ok(operations)
    }

    pub(crate) fn removal_continuation_operations() -> Result<Vec<Operation>, InternalError> {
        let mut operations = Vec::new();
        for record in RootComponentRegistryStore::component_drainings() {
            if !record.continuation_requested {
                continue;
            }
            let view =
                Self::component_draining(record.component)?.ok_or_else(InternalError::invariant)?;
            if !matches!(
                view.deletion,
                Some(RootComponentDeletionProgressView::MembershipRemoved(_))
            ) {
                operations.push(Operation::ComponentRemoval {
                    component: record.component,
                    operation_id: record.operation_id,
                });
            }
        }
        for record in RootComponentRegistryStore::registry_components()
            .into_iter()
            .flat_map(RootComponentRegistryStore::subtree_removals)
        {
            if !record.continuation_requested {
                continue;
            }
            let view = Self::subtree_removal(record.component, record.operation_id)?
                .ok_or_else(InternalError::invariant)?;
            if !matches!(
                view.progress,
                RootComponentSubtreeRemovalProgressView::Completed(_)
            ) {
                operations.push(Operation::SubtreeRemoval {
                    component: record.component,
                    operation_id: record.operation_id,
                });
            }
        }
        Ok(operations)
    }
}
