//! Module: backup::create
//!
//! Responsibility: prepare local backup plans from retained Fleet inventory.
//! Does not own: live membership discovery or snapshot execution.
//! Boundary: refuses unavailable execution before resolving or creating local state.

mod persistence;
mod plan;

use crate::backup::labels::backup_scope_label;
use crate::backup::{
    BackupCommandError, BackupCreateLayout, BackupCreateMode, BackupCreateOptions,
    BackupCreateReport, BackupRunStatus,
};
#[cfg(test)]
use canic_backup::plan::BackupPlan;
use canic_backup::{
    manifest::IdentityMode,
    plan::{
        AuthorityEvidence, BackupPlanBuildInput, BackupScopeKind, ControlAuthority,
        QuiescencePolicy, SnapshotReadAuthority, build_backup_plan, resolve_backup_selector,
    },
};
use canic_host::{
    fleet_ensure::read_last_converged_fleet_inventory, icp_config::resolve_current_canic_icp_root,
};
#[cfg(test)]
use std::path::Path;

use persistence::persist_backup_create_layout;
use plan::{
    backup_plan_id, backup_registry_entries, default_backup_output_path, registry_topology_hash,
};

pub(super) fn backup_create(
    options: &BackupCreateOptions,
) -> Result<BackupCreateReport, BackupCommandError> {
    if !options.dry_run {
        return Err(BackupCommandError::LiveCreateUnavailable);
    }
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
        control_authority: ControlAuthority::root_controller(AuthorityEvidence::Declared),
        snapshot_read_authority: SnapshotReadAuthority::root_configured_read(
            AuthorityEvidence::Declared,
        ),
        quiescence_policy: QuiescencePolicy::RootCoordinated,
        identity_mode: IdentityMode::Relocatable,
    })?;
    let persisted = persist_backup_create_layout(&out, &planned)?;
    let layout = if persisted.reused_existing {
        BackupCreateLayout::Existing
    } else {
        BackupCreateLayout::New
    };
    let plan = persisted.plan;

    Ok(BackupCreateReport {
        fleet: plan.fleet.clone(),
        environment: plan.environment.clone(),
        out,
        plan_id: plan.plan_id.clone(),
        run_id: plan.run_id.clone(),
        mode: BackupCreateMode::DryRun,
        layout,
        status: BackupRunStatus::Planned,
        scope: backup_scope_label(&plan),
        targets: plan.targets.len(),
        operations: plan.phases.len(),
        executed_operations: 0,
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
