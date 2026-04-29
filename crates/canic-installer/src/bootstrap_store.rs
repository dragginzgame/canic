use crate::{
    cargo_command,
    release_set::{config_path, dfx_root, workspace_root},
};
use flate2::{Compression, GzBuilder};
use serde::Deserialize;
use std::{
    fmt::Write as _,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
};

const WASM_STORE_ROLE: &str = "wasm_store";
const WASM_STORE_ARTIFACTS_RELATIVE: &str = ".dfx/local/canisters/wasm_store";
const GENERATED_WRAPPER_RELATIVE: &str = ".dfx/local/generated/canic-wasm-store";
const CANONICAL_WASM_STORE_MANIFEST_RELATIVE: &str = "crates/canic-wasm-store/Cargo.toml";
const CANONICAL_WASM_STORE_DID_FILE: &str = "wasm_store.did";
const CANONICAL_WASM_STORE_CRATE_NAME: &str = "canister_wasm_store";
const GENERATED_WRAPPER_PACKAGE_NAME: &str = "canic-generated-wasm-store";
const CANIC_FAMILY_CRATES: &[&str] = &[
    "canic-cdk",
    "canic-control-plane",
    "canic-core",
    "canic-macros",
    "canic-memory",
];

///
/// BootstrapWasmStoreBuildProfile
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BootstrapWasmStoreBuildProfile {
    Debug,
    Fast,
    Release,
}

impl BootstrapWasmStoreBuildProfile {
    #[must_use]
    pub fn current() -> Self {
        match std::env::var("CANIC_WASM_PROFILE").ok().as_deref() {
            Some("debug") => Self::Debug,
            Some("fast") => Self::Fast,
            _ => Self::Release,
        }
    }

    #[must_use]
    pub const fn cargo_args(self) -> &'static [&'static str] {
        match self {
            Self::Debug => &[],
            Self::Fast => &["--profile", "fast"],
            Self::Release => &["--release"],
        }
    }

    #[must_use]
    pub const fn target_dir_name(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Fast => "fast",
            Self::Release => "release",
        }
    }

    #[must_use]
    pub const fn profile_marker(self) -> &'static str {
        self.target_dir_name()
    }
}

///
/// BootstrapWasmStoreBuildOutput
///

#[derive(Clone, Debug)]
pub struct BootstrapWasmStoreBuildOutput {
    pub artifact_root: PathBuf,
    pub wasm_path: PathBuf,
    pub wasm_gz_path: PathBuf,
    pub did_path: PathBuf,
}

#[derive(Clone, Debug)]
struct BootstrapWasmStoreSource {
    manifest_path: PathBuf,
    source_root: PathBuf,
}

#[derive(Clone, Debug, Deserialize)]
struct CargoMetadata {
    packages: Vec<CargoMetadataPackage>,
}

#[derive(Clone, Debug, Deserialize)]
struct CargoMetadataPackage {
    name: String,
    version: String,
    manifest_path: PathBuf,
}

// Build the implicit bootstrap `wasm_store` artifact and populate the canonical
// local DFX artifact paths for downstream/root builds.
pub fn build_bootstrap_wasm_store_artifact(
    workspace_root: &Path,
    dfx_root: &Path,
    profile: BootstrapWasmStoreBuildProfile,
) -> Result<BootstrapWasmStoreBuildOutput, Box<dyn std::error::Error>> {
    let source = resolve_bootstrap_wasm_store_source(workspace_root, dfx_root)?;
    let artifact_root = dfx_root.join(WASM_STORE_ARTIFACTS_RELATIVE);
    fs::create_dir_all(&artifact_root)?;

    run_wasm_store_cargo_build(
        workspace_root,
        &source.manifest_path,
        &config_path(workspace_root),
        profile,
    )?;

    let target_root = std::env::var_os("CARGO_TARGET_DIR")
        .map_or_else(|| workspace_root.join("target"), PathBuf::from);
    let built_wasm_path = target_root
        .join("wasm32-unknown-unknown")
        .join(profile.target_dir_name())
        .join(format!("{CANONICAL_WASM_STORE_CRATE_NAME}.wasm"));

    let wasm_path = artifact_root.join(format!("{WASM_STORE_ROLE}.wasm"));
    let wasm_gz_path = artifact_root.join(format!("{WASM_STORE_ROLE}.wasm.gz"));
    let did_path = artifact_root.join(format!("{WASM_STORE_ROLE}.did"));
    let profile_path = artifact_root.join(".build-profile");

    fs::copy(&built_wasm_path, &wasm_path)?;
    maybe_shrink_wasm_artifact(&wasm_path)?;
    write_gzip_artifact(&wasm_path, &wasm_gz_path)?;
    fs::write(profile_path, profile.profile_marker())?;
    ensure_wasm_store_did(workspace_root, &source, profile, &did_path)?;

    Ok(BootstrapWasmStoreBuildOutput {
        artifact_root,
        wasm_path,
        wasm_gz_path,
        did_path,
    })
}

