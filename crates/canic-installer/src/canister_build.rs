use crate::{
    bootstrap_store::{
        BootstrapWasmStoreBuildOutput, BootstrapWasmStoreBuildProfile,
        build_bootstrap_wasm_store_artifact,
    },
    release_set::{
        canister_manifest_path, dfx_root, emit_root_release_set_manifest_if_ready, workspace_root,
    },
};
use flate2::{Compression, GzBuilder};
use serde_json::Value;
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
};

const ROOT_ROLE: &str = "root";
const WASM_STORE_ROLE: &str = "wasm_store";
const LOCAL_ARTIFACT_ROOT_RELATIVE: &str = ".dfx/local/canisters";
const WASM_TARGET: &str = "wasm32-unknown-unknown";

///
/// CanisterBuildProfile
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanisterBuildProfile {
    Debug,
    Fast,
    Release,
}

impl CanisterBuildProfile {
    // Resolve the current requested build profile from the explicit Canic wasm selector.
    #[must_use]
    pub fn current() -> Self {
        match std::env::var("CANIC_WASM_PROFILE").ok().as_deref() {
            Some("debug") => Self::Debug,
            Some("fast") => Self::Fast,
            _ => Self::Release,
        }
    }

    // Return the cargo profile flags for one Canic canister build.
    #[must_use]
    pub const fn cargo_args(self) -> &'static [&'static str] {
        match self {
            Self::Debug => &[],
            Self::Fast => &["--profile", "fast"],
            Self::Release => &["--release"],
        }
    }

    // Return the target-profile directory name for one Canic canister build.
    #[must_use]
    pub const fn target_dir_name(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Fast => "fast",
            Self::Release => "release",
        }
    }
}

impl From<CanisterBuildProfile> for BootstrapWasmStoreBuildProfile {
    // Reuse the same release/debug switch for the hidden bootstrap store build.
    fn from(value: CanisterBuildProfile) -> Self {
        match value {
            CanisterBuildProfile::Debug => Self::Debug,
            CanisterBuildProfile::Fast => Self::Fast,
            CanisterBuildProfile::Release => Self::Release,
        }
    }
}

///
/// CanisterArtifactBuildOutput
///

#[derive(Clone, Debug)]
pub struct CanisterArtifactBuildOutput {
    pub artifact_root: PathBuf,
    pub wasm_path: PathBuf,
    pub wasm_gz_path: PathBuf,
    pub did_path: PathBuf,
    pub manifest_path: Option<PathBuf>,
}

// Build one visible Canic canister artifact for the current workspace.
pub fn build_current_workspace_canister_artifact(
    canister_name: &str,
    profile: CanisterBuildProfile,
) -> Result<CanisterArtifactBuildOutput, Box<dyn std::error::Error>> {
    let workspace_root = workspace_root()?;
    let dfx_root = dfx_root()?;
    build_canister_artifact(&workspace_root, &dfx_root, canister_name, profile)
}

// Build one visible Canic canister artifact and keep the thin-root special cases.
pub fn build_canister_artifact(
    workspace_root: &Path,
    dfx_root: &Path,
    canister_name: &str,
    profile: CanisterBuildProfile,
) -> Result<CanisterArtifactBuildOutput, Box<dyn std::error::Error>> {
    if canister_name == WASM_STORE_ROLE {
        return build_hidden_wasm_store_artifact(workspace_root, dfx_root, profile);
    }

    let canister_manifest_path = canister_manifest_path(workspace_root, canister_name);
    let canister_package_name = load_canister_package_name(&canister_manifest_path)?;
    let artifact_root = dfx_root
        .join(LOCAL_ARTIFACT_ROOT_RELATIVE)
        .join(canister_name);
    let wasm_path = artifact_root.join(format!("{canister_name}.wasm"));
    let wasm_gz_path = artifact_root.join(format!("{canister_name}.wasm.gz"));
    let did_path = artifact_root.join(format!("{canister_name}.did"));
    let require_embedded_release_artifacts = canister_name == ROOT_ROLE;

    if require_embedded_release_artifacts {
        build_bootstrap_wasm_store_artifact(workspace_root, dfx_root, profile.into())?;
    }

    fs::create_dir_all(&artifact_root)?;

    let release_wasm_path = run_canister_build(
        workspace_root,
        dfx_root,
        &canister_manifest_path,
        &canister_package_name,
        profile,
        require_embedded_release_artifacts,
    )?;
    write_wasm_artifact(&release_wasm_path, &wasm_path)?;
    maybe_shrink_wasm_artifact(&wasm_path)?;
    write_gzip_artifact(&wasm_path, &wasm_gz_path)?;

    let debug_wasm_path = run_canister_build(
        workspace_root,
        dfx_root,
        &canister_manifest_path,
        &canister_package_name,
        CanisterBuildProfile::Debug,
        require_embedded_release_artifacts,
    )?;
    extract_candid(&debug_wasm_path, &did_path)?;

    let network = std::env::var("DFX_NETWORK").unwrap_or_else(|_| "local".to_string());
    let manifest_path =
        emit_root_release_set_manifest_if_ready(workspace_root, dfx_root, &network)?;

    Ok(CanisterArtifactBuildOutput {
        artifact_root,
        wasm_path,
        wasm_gz_path,
        did_path,
        manifest_path,
    })
}

