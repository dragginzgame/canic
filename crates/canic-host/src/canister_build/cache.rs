//! Module: canic_host::canister_build::cache
//!
//! Responsibility: isolate reusable Cargo state created by canister artifact builds.
//! Does not own: canonical `.icp` artifacts, build profiles, or deployment orchestration.
//! Boundary: resolves one dedicated Wasm target directory while respecting explicit Cargo input.

use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

const DEFAULT_WASM_TARGET_RELATIVE: &str = "target/canic-wasm";

pub fn configure_canister_cargo_command(command: &mut Command, workspace_root: &Path) {
    command.env("CARGO_INCREMENTAL", "0").env(
        "CARGO_TARGET_DIR",
        canister_build_target_root(workspace_root),
    );
}

#[must_use]
pub fn canister_build_target_root(workspace_root: &Path) -> PathBuf {
    resolve_canister_build_target_root(
        workspace_root,
        env::var_os("CARGO_TARGET_DIR").map(PathBuf::from),
    )
}

fn resolve_canister_build_target_root(
    workspace_root: &Path,
    configured_target: Option<PathBuf>,
) -> PathBuf {
    configured_target.map_or_else(
        || workspace_root.join(DEFAULT_WASM_TARGET_RELATIVE),
        |path| {
            if path.is_absolute() {
                path
            } else {
                workspace_root.join(path)
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_target_is_a_dedicated_reusable_workspace_cache() {
        let root = Path::new("/workspace");

        assert_eq!(
            resolve_canister_build_target_root(root, None),
            Path::new("/workspace/target/canic-wasm")
        );
    }

    #[test]
    fn configured_relative_target_remains_workspace_relative() {
        let root = Path::new("/workspace");

        assert_eq!(
            resolve_canister_build_target_root(root, Some(PathBuf::from("custom-target"))),
            Path::new("/workspace/custom-target")
        );
    }
}
