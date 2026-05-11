use super::{
    BackupOperation, BackupOperationKind, BackupPlan, BackupPlanError, BackupScopeKind,
    BackupTarget, ControlAuthority, ControlAuthoritySource, QuiescencePolicy,
    SnapshotReadAuthority,
};
use crate::{
    discovery::{RegistryEntry, SnapshotTarget, targets_from_registry},
    manifest::IdentityMode,
};
use candid::Principal;
use std::{
    collections::{BTreeMap, BTreeSet},
    str::FromStr,
};

///
/// BackupPlanBuildInput
///

pub struct BackupPlanBuildInput<'a> {
    pub plan_id: String,
    pub run_id: String,
    pub fleet: String,
    pub network: String,
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
        network: input.network,
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

/// Resolve an operator selector to one concrete live registry canister id.
pub fn resolve_backup_selector(
    registry: &[RegistryEntry],
    selector: &str,
) -> Result<String, BackupPlanError> {
    validate_nonempty("selector", selector)?;
    if Principal::from_str(selector).is_ok() {
        return registry
            .iter()
            .find(|entry| entry.pid == selector)
            .map(|entry| entry.pid.clone())
            .ok_or_else(|| BackupPlanError::UnknownSelector(selector.to_string()));
    }

    let matches = registry
        .iter()
        .filter(|entry| entry.role.as_deref() == Some(selector))
        .map(|entry| entry.pid.clone())
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [canister] => Ok(canister.clone()),
        [] => Err(BackupPlanError::UnknownSelector(selector.to_string())),
        _ => Err(BackupPlanError::AmbiguousSelector {
            selector: selector.to_string(),
            matches,
        }),
    }
}

fn selected_subtree_root(
    input: &BackupPlanBuildInput<'_>,
) -> Result<Option<String>, BackupPlanError> {
    match input.selected_scope_kind {
        BackupScopeKind::NonRootFleet => Ok(None),
        BackupScopeKind::Member | BackupScopeKind::Subtree | BackupScopeKind::MaintenanceRoot => {
            input
                .selected_canister_id
                .clone()
                .ok_or(BackupPlanError::EmptyField("selected_canister_id"))
                .map(Some)
        }
    }
}

fn snapshot_targets(
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
        BackupScopeKind::NonRootFleet => Ok(input
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

fn backup_target(
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

fn target_depths(registry: &[RegistryEntry]) -> BTreeMap<String, u32> {
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

fn build_backup_phases(targets: &[BackupTarget]) -> Vec<BackupOperation> {
    let mut phases = vec![
        operation(
            "validate-topology",
            BackupOperationKind::ValidateTopology,
            None,
        ),
        operation(
            "validate-control-authority",
            BackupOperationKind::ValidateControlAuthority,
            None,
        ),
        operation(
            "validate-snapshot-read-authority",
            BackupOperationKind::ValidateSnapshotReadAuthority,
            None,
        ),
        operation(
            "validate-quiescence-policy",
            BackupOperationKind::ValidateQuiescencePolicy,
            None,
        ),
    ];

    let mut top_down = targets.iter().collect::<Vec<_>>();
    top_down.sort_by(|left, right| {
        left.depth
            .cmp(&right.depth)
            .then_with(|| left.canister_id.cmp(&right.canister_id))
    });
    for target in &top_down {
        phases.push(operation(
            format!("stop-{}", target.canister_id),
            BackupOperationKind::Stop,
            Some(target.canister_id.clone()),
        ));
    }
    for target in &top_down {
        phases.push(operation(
            format!("snapshot-{}", target.canister_id),
            BackupOperationKind::CreateSnapshot,
            Some(target.canister_id.clone()),
        ));
    }

    let mut bottom_up = top_down;
    bottom_up.reverse();
    for target in &bottom_up {
        phases.push(operation(
            format!("start-{}", target.canister_id),
            BackupOperationKind::Start,
            Some(target.canister_id.clone()),
        ));
    }

    for target in targets {
        phases.push(operation(
            format!("download-{}", target.canister_id),
            BackupOperationKind::DownloadSnapshot,
            Some(target.canister_id.clone()),
        ));
        phases.push(operation(
            format!("verify-{}", target.canister_id),
            BackupOperationKind::VerifyArtifact,
            Some(target.canister_id.clone()),
        ));
    }
    phases.push(operation(
        "finalize-manifest",
        BackupOperationKind::FinalizeManifest,
        None,
    ));

    phases
        .into_iter()
        .enumerate()
        .map(|(index, mut phase)| {
            phase.order = u32::try_from(index).unwrap_or(u32::MAX);
            phase
        })
        .collect()
}

fn operation(
    operation_id: impl Into<String>,
    kind: BackupOperationKind,
    target_canister_id: Option<String>,
) -> BackupOperation {
    BackupOperation {
        operation_id: operation_id.into(),
        order: 0,
        kind,
        target_canister_id,
    }
}

fn validate_nonempty(field: &'static str, value: &str) -> Result<(), BackupPlanError> {
    if value.trim().is_empty() {
        Err(BackupPlanError::EmptyField(field))
    } else {
        Ok(())
    }
}
