use super::wasm::WasmBuildProfile;
use std::{
    fs, io,
    path::Path,
    process::{Command, Output},
    time::SystemTime,
};

/// Check whether one artifact is newer than the inputs that define it.
pub fn artifact_is_fresh_against_inputs(
    workspace_root: &Path,
    artifact_path: &Path,
    watched_relative_paths: &[&str],
) -> io::Result<bool> {
    let artifact_mtime = fs::metadata(artifact_path)?.modified()?;
    let newest_input = newest_watched_input_mtime(workspace_root, watched_relative_paths)?;
    Ok(newest_input <= artifact_mtime)
}

/// Check whether a `dfx` artifact exists and is fresh against watched inputs.
#[must_use]
pub fn dfx_artifact_ready(
    workspace_root: &Path,
    artifact_relative_path: &str,
    watched_relative_paths: &[&str],
) -> bool {
    let artifact_path = workspace_root.join(artifact_relative_path);

    match fs::metadata(&artifact_path) {
        Ok(meta) if meta.is_file() && meta.len() > 0 => {
            artifact_is_fresh_against_inputs(workspace_root, &artifact_path, watched_relative_paths)
                .unwrap_or(false)
        }
        _ => false,
    }
}

/// Build all `dfx` canisters while holding a file lock around the build.
pub fn build_dfx_all(
    workspace_root: &Path,
    lock_relative_path: &str,
    network: &str,
    profile: WasmBuildProfile,
) {
    build_dfx_all_with_env(workspace_root, lock_relative_path, network, profile, &[]);
}

/// Build all `dfx` canisters while holding a file lock around the build and applying
/// additional environment overrides.
pub fn build_dfx_all_with_env(
    workspace_root: &Path,
    lock_relative_path: &str,
    network: &str,
    profile: WasmBuildProfile,
    extra_env: &[(&str, &str)],
) {
    let output = run_dfx_build_with_lock(
        workspace_root,
        lock_relative_path,
        network,
        profile,
        extra_env,
    );
    assert!(
        output.status.success(),
        "dfx build --all failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// Walk watched files and directories and return the newest modification time.
fn newest_watched_input_mtime(
    workspace_root: &Path,
    watched_relative_paths: &[&str],
) -> io::Result<SystemTime> {
    let mut newest = SystemTime::UNIX_EPOCH;

    for relative in watched_relative_paths {
        let path = workspace_root.join(relative);
        newest = newest.max(newest_path_mtime(&path)?);
    }

    Ok(newest)
}

// Recursively compute the newest modification time under one watched path.
fn newest_path_mtime(path: &Path) -> io::Result<SystemTime> {
    let metadata = fs::metadata(path)?;
    let mut newest = metadata.modified()?;

    if metadata.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            newest = newest.max(newest_path_mtime(&entry.path())?);
        }
    }

    Ok(newest)
}

// Invoke `dfx canister create --all` and `dfx build --all` under one file lock when `flock`
// is available.
fn run_dfx_build_with_lock(
    workspace_root: &Path,
    lock_relative_path: &str,
    network: &str,
    profile: WasmBuildProfile,
    extra_env: &[(&str, &str)],
) -> Output {
    let lock_file = workspace_root.join(lock_relative_path);
    let target_dir = workspace_root.join("target/dfx-build");
    if let Some(parent) = lock_file.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::create_dir_all(&target_dir);

    let mut flock = Command::new("flock");
    flock
        .current_dir(workspace_root)
        .arg(lock_file.as_os_str())
        .arg("bash")
        .env("DFX_NETWORK", network)
        .env("RELEASE", profile.dfx_release_value())
        .env("CARGO_TARGET_DIR", &target_dir)
        .args([
            "-lc",
            "dfx canister create --all -qq >/dev/null 2>&1 || true\n\
             dfx build --all",
        ]);
    for (key, value) in extra_env {
        flock.env(key, value);
    }

    match flock.output() {
        Ok(output) => output,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            run_dfx_build(workspace_root, network, profile, extra_env)
        }
        Err(err) => panic!("failed to run `flock` for `dfx build --all`: {err}"),
    }
}

// Invoke `dfx canister create --all` and `dfx build --all` directly when `flock` is
// unavailable.
fn run_dfx_build(
    workspace_root: &Path,
    network: &str,
    profile: WasmBuildProfile,
    extra_env: &[(&str, &str)],
) -> Output {
    let target_dir = workspace_root.join("target/dfx-build");
    let _ = fs::create_dir_all(&target_dir);

    let mut create = Command::new("dfx");
    create
        .current_dir(workspace_root)
        .env("DFX_NETWORK", network)
        .env("RELEASE", profile.dfx_release_value())
        .env("CARGO_TARGET_DIR", &target_dir)
        .args(["canister", "create", "--all", "-qq"]);
    for (key, value) in extra_env {
        create.env(key, value);
    }
    let _ = create.output();

    let mut build = Command::new("dfx");
    build
        .current_dir(workspace_root)
        .env("DFX_NETWORK", network)
        .env("RELEASE", profile.dfx_release_value())
        .env("CARGO_TARGET_DIR", &target_dir)
        .args(["build", "--all"]);
    for (key, value) in extra_env {
        build.env(key, value);
    }

    build.output().expect("failed to run `dfx build --all`")
}
