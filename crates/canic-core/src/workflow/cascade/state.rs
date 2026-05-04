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
    InternalError, InternalErrorOrigin,
    dto::cascade::StateSnapshotInput,
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
            state::{app::AppStateOps, subnet::SubnetStateOps},
        },
    },
    workflow::{
        cascade::{
            snapshot::{
                StateSnapshot, adapter::StateSnapshotAdapter, state_snapshot_debug,
                state_snapshot_is_empty,
            },
            warn_if_large,
        },
        prelude::*,
    },
};

///
/// StateCascadeWorkflow
/// Orchestrates state snapshot propagation and local application.
///
pub struct StateCascadeWorkflow;

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

        let mut failures = 0;

        for (pid, _) in children {
            if let Err(err) = Self::send_snapshot(pid, snapshot).await {
                failures += 1;
                log!(
                    Topic::Sync,
                    Warn,
                    "sync.state: failed to cascade to {pid}: {err}",
                );
            }
        }

        let completion_reason = if failures > 0 {
            MetricReason::PartialFailure
        } else {
            MetricReason::Ok
        };
        CascadeMetrics::record(
            MetricOperation::RootFanout,
            MetricSnapshot::State,
            MetricOutcome::Completed,
            completion_reason,
        );

        if failures > 0 {
            log!(
                Topic::Sync,
                Warn,
                "sync.state: {failures} child cascade(s) failed; continuing"
            );
        }

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

        let mut failures = 0;

        for pid in child_pids {
            if let Err(err) = Self::send_snapshot(pid, &snapshot).await {
                failures += 1;
                log!(
                    Topic::Sync,
                    Warn,
                    "sync.state: failed to cascade to {pid}: {err}",
                );
            }
        }

        let completion_reason = if failures > 0 {
            MetricReason::PartialFailure
        } else {
            MetricReason::Ok
        };
        CascadeMetrics::record(
            MetricOperation::NonrootFanout,
            MetricSnapshot::State,
            MetricOutcome::Completed,
            completion_reason,
        );

        if failures > 0 {
            log!(
                Topic::Sync,
                Warn,
                "sync.state: {failures} child cascade(s) failed; continuing"
            );
        }

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

        if let Some(subnet) = snapshot.subnet_state.clone() {
            SubnetStateOps::import_input(subnet);
        }

        if let Some(index) = &snapshot.app_index {
            AppIndexOps::import_args_allow_incomplete(index.clone())?;
        }

        if let Some(index) = &snapshot.subnet_index {
            SubnetIndexOps::import_args_allow_incomplete(index.clone())?;
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
                Err(InternalError::workflow(
                    InternalErrorOrigin::Workflow,
                    format!("state cascade rejected by child {pid}: {err}"),
                ))
            }
        }
    }
}
