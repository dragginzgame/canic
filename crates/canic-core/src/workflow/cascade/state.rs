//!
//! State cascade workflow.
//!
//! Coordinates propagation of internal state snapshots across the subnet topology.
//! Root canisters initiate cascades; non-root canisters apply and forward snapshots.
//!
//! Layering rules:
//! - Workflow operates on `StateSnapshot` (internal)
//! - `StateSnapshotInput` is used only for transport (RPC / API)
//! - Snapshot assembly lives in `workflow::cascade::snapshot`
//! - Persistence and mutation live in ops

use crate::{
    InternalError,
    cdk::types::Principal,
    dto::cascade::StateSnapshotInput,
    log,
    log::Topic,
    ops::{
        cascade::CascadeOps,
        runtime::{
            env::EnvOps,
            fleet_activation::FleetActivationRuntimeOps,
            metrics::cascade::{
                CascadeMetricOperation as MetricOperation, CascadeMetricOutcome as MetricOutcome,
                CascadeMetricReason as MetricReason, CascadeMetricSnapshot as MetricSnapshot,
                CascadeMetrics,
            },
        },
        storage::{
            children::CanisterChildrenOps, fleet_activation::FleetActivationOps,
            state::fleet::FleetStateOps,
        },
    },
    workflow::cascade::{
        snapshot::{
            StateSnapshot, adapter::StateSnapshotAdapter, state_snapshot_debug,
            state_snapshot_is_empty,
        },
        warn_if_large,
    },
};

///
/// StateCascadeWorkflow
/// Orchestrates state snapshot propagation and local application.
///
pub struct StateCascadeWorkflow;

#[derive(Default)]
struct FanoutFailures {
    count: usize,
    first: Option<InternalError>,
}

impl FanoutFailures {
    const fn push(&mut self, _pid: Principal, error: InternalError) {
        self.count += 1;
        self.first = Some(match self.first.take() {
            None => error,
            Some(first) => first,
        });
    }

    const fn into_error(self) -> Option<InternalError> {
        self.first
    }
}

fn prepared_state_snapshot_hash(
    view: &StateSnapshotInput,
) -> Result<Option<[u8; 32]>, InternalError> {
    if FleetActivationRuntimeOps::is_standalone_local() {
        return Ok(None);
    }
    crate::ops::fleet_activation::FleetActivationEvidenceOps::state_snapshot_hash(view).map(Some)
}

impl StateCascadeWorkflow {
    // ───────────────────────── Root cascade ─────────────────────────

    /// Cascade a state snapshot to one explicit root-owned direct-child inventory.
    pub(crate) async fn root_cascade_state_to(
        snapshot: &StateSnapshot,
        children: &[Principal],
    ) -> Result<(), InternalError> {
        EnvOps::require_root()?;

        if state_snapshot_is_empty(snapshot) {
            CascadeMetrics::record(
                MetricOperation::RootFanout,
                MetricSnapshot::State,
                MetricOutcome::Skipped,
                MetricReason::EmptySnapshot,
            );
            log!(
                Topic::Sync,
                Info,
                "sync.state: root cascade skipped (empty snapshot)"
            );
            return Ok(());
        }

        CascadeMetrics::record(
            MetricOperation::RootFanout,
            MetricSnapshot::State,
            MetricOutcome::Started,
            MetricReason::Ok,
        );

        log!(
            Topic::Sync,
            Info,
            "sync.state: root cascade start snapshot={}",
            state_snapshot_debug(snapshot)
        );

        warn_if_large("root state cascade", children.len());

        let mut failures = FanoutFailures::default();

        for &pid in children {
            if let Err(err) = Self::send_snapshot(pid, snapshot).await {
                log!(
                    Topic::Sync,
                    Warn,
                    "sync.state: failed to cascade to {pid}: {err}",
                );
                failures.push(pid, err);
            }
        }

        if failures.count > 0 {
            CascadeMetrics::record(
                MetricOperation::RootFanout,
                MetricSnapshot::State,
                MetricOutcome::Failed,
                MetricReason::PartialFailure,
            );
            log!(
                Topic::Sync,
                Warn,
                "sync.state: {} child cascade(s) failed",
                failures.count,
            );
            return Err(failures
                .into_error()
                .expect("positive failure count must retain first cause"));
        }

        CascadeMetrics::record(
            MetricOperation::RootFanout,
            MetricSnapshot::State,
            MetricOutcome::Completed,
            MetricReason::Ok,
        );
        Ok(())
    }

    // ──────────────────────── Non-root cascade ──────────────────────

