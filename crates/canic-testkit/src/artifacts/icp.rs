use super::wasm::WasmBuildProfile;
use std::{
    fs, io,
    path::Path,
    process::{Command, Output},
    time::SystemTime,
};

const ICP_BUILD_ENV_STAMP_RELATIVE: &str = ".icp/canic-build-env.stamp";

///
/// WatchedInputSnapshot
///

#[derive(Clone, Copy, Debug)]
pub struct WatchedInputSnapshot {
    newest_input_mtime: SystemTime,
}

impl WatchedInputSnapshot {
    /// Capture the newest modification time across all watched inputs once.
    pub fn capture(workspace_root: &Path, watched_relative_paths: &[&str]) -> io::Result<Self> {
        Ok(Self {
            newest_input_mtime: newest_watched_input_mtime(workspace_root, watched_relative_paths)?,
        })
    }

    /// Check whether one artifact is newer than the captured watched inputs.
    pub fn artifact_is_fresh(self, artifact_path: &Path) -> io::Result<bool> {
        let artifact_mtime = fs::metadata(artifact_path)?.modified()?;
        Ok(self.newest_input_mtime <= artifact_mtime)
    }
}

/// Check whether an ICP artifact exists, is fresh, and matches the expected build env.
#[must_use]
pub fn icp_artifact_ready_for_build(
    workspace_root: &Path,
    artifact_relative_path: &str,
    watched_relative_paths: &[&str],
    network: &str,
    profile: WasmBuildProfile,
    extra_env: &[(&str, &str)],
) -> bool {
    let Ok(watched_inputs) = WatchedInputSnapshot::capture(workspace_root, watched_relative_paths)
    else {
        return false;
    };

    icp_artifact_ready_with_snapshot(
        workspace_root,
        artifact_relative_path,
        watched_inputs,
        network,
        profile,
        extra_env,
    )
}

/// Check one ICP artifact against one already-captured watched-input snapshot.
#[must_use]
pub fn icp_artifact_ready_with_snapshot(
    workspace_root: &Path,
    artifact_relative_path: &str,
    watched_inputs: WatchedInputSnapshot,
    network: &str,
    profile: WasmBuildProfile,
    extra_env: &[(&str, &str)],
) -> bool {
    let artifact_path = workspace_root.join(artifact_relative_path);

    match fs::metadata(&artifact_path) {
        Ok(meta) if meta.is_file() && meta.len() > 0 => {
            watched_inputs
                .artifact_is_fresh(&artifact_path)
                .unwrap_or(false)
                && build_stamp_matches(workspace_root, network, profile, extra_env)
        }
        _ => false,
    }
}