// Resolve the current workspace root and build the bootstrap `wasm_store`
// artifact from there.
pub fn build_current_workspace_bootstrap_wasm_store_artifact(
    profile: BootstrapWasmStoreBuildProfile,
) -> Result<BootstrapWasmStoreBuildOutput, Box<dyn std::error::Error>> {
    let workspace_root = workspace_root()?;
    let dfx_root = dfx_root()?;
    build_bootstrap_wasm_store_artifact(&workspace_root, &dfx_root, profile)
}

// Resolve the canonical published/workspace `canic-wasm-store` source or fall
// back to a generated wrapper when downstreams only depend on `canic`.
fn resolve_bootstrap_wasm_store_source(
    workspace_root: &Path,
    dfx_root: &Path,
) -> Result<BootstrapWasmStoreSource, Box<dyn std::error::Error>> {
    let metadata = cargo_metadata(workspace_root)?;
    let canic_manifest_path = metadata
        .packages
        .iter()
        .find(|package| package.name == "canic")
        .map(|package| package.manifest_path.clone())
        .ok_or_else(|| {
            "unable to locate resolved 'canic' package in cargo metadata; downstreams that build the implicit wasm_store must depend on 'canic'."
                .to_string()
        })?;

    if let Some(source) = resolve_canonical_bootstrap_wasm_store_source(
        workspace_root,
        &metadata,
        &canic_manifest_path,
    ) {
        return Ok(source);
    }

    let wrapper_root =
        ensure_generated_wasm_store_wrapper(dfx_root, workspace_root, &canic_manifest_path)?;
    Ok(BootstrapWasmStoreSource {
        manifest_path: wrapper_root.join("Cargo.toml"),
        source_root: wrapper_root.clone(),
    })
}

// Prefer the local workspace `canic-wasm-store` crate, then a direct metadata
// hit, then a sibling registry checkout next to the resolved `canic` source.
fn resolve_canonical_bootstrap_wasm_store_source(
    workspace_root: &Path,
    metadata: &CargoMetadata,
    canic_manifest_path: &Path,
) -> Option<BootstrapWasmStoreSource> {
    let workspace_manifest = workspace_root.join(CANONICAL_WASM_STORE_MANIFEST_RELATIVE);
    if workspace_manifest.is_file() {
        let source_root = workspace_manifest
            .parent()
            .expect("manifest path must have parent")
            .to_path_buf();
        return Some(BootstrapWasmStoreSource {
            manifest_path: workspace_manifest,
            source_root,
        });
    }

    if let Some(package) = metadata
        .packages
        .iter()
        .find(|package| package.name == "canic-wasm-store")
    {
        let source_root = package
            .manifest_path
            .parent()
            .expect("manifest path must have parent")
            .to_path_buf();
        return Some(BootstrapWasmStoreSource {
            manifest_path: package.manifest_path.clone(),
            source_root,
        });
    }

    let canic_root = canic_manifest_path
        .parent()
        .expect("canic manifest path must have parent");
    let sibling_root = canic_root.parent().expect("canic root must have parent");
    let canic_version = metadata
        .packages
        .iter()
        .find(|package| package.name == "canic")
        .map(|package| package.version.clone())
        .unwrap_or_default();

    let local_sibling = sibling_root.join("canic-wasm-store").join("Cargo.toml");
    if local_sibling.is_file() {
        let source_root = local_sibling
            .parent()
            .expect("manifest path must have parent")
            .to_path_buf();
        return Some(BootstrapWasmStoreSource {
            manifest_path: local_sibling,
            source_root,
        });
    }

    if !canic_version.is_empty() {
        let registry_sibling = sibling_root
            .join(format!("canic-wasm-store-{canic_version}"))
            .join("Cargo.toml");
        if registry_sibling.is_file() {
            let source_root = registry_sibling
                .parent()
                .expect("manifest path must have parent")
                .to_path_buf();
            return Some(BootstrapWasmStoreSource {
                manifest_path: registry_sibling,
                source_root,
            });
        }
    }

    None
}

