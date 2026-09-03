use super::{
    BackupCommandError, BackupCreateLayout, BackupCreateMode, BackupCreateOptions,
    BackupCreateReport, BackupRunStatus,
};
use crate::backup::labels::backup_scope_label;
#[cfg(test)]
use canic_backup::plan::BackupPlan;
use canic_backup::{
    manifest::IdentityMode,
    plan::{BackupPlanBuildInput, BackupScopeKind, build_backup_plan, resolve_backup_selector},
    runner::{BackupRunResponse, BackupRunnerConfig, backup_run_execute_with_executor},
};
use canic_host::{
    fleet_ensure::read_last_converged_fleet_inventory, icp_config::resolve_current_canic_icp_root,
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
    let icp_root = resolve_current_canic_icp_root().map_err(BackupCommandError::IcpRoot)?;
    let inventory =
        read_last_converged_fleet_inventory(&icp_root, &options.environment, &options.fleet)?;
    let registry = backup_registry_entries(&inventory.entries);
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
        BackupScopeKind::NonRootDeployment
    };
    let [root_canister_id] = inventory.roots.as_slice() else {
        return Err(BackupCommandError::AmbiguousFleetSubnetRoot {
            fleet: options.fleet.clone(),
            root_count: inventory.roots.len(),
        });
    };
    let planned = build_backup_plan(BackupPlanBuildInput {
        plan_id,
        run_id,
        fleet: options.fleet.clone(),
        environment: options.environment.clone(),
        root_canister_id: root_canister_id.clone(),
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
        BackupCreateLayout::Existing
    } else {
        BackupCreateLayout::New
    };
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
        fleet: plan.fleet.clone(),
        environment: plan.environment.clone(),
        out,
        plan_id: plan.plan_id.clone(),
        run_id: plan.run_id.clone(),
        mode: if options.dry_run {
            BackupCreateMode::DryRun
        } else {
            BackupCreateMode::Execute
        },
        layout,
        status: run
            .as_ref()
            .map_or(BackupRunStatus::Planned, backup_run_status),
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

const fn backup_run_status(run: &BackupRunResponse) -> BackupRunStatus {
    if run.complete {
        BackupRunStatus::Complete
    } else if run.max_steps_reached {
        BackupRunStatus::Paused
    } else {
        BackupRunStatus::Running
    }
}
