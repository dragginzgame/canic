//! Module: install_root::build_snapshot
//!
//! Responsibility: resolve one immutable configuration-backed install input set.
//! Does not own: Cargo execution, artifact bytes, or manifest publication.
//! Boundary: fresh builds use current workspace evidence; finalized recovery uses retained evidence.

use crate::{
    canister_build::{
        CanisterArtifactBuildSpec, WorkspaceBuildContext, resolve_canister_artifact_build_specs,
    },
    release_build::{
        PlannedReleaseBuild, ReleaseBuildPlanState, validate_finalized_release_build_manifest,
    },
    release_set::{
        AppConfigSnapshot, ApplicationArtifactBuildTarget, RootReleaseSetBuildSnapshot,
        RootReleaseSetBuildTarget, configured_release_roles_from_config,
        load_persisted_application_artifact_union,
        load_persisted_canic_infrastructure_artifact_manifest, load_root_package_version,
        load_root_release_set_manifest, root_release_set_manifest_path, workspace_manifest_path,
    },
    role_contract::{declared_role_manifest_path, finding_detail},
};
use std::{collections::BTreeMap, fs, path::Path};

use canic_core::{
    bootstrap::compiled::{ComponentTopology, ConfigModel},
    cdk::utils::hash::hex_bytes,
    ids::{CanisterRole, ReleaseBuildId},
};

/// One target whose package and output paths were admitted from the workspace snapshot.
#[derive(Clone, Debug)]
pub(super) struct InstallBuildTarget {
    pub(super) role: String,
    pub(super) spec: CanisterArtifactBuildSpec,
}

/// Fresh-build inputs derived from the current workspace role contracts.
#[derive(Clone, Debug)]
pub(super) struct WorkspaceInstallBuildSnapshot {
    pub(super) targets: Vec<InstallBuildTarget>,
    pub(super) manifest: RootReleaseSetBuildSnapshot,
    pub(super) component_topology: ComponentTopology,
    pub(super) application_artifact_targets: Vec<ApplicationArtifactBuildTarget>,
}

/// Read-only inputs that bind one finalized build to the current exact App source.
#[derive(Clone, Debug)]
pub(super) struct FinalizedInstallBuildSnapshot {
    pub(super) release_build_id: ReleaseBuildId,
    pub(super) builder_version: String,
    pub(super) root_role: CanisterRole,
    pub(super) root_manifest_path: std::path::PathBuf,
    pub(super) component_topology: ComponentTopology,
    pub(super) package_by_role: BTreeMap<CanisterRole, String>,
}

/// Complete normal-install authority, separated by whether building is still permitted.
#[derive(Clone, Debug)]
pub(super) enum CompleteInstallBuildSnapshot {
    Workspace(WorkspaceInstallBuildSnapshot),
    Finalized(FinalizedInstallBuildSnapshot),
}

/// Configuration identity plus optional normal-install inputs for one install command.
#[derive(Clone, Debug)]
pub(super) struct ValidatedInstallSnapshot {
    pub(super) app_id: String,
    pub(super) complete_build: Option<CompleteInstallBuildSnapshot>,
    pub(super) release_build: Option<PlannedReleaseBuild>,
}

/// Exact source from which install build inputs may be derived.
pub(super) enum InstallSnapshotSource<'a> {
    DeploymentPlan,
    WorkspaceBuild,
    FinalizedRelease(&'a PlannedReleaseBuild),
}

pub(super) fn resolve_install_snapshot(
    context: &WorkspaceBuildContext,
    root_build_target: &str,
    source: InstallSnapshotSource<'_>,
) -> Result<ValidatedInstallSnapshot, Box<dyn std::error::Error>> {
    let config = AppConfigSnapshot::load(&context.config_path)?;
    let app_id = config.app_id().to_string();

    let complete_build = match source {
        InstallSnapshotSource::DeploymentPlan => None,
        InstallSnapshotSource::WorkspaceBuild => Some(CompleteInstallBuildSnapshot::Workspace(
            resolve_workspace_snapshot(context, root_build_target, &config)?,
        )),
        InstallSnapshotSource::FinalizedRelease(release_build) => {
            Some(CompleteInstallBuildSnapshot::Finalized(
                resolve_finalized_snapshot(context, root_build_target, &config, release_build)?,
            ))
        }
    };

    Ok(ValidatedInstallSnapshot {
        app_id,
        complete_build,
        release_build: None,
    })
}

