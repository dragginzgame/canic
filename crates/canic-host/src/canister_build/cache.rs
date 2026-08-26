//! Module: canic_host::canister_build::cache
//!
//! Responsibility: isolate reusable Cargo state created by canister artifact builds.
//! Does not own: canonical `.icp` artifacts, build profiles, or deployment orchestration.
//! Boundary: resolves one dedicated Wasm target directory while respecting explicit Cargo input.

use std::{
    env,
    ffi::OsStr,
    fs, io,
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
    configure_implicit_sccache(
        command,
        env::var_os("RUSTC_WRAPPER").as_deref(),
        env::var_os("PATH").as_deref(),
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

fn resolve_implicit_sccache_wrapper(
    explicit_wrapper: Option<&OsStr>,
    search_path: Option<&OsStr>,
) -> Option<PathBuf> {
    if explicit_wrapper.is_some() {
        return None;
    }
    search_path.and_then(|search_path| {
        env::split_paths(search_path)
            .map(|directory| directory.join(sccache_executable_name()))
            .find(|candidate| is_executable_file(candidate))
    })
}

fn configure_implicit_sccache(
    command: &mut Command,
    explicit_wrapper: Option<&OsStr>,
    search_path: Option<&OsStr>,
) {
    if let Some(sccache) = resolve_implicit_sccache_wrapper(explicit_wrapper, search_path) {
        command.env("RUSTC_WRAPPER", sccache);
    }
}

#[cfg(windows)]
const fn sccache_executable_name() -> &'static str {
    "sccache.exe"
}

#[cfg(not(windows))]
const fn sccache_executable_name() -> &'static str {
    "sccache"
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;

        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;
    use std::{ffi::OsString, sync::mpsc, thread, time::Duration};

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
    fn install_build_discovers_sccache_without_overriding_explicit_wrapper() {
        let root = temp_dir("canister-build-sccache");
        let bin = root.join("bin");
        fs::create_dir_all(&bin).expect("create cache bin directory");
        let sccache = bin.join(sccache_executable_name());
        fs::write(&sccache, b"cache").expect("write cache executable");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;

            fs::set_permissions(&sccache, fs::Permissions::from_mode(0o755))
                .expect("make cache executable");
        }
        let search_path = env::join_paths([&bin]).expect("join cache search path");

        let mut discovered = Command::new("cargo");
        configure_implicit_sccache(&mut discovered, None, Some(&search_path));
        assert_eq!(
            discovered
                .get_envs()
                .find(|(name, _)| *name == "RUSTC_WRAPPER")
                .and_then(|(_, value)| value),
            Some(sccache.as_os_str())
        );

        let explicit_wrapper = OsString::from("custom-wrapper");
        let mut explicit = Command::new("cargo");
        explicit.env("RUSTC_WRAPPER", &explicit_wrapper);
        configure_implicit_sccache(
            &mut explicit,
            Some(explicit_wrapper.as_os_str()),
            Some(&search_path),
        );
        assert_eq!(
            explicit
                .get_envs()
                .find(|(name, _)| *name == "RUSTC_WRAPPER")
                .and_then(|(_, value)| value),
            Some(explicit_wrapper.as_os_str())
        );

        fs::remove_dir_all(root).expect("remove cache test root");
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
