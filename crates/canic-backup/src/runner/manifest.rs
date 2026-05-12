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

pub(super) fn build_manifest(
    config: &BackupRunnerConfig,
    plan: &BackupPlan,
    journal: &DownloadJournal,
) -> Result<FleetBackupManifest, BackupRunnerError> {
    let roles = plan
        .targets
        .iter()
        .enumerate()
        .map(|(index, target)| target_role(index, target.role.as_deref()))
        .collect::<Vec<_>>();
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
            backup_units: vec![BackupUnit {
                unit_id: "backup-selection".to_string(),
                kind: if plan.targets.len() == 1 {
                    BackupUnitKind::Single
                } else {
                    BackupUnitKind::Subtree
                },
                roles,
            }],
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
