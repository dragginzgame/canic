//! Module: workflow::cascade::state
//!
//! Responsibility: apply state snapshots and collect bounded downstream outcomes.
//! Does not own: endpoint authorization, stable records or RPC serialization.
//! Boundary: exact parent authority precedes local application and role-owned fanout.

use crate::{
    InternalError,
    dto::{
        cascade::{StateCascadeReport, StateSnapshotInput},
        fleet_activation::FleetActivationPhase,
    },
    ids::CanisterRole,
    log,
    log::Topic,
    ops::{
        cascade::CascadeOps,
        cascade_report::StateCascadeReportOps,
        ic::IcOps,
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
    view::state_cascade::{StateCascadeEndpoint, StateCascadeTarget},
    workflow::{
        cascade::{
            snapshot::{StateSnapshot, adapter::StateSnapshotAdapter, state_snapshot_is_empty},
            warn_if_large,
        },
        runtime::cycles::CycleWorkflow,
    },
};

/// State application and fanout through each recipient's current command contract.
pub struct StateCascadeWorkflow;

fn prepared_state_snapshot_hash(
    view: &StateSnapshotInput,
) -> Result<Option<[u8; 32]>, InternalError> {
    if FleetActivationRuntimeOps::is_standalone_local() {
        return Ok(None);
    }
    crate::ops::fleet_activation::FleetActivationEvidenceOps::state_snapshot_hash(view).map(Some)
}

impl StateCascadeWorkflow {
    /// Root targets retain their inventory-derived role through transport.
    pub(crate) async fn root_cascade_state_to(
        snapshot: &StateSnapshot,
        children: &[StateCascadeTarget],
    ) -> Result<StateCascadeReport, InternalError> {
        EnvOps::require_root()?;
        Self::fanout(
            snapshot,
            children,
            StateCascadeReport::default(),
            MetricOperation::RootFanout,
        )
        .await
    }

    /// Apply locally, reconcile the local funding owner and report each subtree.
    pub async fn nonroot_cascade_state(
        view: StateSnapshotInput,
    ) -> Result<StateCascadeReport, InternalError> {
        EnvOps::deny_root()?;
        let activation_hash = prepared_state_snapshot_hash(&view)?;
        let snapshot = StateSnapshotAdapter::from_input(view);
        Self::apply_state_with_activation(&snapshot, activation_hash)?;
        let reconciliation_error = Self::reconcile_funding(&snapshot).err().map(Into::into);
        let report = StateCascadeReportOps::applied(IcOps::canister_self(), reconciliation_error);
        let children = CanisterChildrenOps::pids()
            .into_iter()
            .map(|canister_id| {
                let endpoint = match CanisterChildrenOps::role_parent(canister_id) {
                    Some((role, _)) if role == CanisterRole::WASM_STORE => {
                        StateCascadeEndpoint::Store
                    }
                    _ => StateCascadeEndpoint::Component,
                };
                StateCascadeTarget {
                    canister_id,
                    endpoint,
                }
            })
            .collect::<Vec<_>>();
        Self::fanout(&snapshot, &children, report, MetricOperation::NonrootFanout).await
    }

    fn reconcile_funding(snapshot: &StateSnapshot) -> Result<(), InternalError> {
        if snapshot.fleet_state.is_none() {
            return Ok(());
        }
        let active = if FleetActivationRuntimeOps::is_standalone_local() {
            true
        } else {
            FleetActivationOps::status(false)
                .map_err(crate::ops::storage::StorageOpsError::from)?
                .phase
                == FleetActivationPhase::Active
        };
        if active {
            CycleWorkflow::start()?;
        }
        Ok(())
    }

    async fn fanout(
        snapshot: &StateSnapshot,
        children: &[StateCascadeTarget],
        mut report: StateCascadeReport,
        operation: MetricOperation,
    ) -> Result<StateCascadeReport, InternalError> {
        if state_snapshot_is_empty(snapshot) {
            CascadeMetrics::record(
                operation,
                MetricSnapshot::State,
                MetricOutcome::Skipped,
                MetricReason::EmptySnapshot,
            );
            return Ok(report);
        }
        CascadeMetrics::record(
            operation,
            MetricSnapshot::State,
            MetricOutcome::Started,
            MetricReason::Ok,
        );
        warn_if_large("state cascade", children.len());
        let view = StateSnapshotAdapter::to_input(snapshot);
        for &target in children {
            // A newer local command must not cause the older invocation to send
            // its obsolete snapshot to additional children after an await.
            let result = if snapshot.fleet_state == Some(FleetStateOps::snapshot_input()) {
                Self::send_snapshot(target, &view).await
            } else {
                Err(InternalError::conflict())
            };
            let result =
                result.and_then(|incoming| StateCascadeReportOps::merge(&mut report, incoming));
            if let Err(error) = result {
                log!(
                    Topic::Sync,
                    Warn,
                    "sync.state: target {} remains unconfirmed: {error}",
                    target.canister_id
                );
                StateCascadeReportOps::merge(
                    &mut report,
                    StateCascadeReportOps::unconfirmed(target.canister_id, error),
                )?;
            }
        }
        let complete = StateCascadeReportOps::require_complete(&report).is_ok();
        CascadeMetrics::record(
            operation,
            MetricSnapshot::State,
            if complete {
                MetricOutcome::Completed
            } else {
                MetricOutcome::Failed
            },
            if complete {
                MetricReason::Ok
            } else {
                MetricReason::PartialFailure
            },
        );
        Ok(report)
    }

    fn apply_state_with_activation(
        snapshot: &StateSnapshot,
        activation_hash: Option<[u8; 32]>,
    ) -> Result<(), InternalError> {
        let activation_evidence = activation_hash
            .map(FleetActivationOps::prepare_applied_state_snapshot)
            .transpose()
            .map_err(crate::ops::storage::StorageOpsError::from)?;
        if let Some(fleet) = snapshot.fleet_state {
            FleetStateOps::import_input(fleet);
        }
        if let Some(prepared) = activation_evidence {
            FleetActivationOps::commit_prepared_snapshot(prepared);
        }
        CascadeMetrics::record(
            MetricOperation::LocalApply,
            MetricSnapshot::State,
            MetricOutcome::Completed,
            MetricReason::Ok,
        );
        Ok(())
    }

    async fn send_snapshot(
        target: StateCascadeTarget,
        view: &StateSnapshotInput,
    ) -> Result<StateCascadeReport, InternalError> {
        CascadeMetrics::record(
            MetricOperation::ChildSend,
            MetricSnapshot::State,
            MetricOutcome::Started,
            MetricReason::Ok,
        );
        let result = CascadeOps::send_state_snapshot(target, view).await;
        let complete = result
            .as_ref()
            .is_ok_and(|report| StateCascadeReportOps::require_complete(report).is_ok());
        CascadeMetrics::record(
            MetricOperation::ChildSend,
            MetricSnapshot::State,
            if complete {
                MetricOutcome::Completed
            } else {
                MetricOutcome::Failed
            },
            if complete {
                MetricReason::Ok
            } else {
                MetricReason::PartialFailure
            },
        );
        result
    }
}
