use ic_testkit::artifacts::{WasmBuildProfile, WatchedInputSnapshot, build_wasm_canisters};
use std::{
    fs, io,
    path::Path,
    process::{Command, Output},
};

pub const INTERNAL_TEST_ENDPOINTS_ENV: (&str, &str) = ("CANIC_INTERNAL_TEST_ENDPOINTS", "1");
const ICP_BUILD_ENV_STAMP_RELATIVE: &str = ".icp/canic-build-env.stamp";

pub fn build_internal_test_wasm_canisters(
    workspace_root: &Path,
    target_dir: &Path,
    packages: &[&str],
    profile: WasmBuildProfile,
) {
    build_internal_test_wasm_canisters_with_env(workspace_root, target_dir, packages, profile, &[]);
}

pub fn build_internal_test_wasm_canisters_with_env(
    workspace_root: &Path,
    target_dir: &Path,
    packages: &[&str],
    profile: WasmBuildProfile,
    extra_env: &[(&str, &str)],
) {
    let mut build_env = Vec::with_capacity(extra_env.len() + 2);
    build_env.push(("ICP_ENVIRONMENT", "local"));
    build_env.push(INTERNAL_TEST_ENDPOINTS_ENV);
    build_env.extend_from_slice(extra_env);
    build_wasm_canisters(workspace_root, target_dir, packages, profile, &build_env);
}

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

const fn canic_wasm_profile_value(profile: WasmBuildProfile) -> &'static str {
    match profile {
        WasmBuildProfile::Debug => "debug",
        WasmBuildProfile::Fast => "fast",
        WasmBuildProfile::Release => "release",
    }
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
        format!("CANIC_WASM_PROFILE={}", canic_wasm_profile_value(profile)),
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
        .env("CANIC_WASM_PROFILE", canic_wasm_profile_value(profile))
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
        .env("CANIC_WASM_PROFILE", canic_wasm_profile_value(profile))
        .env("CARGO_TARGET_DIR", &target_dir)
        .arg("scripts/ci/build-ci-wasm-artifacts.sh");
    for (key, value) in extra_env {
        build.env(key, value);
    }

    build
        .output()
        .expect("failed to run local artifact build helper")
}