// Query cargo metadata for the current downstream workspace.
fn cargo_metadata(workspace_root: &Path) -> Result<CargoMetadata, Box<dyn std::error::Error>> {
    let output = cargo_command()
        .current_dir(workspace_root)
        .args([
            "metadata",
            "--format-version=1",
            "--manifest-path",
            &workspace_root.join("Cargo.toml").display().to_string(),
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    Ok(serde_json::from_slice(&output.stdout)?)
}

// Render the generated wrapper under `.dfx/local/generated/canic-wasm-store`.
fn ensure_generated_wasm_store_wrapper(
    dfx_root: &Path,
    workspace_root: &Path,
    canic_manifest_path: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let wrapper_root = dfx_root.join(GENERATED_WRAPPER_RELATIVE);
    fs::create_dir_all(wrapper_root.join("src"))?;

    let canic_root = canic_manifest_path
        .parent()
        .expect("canic manifest path must have parent");
    let patch_table = generated_wasm_store_wrapper_patch_table(canic_manifest_path);
    let mut cargo_toml = format!(
        "[package]\n\
name = \"{GENERATED_WRAPPER_PACKAGE_NAME}\"\n\
version = \"0.0.0\"\n\
edition = \"2024\"\n\
publish = false\n\n\
[workspace]\n\n\
[lib]\n\
name = \"{CANONICAL_WASM_STORE_CRATE_NAME}\"\n\
crate-type = [\"cdylib\", \"rlib\"]\n\n\
[dependencies]\n\
canic = {{ path = \"{}\", features = [\"control-plane\"] }}\n\
ic-cdk = \"0.20.0\"\n\
candid = {{ version = \"0.10\", default-features = false }}\n\n\
[build-dependencies]\n\
canic = {{ path = \"{}\" }}\n",
        canic_root.display(),
        canic_root.display()
    );

    cargo_toml.push_str(
        "\n[profile.release]\n\
opt-level = \"z\"\n\
lto = true\n\
codegen-units = 1\n\
strip = \"symbols\"\n\
debug = false\n\
panic = \"abort\"\n\
overflow-checks = false\n\
incremental = false\n\
\n\
[profile.fast]\n\
inherits = \"release\"\n\
lto = false\n\
codegen-units = 16\n\
incremental = true\n",
    );

    if !patch_table.is_empty() {
        cargo_toml.push('\n');
        cargo_toml.push_str(&patch_table);
    }

    fs::write(wrapper_root.join("Cargo.toml"), cargo_toml)?;
    fs::write(
        wrapper_root.join("build.rs"),
        "fn main() {\n    let config_path = std::env::var(\"CANIC_CONFIG_PATH\")\n        .expect(\"CANIC_CONFIG_PATH must be set for generated wasm_store wrapper\");\n\n    canic::build!(config_path);\n}\n",
    )?;
    fs::write(
        wrapper_root.join("src/lib.rs"),
        "#![allow(clippy::unused_async)]\n\ncanic::start_wasm_store!();\ncanic::cdk::export_candid_debug!();\n",
    )?;

    let workspace_lock = workspace_root.join("Cargo.lock");
    if workspace_lock.is_file() {
        fs::copy(workspace_lock, wrapper_root.join("Cargo.lock"))?;
    }

    Ok(wrapper_root)
}

// Generate the `[patch.crates-io]` table for sibling packaged Canic crates.
fn generated_wasm_store_wrapper_patch_table(canic_manifest_path: &Path) -> String {
    let canic_root = canic_manifest_path
        .parent()
        .expect("canic manifest path must have parent");
    let sibling_root = canic_root.parent().expect("canic root must have parent");
    let registry_version = registry_package_version_suffix(canic_manifest_path, "canic");
    let mut rendered = String::new();

    for crate_name in CANIC_FAMILY_CRATES {
        let mut manifest_path = sibling_root.join(crate_name).join("Cargo.toml");

        if !manifest_path.is_file() {
            manifest_path =
                find_versioned_sibling_manifest(sibling_root, crate_name, registry_version)
                    .unwrap_or_default();
        }

        if !manifest_path.is_file() {
            continue;
        }

        let crate_root = manifest_path
            .parent()
            .expect("manifest path must have parent");
        let _ = writeln!(
            rendered,
            "{crate_name} = {{ path = \"{}\" }}",
            crate_root.display()
        );
    }

    if rendered.is_empty() {
        String::new()
    } else {
        format!("[patch.crates-io]\n{rendered}")
    }
}

fn registry_package_version_suffix<'a>(
    manifest_path: &'a Path,
    crate_name: &str,
) -> Option<&'a str> {
    let parent_name = manifest_path.parent()?.file_name()?.to_str()?;
    parent_name.strip_prefix(&format!("{crate_name}-"))
}

