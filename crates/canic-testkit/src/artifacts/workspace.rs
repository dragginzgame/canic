use std::{
    env,
    path::{Path, PathBuf},
};

/// Resolve the workspace root from a crate manifest directory.
#[must_use]
pub fn workspace_root_for(crate_manifest_dir: &str) -> PathBuf {
    PathBuf::from(crate_manifest_dir)
        .parent()
        .and_then(|path| path.parent())
        .map(PathBuf::from)
        .expect("workspace root")
}

/// Resolve an optional prebuilt wasm directory override from the environment.
#[must_use]
pub fn prebuilt_wasm_dir(env_var: &str) -> Option<PathBuf> {
    env::var(env_var).ok().map(PathBuf::from)
}

/// Return a stable target directory for host-side wasm test artifacts.
#[must_use]
pub fn test_target_dir(workspace_root: &Path, name: &str) -> PathBuf {
    workspace_root.join("target").join(name)
}
