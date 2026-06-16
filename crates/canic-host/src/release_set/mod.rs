//! Release-set discovery, manifest emission, and staging helpers.

use std::time::{SystemTime, UNIX_EPOCH};

mod config;
mod manifest;
mod paths;
pub(crate) mod stage;

pub use config::{
    AttachedFleetRole, ConfiguredPoolExpectation, ConfiguredRoleLifecycle, DeclaredFleetRole,
    LOCAL_ROOT_MIN_READY_CYCLES, RenamedFleetRole, attach_fleet_role, configured_bootstrap_roles,
    configured_controllers, configured_deployable_roles, configured_fleet_name,
    configured_install_targets, configured_local_root_create_cycles, configured_pool_expectations,
    configured_release_roles, configured_role_auto_create, configured_role_capabilities,
    configured_role_details, configured_role_kinds, configured_role_lifecycle,
    configured_role_metrics_profiles, configured_role_topups, declare_fleet_role,
    matching_fleet_config_paths, rename_fleet_role,
};
pub use manifest::{
    ReleaseSetEntry, RootReleaseSetManifest, emit_root_release_set_manifest,
    emit_root_release_set_manifest_if_ready, emit_root_release_set_manifest_with_config,
    load_root_release_set_manifest,
};
pub use paths::{
    canister_manifest_path, canisters_root, config_path, display_workspace_path, icp_root,
    load_root_package_version, load_workspace_package_version, resolve_artifact_root,
    root_manifest_path, root_release_set_manifest_path, workspace_manifest_path, workspace_root,
};
use stage::build_release_set_entry;
pub(crate) use stage::icp_query_on_network;
pub use stage::{resume_root_bootstrap, stage_root_release_set};

#[cfg(test)]
use stage::read_release_artifact;

#[cfg(test)]
use config::{
    attach_fleet_role_source, configured_bootstrap_roles_from_source,
    configured_controllers_from_source, configured_deployable_roles_from_source,
    configured_fleet_name_from_source, configured_local_root_create_cycles_from_source,
    configured_pool_expectations_from_source, configured_release_roles_from_source,
    configured_role_auto_create_from_source, configured_role_capabilities_from_source,
    configured_role_details_from_source, configured_role_kinds_from_source,
    configured_role_lifecycle_from_source, configured_role_metrics_profiles_from_source,
    configured_role_topups_from_source, declare_fleet_role_source, rename_fleet_role_source,
};

pub(super) const CANISTERS_ROOT_RELATIVE: &str = "fleets";
pub(super) const ROOT_CONFIG_FILE: &str = "canic.toml";
pub(super) const WORKSPACE_MANIFEST_RELATIVE: &str = "Cargo.toml";
pub(crate) const ROOT_RELEASE_SET_MANIFEST_FILE: &str = "root.release-set.json";
pub(super) const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];
pub(super) const WASM_MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6d];

// Read the current host wall clock so staged manifests use a stable whole-second
// timestamp without depending on an exported root time endpoint.
pub(super) fn root_time_secs(root_canister: &str) -> Result<u64, Box<dyn std::error::Error>> {
    let _ = root_canister;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("system clock before unix epoch: {err}"))?;
    Ok(now.as_secs())
}

#[cfg(test)]
mod tests;
