//! Release-set discovery, manifest emission, and staging helpers.

use std::time::{SystemTime, UNIX_EPOCH};

mod config;
mod manifest;
mod paths;
mod stage;

pub(crate) use config::configured_release_roles_from_config;
pub use config::{
    AppConfigDeclaration, AppConfigError, AppConfigIoOperation, AppConfigMutationConflict,
    AppConfigNameField, AppConfigNameIssue, AppConfigOperation, AppConfigPackageIssue,
    AppConfigSnapshot, AppConfigTomlOperation, AttachedFleetRole, ConfiguredPoolExpectation,
    ConfiguredRoleLifecycle, DeclaredFleetRole, LOCAL_ROOT_MIN_READY_CYCLES, RenamedFleetRole,
    attach_fleet_role, declare_fleet_role, plan_attach_fleet_role, plan_declare_fleet_role,
    plan_rename_fleet_role, read_app_config_identity, rename_fleet_role,
};
pub use manifest::{ReleaseSetEntry, RootReleaseSetManifest, load_root_release_set_manifest};
pub(crate) use manifest::{
    RootReleaseSetBuildSnapshot, RootReleaseSetBuildTarget,
    emit_root_release_set_manifest_from_build, validate_root_release_set_manifest,
};
pub use paths::{
    ArtifactRootError, WorkspaceDiscoveryError, artifact_root_path, canisters_root, config_path,
    display_workspace_path, icp_root, load_root_package_version, load_workspace_package_version,
    resolve_artifact_root, root_release_set_manifest_path, workspace_manifest_path, workspace_root,
};
pub(crate) use stage::icp_query_in_environment;
use stage::{build_release_set_entry, validate_release_artifact_relative_path};
pub use stage::{resume_root_bootstrap, stage_root_release_set};

pub(super) const CANISTERS_ROOT_RELATIVE: &str = "fleets";
pub(super) const ROOT_CONFIG_FILE: &str = "canic.toml";
pub(super) const WORKSPACE_MANIFEST_RELATIVE: &str = "Cargo.toml";
pub(crate) const ROOT_RELEASE_SET_MANIFEST_FILE: &str = "root.release-set.json";
pub(super) const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];
pub(super) const WASM_MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6d];

// Read the current host wall clock so staged manifests use a stable whole-second
// timestamp without depending on an exported root time endpoint.
pub(super) fn root_time_secs() -> Result<u64, Box<dyn std::error::Error>> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("system clock before unix epoch: {err}"))?;
    Ok(now.as_secs())
}

#[cfg(test)]
mod tests;
