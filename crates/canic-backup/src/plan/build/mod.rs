//! Module: plan::build
//!
//! Responsibility: construct backup plans from registry-backed selections.
//! Does not own: registry querying, preflight execution, or journal state.
//! Boundary: maps discovered targets into validated plan operations.

mod phases;
mod selector;
mod targets;

pub use selector::resolve_backup_selector;

use crate::plan::{
    BackupPlan, BackupPlanError, BackupScopeKind, ControlAuthority, ControlAuthoritySource,
    QuiescencePolicy, SnapshotReadAuthority,
};
use crate::{manifest::IdentityMode, registry::RegistryEntry};
use phases::build_backup_phases;
use targets::{backup_target, selected_subtree_root, snapshot_targets, target_depths};

///
/// BackupPlanBuildInput
///
/// Input bundle required to build a backup plan from registry entries.
/// Owned by backup plan construction and supplied by higher-level workflows.
///

pub struct BackupPlanBuildInput<'a> {
    pub plan_id: String,
    pub run_id: String,
    pub fleet: String,
    pub environment: String,
    pub root_canister_id: String,
    pub selected_canister_id: Option<String>,
    pub selected_scope_kind: BackupScopeKind,
    pub include_descendants: bool,
    pub topology_hash_before_quiesce: String,
    pub registry: &'a [RegistryEntry],
    pub control_authority: ControlAuthority,
    pub snapshot_read_authority: SnapshotReadAuthority,
    pub quiescence_policy: QuiescencePolicy,
    pub identity_mode: IdentityMode,
}

/// Build a validated backup plan from the live root registry projection.
pub fn build_backup_plan(input: BackupPlanBuildInput<'_>) -> Result<BackupPlan, BackupPlanError> {
    let snapshot_read_authority = input.snapshot_read_authority.clone();
    let quiescence_policy = input.quiescence_policy.clone();
    let root_included = input.selected_scope_kind == BackupScopeKind::MaintenanceRoot;
    let selected_subtree_root = selected_subtree_root(&input)?;
    let snapshot_targets = snapshot_targets(&input)?;
    let target_depths = target_depths(input.registry);
    let targets = snapshot_targets
        .into_iter()
        .map(|target| {
            backup_target(
                target,
                &target_depths,
                input.control_authority.clone(),
                input.snapshot_read_authority.clone(),
                input.identity_mode.clone(),
            )
        })
        .collect::<Vec<_>>();
    let phases = build_backup_phases(&targets);

    let plan = BackupPlan {
        plan_id: input.plan_id,
        run_id: input.run_id,
        fleet: input.fleet,
        environment: input.environment,
        root_canister_id: input.root_canister_id,
        selected_subtree_root,
        selected_scope_kind: input.selected_scope_kind,
        include_descendants: input.include_descendants,
        root_included,
        requires_root_controller: input.control_authority.source
            == ControlAuthoritySource::RootController,
        snapshot_read_authority,
        quiescence_policy,
        topology_hash_before_quiesce: input.topology_hash_before_quiesce,
        targets,
        phases,
    };
    plan.validate()?;
    Ok(plan)
}
