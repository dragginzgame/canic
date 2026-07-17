use crate::workspace_discovery::{
    CanisterManifestError, resolve_canister_manifest_from_metadata_under,
};
use std::path::{Path, PathBuf};

use super::workspace::canisters_root;

// Resolve the downstream root canister manifest path.
pub fn root_manifest_path(workspace_root: &Path) -> Result<PathBuf, CanisterManifestError> {
    canister_manifest_path(workspace_root, "root")
}

// Resolve the downstream manifest path for one visible canister role.
pub fn canister_manifest_path(
    workspace_root: &Path,
    canister_name: &str,
) -> Result<PathBuf, CanisterManifestError> {
    let root = canisters_root(workspace_root);
    resolve_canister_manifest_from_metadata_under(workspace_root, canister_name, &root)
}
