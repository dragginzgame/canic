use super::{BackupCommandError, BackupCreateOptions, BackupCreateReport};
use crate::{
    backup::labels::backup_scope_label,
    support::path_stamp::{current_backup_directory_stamp, file_safe_component},
};
use candid::Principal;
use canic_backup::{
    execution::BackupExecutionJournal,
    manifest::IdentityMode,
    persistence::BackupLayout,
    plan::{
        AuthorityEvidence, AuthorityProofSource, BackupExecutionPreflightReceipts, BackupPlan,
        BackupPlanBuildInput, BackupScopeKind, ControlAuthority, ControlAuthorityReceipt,
        QuiescencePreflightReceipt, QuiescencePreflightTarget, SnapshotReadAuthority,
        SnapshotReadAuthorityReceipt, TopologyPreflightReceipt, TopologyPreflightTarget,
        build_backup_plan, resolve_backup_selector,
    },
    registry::RegistryEntry as BackupRegistryEntry,
    runner::{
        BackupRunResponse, BackupRunnerCommandError, BackupRunnerConfig, BackupRunnerExecutor,
        BackupRunnerSnapshotReceipt, backup_run_execute_with_executor,
    },
    topology::{TopologyHasher, TopologyRecord},
};
use canic_host::{
    icp::{IcpCli, IcpCommandError},
    icp_config::resolve_current_canic_icp_root,
    installed_fleet::{
        InstalledFleetError, InstalledFleetRequest, resolve_installed_fleet_from_root,
    },
    registry::{RegistryEntry as HostRegistryEntry, parse_registry_entries},
    replica_query,
};
use std::path::{Path, PathBuf};

pub(super) fn backup_create(
    options: &BackupCreateOptions,
) -> Result<BackupCreateReport, BackupCommandError> {
    let icp_root = resolve_current_canic_icp_root(None)
        .map_err(|err| BackupCommandError::InstallState(err.to_string()))?;
    let installed = resolve_installed_fleet_from_root(
        &InstalledFleetRequest {
            fleet: options.fleet.clone(),
            network: options.network.clone(),
            icp: options.icp.clone(),
            detect_lost_local_root: true,
        },
        &icp_root,
    )
    .map_err(backup_installed_fleet_error)?;
    let registry = backup_registry_entries(&installed.registry.entries);
    let topology_hash = registry_topology_hash(&registry)?;
    let plan_id = backup_plan_id(&options.fleet);
    let run_id = plan_id.replace("plan-", "run-");
    let out = options
        .out
        .clone()
        .unwrap_or_else(|| default_backup_output_path(&options.fleet));
    let selected_canister_id = options
        .subtree
        .as_deref()
        .map(|selector| resolve_backup_selector(&registry, selector))
        .transpose()?;
    let selected_scope_kind = if selected_canister_id.is_some() {
        BackupScopeKind::Subtree
    } else {
        BackupScopeKind::NonRootFleet
    };
    let plan = build_backup_plan(BackupPlanBuildInput {
        plan_id,
        run_id,
        fleet: options.fleet.clone(),
        network: options.network.clone(),
        root_canister_id: installed.state.root_canister_id,
        selected_canister_id,
        selected_scope_kind,
        include_descendants: true,
        topology_hash_before_quiesce: topology_hash,
        registry: &registry,
        control_authority: backup_control_authority(options.dry_run),
        snapshot_read_authority: backup_snapshot_read_authority(options.dry_run),
        quiescence_policy: backup_quiescence_policy(options.dry_run),
        identity_mode: IdentityMode::Relocatable,
    })?;
    persist_backup_create_layout(&out, &plan)?;

    let run = if options.dry_run {
        None
    } else {
        let mut executor = BackupIcpRunnerExecutor::new(options, icp_root);
        Some(backup_run_execute_with_executor(
            &BackupRunnerConfig {
                out: out.clone(),
                max_steps: None,
                updated_at: None,
                tool_name: "canic".to_string(),
                tool_version: env!("CARGO_PKG_VERSION").to_string(),
            },
            &mut executor,
        )?)
    };

    Ok(BackupCreateReport {
        fleet: plan.fleet.clone(),
        network: plan.network.clone(),
        out,
        plan_id: plan.plan_id.clone(),
        run_id: plan.run_id.clone(),
        mode: if options.dry_run {
            "dry-run"
        } else {
            "execute"
        }
        .to_string(),
        status: run
            .as_ref()
            .map_or_else(|| "planned".to_string(), backup_run_status),
        scope: backup_scope_label(&plan),
        targets: plan.targets.len(),
        operations: plan.phases.len(),
        executed_operations: run.as_ref().map_or(0, |run| run.executed_operation_count),
    })
}

