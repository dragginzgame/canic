//!
//! Topology cascade workflow.
//!
//! Coordinates propagation of topology snapshots from root to leaves.
//! Enforces cascade invariants and delegates transport to `CascadeOps`.

use crate::{
    InternalError, InternalErrorOrigin,
    dto::cascade::TopologySnapshotInput,
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
        storage::children::CanisterChildrenOps,
    },
    workflow::{
        cascade::{
            snapshot::{
                TopologyPathNode, TopologySnapshot, TopologySnapshotBuilder,
                adapter::TopologySnapshotAdapter,
            },
            warn_if_large,
        },
        prelude::*,
    },
};
use std::collections::HashMap;

///
/// TopologyCascadeWorkflow
/// Orchestrates topology snapshot propagation across the canister tree.
///
pub struct TopologyCascadeWorkflow;

impl TopologyCascadeWorkflow {
    // ───────────────────────── Root cascades ─────────────────────────

    /// Initiates a topology cascade from the root canister toward `target_pid`.
    pub async fn root_cascade_topology_for_pid(target_pid: Principal) -> Result<(), InternalError> {
        EnvOps::require_root()?;

        Self::record(
            MetricOperation::RootFanout,
            MetricOutcome::Started,
            MetricReason::Ok,
        );

        let snapshot = match TopologySnapshotBuilder::for_target(target_pid) {
            Ok(builder) => builder.build(),
            Err(err) => {
                Self::record(
                    MetricOperation::RootFanout,
                    MetricOutcome::Failed,
                    MetricReason::from_error(&err),
                );
                return Err(err);
            }
        };

        let root_pid = IcOps::canister_self();
        let first_child = match Self::next_child_on_path(root_pid, &snapshot.parents) {
            Ok(Some(first_child)) => first_child,
            Ok(None) => {
                Self::record(
                    MetricOperation::RouteResolve,
                    MetricOutcome::Skipped,
                    MetricReason::NoRoute,
                );
                Self::record(
                    MetricOperation::RootFanout,
                    MetricOutcome::Skipped,
                    MetricReason::NoRoute,
                );
                log!(
                    Topic::Sync,
                    Warn,
                    "sync.topology: no branch path to {target_pid}, skipping cascade"
                );
                return Ok(());
            }
            Err(err) => {
                Self::record(
                    MetricOperation::RouteResolve,
                    MetricOutcome::Failed,
                    MetricReason::from_error(&err),
                );
                Self::record(
                    MetricOperation::RootFanout,
                    MetricOutcome::Failed,
                    MetricReason::from_error(&err),
                );
                return Err(err);
            }
        };

        let child_snapshot = match Self::slice_snapshot_for_child(first_child, &snapshot) {
            Ok(snapshot) => snapshot,
            Err(err) => {
                Self::record(
                    MetricOperation::RouteResolve,
                    MetricOutcome::Failed,
                    MetricReason::from_error(&err),
                );
                Self::record(
                    MetricOperation::RootFanout,
                    MetricOutcome::Failed,
                    MetricReason::from_error(&err),
                );
                return Err(err);
            }
        };
        Self::record(
            MetricOperation::RouteResolve,
            MetricOutcome::Completed,
            MetricReason::Ok,
        );

        match Self::send_snapshot(&first_child, &child_snapshot).await {
            Ok(()) => {
                Self::record(
                    MetricOperation::RootFanout,
                    MetricOutcome::Completed,
                    MetricReason::Ok,
                );
                Ok(())
            }
            Err(err) => {
                Self::record(
                    MetricOperation::RootFanout,
                    MetricOutcome::Failed,
                    MetricReason::SendFailed,
                );
                Err(err)
            }
        }
    }

    // ──────────────────────── Non-root cascades ──────────────────────

