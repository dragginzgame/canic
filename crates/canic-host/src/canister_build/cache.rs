//! Module: canic_host::canister_build::cache
//!
//! Responsibility: isolate and bound transient Cargo state created by canister artifact builds.
//! Does not own: canonical `.icp` artifacts, build profiles, or deployment orchestration.
//! Boundary: resolves one Wasm target directory and removes only its default disposable cache.

use std::{
    env, fs,
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
    env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .map_or_else(
            || workspace_root.join(DEFAULT_WASM_TARGET_RELATIVE),
            |path| absolute_from(workspace_root, path),
        )
}

/// Removes the default transient Wasm target when an install invocation exits.
pub struct DefaultCanisterBuildCacheCleanup {
    path: Option<PathBuf>,
}

impl DefaultCanisterBuildCacheCleanup {
    #[must_use]
    pub fn for_install(workspace_root: &Path) -> Self {
        let path = if env::var_os("CARGO_TARGET_DIR").is_some() {
            None
        } else {
            Some(workspace_root.join(DEFAULT_WASM_TARGET_RELATIVE))
        };
        Self { path }
    }
}

impl Drop for DefaultCanisterBuildCacheCleanup {
    fn drop(&mut self) {
        let Some(path) = self.path.take() else {
            return;
        };
        match fs::remove_dir_all(&path) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => eprintln!(
                "warning: failed to clear transient Wasm build cache {}: {err}",
                path.display()
            ),
        }
    }
}

fn absolute_from(workspace_root: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        workspace_root.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;

    #[test]
    fn cleanup_removes_only_the_selected_transient_cache() {
        let root = temp_dir("canic-wasm-cache-cleanup");
        let cache = root.join(DEFAULT_WASM_TARGET_RELATIVE);
        let retained = root.join("target/retained.txt");
        fs::create_dir_all(&cache).expect("create transient cache");
        fs::write(cache.join("artifact"), b"cache").expect("write transient cache file");
        fs::write(&retained, b"retained").expect("write retained file");

        drop(DefaultCanisterBuildCacheCleanup {
            path: Some(cache.clone()),
        });

        assert!(!cache.exists());
        assert!(retained.is_file());
        fs::remove_dir_all(root).expect("remove test root");
    }
}
