//! Module: plan::build::targets
//!
//! Responsibility: expand selected backup scope into ordered snapshot targets.
//! Does not own: registry discovery, phase construction, or plan validation.
//! Boundary: maps registry rows into backup target records for plan building.

use crate::{
    discovery::{SnapshotTarget, targets_from_registry},
    manifest::IdentityMode,
    plan::{
        BackupPlanBuildInput, BackupPlanError, BackupScopeKind, BackupTarget, ControlAuthority,
        SnapshotReadAuthority,
    },
    registry::RegistryEntry,
};

use std::collections::{BTreeMap, BTreeSet};

pub(super) fn selected_subtree_root(
    input: &BackupPlanBuildInput<'_>,
) -> Result<Option<String>, BackupPlanError> {
    match input.selected_scope_kind {
        BackupScopeKind::NonRootDeployment => Ok(None),
        BackupScopeKind::Member | BackupScopeKind::Subtree | BackupScopeKind::MaintenanceRoot => {
            input
                .selected_canister_id
                .clone()
                .ok_or(BackupPlanError::EmptyField("selected_canister_id"))
                .map(Some)
        }
    }
}

pub(super) fn snapshot_targets(
    input: &BackupPlanBuildInput<'_>,
) -> Result<Vec<SnapshotTarget>, BackupPlanError> {
    match input.selected_scope_kind {
        BackupScopeKind::Member | BackupScopeKind::Subtree | BackupScopeKind::MaintenanceRoot => {
            let selected = input
                .selected_canister_id
                .as_deref()
                .ok_or(BackupPlanError::EmptyField("selected_canister_id"))?;
            let recursive = input.selected_scope_kind == BackupScopeKind::Subtree
                || input.include_descendants
                || input.selected_scope_kind == BackupScopeKind::MaintenanceRoot;
            targets_from_registry(input.registry, selected, recursive)
                .map_err(BackupPlanError::from)
        }
        BackupScopeKind::NonRootDeployment => Ok(input
            .registry
            .iter()
            .filter(|entry| entry.pid != input.root_canister_id)
            .map(|entry| SnapshotTarget {
                canister_id: entry.pid.clone(),
                role: entry.role.clone(),
                parent_canister_id: entry.parent_pid.clone(),
                module_hash: entry.module_hash.clone(),
            })
            .collect()),
    }
}

pub(super) fn backup_target(
    target: SnapshotTarget,
    target_depths: &BTreeMap<String, u32>,
    control_authority: ControlAuthority,
    snapshot_read_authority: SnapshotReadAuthority,
    identity_mode: IdentityMode,
) -> BackupTarget {
    BackupTarget {
        depth: target_depths
            .get(&target.canister_id)
            .copied()
            .unwrap_or_default(),
        canister_id: target.canister_id,
        role: target.role,
        parent_canister_id: target.parent_canister_id,
        control_authority,
        snapshot_read_authority,
        identity_mode,
        expected_module_hash: target.module_hash,
    }
}

pub(super) fn target_depths(registry: &[RegistryEntry]) -> BTreeMap<String, u32> {
    let parents = registry
        .iter()
        .map(|entry| (entry.pid.as_str(), entry.parent_pid.as_deref()))
        .collect::<BTreeMap<_, _>>();
    registry
        .iter()
        .map(|entry| {
            (
                entry.pid.clone(),
                target_depth(entry.pid.as_str(), &parents),
            )
        })
        .collect()
}

fn target_depth(canister_id: &str, parents: &BTreeMap<&str, Option<&str>>) -> u32 {
    let mut depth = 0;
    let mut current = canister_id;
    let mut seen = BTreeSet::new();

    while let Some(Some(parent)) = parents.get(current) {
        if !seen.insert(current) {
            break;
        }
        depth += 1;
        current = parent;
    }

    depth
}