    /// Handle a received state snapshot on a non-root canister:
    /// - apply it locally
    /// - forward it to direct children using the children cache
    pub async fn nonroot_cascade_state(view: StateSnapshotInput) -> Result<(), InternalError> {
        EnvOps::deny_root()?;
        let activation_hash = prepared_state_snapshot_hash(&view)?;

        let snapshot = StateSnapshotAdapter::from_input(view);

        if state_snapshot_is_empty(&snapshot) {
            CascadeMetrics::record(
                MetricOperation::NonrootFanout,
                MetricSnapshot::State,
                MetricOutcome::Skipped,
                MetricReason::EmptySnapshot,
            );
            log!(
                Topic::Sync,
                Info,
                "sync.state: non-root cascade skipped (empty snapshot)"
            );
            return Ok(());
        }

        CascadeMetrics::record(
            MetricOperation::NonrootFanout,
            MetricSnapshot::State,
            MetricOutcome::Started,
            MetricReason::Ok,
        );

        log!(
            Topic::Sync,
            Info,
            "sync.state: non-root cascade start snapshot={}",
            state_snapshot_debug(&snapshot)
        );

        // Apply locally before forwarding.
        CascadeMetrics::record(
            MetricOperation::LocalApply,
            MetricSnapshot::State,
            MetricOutcome::Started,
            MetricReason::Ok,
        );
        if let Err(err) = Self::apply_state_with_activation(&snapshot, activation_hash) {
            CascadeMetrics::record(
                MetricOperation::LocalApply,
                MetricSnapshot::State,
                MetricOutcome::Failed,
                MetricReason::from_error(&err),
            );
            CascadeMetrics::record(
                MetricOperation::NonrootFanout,
                MetricSnapshot::State,
                MetricOutcome::Failed,
                MetricReason::from_error(&err),
            );
            return Err(err);
        }
        CascadeMetrics::record(
            MetricOperation::LocalApply,
            MetricSnapshot::State,
            MetricOutcome::Completed,
            MetricReason::Ok,
        );
        // Cascade using children cache only (never registry).
        let child_pids = CanisterChildrenOps::pids();
        warn_if_large("non-root state cascade", child_pids.len());

        let mut failures = FanoutFailures::default();

        for pid in child_pids {
            if let Err(err) = Self::send_snapshot(pid, &snapshot).await {
                log!(
                    Topic::Sync,
                    Warn,
                    "sync.state: failed to cascade to {pid}: {err}",
                );
                failures.push(pid, err);
            }
        }

        if failures.count > 0 {
            CascadeMetrics::record(
                MetricOperation::NonrootFanout,
                MetricSnapshot::State,
                MetricOutcome::Failed,
                MetricReason::PartialFailure,
            );
            log!(
                Topic::Sync,
                Warn,
                "sync.state: {} child cascade(s) failed",
                failures.count,
            );
            return Err(failures
                .into_error()
                .expect("positive failure count must retain first cause"));
        }

        CascadeMetrics::record(
            MetricOperation::NonrootFanout,
            MetricSnapshot::State,
            MetricOutcome::Completed,
            MetricReason::Ok,
        );
        Ok(())
    }

    // ─────────────────────── Local application ──────────────────────

    /// Prepare and apply one received non-root snapshot with exact activation evidence.
    fn apply_state_with_activation(
        snapshot: &StateSnapshot,
        activation_hash: Option<[u8; 32]>,
    ) -> Result<(), InternalError> {
        let activation_evidence = activation_hash
            .map(FleetActivationOps::prepare_applied_state_snapshot)
            .transpose()
            .map_err(crate::ops::storage::StorageOpsError::from)?;
        Self::apply_state_replacements(snapshot);
        if let Some(prepared) = activation_evidence {
            FleetActivationOps::commit_prepared_snapshot(prepared);
        }
        Ok(())
    }

    fn apply_state_replacements(snapshot: &StateSnapshot) {
        if let Some(fleet) = snapshot.fleet_state {
            FleetStateOps::import_input(fleet);
        }
    }

    // ───────────────────────── Transport ────────────────────────────

    /// Send a state snapshot to another canister.
    ///
    /// Converts internal snapshot → DTO exactly once.
    async fn send_snapshot(pid: Principal, snapshot: &StateSnapshot) -> Result<(), InternalError> {
        let view = StateSnapshotAdapter::to_input(snapshot);

        CascadeMetrics::record(
            MetricOperation::ChildSend,
            MetricSnapshot::State,
            MetricOutcome::Started,
            MetricReason::Ok,
        );

        match CascadeOps::send_state_snapshot(pid, &view).await {
            Ok(()) => {
                CascadeMetrics::record(
                    MetricOperation::ChildSend,
                    MetricSnapshot::State,
                    MetricOutcome::Completed,
                    MetricReason::Ok,
                );
                Ok(())
            }
            Err(err) => {
                CascadeMetrics::record(
                    MetricOperation::ChildSend,
                    MetricSnapshot::State,
                    MetricOutcome::Failed,
                    MetricReason::SendFailed,
                );
                Err(err)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fanout_failures_preserve_first_typed_cause() {
        let mut failures = FanoutFailures::default();
        failures.push(
            Principal::from_slice(&[1; 29]),
            InternalError::auth_material_stale(),
        );
        failures.push(
            Principal::from_slice(&[2; 29]),
            InternalError::lifecycle_failure(),
        );

        let err = failures.into_error().expect("failure must be retained");
        assert_eq!(err.code(), crate::diagnostics::codes::SECURITY_CONFLICT);
        assert_eq!(
            err.public_error().code(),
            crate::diagnostics::codes::SECURITY_CONFLICT.raw_code()
        );
    }
}