// Read the real package name from one canister manifest so downstreams are not
// forced to mirror the reference `canister_<role>` naming scheme.
fn load_canister_package_name(manifest_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let manifest_source = fs::read_to_string(manifest_path)?;
    let manifest = toml::from_str::<Value>(&manifest_source)?;
    let package_name = manifest
        .get("package")
        .and_then(Value::as_object)
        .and_then(|package| package.get("name"))
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing package.name in {}", manifest_path.display()))?;

    Ok(package_name.to_string())
}

// Run one wasm-target cargo build for the requested canister manifest/profile.
fn run_canister_build(
    workspace_root: &Path,
    dfx_root: &Path,
    manifest_path: &Path,
    package_name: &str,
    profile: CanisterBuildProfile,
    require_embedded_release_artifacts: bool,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let target_root = std::env::var_os("CARGO_TARGET_DIR")
        .map_or_else(|| workspace_root.join("target"), PathBuf::from);
    let mut command = Command::new("cargo");
    command
        .current_dir(workspace_root)
        .env("CARGO_TARGET_DIR", &target_root)
        .env("CANIC_DFX_ROOT", dfx_root)
        .args([
            "build",
            "--manifest-path",
            &manifest_path.display().to_string(),
            "--target",
            WASM_TARGET,
        ])
        .args(profile.cargo_args());

    if require_embedded_release_artifacts {
        command.env("CANIC_REQUIRE_EMBEDDED_RELEASE_ARTIFACTS", "1");
    }

    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "cargo build failed for {}: {}",
            manifest_path.display(),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    Ok(target_root
        .join(WASM_TARGET)
        .join(profile.target_dir_name())
        .join(format!("{}.wasm", package_name.replace('-', "_"))))
}

// Extract the service `.did` from one debug wasm so Candid stays deterministic.
fn extract_candid(
    debug_wasm_path: &Path,
    did_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("candid-extractor")
        .arg(debug_wasm_path)
        .output()
        .map_err(|err| {
            format!(
                "failed to run candid-extractor for {}: {err}",
                debug_wasm_path.display()
            )
        })?;

    if !output.status.success() {
        return Err(format!(
            "candid-extractor failed for {}: {}",
            debug_wasm_path.display(),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    write_bytes_atomically(did_path, &output.stdout)?;
    Ok(())
}

// Route the hidden bootstrap store through the published public builder.
fn build_hidden_wasm_store_artifact(
    workspace_root: &Path,
    dfx_root: &Path,
    profile: CanisterBuildProfile,
) -> Result<CanisterArtifactBuildOutput, Box<dyn std::error::Error>> {
    let output = build_bootstrap_wasm_store_artifact(workspace_root, dfx_root, profile.into())?;
    Ok(map_bootstrap_output(output))
}

// Apply `ic-wasm shrink` when available so visible canister artifacts follow
// the same size-reduction path as the hidden bootstrap store artifact.
fn maybe_shrink_wasm_artifact(wasm_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let shrunk_path = wasm_path.with_extension("wasm.shrunk");
    match Command::new("ic-wasm")
        .arg(wasm_path)
        .arg("-o")
        .arg(&shrunk_path)
        .arg("shrink")
        .status()
    {
        Ok(status) if status.success() => {
            fs::rename(shrunk_path, wasm_path)?;
        }
        Ok(_) => {
            let _ = fs::remove_file(shrunk_path);
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => {
            return Err(format!("failed to run ic-wasm for {}: {err}", wasm_path.display()).into());
        }
    }

    Ok(())
}

// Normalize the hidden bootstrap builder output to the public canister-artifact shape.
fn map_bootstrap_output(output: BootstrapWasmStoreBuildOutput) -> CanisterArtifactBuildOutput {
    CanisterArtifactBuildOutput {
        artifact_root: output.artifact_root,
        wasm_path: output.wasm_path,
        wasm_gz_path: output.wasm_gz_path,
        did_path: output.did_path,
        manifest_path: None,
    }
}

// Copy one `.wasm` artifact atomically into the DFX artifact tree.
fn write_wasm_artifact(
    source_path: &Path,
    target_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read(source_path)?;
    write_bytes_atomically(target_path, &bytes)?;
    Ok(())
}

// Write one deterministic `.wasm.gz` artifact with a zeroed gzip timestamp.
fn write_gzip_artifact(
    wasm_path: &Path,
    wasm_gz_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut wasm_bytes = Vec::new();
    fs::File::open(wasm_path)?.read_to_end(&mut wasm_bytes)?;

    let mut encoder = GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), Compression::best());
    encoder.write_all(&wasm_bytes)?;
    let gz_bytes = encoder.finish()?;
    write_bytes_atomically(wasm_gz_path, &gz_bytes)?;
    Ok(())
}

// Persist one file through a sibling temp path so readers never observe a partial write.
fn write_bytes_atomically(
    target_path: &Path,
    bytes: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let tmp_path = target_path.with_extension(format!(
        "{}.tmp",
        target_path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
    ));
    fs::write(&tmp_path, bytes)?;
    fs::rename(tmp_path, target_path)?;
    Ok(())
}
