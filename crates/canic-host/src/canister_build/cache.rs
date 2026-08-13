//! Module: canic_host::canister_build::cache
//!
//! Responsibility: isolate reusable Cargo state created by canister artifact builds.
//! Does not own: canonical `.icp` artifacts, build profiles, or deployment orchestration.
//! Boundary: resolves one dedicated Wasm target directory while respecting explicit Cargo input.

use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::Command,
};

use crate::durable_io::{RegularFileLockError, lock_regular_file_with_parents};

const DEFAULT_WASM_TARGET_RELATIVE: &str = "target/canic-wasm";
const CANISTER_BUILD_LOCK_RELATIVE: &str = ".canic/locks/canister-artifact-build.lock";

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

/// Lock the complete shared Cargo-target build and artifact-materialization boundary.
pub fn lock_canister_build_target(workspace_root: &Path) -> io::Result<fs::File> {
    let path = workspace_root.join(CANISTER_BUILD_LOCK_RELATIVE);
    lock_regular_file_with_parents(&path).map_err(|error| match error {
        RegularFileLockError::NotRegular => io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Canic artifact-build lock is not a regular file: {}",
                path.display()
            ),
        ),
        RegularFileLockError::Io(source) => io::Error::new(
            source.kind(),
            format!(
                "failed to lock Canic artifact-build target {}: {source}",
                path.display()
            ),
        ),
        #[cfg(windows)]
        RegularFileLockError::UnsupportedPlatform => io::Error::new(
            io::ErrorKind::Unsupported,
            "Canic artifact-build locking is unsupported on Windows",
        ),
    })
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
    use crate::test_support::temp_dir;
    use std::{sync::mpsc, thread, time::Duration};

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

    #[test]
    fn artifact_materialization_lock_excludes_a_second_builder() {
        let root = temp_dir("canic-artifact-build-lock");
        let _ = fs::remove_dir_all(&root);
        let lock_path = root.join(CANISTER_BUILD_LOCK_RELATIVE);
        assert!(lock_path.starts_with(root.join(".canic")));
        assert!(!lock_path.starts_with(root.join("target")));
        let first = lock_canister_build_target(&root).expect("acquire first build lock");
        let contender_root = root.clone();
        let (attempted_tx, attempted_rx) = mpsc::channel();
        let (acquired_tx, acquired_rx) = mpsc::channel();
        let contender = thread::spawn(move || {
            attempted_tx.send(()).expect("report lock attempt");
            let acquired = lock_canister_build_target(&contender_root).is_ok();
            acquired_tx.send(acquired).expect("report lock result");
        });

        attempted_rx.recv().expect("observe contender attempt");
        assert!(
            acquired_rx
                .recv_timeout(Duration::from_millis(100))
                .is_err()
        );
        drop(first);
        assert_eq!(acquired_rx.recv_timeout(Duration::from_secs(2)), Ok(true));
        contender.join().expect("join lock contender");
        fs::remove_dir_all(root).expect("clean build-lock fixture");
    }
}
