//! Module: install_root::reused_build
//!
//! Responsibility: reconstruct current install inputs from one finalized release build.
//! Does not own: release-build selection, artifact mutation, or Fleet activation.
//! Boundary: persisted topology and artifact manifests must match the current App snapshot.

use super::build_snapshot::CompleteInstallBuildSnapshot;
use crate::{
    canister_build::{CanisterArtifactBuildOutput, CurrentCanisterArtifactBuildOutput},
    release_set::{
        CanicInfrastructureArtifactBuildOutput, CanicInfrastructureRole,
        load_persisted_application_artifact_union,
        load_persisted_canic_infrastructure_artifact_manifest, resolve_release_artifact_path,
    },
};
use canic_core::ids::{CanisterRole, ReleaseBuildId};
use std::path::Path;

pub(super) struct ReusedInstallBuild {
    pub(super) outputs: Vec<CurrentCanisterArtifactBuildOutput>,
    pub(super) infrastructure_outputs: Vec<CanicInfrastructureArtifactBuildOutput>,
}

pub(super) fn load_reused_install_build(
    icp_root: &Path,
    snapshot: &CompleteInstallBuildSnapshot,
    release_build_id: ReleaseBuildId,
) -> Result<ReusedInstallBuild, Box<dyn std::error::Error>> {
    let application = load_persisted_application_artifact_union(
        icp_root,
        &snapshot.component_topology,
        release_build_id,
    )?;
    let infrastructure =
        load_persisted_canic_infrastructure_artifact_manifest(icp_root, release_build_id)?;
    let root = infrastructure
        .manifest
        .entries
        .iter()
        .find(|entry| entry.role == CanicInfrastructureRole::FleetSubnetRoot)
        .ok_or("finalized infrastructure manifest has no Fleet Subnet Root")?;

    let mut outputs = Vec::with_capacity(snapshot.targets.len());
    for target in &snapshot.targets {
        let (package, wasm_relative_path, wasm_gz_relative_path, candid_sha256, profile_digest) =
            if target.role
                == snapshot
                    .targets
                    .first()
                    .ok_or("complete install snapshot has no root target")?
                    .role
            {
                (
                    root.package.as_str(),
                    root.wasm_relative_path.as_str(),
                    root.wasm_gz_relative_path.as_str(),
                    root.candid_sha256,
                    root.protocol_profile_digest,
                )
            } else {
                let role = CanisterRole::owned(target.role.clone());
                let entry = application
                    .union
                    .entries
                    .iter()
                    .find(|entry| entry.role == role)
                    .ok_or_else(|| format!("finalized application union has no role {role}"))?;
                (
                    entry.package.as_str(),
                    entry.wasm_relative_path.as_str(),
                    entry.wasm_gz_relative_path.as_str(),
                    entry.candid_sha256,
                    entry.protocol_profile_digest,
                )
            };
        if package != target.spec.package_name {
            return Err(format!(
                "finalized role {} package {package} differs from current App package {}",
                target.role, target.spec.package_name
            )
            .into());
        }
        outputs.push(CurrentCanisterArtifactBuildOutput {
            role: target.role.clone(),
            output: reused_output(
                icp_root,
                &target.role,
                package,
                &target.spec.package_version,
                &target.spec.canic_version,
                &target.spec.capabilities,
                wasm_relative_path,
                wasm_gz_relative_path,
                candid_sha256,
                profile_digest,
            )?,
        });
    }

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
                release_build_id,
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

    Ok(ReusedInstallBuild {
        outputs,
        infrastructure_outputs,
    })
}

#[expect(
    clippy::too_many_arguments,
    reason = "one flat finalized artifact entry is copied without adding a parallel model"
)]
fn reused_output(
    icp_root: &Path,
    role: &str,
    package_name: &str,
    package_version: &str,
    protocol_release_identity: &str,
    protocol_capabilities: &std::collections::BTreeSet<
        canic_core::role_contract::RoleCapabilityKey,
    >,
    wasm_relative_path: &str,
    wasm_gz_relative_path: &str,
    candid_sha256: [u8; 32],
    protocol_profile_digest: canic_core::role_contract::ProtocolProfileDigest,
) -> Result<CanisterArtifactBuildOutput, Box<dyn std::error::Error>> {
    let wasm_path = resolve_release_artifact_path(icp_root, wasm_relative_path)?;
    let wasm_gz_path = resolve_release_artifact_path(icp_root, wasm_gz_relative_path)?;
    let artifact_root = wasm_path
        .parent()
        .ok_or_else(|| format!("finalized role {role} Wasm has no artifact directory"))?
        .to_path_buf();
    Ok(CanisterArtifactBuildOutput {
        package_name: package_name.to_string(),
        package_version: package_version.to_string(),
        protocol_release_identity: protocol_release_identity.to_string(),
        protocol_role: CanisterRole::owned(role.to_string()),
        protocol_capabilities: protocol_capabilities.clone(),
        did_path: artifact_root.join(format!("{role}.did")),
        artifact_root,
        wasm_path,
        wasm_gz_path,
        candid_sha256,
        protocol_profile_digest,
        transforms: Vec::new(),
    })
}