#[cfg(test)]
pub(super) fn persist_backup_create_dry_run(
    out: &Path,
    plan: &BackupPlan,
) -> Result<(), BackupCommandError> {
    persist_backup_create_layout(out, plan)
}

fn persist_backup_create_layout(out: &Path, plan: &BackupPlan) -> Result<(), BackupCommandError> {
    let journal = BackupExecutionJournal::from_plan(plan)?;
    let layout = BackupLayout::new(out.to_path_buf());
    layout.write_backup_plan(plan)?;
    layout.write_execution_journal(&journal)?;
    layout.verify_execution_integrity()?;
    Ok(())
}

const fn backup_control_authority(dry_run: bool) -> ControlAuthority {
    if dry_run {
        ControlAuthority::root_controller(AuthorityEvidence::Declared)
    } else {
        ControlAuthority::operator_controller(AuthorityEvidence::Proven)
    }
}

const fn backup_snapshot_read_authority(dry_run: bool) -> SnapshotReadAuthority {
    if dry_run {
        SnapshotReadAuthority::root_configured_read(AuthorityEvidence::Declared)
    } else {
        SnapshotReadAuthority::operator_controller(AuthorityEvidence::Proven)
    }
}

const fn backup_quiescence_policy(dry_run: bool) -> canic_backup::plan::QuiescencePolicy {
    if dry_run {
        canic_backup::plan::QuiescencePolicy::RootCoordinated
    } else {
        canic_backup::plan::QuiescencePolicy::CrashConsistent
    }
}

fn backup_run_status(run: &BackupRunResponse) -> String {
    if run.complete {
        "complete"
    } else if run.max_steps_reached {
        "paused"
    } else {
        "running"
    }
    .to_string()
}

///
/// BackupIcpRunnerExecutor
///

struct BackupIcpRunnerExecutor {
    options: BackupCreateOptions,
    icp_root: PathBuf,
    icp: IcpCli,
}

impl BackupIcpRunnerExecutor {
    fn new(options: &BackupCreateOptions, icp_root: PathBuf) -> Self {
        Self {
            options: options.clone(),
            icp: IcpCli::new(&options.icp, None, Some(options.network.clone())).with_cwd(&icp_root),
            icp_root,
        }
    }
}

