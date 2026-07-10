use ic_testkit::artifacts::{WatchedInputSnapshot, build_wasm_canisters};
use std::{
    fs, io,
    path::{Path, PathBuf},
    process::{Command, Output},
};

pub(super) const INTERNAL_TEST_ENDPOINTS_ENV: (&str, &str) = ("CANIC_INTERNAL_TEST_ENDPOINTS", "1");

///
/// CanicWasmBuildProfile
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanicWasmBuildProfile {
    Debug,
    Fast,
}

impl CanicWasmBuildProfile {
    #[must_use]
    pub(super) const fn cargo_profile_args(self) -> &'static [&'static str] {
        match self {
            Self::Debug => &[],
            Self::Fast => &["--profile", "fast"],
        }
    }

    #[must_use]
    pub const fn target_dir_name(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Fast => "fast",
        }
    }

    #[must_use]
    const fn canic_wasm_profile_value(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Fast => "fast",
        }
    }
}

pub fn build_internal_test_wasm_canisters(
    workspace_root: &Path,
    target_dir: &Path,
    packages: &[&str],
    profile: CanicWasmBuildProfile,
) {
    build_internal_test_wasm_canisters_with_env(workspace_root, target_dir, packages, profile, &[]);
}

pub(super) fn build_internal_test_wasm_canisters_with_env(
    workspace_root: &Path,
    target_dir: &Path,
    packages: &[&str],
    profile: CanicWasmBuildProfile,
    extra_env: &[(&str, &str)],
) {
    let mut cargo_args = profile.cargo_profile_args().to_vec();
    cargo_args.push("--locked");

    let mut build_env = vec![
        ("CARGO_INCREMENTAL", "0"),
        ("ICP_ENVIRONMENT", "local"),
        INTERNAL_TEST_ENDPOINTS_ENV,
    ];
    build_env.extend_from_slice(extra_env);
    build_wasm_canisters(
        workspace_root,
        target_dir,
        packages,
        &cargo_args,
        &build_env,
    );
}

#[must_use]
pub(super) fn icp_artifact_ready_with_snapshot(
    workspace_root: &Path,
    artifact_path: &Path,
    watched_inputs: WatchedInputSnapshot,
    network: &str,
    profile: CanicWasmBuildProfile,
    extra_env: &[(&str, &str)],
) -> bool {
    match fs::metadata(artifact_path) {
        Ok(meta) if meta.is_file() && meta.len() > 0 => {
            watched_inputs
                .artifact_is_fresh(artifact_path)
                .unwrap_or(false)
                && build_stamp_matches(workspace_root, network, profile, extra_env)
        }
        _ => false,
    }
}

pub(super) fn build_icp_all_with_env(
    workspace_root: &Path,
    lock_path: &Path,
    network: &str,
    profile: CanicWasmBuildProfile,
    extra_env: &[(&str, &str)],
) {
    let output =
        run_local_artifact_build_with_lock(workspace_root, lock_path, network, profile, extra_env);
    assert!(
        output.status.success(),
        "local artifact build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    write_build_stamp(workspace_root, network, profile, extra_env)
        .expect("write local artifact build env stamp");
}

fn build_stamp_matches(
    workspace_root: &Path,
    network: &str,
    profile: CanicWasmBuildProfile,
    extra_env: &[(&str, &str)],
) -> bool {
    fs::read_to_string(icp_build_env_stamp_path(workspace_root))
        .is_ok_and(|current| current == build_stamp_contents(network, profile, extra_env))
}

fn write_build_stamp(
    workspace_root: &Path,
    network: &str,
    profile: CanicWasmBuildProfile,
    extra_env: &[(&str, &str)],
) -> io::Result<()> {
    let stamp_path = icp_build_env_stamp_path(workspace_root);
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
    profile: CanicWasmBuildProfile,
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

fn run_local_artifact_build_with_lock(
    workspace_root: &Path,
    lock_file: &Path,
    network: &str,
    profile: CanicWasmBuildProfile,
    extra_env: &[(&str, &str)],
) -> Output {
    let target_dir = icp_build_target_dir(workspace_root);
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
        .arg(build_ci_wasm_artifacts_script(workspace_root));
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

fn run_local_artifact_build(
    workspace_root: &Path,
    network: &str,
    profile: CanicWasmBuildProfile,
    extra_env: &[(&str, &str)],
) -> Output {
    let target_dir = icp_build_target_dir(workspace_root);
    let _ = fs::create_dir_all(&target_dir);

    let mut build = Command::new("bash");
    build
        .current_dir(workspace_root)
        .env("ICP_ENVIRONMENT", network)
        .env("CANIC_WASM_PROFILE", profile.canic_wasm_profile_value())
        .env("CARGO_TARGET_DIR", &target_dir)
        .arg(build_ci_wasm_artifacts_script(workspace_root));
    for (key, value) in extra_env {
        build.env(key, value);
    }

    build
        .output()
        .expect("failed to run local artifact build helper")
}

fn icp_build_env_stamp_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join(".icp").join("canic-build-env.stamp")
}

fn icp_build_target_dir(workspace_root: &Path) -> PathBuf {
    workspace_root.join("target").join("icp-build")
}

fn build_ci_wasm_artifacts_script(workspace_root: &Path) -> PathBuf {
    workspace_root
        .join("scripts")
        .join("ci")
        .join("build-ci-wasm-artifacts.sh")
}
