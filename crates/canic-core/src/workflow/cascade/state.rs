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
            fleet_activation::FleetActivationRuntimeOps,
            metrics::cascade::{
                CascadeMetricOperation as MetricOperation, CascadeMetricOutcome as MetricOutcome,
                CascadeMetricReason as MetricReason, CascadeMetricSnapshot as MetricSnapshot,
                CascadeMetrics,
            },
        },
        storage::{
            children::CanisterChildrenOps, directory::fleet::FleetDirectoryOps,
            fleet_activation::FleetActivationOps, registry::subnet::SubnetRegistryOps,
            state::fleet::FleetStateOps,
        },
        topology::directory::validate_provenance,
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

    /// Cascade a state snapshot from the root canister to its direct children.
    ///
    /// No-op if the snapshot is empty.
    pub async fn root_cascade_state(snapshot: &StateSnapshot) -> Result<(), InternalError> {
        EnvOps::require_root()?;
        let root_pid = IcOps::canister_self();
        let children = SubnetRegistryOps::children(root_pid)
            .into_iter()
            .map(|entry| entry.pid)
            .collect::<Vec<_>>();
        Self::root_cascade_state_to(snapshot, &children).await
    }

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
        Self::apply_state_replacements(snapshot)?;
        if let Some(prepared) = activation_evidence {
            FleetActivationOps::commit_prepared_snapshot(prepared);
        }
        Ok(())
    }

    fn apply_state_replacements(snapshot: &StateSnapshot) -> Result<(), InternalError> {
        if let Some(directory) = &snapshot.fleet_directory {
            validate_provenance(&directory.provenance)?;
        }
        let fleet_directory = snapshot
            .fleet_directory
            .as_ref()
            .map(|directory| {
                let filtered = FleetDirectoryOps::filter_args_for_local_config(directory.clone())?;
                FleetDirectoryOps::prepare_args_allow_incomplete(filtered)
            })
            .transpose()?;
        if let Some(fleet) = snapshot.fleet_state {
            FleetStateOps::import_input(fleet);
        }

        if let Some(directory) = fleet_directory {
            FleetDirectoryOps::commit_prepared(directory);
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
mod state_apply_tests {
    use super::{StateCascadeWorkflow, StateSnapshot};
    use crate::{
        config::schema::CanisterKind,
        dto::{
            state::{FleetMode, FleetStateInput},
            topology::{DirectoryEntryInput, DirectoryProvenance, FleetDirectoryInput},
        },
        ids::{
            AppId, CanisterRole, CanonicalNetworkId, ComponentSpecId, FleetBinding, FleetId,
            FleetKey, ReleaseBuildId, ReleaseBuildNonce,
        },
        ops::storage::{
            directory::fleet::FleetDirectoryOps, fleet_activation::FleetActivationOps,
            state::fleet::FleetStateOps,
        },
        test::{
            config::ConfigTestBuilder,
            seams::{lock, p},
            support::import_test_env,
        },
    };

    #[test]
    fn state_apply_prepares_every_replacement_before_mutating_local_state() {
        let _guard = lock();
        let service = CanisterRole::new("service");
        let root = p(1);
        let original = p(2);
        let replacement = p(3);
        let fleet = FleetBinding {
            fleet: FleetKey {
                canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                fleet_id: FleetId::from_generated_bytes([4; 32]),
            },
            app: AppId::from("test"),
        };

        let _config = ConfigTestBuilder::new()
            .with_default_canister_kind(service.clone(), CanisterKind::Service)
            .install();
        import_test_env(
            service.clone(),
            ComponentSpecId::try_from(String::from("default")).expect("default Component Spec ID"),
            root,
        );

        FleetActivationOps::reset_for_tests();
        let release = ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([5; 32]));
        FleetActivationOps::initialize_nonroot_prepared(
            fleet.clone(),
            [6; 32],
            release,
            release,
            None,
            None,
        )
        .expect("initialize protected Fleet binding");

        let original_state = FleetStateInput {
            mode: FleetMode::Disabled,
            cycles_funding_enabled: false,
        };
        FleetStateOps::import_input(original_state);
        let provenance = DirectoryProvenance {
            fleet,
            source_root: root,
        };
        FleetDirectoryOps::import_args_allow_incomplete(FleetDirectoryInput {
            provenance: provenance.clone(),
            entries: vec![DirectoryEntryInput {
                role: service.clone(),
                pid: original,
            }],
        })
        .expect("seed Fleet Directory");
        let snapshot = StateSnapshot {
            fleet_state: Some(FleetStateInput {
                mode: FleetMode::Enabled,
                cycles_funding_enabled: true,
            }),
            fleet_directory: Some(FleetDirectoryInput {
                provenance,
                entries: vec![
                    DirectoryEntryInput {
                        role: service.clone(),
                        pid: replacement,
                    },
                    DirectoryEntryInput {
                        role: service.clone(),
                        pid: p(4),
                    },
                ],
            }),
        };

        StateCascadeWorkflow::apply_state_with_activation(&snapshot, Some([7; 32]))
            .expect_err("duplicate Fleet Directory role must reject the complete snapshot");

        assert_eq!(FleetStateOps::snapshot_input(), original_state);
        assert_eq!(FleetDirectoryOps::get(&service), Some(original));
        assert!(!FleetActivationOps::has_partial_snapshot_evidence_for_tests());
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