    /// Continues a topology cascade on a non-root canister.
    pub async fn nonroot_cascade_topology(
        view: TopologySnapshotInput,
    ) -> Result<(), InternalError> {
        EnvOps::deny_root()?;

        let snapshot = TopologySnapshotAdapter::from_input(view);

        let self_pid = IcOps::canister_self();
        Self::record(
            MetricOperation::NonrootFanout,
            MetricOutcome::Started,
            MetricReason::Ok,
        );

        let next = match Self::next_child_on_path(self_pid, &snapshot.parents) {
            Ok(next) => next,
            Err(err) => {
                Self::record(
                    MetricOperation::RouteResolve,
                    MetricOutcome::Failed,
                    MetricReason::from_error(&err),
                );
                Self::record(
                    MetricOperation::NonrootFanout,
                    MetricOutcome::Failed,
                    MetricReason::from_error(&err),
                );
                return Err(err);
            }
        };

        let children = snapshot
            .children_map
            .get(&self_pid)
            .cloned()
            .unwrap_or_default();

        warn_if_large("nonroot fanout", children.len());

        Self::record(
            MetricOperation::LocalApply,
            MetricOutcome::Started,
            MetricReason::Ok,
        );

        let children_entries = children
            .into_iter()
            .map(|child| (child.pid, child.role))
            .collect();

        CanisterChildrenOps::import_direct_children(self_pid, children_entries);

        Self::record(
            MetricOperation::LocalApply,
            MetricOutcome::Completed,
            MetricReason::Ok,
        );

        if let Some(next_pid) = next {
            let next_snapshot = match Self::slice_snapshot_for_child(next_pid, &snapshot) {
                Ok(snapshot) => snapshot,
                Err(err) => {
                    Self::record(
                        MetricOperation::RouteResolve,
                        MetricOutcome::Failed,
                        MetricReason::from_error(&err),
                    );
                    Self::record(
                        MetricOperation::NonrootFanout,
                        MetricOutcome::Failed,
                        MetricReason::from_error(&err),
                    );
                    return Err(err);
                }
            };
            Self::record(
                MetricOperation::RouteResolve,
                MetricOutcome::Completed,
                MetricReason::Ok,
            );
            if let Err(err) = Self::send_snapshot(&next_pid, &next_snapshot).await {
                Self::record(
                    MetricOperation::NonrootFanout,
                    MetricOutcome::Failed,
                    MetricReason::SendFailed,
                );
                return Err(err);
            }
        } else {
            Self::record(
                MetricOperation::RouteResolve,
                MetricOutcome::Skipped,
                MetricReason::NoRoute,
            );
        }

        Self::record(
            MetricOperation::NonrootFanout,
            MetricOutcome::Completed,
            MetricReason::Ok,
        );

        Ok(())
    }

    // ───────────────────────── Internal helpers ──────────────────────

    // Record one topology cascade metric row using the fixed topology snapshot label.
    fn record(operation: MetricOperation, outcome: MetricOutcome, reason: MetricReason) {
        CascadeMetrics::record(operation, MetricSnapshot::Topology, outcome, reason);
    }

    // Send a topology snapshot to one child and record bounded transport outcome metrics.
    async fn send_snapshot(
        pid: &Principal,
        snapshot: &TopologySnapshot,
    ) -> Result<(), InternalError> {
        let view = TopologySnapshotAdapter::to_input(snapshot);

        Self::record(
            MetricOperation::ChildSend,
            MetricOutcome::Started,
            MetricReason::Ok,
        );

        match CascadeOps::send_topology_snapshot(*pid, &view).await {
            Ok(()) => {
                Self::record(
                    MetricOperation::ChildSend,
                    MetricOutcome::Completed,
                    MetricReason::Ok,
                );
                Ok(())
            }
            Err(err) => {
                Self::record(
                    MetricOperation::ChildSend,
                    MetricOutcome::Failed,
                    MetricReason::SendFailed,
                );
                Err(InternalError::workflow(
                    InternalErrorOrigin::Workflow,
                    format!("topology cascade rejected by child {pid}: {err}"),
                ))
            }
        }
    }

    // Resolve the next child hop from a topology parent chain rooted at this canister.
    fn next_child_on_path(
        self_pid: Principal,
        parents: &[TopologyPathNode],
    ) -> Result<Option<Principal>, InternalError> {
        let Some(first) = parents.first() else {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Workflow,
                "topology parent chain is empty",
            ));
        };

        if first.pid != self_pid {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Workflow,
                format!("topology parent chain does not start with self pid {self_pid}"),
            ));
        }

        Ok(parents.get(1).map(|p| p.pid))
    }

    // Slice a topology snapshot so the next child receives only its branch.
    fn slice_snapshot_for_child(
        next_pid: Principal,
        snapshot: &TopologySnapshot,
    ) -> Result<TopologySnapshot, InternalError> {
        let mut sliced_parents = Vec::new();
        let mut include = false;

        for parent in &snapshot.parents {
            if parent.pid == next_pid {
                include = true;
            }
            if include {
                sliced_parents.push(parent.clone());
            }
        }

        if sliced_parents.is_empty() {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Workflow,
                format!("topology next hop {next_pid} not found in parent chain"),
            ));
        }

        let mut sliced_children_map = HashMap::new();
        for parent in &sliced_parents {
            let children = snapshot
                .children_map
                .get(&parent.pid)
                .cloned()
                .unwrap_or_default();
            sliced_children_map.insert(parent.pid, children);
        }

        Ok(TopologySnapshot {
            parents: sliced_parents,
            children_map: sliced_children_map,
        })
    }
}
