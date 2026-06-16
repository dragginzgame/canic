use super::{BackupCommandError, BackupCreateOptions, BackupCreateReport};
use crate::backup::labels::backup_scope_label;
#[cfg(test)]
use canic_backup::plan::BackupPlan;
use canic_backup::{
    manifest::IdentityMode,
    plan::{BackupPlanBuildInput, BackupScopeKind, build_backup_plan, resolve_backup_selector},
    runner::{BackupRunResponse, BackupRunnerConfig, backup_run_execute_with_executor},
};
use canic_host::{
    icp_config::resolve_current_canic_icp_root,
    installed_deployment::{
        InstalledDeploymentError, InstalledDeploymentRequest,
        resolve_installed_deployment_from_root,
    },
};
#[cfg(test)]
use std::path::Path;

mod executor;
mod persistence;
mod plan;

use executor::BackupIcpRunnerExecutor;
use persistence::persist_backup_create_layout;
use plan::{
    backup_control_authority, backup_plan_id, backup_quiescence_policy, backup_registry_entries,
    backup_snapshot_read_authority, default_backup_output_path, registry_topology_hash,
};

pub(super) fn backup_create(
    options: &BackupCreateOptions,
) -> Result<BackupCreateReport, BackupCommandError> {
    let icp_root = resolve_current_canic_icp_root()
        .map_err(|err| BackupCommandError::InstallState(err.to_string()))?;
    let installed = resolve_installed_deployment_from_root(
        &InstalledDeploymentRequest {
            deployment: options.deployment.clone(),
            network: options.network.clone(),
            icp: options.icp.clone(),
            detect_lost_local_root: true,
        },
        &icp_root,
    )
    .map_err(backup_installed_deployment_error)?;
    let registry = backup_registry_entries(&installed.registry.entries);
    let topology_hash = registry_topology_hash(&registry)?;
    let plan_id = backup_plan_id(&options.deployment);
    let run_id = plan_id.replace("plan-", "run-");
    let out = options
        .out
        .clone()
        .unwrap_or_else(|| default_backup_output_path(&options.deployment));
    let selected_canister_id = options
        .subtree
        .as_deref()
        .map(|selector| resolve_backup_selector(&registry, selector))
        .transpose()?;
    let selected_scope_kind = if selected_canister_id.is_some() {
        BackupScopeKind::Subtree
    } else {
        BackupScopeKind::NonRootDeployment
    };
    let planned = build_backup_plan(BackupPlanBuildInput {
        plan_id,
        run_id,
        fleet: options.deployment.clone(),
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
    let persisted = persist_backup_create_layout(&out, &planned)?;
    let layout = if persisted.reused_existing {
        "existing"
    } else {
        "new"
    }
    .to_string();
    let plan = persisted.plan;

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
        deployment: plan.fleet.clone(),
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
        layout,
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
) -> Result<BackupPlan, BackupCommandError> {
    persist_backup_create_layout(out, plan).map(|layout| layout.plan)
}

#[cfg(test)]
pub(super) fn persist_backup_create_dry_run_with_layout(
    out: &Path,
    plan: &BackupPlan,
) -> Result<(BackupPlan, bool), BackupCommandError> {
    persist_backup_create_layout(out, plan).map(|layout| (layout.plan, layout.reused_existing))
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

fn backup_installed_deployment_error(error: InstalledDeploymentError) -> BackupCommandError {
    match error {
        InstalledDeploymentError::NoInstalledDeployment {
            network,
            deployment,
        } => BackupCommandError::NoInstalledDeployment {
            network,
            deployment,
        },
        InstalledDeploymentError::InstallState(error) => BackupCommandError::InstallState(error),
        InstalledDeploymentError::ReplicaQuery(error) => BackupCommandError::ReplicaQuery(error),
        InstalledDeploymentError::IcpFailed { command, stderr } => {
            BackupCommandError::IcpFailed { command, stderr }
        }
        InstalledDeploymentError::LostLocalDeployment {
            network,
            deployment,
            root,
        } => BackupCommandError::LostLocalDeployment {
            network,
            deployment,
            root,
        },
        InstalledDeploymentError::Registry(error) => BackupCommandError::Registry(error),
        InstalledDeploymentError::Io(error) => BackupCommandError::Io(error),
    }
}
