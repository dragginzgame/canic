use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

///
/// WasmBuildProfile
///

const WASM_RELEASE_PROFILE_NAME: &str = "wasm-release";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WasmBuildProfile {
    Debug,
    Release,
}

impl WasmBuildProfile {
    /// Return the Cargo profile arguments for this build profile.
    #[must_use]
    pub const fn cargo_args(self) -> &'static [&'static str] {
        match self {
            Self::Debug => &[],
            Self::Release => &["--profile", WASM_RELEASE_PROFILE_NAME],
        }
    }

    /// Return the target-directory component for this build profile.
    #[must_use]
    pub const fn target_dir_name(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Release => WASM_RELEASE_PROFILE_NAME,
        }
    }

    /// Return the `RELEASE` environment value expected by Canic local builders.
    #[must_use]
    pub const fn dfx_release_value(self) -> &'static str {
        match self {
            Self::Debug => "0",
            Self::Release => "1",
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