fn resolve_workspace_snapshot(
    context: &WorkspaceBuildContext,
    root_build_target: &str,
    config: &AppConfigSnapshot,
) -> Result<WorkspaceInstallBuildSnapshot, Box<dyn std::error::Error>> {
    let release_roles = configured_release_roles_from_config(config.model());
    let component_topology = config.model().compile_component_topology()?;
    let mut roles = Vec::with_capacity(release_roles.len() + 1);
    roles.push(root_build_target.to_string());
    roles.extend(release_roles.iter().cloned());

    let specs = resolve_canister_artifact_build_specs(context, config.model(), &roles)?;
    let targets = roles
        .into_iter()
        .zip(specs)
        .map(|(role, spec)| InstallBuildTarget { role, spec })
        .collect::<Vec<_>>();

    let root_target = targets
        .first()
        .ok_or_else(|| "complete install build has no root target".to_string())?;
    let release_version = load_root_package_version(
        &root_target.spec.package_manifest_path,
        &workspace_manifest_path(&context.workspace_root),
    )?;
    let artifact_root = context.artifact_root();
    let manifest_path = root_release_set_manifest_path(&artifact_root);
    let manifest_targets = targets
        .iter()
        .map(|target| RootReleaseSetBuildTarget {
            role: target.role.clone(),
            expected_wasm_gz_path: target.spec.wasm_gz_path.clone(),
            publish_entry: release_roles.iter().any(|role| role == &target.role),
        })
        .collect();
    let application_artifact_targets = targets
        .iter()
        .filter(|target| release_roles.iter().any(|role| role == &target.role))
        .map(|target| application_artifact_target(&context.icp_root, target))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(WorkspaceInstallBuildSnapshot {
        targets,
        manifest: RootReleaseSetBuildSnapshot {
            icp_root: context.icp_root.clone(),
            manifest_path,
            release_version,
            targets: manifest_targets,
        },
        component_topology,
        application_artifact_targets,
    })
}

fn resolve_finalized_snapshot(
    context: &WorkspaceBuildContext,
    root_build_target: &str,
    config: &AppConfigSnapshot,
    release_build: &PlannedReleaseBuild,
) -> Result<FinalizedInstallBuildSnapshot, Box<dyn std::error::Error>> {
    if !matches!(
        release_build.record.state,
        ReleaseBuildPlanState::Finalized { .. }
    ) {
        return Err("retained install snapshot requires a finalized release build".into());
    }
    let release_build_id = release_build.record.release_build_id;
    let component_topology = config.model().compile_component_topology()?;
    let application = load_persisted_application_artifact_union(
        &context.icp_root,
        &component_topology,
        release_build_id,
    )?;
    let infrastructure =
        load_persisted_canic_infrastructure_artifact_manifest(&context.icp_root, release_build_id)?;
    for entry in &infrastructure.manifest.entries {
        if entry.protocol_release_identity != release_build.record.builder_version {
            return Err(format!(
                "finalized infrastructure role {} belongs to Canic {}, not retained builder {}",
                entry.role.as_str(),
                entry.protocol_release_identity,
                release_build.record.builder_version
            )
            .into());
        }
    }

    let mut package_by_role = BTreeMap::new();
    let root_role = CanisterRole::owned(root_build_target.to_string());
    let root_package = declared_package_name(&context.config_path, config.model(), &root_role)?;
    let retained_root = infrastructure
        .manifest
        .entries
        .iter()
        .find(|entry| entry.role == crate::release_set::CanicInfrastructureRole::FleetSubnetRoot)
        .ok_or("finalized infrastructure manifest has no Fleet Subnet Root")?;
    require_package_match(&root_role, &root_package, &retained_root.package)?;
    package_by_role.insert(root_role.clone(), root_package);

    for entry in &application.union.entries {
        let package = declared_package_name(&context.config_path, config.model(), &entry.role)?;
        require_package_match(&entry.role, &package, &entry.package)?;
        package_by_role.insert(entry.role.clone(), package);
    }

    let root_manifest_path = root_release_set_manifest_path(&context.artifact_root());
    let validated_release = validate_finalized_release_build_manifest(
        &context.icp_root,
        release_build_id,
        &root_manifest_path,
    )?;
    if validated_release.record != release_build.record {
        return Err("finalized release build changed while resolving retained artifacts".into());
    }
    let root_manifest = load_root_release_set_manifest(&root_manifest_path)?;
    let root_manifest_path_from_config = declared_role_manifest_path(
        &context.config_path,
        config.model(),
        &CanisterRole::owned(root_build_target.to_string()),
    )
    .map_err(|finding| format!("{}: {}", finding.code(), finding_detail(&finding)))?;
    let root_manifest_version = load_root_package_version(
        &root_manifest_path_from_config,
        &workspace_manifest_path(&context.workspace_root),
    )?;
    if root_manifest.release_version != root_manifest_version {
        return Err(format!(
            "finalized root release-set version {} differs from current App root package {}",
            root_manifest.release_version, root_manifest_version
        )
        .into());
    }
    require_root_manifest_matches_application(&root_manifest, &application.union)?;

    Ok(FinalizedInstallBuildSnapshot {
        release_build_id,
        builder_version: release_build.record.builder_version.clone(),
        root_role,
        root_manifest_path,
        component_topology,
        package_by_role,
    })
}

