use super::{
    SnapshotArtifact, SnapshotManifestError, SnapshotManifestInput,
    support::{safe_path_segment, target_role},
};
use crate::{
    discovery::SnapshotTarget,
    manifest::{
        BackupUnit, BackupUnitKind, ConsistencySection, FleetBackupManifest, FleetMember,
        FleetSection, IdentityMode, SourceMetadata, SourceSnapshot, ToolMetadata,
        VerificationCheck, VerificationPlan,
    },
};
use std::collections::BTreeSet;

/// Build a validated fleet backup manifest for one successful snapshot run.
pub fn build_snapshot_manifest(
    input: SnapshotManifestInput<'_>,
) -> Result<FleetBackupManifest, SnapshotManifestError> {
    let roles = input
        .targets
        .iter()
        .enumerate()
        .map(|(index, target)| target_role(&input.selected_canister, index, target))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let manifest = FleetBackupManifest {
        manifest_version: 1,
        backup_id: input.backup_id,
        created_at: input.created_at,
        tool: ToolMetadata {
            name: input.tool_name,
            version: input.tool_version,
        },
        source: SourceMetadata {
            environment: input.environment,
            root_canister: input.root_canister.clone(),
        },
        consistency: ConsistencySection {
            backup_units: vec![BackupUnit {
                unit_id: "snapshot-selection".to_string(),
                kind: if input.include_children {
                    BackupUnitKind::Subtree
                } else {
                    BackupUnitKind::Single
                },
                roles,
            }],
        },
        fleet: FleetSection {
            topology_hash_algorithm: input.discovery_topology_hash.algorithm,
            topology_hash_input: input.discovery_topology_hash.input,
            discovery_topology_hash: input.discovery_topology_hash.hash.clone(),
            pre_snapshot_topology_hash: input.pre_snapshot_topology_hash.hash,
            topology_hash: input.discovery_topology_hash.hash,
            members: input
                .targets
                .iter()
                .enumerate()
                .map(|(index, target)| {
                    fleet_member(
                        &input.selected_canister,
                        Some(input.root_canister.as_str()).filter(|_| input.include_children),
                        index,
                        target,
                        input.artifacts,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?,
        },
        verification: VerificationPlan::default(),
    };

    manifest.validate()?;
    Ok(manifest)
}

fn fleet_member(
    selected_canister: &str,
    subnet_canister_id: Option<&str>,
    index: usize,
    target: &SnapshotTarget,
    artifacts: &[SnapshotArtifact],
) -> Result<FleetMember, SnapshotManifestError> {
    let Some(artifact) = artifacts
        .iter()
        .find(|artifact| artifact.canister_id == target.canister_id)
    else {
        return Err(SnapshotManifestError::MissingArtifact(
            target.canister_id.clone(),
        ));
    };
    let role = target_role(selected_canister, index, target);

    Ok(FleetMember {
        role: role.clone(),
        canister_id: target.canister_id.clone(),
        parent_canister_id: target.parent_canister_id.clone(),
        subnet_canister_id: subnet_canister_id.map(str::to_string),
        controller_hint: None,
        identity_mode: if target.canister_id == selected_canister {
            IdentityMode::Fixed
        } else {
            IdentityMode::Relocatable
        },
        verification_checks: vec![VerificationCheck {
            kind: "status".to_string(),
            roles: vec![role],
        }],
        source_snapshot: SourceSnapshot {
            snapshot_id: artifact.snapshot_id.clone(),
            module_hash: target.module_hash.clone(),
            code_version: None,
            artifact_path: safe_path_segment(&target.canister_id),
            checksum_algorithm: "sha256".to_string(),
            checksum: Some(artifact.checksum.clone()),
        },
    })
}