// Locate a versioned sibling packaged crate under the same registry source root.
fn find_versioned_sibling_manifest(
    sibling_root: &Path,
    crate_name: &str,
    version_hint: Option<&str>,
) -> Option<PathBuf> {
    if let Some(version) = version_hint {
        let preferred = sibling_root
            .join(format!("{crate_name}-{version}"))
            .join("Cargo.toml");
        if preferred.is_file() {
            return Some(preferred);
        }
    }

    let mut candidates = fs::read_dir(sibling_root).ok()?;
    while let Some(Ok(entry)) = candidates.next() {
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if !file_name.starts_with(&format!("{crate_name}-")) {
            continue;
        }

        let manifest_path = entry.path().join("Cargo.toml");
        if manifest_path.is_file() {
            return Some(manifest_path);
        }
    }

    None
}

// Build the chosen `canic-wasm-store` source/wrapper for one target profile.
fn run_wasm_store_cargo_build(
    workspace_root: &Path,
    manifest_path: &Path,
    config_path: &Path,
    profile: BootstrapWasmStoreBuildProfile,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut command = cargo_command();
    command
        .current_dir(workspace_root)
        .env("CANIC_CONFIG_PATH", config_path)
        .env(
            "CARGO_TARGET_DIR",
            std::env::var_os("CARGO_TARGET_DIR")
                .map_or_else(|| workspace_root.join("target"), PathBuf::from),
        )
        .args([
            "build",
            "--manifest-path",
            &manifest_path.display().to_string(),
            "--target",
            "wasm32-unknown-unknown",
        ])
        .args(profile.cargo_args());

    let output = command.output()?;
    if output.status.success() {
        return Ok(());
    }

    Err(format!(
        "cargo build failed for bootstrap wasm_store: {}",
        String::from_utf8_lossy(&output.stderr)
    )
    .into())
}

// Copy or regenerate the `.did` file that matches the built bootstrap artifact.
fn ensure_wasm_store_did(
    workspace_root: &Path,
    source: &BootstrapWasmStoreSource,
    profile: BootstrapWasmStoreBuildProfile,
    artifact_did_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let source_did_path = source.source_root.join(CANONICAL_WASM_STORE_DID_FILE);

    // Ordinary artifact builds must treat the checked-in bootstrap `.did` as
    // canonical source, not as a cache file that gets rewritten on unrelated
    // workspace changes. Regeneration is explicit.
    if source_did_path.is_file() && !refresh_canonical_wasm_store_did_enabled() {
        fs::copy(source_did_path, artifact_did_path)?;
        return Ok(());
    }

    run_wasm_store_cargo_build(
        workspace_root,
        &source.manifest_path,
        &config_path(workspace_root),
        BootstrapWasmStoreBuildProfile::Debug,
    )?;

    let target_root = std::env::var_os("CARGO_TARGET_DIR")
        .map_or_else(|| workspace_root.join("target"), PathBuf::from);
    let debug_wasm_path = target_root
        .join("wasm32-unknown-unknown")
        .join(BootstrapWasmStoreBuildProfile::Debug.target_dir_name())
        .join(format!("{CANONICAL_WASM_STORE_CRATE_NAME}.wasm"));
    let output = Command::new("candid-extractor")
        .arg(&debug_wasm_path)
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "candid-extractor failed for bootstrap wasm_store: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    if source_did_path
        .parent()
        .expect("bootstrap wasm_store did path must have parent")
        .exists()
    {
        fs::write(&source_did_path, &output.stdout)?;
    }
    fs::copy(source_did_path, artifact_did_path)?;
    if profile == BootstrapWasmStoreBuildProfile::Debug {
        let artifact_root = artifact_did_path
            .parent()
            .expect("artifact did path must have parent");
        let wasm_path = artifact_root.join(format!("{WASM_STORE_ROLE}.wasm"));
        let wasm_gz_path = artifact_root.join(format!("{WASM_STORE_ROLE}.wasm.gz"));
        if wasm_path.is_file() && wasm_gz_path.is_file() {
            fs::write(artifact_root.join(".build-profile"), "debug")?;
        }
    }

    Ok(())
}

// Regeneration of the canonical bootstrap-store `.did` is explicit so normal
// artifact builds do not rewrite checked-in source files as a side effect.
fn refresh_canonical_wasm_store_did_enabled() -> bool {
    matches!(
        std::env::var("CANIC_REFRESH_WASM_STORE_DID")
            .ok()
            .as_deref(),
        Some("1" | "true" | "TRUE" | "yes" | "YES")
    )
}

// Apply `ic-wasm shrink` when available so the bootstrap artifact matches the
// normal custom-build path.
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
        Err(err) => return Err(err.into()),
    }

    Ok(())
}

// Write one deterministic `.wasm.gz` artifact with zeroed gzip mtime.
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
    fs::write(wasm_gz_path, gz_bytes)?;
    Ok(())
}