/// Build all local `.icp` canister artifacts while holding a file lock around the build and
/// applying additional environment overrides.
pub fn build_icp_all_with_env(
    workspace_root: &Path,
    lock_relative_path: &str,
    network: &str,
    profile: WasmBuildProfile,
    extra_env: &[(&str, &str)],
) {
    let output = run_local_artifact_build_with_lock(
        workspace_root,
        lock_relative_path,
        network,
        profile,
        extra_env,
    );
    assert!(
        output.status.success(),
        "local artifact build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    write_build_stamp(workspace_root, network, profile, extra_env)
        .expect("write local artifact build env stamp");
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

fn build_stamp_matches(
    workspace_root: &Path,
    network: &str,
    profile: WasmBuildProfile,
    extra_env: &[(&str, &str)],
) -> bool {
    fs::read_to_string(workspace_root.join(ICP_BUILD_ENV_STAMP_RELATIVE))
        .is_ok_and(|current| current == build_stamp_contents(network, profile, extra_env))
}

fn write_build_stamp(
    workspace_root: &Path,
    network: &str,
    profile: WasmBuildProfile,
    extra_env: &[(&str, &str)],
) -> io::Result<()> {
    let stamp_path = workspace_root.join(ICP_BUILD_ENV_STAMP_RELATIVE);
    if let Some(parent) = stamp_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        stamp_path,
        build_stamp_contents(network, profile, extra_env),
    )
}

fn build_stamp_contents(
    network: &str,
    profile: WasmBuildProfile,
    extra_env: &[(&str, &str)],
) -> String {
    let mut lines = vec![
        format!("ICP_ENVIRONMENT={network}"),
        format!("CANIC_WASM_PROFILE={}", profile.canic_wasm_profile_value()),
    ];

    let mut extra = extra_env.to_vec();
    extra.sort_unstable_by_key(|(left, _)| *left);
    lines.extend(
        extra
            .into_iter()
            .map(|(key, value)| format!("{key}={value}")),
    );
    lines.push(String::new());
    lines.join("\n")
}

// Invoke the shared local artifact build helper under one file lock when `flock` is available.
fn run_local_artifact_build_with_lock(
    workspace_root: &Path,
    lock_relative_path: &str,
    network: &str,
    profile: WasmBuildProfile,
    extra_env: &[(&str, &str)],
) -> Output {
    let lock_file = workspace_root.join(lock_relative_path);
    let target_dir = workspace_root.join("target/icp-build");
    if let Some(parent) = lock_file.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::create_dir_all(&target_dir);

    let mut flock = Command::new("flock");
    flock
        .current_dir(workspace_root)
        .arg(lock_file.as_os_str())
        .arg("bash")
        .env("ICP_ENVIRONMENT", network)
        .env("CANIC_WASM_PROFILE", profile.canic_wasm_profile_value())
        .env("CARGO_TARGET_DIR", &target_dir)
        .arg("scripts/ci/build-ci-wasm-artifacts.sh");
    for (key, value) in extra_env {
        flock.env(key, value);
    }

    match flock.output() {
        Ok(output) => output,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            run_local_artifact_build(workspace_root, network, profile, extra_env)
        }
        Err(err) => panic!("failed to run `flock` for local artifact build: {err}"),
    }
}

// Invoke the shared local artifact build helper directly when `flock` is unavailable.
fn run_local_artifact_build(
    workspace_root: &Path,
    network: &str,
    profile: WasmBuildProfile,
    extra_env: &[(&str, &str)],
) -> Output {
    let target_dir = workspace_root.join("target/icp-build");
    let _ = fs::create_dir_all(&target_dir);

    let mut build = Command::new("bash");
    build
        .current_dir(workspace_root)
        .env("ICP_ENVIRONMENT", network)
        .env("CANIC_WASM_PROFILE", profile.canic_wasm_profile_value())
        .env("CARGO_TARGET_DIR", &target_dir)
        .arg("scripts/ci/build-ci-wasm-artifacts.sh");
    for (key, value) in extra_env {
        build.env(key, value);
    }

    build
        .output()
        .expect("failed to run local artifact build helper")
}

#[cfg(test)]
mod tests {
    use super::{build_stamp_contents, icp_artifact_ready_for_build};
    use crate::artifacts::WasmBuildProfile;
    use std::{
        fs,
        path::PathBuf,
        thread::sleep,
        time::Duration,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_workspace() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("canic-icp-artifact-test-{unique}"));
        fs::create_dir_all(path.join(".icp/local/canisters/root")).expect("create temp workspace");
        path
    }

    #[test]
    fn icp_artifact_ready_requires_matching_build_env_stamp() {
        let workspace_root = temp_workspace();
        let artifact_relative_path = ".icp/local/canisters/root/root.wasm.gz";
        let artifact_path = workspace_root.join(artifact_relative_path);
        fs::write(workspace_root.join("Cargo.toml"), "workspace").expect("write watched input");
        sleep(Duration::from_millis(20));
        fs::write(&artifact_path, b"wasm").expect("write artifact");
        fs::write(
            workspace_root.join(".icp/canic-build-env.stamp"),
            build_stamp_contents("local", WasmBuildProfile::Debug, &[]),
        )
        .expect("write build stamp");

        assert!(icp_artifact_ready_for_build(
            &workspace_root,
            artifact_relative_path,
            &["Cargo.toml"],
            "local",
            WasmBuildProfile::Debug,
            &[],
        ));
        assert!(!icp_artifact_ready_for_build(
            &workspace_root,
            artifact_relative_path,
            &["Cargo.toml"],
            "local",
            WasmBuildProfile::Debug,
            &[("RUSTFLAGS", "--cfg canic_test_small_wasm_store")],
        ));

        let _ = fs::remove_dir_all(workspace_root);
    }
}
