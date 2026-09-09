//! Module: ops::component_provisioning::failure
//!
//! Responsibility: retain one exact originating failure and bounded retry projection.
//! Does not own: timer scheduling, remote effects, or caller authentication.
//! Boundary: mutations compare the current operation and never alter effect receipts.

use crate::{
    ops::component_provisioning::{
        RootComponentProvisioningOps, map_commit_error, validated_record,
    },
    storage::stable::component_provisioning::{
        RootComponentProvisioningFailureRecord, RootComponentProvisioningStore,
    },
    view::component_provisioning::{
        RootComponentProvisioningFailureView, RootComponentProvisioningView,
    },
};
use canic_core::control_plane_support::error::{
    InternalError, ProvisioningFailureView, retry_delay_seconds,
};
use canic_core::dto::component_provisioning::RootComponentProvisioningFailure;

/// Durable work counters, deliberately excluding diagnostic observations.
#[derive(Eq, PartialEq)]
struct ProvisioningProgress {
    phase: canic_core::dto::component_provisioning::RootComponentProvisioningPhase,
    reserved: u32,
    claimed: u32,
    installed: u32,
    committed: u32,
    published: u32,
    activated: u32,
    root_active: bool,
}

impl RootComponentProvisioningOps {
    /// Let the callback that first retained a failure own its next retry.
    pub(crate) fn failure_changed(
        before: &RootComponentProvisioningView,
        after: &canic_core::dto::component_provisioning::RootComponentProvisioningStatusResponse,
    ) -> bool {
        before.last_failure.map(failure_response) != after.last_failure
    }

    /// Commit failure evidence while retaining every current phase and issued receipt.
    pub(crate) fn record_failure(
        operation_id: [u8; 32],
        plan_hash: [u8; 32],
        origin: ProvisioningFailureView,
        now_ns: u64,
    ) -> Result<RootComponentProvisioningFailureView, InternalError> {
        let before = RootComponentProvisioningStore::operation(operation_id)
            .ok_or_else(InternalError::unavailable)?;
        let view = validated_record(before.clone())?;
        let time_is_current = now_ns >= view.accepted_at_ns
            && before
                .last_failure
                .is_none_or(|failure| now_ns >= failure.failed_at_ns);
        if view.plan_hash != plan_hash || !time_is_current {
            return Err(InternalError::conflict());
        }
        let failures = before
            .last_failure
            .map_or(1, |failure| failure.consecutive_failures.saturating_add(1));
        let retry_at_ns = retry_delay_seconds(origin.retry_category, failures)
            .and_then(|seconds| seconds.checked_mul(1_000_000_000))
            .and_then(|delay| now_ns.checked_add(delay));
        let failure = RootComponentProvisioningFailureRecord {
            stage: origin.stage,
            target: origin.target,
            operation_id: origin.operation_id,
            diagnostic_code: origin.diagnostic_code,
            retry_category: origin.retry_category,
            failed_at_ns: now_ns,
            consecutive_failures: failures,
            retry_at_ns,
        };
        validate_failure(Some(failure), view.accepted_at_ns)?;
        let mut after = before.clone();
        after.last_failure = Some(failure);
        RootComponentProvisioningStore::replace_operation(&before, after)
            .map_err(map_commit_error)?;
        Ok(failure_view(failure))
    }

    /// Clear retry backoff only after observable provisioning progress or completion.
    pub(crate) fn clear_failure(
        operation_id: [u8; 32],
        plan_hash: [u8; 32],
    ) -> Result<(), InternalError> {
        let before = RootComponentProvisioningStore::operation(operation_id)
            .ok_or_else(InternalError::unavailable)?;
        let view = validated_record(before.clone())?;
        if view.plan_hash != plan_hash {
            return Err(InternalError::conflict());
        }
        if before.last_failure.is_some() {
            let mut after = before.clone();
            after.last_failure = None;
            RootComponentProvisioningStore::replace_operation(&before, after)
                .map_err(map_commit_error)?;
        }
        Ok(())
    }

    /// Compare only durable work progress; failure timestamps and attempt counts are excluded.
    pub(crate) fn progress_changed(
        before: &RootComponentProvisioningView,
        after: &canic_core::dto::component_provisioning::RootComponentProvisioningStatusResponse,
    ) -> bool {
        let previous = ProvisioningProgress {
            phase: before.phase,
            reserved: before.reservation_cursor.reserved_component_count,
            claimed: before.claim_cursor.claimed_component_count,
            installed: before.install_cursor.installed_component_count,
            committed: before.registry_cursor.registry_committed_component_count,
            published: before.published_component_count,
            activated: before.activated_component_count,
            root_active: before.root_runtime_active,
        };
        let observed = ProvisioningProgress {
            phase: after.phase,
            reserved: after.reserved_component_count,
            claimed: after.claimed_component_count,
            installed: after.installed_component_count,
            committed: after.registry_committed_component_count,
            published: after.published_component_count,
            activated: after.activated_component_count,
            root_active: after.root_runtime_active,
        };
        previous != observed
    }
}

pub(super) const fn failure_view(
    record: RootComponentProvisioningFailureRecord,
) -> RootComponentProvisioningFailureView {
    RootComponentProvisioningFailureView {
        origin: ProvisioningFailureView {
            recorded_at_ns: Some(record.failed_at_ns),
            stage: record.stage,
            target: record.target,
            operation_id: record.operation_id,
            diagnostic_code: record.diagnostic_code,
            retry_category: record.retry_category,
        },
        failed_at_ns: record.failed_at_ns,
        consecutive_failures: record.consecutive_failures,
        retry_at_ns: record.retry_at_ns,
    }
}

pub(super) const fn failure_response(
    view: RootComponentProvisioningFailureView,
) -> RootComponentProvisioningFailure {
    RootComponentProvisioningFailure {
        stage: view.origin.stage,
        target: view.origin.target,
        operation_id: view.origin.operation_id,
        diagnostic_code: view.origin.diagnostic_code,
        retry_category: view.origin.retry_category,
        failed_at_ns: view.failed_at_ns,
        consecutive_failures: view.consecutive_failures,
        retry_at_ns: view.retry_at_ns,
    }
}

/// Validate the bounded diagnostic independently of canonical effect receipts.
pub(super) fn validate_failure(
    failure: Option<RootComponentProvisioningFailureRecord>,
    accepted_at_ns: u64,
) -> Result<(), InternalError> {
    let Some(failure) = failure else {
        return Ok(());
    };
    let valid_identity = failure.target != candid::Principal::anonymous()
        && failure.operation_id != [0; 32]
        && failure.diagnostic_code != 0;
    let valid_time = failure.failed_at_ns >= accepted_at_ns && failure.consecutive_failures > 0;
    let expected_retry = retry_delay_seconds(failure.retry_category, failure.consecutive_failures)
        .and_then(|seconds| seconds.checked_mul(1_000_000_000))
        .and_then(|delay| failure.failed_at_ns.checked_add(delay));
    if !valid_identity || !valid_time || failure.retry_at_ns != expected_retry {
        return Err(InternalError::invariant());
    }
    Ok(())
}
