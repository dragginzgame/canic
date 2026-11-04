//! Coordinator retry windows are committed beside the retained operation journals.
use super::*;
use crate::{
    ops::continuation_retry::Progress,
    storage::stable::{
        continuation_retry::ContinuationRetryRecord,
        fleet_coordinator::CoordinatorContinuationRetryRecord,
    },
};
use canic_contracts::dto::operation_retry::Operation;

pub(super) fn validate(current: &FleetCoordinatorRegistryRecord) -> Result<(), InternalError> {
    if current.continuation_retries.len() > canic_contracts::ids::MAX_FLEET_ROOT_FUNDING_SLOTS + 3
        || !current
            .continuation_retries
            .windows(2)
            .all(|rows| rows[0].operation < rows[1].operation)
    {
        return Err(InternalError::invariant());
    }
    for row in &current.continuation_retries {
        let (Operation::CoordinatorAdmission { operation_id }
        | Operation::CoordinatorFunding { operation_id }
        | Operation::CoordinatorProvisioning { operation_id }
        | Operation::CoordinatorRootRemoval { operation_id }) = row.operation
        else {
            return Err(InternalError::invariant());
        };
        if operation_id == [0; 32] {
            return Err(InternalError::invariant());
        }
        crate::ops::continuation_retry::validate(&row.retry)?;
    }
    Ok(())
}

impl FleetCoordinatorOps {
    pub(crate) fn continuation_retry(
        operation: Operation,
    ) -> Result<(ContinuationRetryRecord, Progress), InternalError> {
        let current = Self::current()?;
        let progress = match operation {
            Operation::CoordinatorAdmission { operation_id } => Progress::Admission(Box::new(
                crate::ops::fleet_admission::FleetAdmissionOps::operation_status(
                    &current.registry,
                    operation_id,
                )?
                .ok_or_else(InternalError::unavailable)?,
            )),
            Operation::CoordinatorFunding { operation_id } => Progress::Funding(Box::new(
                Self::funding_policy_rotation_status(operation_id)?
                    .ok_or_else(InternalError::unavailable)?,
            )),
            Operation::CoordinatorProvisioning { operation_id } => {
                let mut status = Self::provisioning_operation_observation(&current, operation_id)?
                    .ok_or_else(InternalError::unavailable)?;
                status.pending_root_failure = None;
                // Funding deadlines and diagnostics are observations, not completed work.
                status.estate_funding_required = None;
                Progress::Provisioning(Box::new(status))
            }
            Operation::CoordinatorRootRemoval { operation_id } => {
                Progress::RootRemoval(Box::new(Self::root_removal_operation_status(operation_id)?))
            }
            _ => return Err(InternalError::invalid_input()),
        };
        let retry = current
            .continuation_retries
            .iter()
            .find(|row| row.operation == operation)
            .map_or_else(ContinuationRetryRecord::default, |row| row.retry.clone());
        Ok((retry, progress))
    }

    pub(crate) fn update_continuation_retry(
        operation: Operation,
        mutate: impl FnOnce(&mut ContinuationRetryRecord) -> Result<(), InternalError>,
    ) -> Result<(), InternalError> {
        Self::continuation_retry(operation)?;
        let current = Self::current()?;
        let mut next = current.clone();
        if !next
            .continuation_retries
            .iter()
            .any(|row| row.operation == operation)
        {
            // Only current autonomous demand needs a window; completed historical operations
            // must not retain an ever-growing diagnostic ledger.
            let pending = Self::continuation_operations()?;
            next.continuation_retries
                .retain(|row| pending.contains(&row.operation));
            if !pending.contains(&operation) {
                return Err(InternalError::conflict());
            }
            next.continuation_retries
                .push(CoordinatorContinuationRetryRecord {
                    operation,
                    retry: ContinuationRetryRecord::default(),
                });
            next.continuation_retries.sort_by_key(|row| row.operation);
        }
        let retry = &mut next
            .continuation_retries
            .iter_mut()
            .find(|row| row.operation == operation)
            .ok_or_else(InternalError::invariant)?
            .retry;
        mutate(retry)?;
        // This changes diagnostics only, including during funding rotation. Registry head,
        // policy publication and all effect authority remain byte-for-byte unchanged.
        FleetCoordinatorRegistryStore::commit_transition(&current, next)
            .map_err(|_| InternalError::conflict())?;
        Ok(())
    }

    pub(crate) fn continuation_operations() -> Result<Vec<Operation>, InternalError> {
        let registry = Self::registry()?;
        let mut operations = Vec::new();
        if let Some(operation_id) =
            crate::ops::fleet_admission::FleetAdmissionOps::pending_operation(&registry)?
        {
            operations.push(Operation::CoordinatorAdmission { operation_id });
        }
        if let Some(operation_id) = Self::pending_funding_rotation()? {
            operations.push(Operation::CoordinatorFunding { operation_id });
        }
        if let Some(status) = Self::pending_component_provisioning()? {
            operations.push(Operation::CoordinatorProvisioning {
                operation_id: status.operation_id,
            });
        }
        operations.extend(
            Self::pending_root_removals()?
                .into_iter()
                .map(|operation_id| Operation::CoordinatorRootRemoval { operation_id }),
        );
        Ok(operations)
    }
}
