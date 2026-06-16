use crate::workspace_discovery::{
    normalize_workspace_path, resolve_canister_manifest_from_metadata_under,
};
use std::path::{Path, PathBuf};

use super::workspace::canisters_root;

// Resolve the downstream root canister manifest path.
pub fn root_manifest_path(workspace_root: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(path) = std::env::var_os("CANIC_ROOT_MANIFEST_PATH") {
        return Ok(normalize_workspace_path(
            workspace_root,
            PathBuf::from(path),
        ));
    }

    canister_manifest_path(workspace_root, "root")
}

// Resolve the downstream manifest path for one visible canister role.
pub fn canister_manifest_path(
    workspace_root: &Path,
    canister_name: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let root = canisters_root(workspace_root);
    resolve_canister_manifest_from_metadata_under(workspace_root, canister_name, &root).map_err(
        |err| {
            format!(
                "{err}; selected canister root is {}. Set CANIC_CANISTERS_ROOT or CANIC_CONFIG_PATH so it points at the fleet canister directory.",
                root.display()
            )
            .into()
        },
    )
}