impl BackupRunnerExecutor for BackupIcpRunnerExecutor {
    fn preflight_receipts(
        &mut self,
        plan: &BackupPlan,
        preflight_id: &str,
        validated_at: &str,
        expires_at: &str,
    ) -> Result<BackupExecutionPreflightReceipts, BackupRunnerCommandError> {
        let registry_json =
            call_subnet_registry(&self.options, &self.icp_root, &plan.root_canister_id)
                .map_err(preflight_error)?;
        let host_registry = parse_registry_entries(&registry_json).map_err(preflight_error)?;
        let registry = backup_registry_entries(&host_registry);
        let topology_hash = registry_topology_hash(&registry).map_err(preflight_error)?;
        for target in &plan.targets {
            let status = self
                .icp
                .canister_status_report(&target.canister_id)
                .map_err(runner_icp_error)?;
            if status
                .settings
                .as_ref()
                .is_none_or(|settings| settings.controllers.is_empty())
            {
                return Err(BackupRunnerCommandError::failed(
                    "preflight",
                    format!(
                        "icp canister status --json for {} did not include controllers",
                        target.canister_id
                    ),
                ));
            }
        }

        Ok(BackupExecutionPreflightReceipts {
            plan_id: plan.plan_id.clone(),
            preflight_id: preflight_id.to_string(),
            validated_at: validated_at.to_string(),
            expires_at: expires_at.to_string(),
            topology: TopologyPreflightReceipt {
                plan_id: plan.plan_id.clone(),
                preflight_id: preflight_id.to_string(),
                topology_hash_before_quiesce: plan.topology_hash_before_quiesce.clone(),
                topology_hash_at_preflight: topology_hash,
                targets: plan
                    .targets
                    .iter()
                    .map(TopologyPreflightTarget::from)
                    .collect(),
                validated_at: validated_at.to_string(),
                expires_at: expires_at.to_string(),
                message: Some("root registry matched planned topology".to_string()),
            },
            control_authority: plan
                .targets
                .iter()
                .map(|target| ControlAuthorityReceipt {
                    plan_id: plan.plan_id.clone(),
                    preflight_id: preflight_id.to_string(),
                    target_canister_id: target.canister_id.clone(),
                    authority: ControlAuthority::operator_controller(AuthorityEvidence::Proven),
                    proof_source: AuthorityProofSource::ManagementStatus,
                    validated_at: validated_at.to_string(),
                    expires_at: expires_at.to_string(),
                    message: Some(
                        "icp canister status --json proved controller status access".to_string(),
                    ),
                })
                .collect(),
            snapshot_read_authority: plan
                .targets
                .iter()
                .map(|target| SnapshotReadAuthorityReceipt {
                    plan_id: plan.plan_id.clone(),
                    preflight_id: preflight_id.to_string(),
                    target_canister_id: target.canister_id.clone(),
                    authority: SnapshotReadAuthority::operator_controller(
                        AuthorityEvidence::Proven,
                    ),
                    proof_source: AuthorityProofSource::ManagementStatus,
                    validated_at: validated_at.to_string(),
                    expires_at: expires_at.to_string(),
                    message: Some("operator control permits snapshot read".to_string()),
                })
                .collect(),
            quiescence: QuiescencePreflightReceipt {
                plan_id: plan.plan_id.clone(),
                preflight_id: preflight_id.to_string(),
                quiescence_policy: plan.quiescence_policy.clone(),
                accepted: true,
                targets: plan
                    .targets
                    .iter()
                    .map(QuiescencePreflightTarget::from)
                    .collect(),
                validated_at: validated_at.to_string(),
                expires_at: expires_at.to_string(),
                message: Some("crash-consistent operator backup accepted".to_string()),
            },
        })
    }

    fn stop_canister(&mut self, canister_id: &str) -> Result<(), BackupRunnerCommandError> {
        self.icp
            .stop_canister(canister_id)
            .map_err(runner_icp_error)
    }

    fn start_canister(&mut self, canister_id: &str) -> Result<(), BackupRunnerCommandError> {
        self.icp
            .start_canister(canister_id)
            .map_err(runner_icp_error)
    }

    fn create_snapshot(
        &mut self,
        canister_id: &str,
    ) -> Result<BackupRunnerSnapshotReceipt, BackupRunnerCommandError> {
        self.icp
            .snapshot_create_receipt(canister_id)
            .map(|receipt| BackupRunnerSnapshotReceipt {
                snapshot_id: receipt.snapshot_id,
                taken_at_timestamp: receipt.taken_at_timestamp,
                total_size_bytes: receipt.total_size_bytes,
            })
            .map_err(runner_icp_error)
    }

    fn download_snapshot(
        &mut self,
        canister_id: &str,
        snapshot_id: &str,
        artifact_path: &Path,
    ) -> Result<(), BackupRunnerCommandError> {
        self.icp
            .snapshot_download(canister_id, snapshot_id, artifact_path)
            .map_err(runner_icp_error)
    }
}

fn preflight_error(error: impl std::error::Error) -> BackupRunnerCommandError {
    BackupRunnerCommandError::failed("preflight", error.to_string())
}

fn runner_icp_error(error: IcpCommandError) -> BackupRunnerCommandError {
    BackupRunnerCommandError::failed("icp", error.to_string())
}

