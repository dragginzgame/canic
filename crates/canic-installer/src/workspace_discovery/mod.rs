use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, OnceLock},
};

const WORKSPACE_MANIFEST_RELATIVE: &str = "Cargo.toml";
const DFX_CONFIG_FILE: &str = "dfx.json";

#[derive(Clone, Debug, Deserialize)]
struct CargoMetadata {
    packages: Vec<CargoMetadataPackage>,
}

#[derive(Clone, Debug, Deserialize)]
struct CargoMetadataPackage {
    name: String,
    manifest_path: PathBuf,
    metadata: Option<JsonValue>,
}

static CARGO_METADATA_CACHE: OnceLock<Mutex<HashMap<PathBuf, CargoMetadata>>> = OnceLock::new();

// Resolve the nearest Cargo workspace root from one file or directory path.
pub fn discover_workspace_root_from(path: &Path) -> Option<PathBuf> {
    let start = if path.is_file() { path.parent()? } else { path };

    for candidate in start.ancestors() {
        let manifest_path = candidate.join(WORKSPACE_MANIFEST_RELATIVE);
        if !manifest_path.is_file() {
            continue;
        }

        let manifest = fs::read_to_string(&manifest_path).ok()?;
        if manifest.contains("[workspace]") {
            return candidate.canonicalize().ok();
        }
    }

    None
}

// Resolve the nearest DFX root from one file or directory path.
pub fn discover_dfx_root_from(path: &Path) -> Option<PathBuf> {
    let start = if path.is_file() { path.parent()? } else { path };

    for candidate in start.ancestors() {
        let dfx_config = candidate.join(DFX_CONFIG_FILE);
        if dfx_config.is_file() {
            return candidate.canonicalize().ok();
        }
    }

    None
}

// Normalize one workspace-relative path against the chosen workspace root.
pub fn normalize_workspace_path(workspace_root: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        workspace_root.join(path)
    }
}

// Discover the manifest for one role using workspace metadata first, then package naming.
pub fn discover_canister_manifest_from_metadata(
    workspace_root: &Path,
    role_name: &str,
) -> Option<PathBuf> {
    let metadata = cargo_metadata_cached(workspace_root).ok()?;
    let expected_package_name = format!("canister_{role_name}");

    metadata
        .packages
        .into_iter()
        .find(|package| {
            package_declares_role(package, role_name) || package.name == expected_package_name
        })
        .map(|package| package.manifest_path)
}

// Check whether one package declares the requested Canic role in Cargo metadata.
fn package_declares_role(package: &CargoMetadataPackage, role_name: &str) -> bool {
    package
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.get("canic"))
        .and_then(|canic| canic.get("role"))
        .and_then(JsonValue::as_str)
        == Some(role_name)
}

// Load Cargo workspace metadata once from the requested workspace root.
fn cargo_metadata(workspace_root: &Path) -> Result<CargoMetadata, Box<dyn std::error::Error>> {
    let output = Command::new("cargo")
        .current_dir(workspace_root)
        .args([
            "metadata",
            "--format-version=1",
            "--no-deps",
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

// Reuse one per-process Cargo metadata snapshot for repeated manifest discovery.
fn cargo_metadata_cached(
    workspace_root: &Path,
) -> Result<CargoMetadata, Box<dyn std::error::Error>> {
    let cache_key = workspace_root
        .canonicalize()
        .unwrap_or_else(|_| workspace_root.to_path_buf());
    let cache = CARGO_METADATA_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    {
        let cache = cache.lock().expect("cargo metadata cache lock poisoned");
        if let Some(metadata) = cache.get(&cache_key) {
            return Ok(metadata.clone());
        }
    }

    let metadata = cargo_metadata(workspace_root)?;
    let mut cache = cache.lock().expect("cargo metadata cache lock poisoned");
    cache.insert(cache_key, metadata.clone());
    Ok(metadata)
}
