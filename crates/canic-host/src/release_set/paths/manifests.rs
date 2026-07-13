use crate::workspace_discovery::resolve_canister_manifest_from_metadata_under;
use std::path::{Path, PathBuf};

use super::workspace::canisters_root;

// Resolve the downstream root canister manifest path.
pub fn root_manifest_path(workspace_root: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
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
                "{err}; selected canister root is {}. Declare the role package in Cargo workspace metadata.",
                root.display()
            )
            .into()
        },
    )
}