fn backup_installed_fleet_error(error: InstalledFleetError) -> BackupCommandError {
    match error {
        InstalledFleetError::NoInstalledFleet { network, fleet } => {
            BackupCommandError::NoInstalledFleet { network, fleet }
        }
        InstalledFleetError::InstallState(error) => BackupCommandError::InstallState(error),
        InstalledFleetError::ReplicaQuery(error) => BackupCommandError::ReplicaQuery(error),
        InstalledFleetError::IcpFailed { command, stderr } => {
            BackupCommandError::IcpFailed { command, stderr }
        }
        InstalledFleetError::LostLocalFleet {
            network,
            fleet,
            root,
        } => BackupCommandError::LostLocalFleet {
            network,
            fleet,
            root,
        },
        InstalledFleetError::Registry(error) => BackupCommandError::Registry(error),
        InstalledFleetError::Io(error) => BackupCommandError::Io(error),
    }
}

fn call_subnet_registry(
    options: &BackupCreateOptions,
    icp_root: &Path,
    root: &str,
) -> Result<String, BackupCommandError> {
    if replica_query::should_use_local_replica_query(Some(&options.network)) {
        return replica_query::query_subnet_registry_json_from_root(
            Some(&options.network),
            root,
            icp_root,
        )
        .map_err(|err| BackupCommandError::ReplicaQuery(err.to_string()));
    }

    IcpCli::new(&options.icp, None, Some(options.network.clone()))
        .with_cwd(icp_root)
        .canister_call_output(root, "canic_subnet_registry", Some("json"))
        .map_err(backup_icp_error)
}

fn backup_icp_error(error: IcpCommandError) -> BackupCommandError {
    match error {
        IcpCommandError::Io(err) => BackupCommandError::Io(err),
        IcpCommandError::Failed { command, stderr } => {
            BackupCommandError::IcpFailed { command, stderr }
        }
        IcpCommandError::Json {
            command, output, ..
        } => BackupCommandError::IcpFailed {
            command,
            stderr: output,
        },
        IcpCommandError::SnapshotIdUnavailable { output } => BackupCommandError::IcpFailed {
            command: "icp canister snapshot create".to_string(),
            stderr: output,
        },
    }
}

fn backup_registry_entries(entries: &[HostRegistryEntry]) -> Vec<BackupRegistryEntry> {
    entries
        .iter()
        .map(|entry| BackupRegistryEntry {
            pid: entry.pid.clone(),
            role: entry.role.clone(),
            kind: entry.kind.clone(),
            parent_pid: entry.parent_pid.clone(),
            module_hash: entry.module_hash.clone(),
        })
        .collect()
}

fn registry_topology_hash(registry: &[BackupRegistryEntry]) -> Result<String, BackupCommandError> {
    let records = registry
        .iter()
        .map(|entry| {
            Ok(TopologyRecord {
                pid: Principal::from_text(&entry.pid).map_err(|_| {
                    BackupCommandError::InvalidRegistryPrincipal {
                        canister_id: entry.pid.clone(),
                    }
                })?,
                parent_pid: entry
                    .parent_pid
                    .as_deref()
                    .map(Principal::from_text)
                    .transpose()
                    .map_err(|_| BackupCommandError::InvalidRegistryPrincipal {
                        canister_id: entry.parent_pid.clone().unwrap_or_default(),
                    })?,
                role: entry.role.clone().unwrap_or_default(),
                module_hash: entry.module_hash.clone(),
            })
        })
        .collect::<Result<Vec<_>, BackupCommandError>>()?;

    Ok(TopologyHasher::hash(&records).hash)
}

fn backup_plan_id(fleet: &str) -> String {
    format!(
        "plan-{}-{}",
        file_safe_component(fleet),
        current_backup_directory_stamp()
    )
}

fn default_backup_output_path(fleet: &str) -> PathBuf {
    PathBuf::from("backups").join(format!(
        "fleet-{}-{}",
        file_safe_component(fleet),
        current_backup_directory_stamp()
    ))
}
