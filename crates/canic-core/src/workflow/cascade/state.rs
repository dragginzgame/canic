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
        ic::IcOps,
        runtime::{
            env::EnvOps,
            metrics::cascade::{
                CascadeMetricOperation as MetricOperation, CascadeMetricOutcome as MetricOutcome,
                CascadeMetricReason as MetricReason, CascadeMetricSnapshot as MetricSnapshot,
                CascadeMetrics,
            },
        },
        storage::{
            children::CanisterChildrenOps,
            index::{app::AppIndexOps, subnet::SubnetIndexOps},
            registry::subnet::SubnetRegistryOps,
            state::app::AppStateOps,
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
    fn push(&mut self, pid: Principal, error: InternalError) {
        self.count += 1;
        self.first = Some(match self.first.take() {
            None => error.with_diagnostic_context(format!("state cascade child {pid} failed")),
            Some(first) => first.with_diagnostic_context(format!(
                "additional state cascade child {pid} failure: {error}"
            )),
        });
    }

    fn into_error(self) -> Option<InternalError> {
        self.first
    }
}

impl StateCascadeWorkflow {
    // ───────────────────────── Root cascade ─────────────────────────

    /// Cascade a state snapshot from the root canister to its direct children.
    ///
    /// No-op if the snapshot is empty.
    pub async fn root_cascade_state(snapshot: &StateSnapshot) -> Result<(), InternalError> {
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

        let root_pid = IcOps::canister_self();
        let children = SubnetRegistryOps::children(root_pid);
        warn_if_large("root state cascade", children.len());

        let mut failures = FanoutFailures::default();

        for entry in children {
            let pid = entry.pid;
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
        if let Err(err) = Self::apply_state(&snapshot) {
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

    /// Apply a received state snapshot locally.
    ///
    /// Valid only on non-root canisters.
    fn apply_state(snapshot: &StateSnapshot) -> Result<(), InternalError> {
        EnvOps::deny_root()?;

        if let Some(app) = snapshot.app_state {
            AppStateOps::import_input(app);
        }

        if let Some(index) = &snapshot.app_index {
            let filtered = AppIndexOps::filter_args_for_local_config(index.clone())?;
            AppIndexOps::import_args_allow_incomplete(filtered)?;
        }

        if let Some(index) = &snapshot.subnet_index {
            let filtered = SubnetIndexOps::filter_args_for_local_config(index.clone())?;
            SubnetIndexOps::import_args_allow_incomplete(filtered)?;
        }

        Ok(())
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
                Err(err.with_diagnostic_context(format!("state cascade rejected by child {pid}")))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InternalErrorOrigin, dto::error::ErrorCode};

    #[test]
    fn fanout_failures_preserve_first_typed_cause() {
        let mut failures = FanoutFailures::default();
        failures.push(
            Principal::from_slice(&[1; 29]),
            InternalError::auth_material_stale("child auth state is stale"),
        );
        failures.push(
            Principal::from_slice(&[2; 29]),
            InternalError::workflow(InternalErrorOrigin::Workflow, "transport failed"),
        );

        let err = failures.into_error().expect("failure must be retained");
        assert_eq!(err.class(), crate::InternalErrorClass::Domain);
        assert_eq!(err.origin(), InternalErrorOrigin::Domain);
        assert_eq!(
            err.public_error().map(|public| public.code),
            Some(ErrorCode::AuthMaterialStale)
        );
    }
}
