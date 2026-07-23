//! Module: install_root::build_snapshot
//!
//! Responsibility: resolve one immutable configuration-backed complete-build input set.
//! Does not own: Cargo execution, artifact bytes, or manifest publication.
//! Boundary: builders and the manifest writer consume only values derived here.

use crate::{
    canister_build::{
        CanisterArtifactBuildSpec, WorkspaceBuildContext, resolve_canister_artifact_build_spec,
    },
    release_build::PlannedReleaseBuild,
    release_set::{
        AppConfigSnapshot, RootReleaseSetBuildSnapshot, RootReleaseSetBuildTarget,
        artifact_root_path, configured_release_roles_from_config, load_root_package_version,
        root_release_set_manifest_path, workspace_manifest_path,
    },
};

/// One target whose package and output paths were admitted from the install snapshot.
#[derive(Clone, Debug)]
pub(super) struct InstallBuildTarget {
    pub(super) role: String,
    pub(super) spec: CanisterArtifactBuildSpec,
}

/// All inputs shared by the builders and normal release-set writer.
#[derive(Clone, Debug)]
pub(super) struct CompleteInstallBuildSnapshot {
    pub(super) targets: Vec<InstallBuildTarget>,
    pub(super) manifest: RootReleaseSetBuildSnapshot,
}

/// Configuration identity plus optional normal-build inputs for one install command.
#[derive(Clone, Debug)]
pub(super) struct ValidatedInstallSnapshot {
    pub(super) app_id: String,
    pub(super) complete_build: Option<CompleteInstallBuildSnapshot>,
    pub(super) release_build: Option<PlannedReleaseBuild>,
}

pub(super) fn resolve_install_snapshot(
    context: &WorkspaceBuildContext,
    root_build_target: &str,
    uses_deployment_plan: bool,
) -> Result<ValidatedInstallSnapshot, Box<dyn std::error::Error>> {
    let config = AppConfigSnapshot::load(&context.config_path)?;
    let app_id = config.app_id().to_string();

    if uses_deployment_plan {
        return Ok(ValidatedInstallSnapshot {
            app_id,
            complete_build: None,
            release_build: None,
        });
    }

    let release_roles = configured_release_roles_from_config(config.model());
    let mut roles = Vec::with_capacity(release_roles.len() + 1);
    roles.push(root_build_target.to_string());
    roles.extend(release_roles.iter().cloned());

    let mut targets = Vec::with_capacity(roles.len());
    for role in roles {
        let target_context = context.with_role(&role);
        let spec = resolve_canister_artifact_build_spec(&target_context, config.model())?;
        targets.push(InstallBuildTarget { role, spec });
    }

    let root_target = targets
        .first()
        .ok_or_else(|| "complete install build has no root target".to_string())?;
    let release_version = load_root_package_version(
        &root_target.spec.package_manifest_path,
        &workspace_manifest_path(&context.workspace_root),
    )?;
    let artifact_root = artifact_root_path(&context.icp_root, "local");
    let manifest_path = root_release_set_manifest_path(&artifact_root);
    let manifest_targets = targets
        .iter()
        .map(|target| RootReleaseSetBuildTarget {
            role: target.role.clone(),
            expected_wasm_gz_path: target.spec.wasm_gz_path.clone(),
            publish_entry: release_roles.iter().any(|role| role == &target.role),
        })
        .collect();

    Ok(ValidatedInstallSnapshot {
        app_id,
        complete_build: Some(CompleteInstallBuildSnapshot {
            targets,
            manifest: RootReleaseSetBuildSnapshot {
                icp_root: context.icp_root.clone(),
                manifest_path,
                release_version,
                targets: manifest_targets,
            },
        }),
        release_build: None,
    })
}
