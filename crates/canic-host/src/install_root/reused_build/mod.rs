//! Module: install_root::reused_build
//!
//! Responsibility: validate one finalized release build for read-only install reuse.
//! Does not own: release-build selection, artifact mutation, or Fleet activation.
//! Boundary: retained manifests, bytes, topology, packages, and builder identity must agree.

use super::build_snapshot::FinalizedInstallBuildSnapshot;
use crate::release_set::{
    ApplicationArtifactBuildTarget, ApplicationArtifactFileBuildOutput,
    CanicInfrastructureArtifactBuildOutput, CanicInfrastructureArtifactEntry,
    CanicInfrastructureRole, compile_and_persist_application_artifact_union,
    compile_and_persist_canic_infrastructure_artifact_manifest,
    load_persisted_application_artifact_union,
    load_persisted_canic_infrastructure_artifact_manifest, resolve_release_artifact_path,
};
use std::path::Path;

pub(super) fn validate_reused_install_build(
    icp_root: &Path,
    snapshot: &FinalizedInstallBuildSnapshot,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let application = load_persisted_application_artifact_union(
        icp_root,
        &snapshot.component_topology,
        snapshot.release_build_id,
    )?;
    let infrastructure =
        load_persisted_canic_infrastructure_artifact_manifest(icp_root, snapshot.release_build_id)?;

    validate_infrastructure_authority(snapshot, &infrastructure.manifest.entries)?;
    for entry in &application.union.entries {
        require_package_match(snapshot, &entry.role, &entry.package)?;
    }

    let application_targets = application
        .union
        .entries
        .iter()
        .map(|entry| ApplicationArtifactBuildTarget {
            role: entry.role.clone(),
            package: entry.package.clone(),
            wasm_relative_path: entry.wasm_relative_path.clone(),
            wasm_gz_relative_path: entry.wasm_gz_relative_path.clone(),
        })
        .collect::<Vec<_>>();
    let application_outputs = application
        .union
        .entries
        .iter()
        .map(|entry| {
            Ok(ApplicationArtifactFileBuildOutput {
                role: entry.role.clone(),
                package: entry.package.clone(),
                release_build_id: snapshot.release_build_id,
                wasm_path: resolve_release_artifact_path(icp_root, &entry.wasm_relative_path)?,
                wasm_gz_path: resolve_release_artifact_path(
                    icp_root,
                    &entry.wasm_gz_relative_path,
                )?,
                candid_sha256: entry.candid_sha256,
                protocol_profile_digest: entry.protocol_profile_digest,
            })
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    compile_and_persist_application_artifact_union(
        icp_root,
        &snapshot.component_topology,
        snapshot.release_build_id,
        &application_targets,
        &application_outputs,
    )?;

    let infrastructure_outputs = infrastructure
        .manifest
        .entries
        .iter()
        .map(|entry| {
            Ok(CanicInfrastructureArtifactBuildOutput {
                role: entry.role,
                package: entry.package.clone(),
                protocol_release_identity: entry.protocol_release_identity.clone(),
                protocol_role: entry.protocol_role.clone(),
                protocol_capabilities: entry.protocol_capabilities.clone(),
                release_build_id: snapshot.release_build_id,
                wasm_path: resolve_release_artifact_path(icp_root, &entry.wasm_relative_path)?,
                wasm_gz_path: resolve_release_artifact_path(
                    icp_root,
                    &entry.wasm_gz_relative_path,
                )?,
                candid_sha256: entry.candid_sha256,
                protocol_profile_digest: entry.protocol_profile_digest,
            })
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    compile_and_persist_canic_infrastructure_artifact_manifest(
        icp_root,
        snapshot.release_build_id,
        &infrastructure_outputs,
    )?;

    Ok(snapshot
        .package_by_role
        .keys()
        .map(ToString::to_string)
        .chain(["fleet_coordinator".to_string(), "wasm_store".to_string()])
        .collect())
}

fn validate_infrastructure_authority(
    snapshot: &FinalizedInstallBuildSnapshot,
    entries: &[CanicInfrastructureArtifactEntry],
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in entries {
        if entry.protocol_release_identity != snapshot.builder_version {
            return Err(format!(
                "finalized infrastructure role {} belongs to Canic {}, not retained builder {}",
                entry.role.as_str(),
                entry.protocol_release_identity,
                snapshot.builder_version
            )
            .into());
        }
    }
    let root = entries
        .iter()
        .find(|entry| entry.role == CanicInfrastructureRole::FleetSubnetRoot)
        .ok_or("finalized infrastructure manifest has no Fleet Subnet Root")?;
    if root.protocol_role != snapshot.root_role {
        return Err(format!(
            "finalized Root protocol role {} differs from current App Root role {}",
            root.protocol_role, snapshot.root_role
        )
        .into());
    }
    require_package_match(snapshot, &snapshot.root_role, &root.package)
}

fn require_package_match(
    snapshot: &FinalizedInstallBuildSnapshot,
    role: &canic_core::ids::CanisterRole,
    retained: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let current = snapshot
        .package_by_role
        .get(role)
        .ok_or_else(|| format!("current App has no finalized role {role}"))?;
    if current == retained {
        return Ok(());
    }
    Err(format!(
        "finalized role {role} package {retained} differs from current App package {current}"
    )
    .into())
}
