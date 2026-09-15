//! Module: ops::component_registry::child_failure
//!
//! Responsibility: retain bounded child-allocation diagnostics independently of work progress.
//! Does not own: timers, remote effects, or funding authority.
//! Boundary: compare-and-commit preserves reservation, partition and paid-effect receipts.

use crate::{
    ops::component_registry::{
        ComponentRegistryOps, child_allocation_is_terminal, map_allocation_commit_error,
        validate_child_allocation_record, validate_partition_record,
    },
    storage::stable::component_registry::{
        RootComponentChildAllocationFailureRecord, RootComponentRegistryStore,
    },
    view::component_registry::{
        RootComponentChildAllocationFailureView, RootComponentChildAllocationView,
    },
};
use canic_core::{
    control_plane_support::error::{
        InternalError, ProvisioningFailureStage, ProvisioningFailureView, retry_delay_seconds,
    },
    diagnostics::codes,
    dto::component_provisioning::ProvisioningRetryCategory,
    dto::component_registry::{ComponentLifecycleStatus, RootComponentChildAllocationFailure},
    ids::ComponentInstanceId,
};

impl ComponentRegistryOps {
    /// Observe a retained bootstrap dependency behind a generic parent activation failure.
    /// The Root owns the child operation even before a child canister exists.
    pub(crate) fn initial_child_failure(
        component: ComponentInstanceId,
        parent_error: &InternalError,
    ) -> Result<Option<ProvisioningFailureView>, InternalError> {
        let code = parent_error.public_error().code();
        if parent_error.provisioning_failure().is_some()
            || (code != codes::STATE_UNAVAILABLE.raw_code()
                && code != codes::STATE_INVALID.raw_code())
        {
            return Ok(None);
        }
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
        validate_partition_record(&partition)?;
        if partition.status != ComponentLifecycleStatus::Prepared {
            return Ok(None);
        }
        let registry =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        // The store's exact Component/operation ordering makes selection stable
        // across retries; diagnostic timestamp churn does not choose another child.
        for allocation in RootComponentRegistryStore::child_allocations(component) {
            validate_child_allocation_record(&allocation)?;
            if !allocation.initial_bootstrap || child_allocation_is_terminal(&allocation) {
                continue;
            }
            if let Some(failure) = allocation.last_failure {
                return Ok(Some(ProvisioningFailureView {
                    recorded_at_ns: Some(failure.failed_at_ns),
                    stage: ProvisioningFailureStage::ComponentChildAllocation,
                    target: registry.root.fleet_subnet_root,
                    operation_id: allocation.operation_id,
                    diagnostic_code: failure.diagnostic_code,
                    retry_category: ProvisioningRetryCategory::Backoff,
                }));
            }
        }
        Ok(None)
    }

    /// Retain the exact public diagnostic without changing issued effects or capacity.
    pub(crate) fn record_child_failure(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        error: &InternalError,
        now_ns: u64,
    ) -> Result<RootComponentChildAllocationFailureView, InternalError> {
        let before = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(InternalError::unavailable)?;
        validate_child_allocation_record(&before)?;
        if before
            .last_failure
            .is_some_and(|failure| now_ns < failure.failed_at_ns)
        {
            return Err(InternalError::conflict());
        }
        let diagnostic_code = error.public_error().raw_code();
        let consecutive_failures = before
            .last_failure
            .filter(|failure| failure.diagnostic_code == diagnostic_code)
            .map_or(1, |failure| failure.consecutive_failures.saturating_add(1));
        let failure = RootComponentChildAllocationFailureRecord {
            diagnostic_code,
            failed_at_ns: now_ns,
            consecutive_failures,
            retry_at_ns: retry_at(now_ns, consecutive_failures)?,
        };
        validate(Some(failure))?;
        let mut after = before.clone();
        after.last_failure = Some(failure);
        replace(&before, after)?;
        Ok(view(failure))
    }

    /// Diagnostic changes are not progress; clear only on work advancement or completion.
    pub(crate) fn clear_child_failure_after_progress(
        previous: &RootComponentChildAllocationView,
        complete: bool,
    ) -> Result<(), InternalError> {
        let before =
            RootComponentRegistryStore::child_allocation(previous.component, previous.operation_id)
                .ok_or_else(InternalError::unavailable)?;
        validate_child_allocation_record(&before)?;
        let current = super::child_allocation_record_to_view(before.clone());
        if before.last_failure.is_some() && (complete || current.progress != previous.progress) {
            let mut after = before.clone();
            after.last_failure = None;
            replace(&before, after)?;
        }
        Ok(())
    }

    pub(crate) const fn child_failure_response(
        failure: RootComponentChildAllocationFailureView,
    ) -> RootComponentChildAllocationFailure {
        RootComponentChildAllocationFailure {
            diagnostic_code: failure.diagnostic_code,
            failed_at_ns: failure.failed_at_ns,
            consecutive_failures: failure.consecutive_failures,
            retry_at_ns: failure.retry_at_ns,
        }
    }
}

fn replace(
    before: &crate::storage::stable::component_registry::RootComponentChildAllocationRecord,
    after: crate::storage::stable::component_registry::RootComponentChildAllocationRecord,
) -> Result<(), InternalError> {
    let current = RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
    let partition = RootComponentRegistryStore::partition(before.component)
        .ok_or_else(InternalError::unavailable)?;
    RootComponentRegistryStore::replace_child_allocation(
        &current,
        current.clone(),
        &partition,
        partition.clone(),
        before,
        after,
    )
    .map_err(map_allocation_commit_error)
}

fn retry_at(now_ns: u64, failures: u32) -> Result<u64, InternalError> {
    retry_delay_seconds(ProvisioningRetryCategory::Backoff, failures)
        .and_then(|seconds| seconds.checked_mul(1_000_000_000))
        .and_then(|delay| now_ns.checked_add(delay))
        .ok_or_else(InternalError::invariant)
}

pub(super) fn validate(
    failure: Option<RootComponentChildAllocationFailureRecord>,
) -> Result<(), InternalError> {
    if let Some(failure) = failure
        && (failure.diagnostic_code == 0
            || failure.consecutive_failures == 0
            || failure.retry_at_ns != retry_at(failure.failed_at_ns, failure.consecutive_failures)?)
    {
        return Err(InternalError::invariant());
    }
    Ok(())
}

pub(super) const fn view(
    failure: RootComponentChildAllocationFailureRecord,
) -> RootComponentChildAllocationFailureView {
    RootComponentChildAllocationFailureView {
        diagnostic_code: failure.diagnostic_code,
        failed_at_ns: failure.failed_at_ns,
        consecutive_failures: failure.consecutive_failures,
        retry_at_ns: failure.retry_at_ns,
    }
}