fn declared_package_name(
    config_path: &Path,
    config: &ConfigModel,
    role: &CanisterRole,
) -> Result<String, Box<dyn std::error::Error>> {
    let manifest_path = declared_role_manifest_path(config_path, config, role)
        .map_err(|finding| format!("{}: {}", finding.code(), finding_detail(&finding)))?;
    let source = fs::read_to_string(&manifest_path)?;
    let document = toml::from_str::<toml::Value>(&source)?;
    document
        .get("package")
        .and_then(toml::Value::as_table)
        .and_then(|package| package.get("name"))
        .and_then(toml::Value::as_str)
        .filter(|name| !name.trim().is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| {
            format!(
                "declared role {role} package manifest {} has no package.name",
                manifest_path.display()
            )
            .into()
        })
}

fn require_package_match(
    role: &CanisterRole,
    current: &str,
    retained: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if current == retained {
        return Ok(());
    }
    Err(format!(
        "finalized role {role} package {retained} differs from current App package {current}"
    )
    .into())
}

fn require_root_manifest_matches_application(
    root: &crate::release_set::RootReleaseSetManifest,
    application: &crate::release_set::ApplicationArtifactUnion,
) -> Result<(), Box<dyn std::error::Error>> {
    if root.entries.len() != application.entries.len() {
        return Err("finalized root release-set and application union role counts differ".into());
    }
    for application_entry in &application.entries {
        let role = application_entry.role.as_str();
        let root_entry = root
            .entries
            .iter()
            .find(|entry| entry.role == role)
            .ok_or_else(|| format!("finalized root release-set has no application role {role}"))?;
        let matches = root_entry.artifact_relative_path == application_entry.wasm_gz_relative_path
            && root_entry.candid_sha256_hex == hex_bytes(application_entry.candid_sha256)
            && root_entry.protocol_profile_digest_hex
                == application_entry.protocol_profile_digest.to_string()
            && root_entry.payload_size_bytes == application_entry.wasm_gz_size_bytes
            && root_entry.payload_sha256_hex == application_entry.wasm_gz_sha256_hex;
        if !matches {
            return Err(format!(
                "finalized root release-set evidence differs from application union for role {role}"
            )
            .into());
        }
    }
    Ok(())
}

fn application_artifact_target(
    icp_root: &Path,
    target: &InstallBuildTarget,
) -> Result<ApplicationArtifactBuildTarget, Box<dyn std::error::Error>> {
    Ok(ApplicationArtifactBuildTarget {
        role: CanisterRole::owned(target.role.clone()),
        package: target.spec.package_name.clone(),
        wasm_relative_path: artifact_relative_path(icp_root, &target.spec.wasm_path)?,
        wasm_gz_relative_path: artifact_relative_path(icp_root, &target.spec.wasm_gz_path)?,
    })
}

fn artifact_relative_path(
    icp_root: &Path,
    path: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let relative = path.strip_prefix(icp_root).map_err(|_| {
        format!(
            "planned application artifact {} is outside ICP root {}",
            path.display(),
            icp_root.display()
        )
    })?;
    let relative = relative.to_str().ok_or_else(|| {
        format!(
            "planned application artifact path is not UTF-8: {}",
            path.display()
        )
    })?;
    crate::release_set::validate_release_artifact_relative_path(relative)?;
    Ok(relative.to_string())
}
