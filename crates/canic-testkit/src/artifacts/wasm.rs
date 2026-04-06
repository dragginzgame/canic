use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub const INTERNAL_TEST_ENDPOINTS_ENV: (&str, &str) = ("CANIC_INTERNAL_TEST_ENDPOINTS", "1");

///
/// WasmBuildProfile
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WasmBuildProfile {
    Debug,
    Fast,
    Release,
}

impl WasmBuildProfile {
    /// Return the Cargo profile arguments for this build profile.
    #[must_use]
    pub const fn cargo_args(self) -> &'static [&'static str] {
        match self {
            Self::Debug => &[],
            Self::Fast => &["--profile", "fast"],
            Self::Release => &["--release"],
        }
    }

    /// Return the target-directory component for this build profile.
    #[must_use]
    pub const fn target_dir_name(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Fast => "fast",
            Self::Release => "release",
        }
    }

    /// Return the explicit Canic wasm-profile selector for local builders.
    #[must_use]
    pub const fn canic_wasm_profile_value(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Fast => "fast",
            Self::Release => "release",
        }
    }
}

/// Resolve the wasm artifact path for one crate under a target directory.
#[must_use]
pub fn wasm_path(target_dir: &Path, crate_name: &str, profile: WasmBuildProfile) -> PathBuf {
    target_dir
        .join("wasm32-unknown-unknown")
        .join(profile.target_dir_name())
        .join(format!("{crate_name}.wasm"))
}

/// Check whether all requested wasm artifacts already exist.
#[must_use]
pub fn wasm_artifacts_ready(
    target_dir: &Path,
    canisters: &[&str],
    profile: WasmBuildProfile,
) -> bool {
    canisters
        .iter()
        .all(|name| wasm_path(target_dir, name, profile).is_file())
}

/// Read a compiled wasm artifact for one crate.
#[must_use]
pub fn read_wasm(target_dir: &Path, crate_name: &str, profile: WasmBuildProfile) -> Vec<u8> {
    let path = wasm_path(target_dir, crate_name, profile);
    fs::read(&path).unwrap_or_else(|err| panic!("failed to read {crate_name} wasm: {err}"))
}

/// Build one or more wasm canisters into the provided target directory.
pub fn build_wasm_canisters(
    workspace_root: &Path,
    target_dir: &Path,
    packages: &[&str],
    profile: WasmBuildProfile,
    extra_env: &[(&str, &str)],
) {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(workspace_root);
    cmd.env("CARGO_TARGET_DIR", target_dir);
    cmd.env("DFX_NETWORK", "local");
    cmd.args(["build", "--target", "wasm32-unknown-unknown"]);
    cmd.args(profile.cargo_args());

    for (key, value) in extra_env {
        cmd.env(key, value);
    }

    for name in packages {
        cmd.args(["-p", name]);
    }

    let output = cmd.output().expect("failed to run cargo build");
    assert!(
        output.status.success(),
        "cargo build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Build one or more wasm canisters with the internal test endpoint surface enabled.
pub fn build_internal_test_wasm_canisters(
    workspace_root: &Path,
    target_dir: &Path,
    packages: &[&str],
    profile: WasmBuildProfile,
) {
    build_wasm_canisters(
        workspace_root,
        target_dir,
        packages,
        profile,
        &[INTERNAL_TEST_ENDPOINTS_ENV],
    );
}

/// Build one or more wasm canisters with the internal test endpoint surface
/// enabled plus any additional caller-provided environment overrides.
pub fn build_internal_test_wasm_canisters_with_env<'a>(
    workspace_root: &Path,
    target_dir: &Path,
    packages: &[&str],
    profile: WasmBuildProfile,
    extra_env: &[(&'a str, &'a str)],
) {
    let mut build_env = Vec::with_capacity(extra_env.len() + 1);
    build_env.push(INTERNAL_TEST_ENDPOINTS_ENV);
    build_env.extend_from_slice(extra_env);
    build_wasm_canisters(workspace_root, target_dir, packages, profile, &build_env);
}
