use super::{BackupRunnerConfig, BackupRunnerError, support::state_updated_at};
use crate::{
    journal::{ArtifactState, DownloadJournal},
    manifest::{
        BackupUnit, BackupUnitKind, ConsistencySection, FleetBackupManifest, FleetMember,
        FleetSection, SourceMetadata, SourceSnapshot, ToolMetadata, VerificationCheck,
        VerificationPlan,
    },
    plan::{BackupPlan, BackupTarget, ControlAuthoritySource},
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub(super) fn build_manifest(
    config: &BackupRunnerConfig,
    plan: &BackupPlan,
    journal: &DownloadJournal,
) -> Result<FleetBackupManifest, BackupRunnerError> {
    let manifest = FleetBackupManifest {
        manifest_version: 1,
        backup_id: plan.run_id.clone(),
        created_at: state_updated_at(config.updated_at.as_ref()),
        tool: ToolMetadata {
            name: config.tool_name.clone(),
            version: config.tool_version.clone(),
        },
        source: SourceMetadata {
            environment: plan.network.clone(),
            root_canister: plan.root_canister_id.clone(),
        },
        consistency: ConsistencySection {
            backup_units: build_backup_units(plan),
        },
        fleet: FleetSection {
            topology_hash_algorithm: "sha256".to_string(),
            topology_hash_input: format!("canic-backup-plan:{}", plan.plan_id),
            discovery_topology_hash: plan.topology_hash_before_quiesce.clone(),
            pre_snapshot_topology_hash: plan.topology_hash_before_quiesce.clone(),
            topology_hash: plan.topology_hash_before_quiesce.clone(),
            members: plan
                .targets
                .iter()
                .enumerate()
                .map(|(index, target)| manifest_member(index, target, plan, journal))
                .collect::<Result<Vec<_>, BackupRunnerError>>()?,
        },
        verification: VerificationPlan::default(),
    };
    manifest.validate()?;
    Ok(manifest)
}

fn build_backup_units(plan: &BackupPlan) -> Vec<BackupUnit> {
    let target_ids = plan
        .targets
        .iter()
        .map(|target| target.canister_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut children_by_parent = BTreeMap::<&str, Vec<usize>>::new();
    for (index, target) in plan.targets.iter().enumerate() {
        if let Some(parent) = target.parent_canister_id.as_deref()
            && target_ids.contains(parent)
        {
            children_by_parent.entry(parent).or_default().push(index);
        }
    }

    let roots = plan
        .targets
        .iter()
        .enumerate()
        .filter_map(|(index, target)| {
            let parent_in_selection = target
                .parent_canister_id
                .as_deref()
                .is_some_and(|parent| target_ids.contains(parent));
            (!parent_in_selection).then_some(index)
        })
        .collect::<Vec<_>>();
    let mut visited = BTreeSet::new();
    let mut components = Vec::new();
    for root in roots {
        if visited.contains(&root) {
            continue;
        }
        components.push(collect_component(
            root,
            plan,
            &children_by_parent,
            &mut visited,
        ));
    }
    for index in 0..plan.targets.len() {
        if !visited.contains(&index) {
            components.push(collect_component(
                index,
                plan,
                &children_by_parent,
                &mut visited,
            ));
        }
    }

    let multiple_units = components.len() > 1;
    components
        .into_iter()
        .enumerate()
        .map(|(unit_index, component)| {
            let roles = component
                .iter()
                .map(|index| target_role(*index, plan.targets[*index].role.as_deref()))
                .collect::<Vec<_>>();
            BackupUnit {
                unit_id: if multiple_units {
                    format!("backup-selection-{}", unit_index + 1)
                } else {
                    "backup-selection".to_string()
                },
                kind: if roles.len() == 1 {
                    BackupUnitKind::Single
                } else {
                    BackupUnitKind::Subtree
                },
                roles,
            }
        })
        .collect()
}

fn collect_component(
    root: usize,
    plan: &BackupPlan,
    children_by_parent: &BTreeMap<&str, Vec<usize>>,
    visited: &mut BTreeSet<usize>,
) -> Vec<usize> {
    let mut queue = VecDeque::from([root]);
    let mut component = Vec::new();
    while let Some(index) = queue.pop_front() {
        if !visited.insert(index) {
            continue;
        }
        component.push(index);
        if let Some(children) = children_by_parent.get(plan.targets[index].canister_id.as_str()) {
            queue.extend(children.iter().copied());
        }
    }
    component
}

fn manifest_member(
    index: usize,
    target: &BackupTarget,
    plan: &BackupPlan,
    journal: &DownloadJournal,
) -> Result<FleetMember, BackupRunnerError> {
    let role = target_role(index, target.role.as_deref());
    let entry = journal
        .artifacts
        .iter()
        .find(|entry| {
            entry.canister_id == target.canister_id && entry.state == ArtifactState::Durable
        })
        .ok_or_else(|| BackupRunnerError::MissingArtifactEntry {
            sequence: usize::MAX,
            target_canister_id: target.canister_id.clone(),
        })?;
    Ok(FleetMember {
        role: role.clone(),
        canister_id: target.canister_id.clone(),
        parent_canister_id: target.parent_canister_id.clone(),
        subnet_canister_id: None,
        controller_hint: controller_hint(plan, target),
        identity_mode: target.identity_mode.clone(),
        verification_checks: vec![VerificationCheck {
            kind: "status".to_string(),
            roles: vec![role],
        }],
        source_snapshot: SourceSnapshot {
            snapshot_id: entry.snapshot_id.clone(),
            module_hash: target.expected_module_hash.clone(),
            code_version: None,
            artifact_path: entry.artifact_path.clone(),
            checksum_algorithm: entry.checksum_algorithm.clone(),
            checksum: entry.checksum.clone(),
        },
    })
}

fn controller_hint(plan: &BackupPlan, target: &BackupTarget) -> Option<String> {
    if matches!(
        target.control_authority.source,
        ControlAuthoritySource::RootController
    ) {
        Some(plan.root_canister_id.clone())
    } else {
        None
    }
}

fn target_role(index: usize, role: Option<&str>) -> String {
    role.map_or_else(|| format!("member-{index}"), str::to_string)
}
