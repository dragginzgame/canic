use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use crate::cargo_command;

///
/// CargoMetadata
///

#[derive(Clone, Debug, Deserialize)]
pub struct CargoMetadata {
    pub packages: Vec<CargoMetadataPackage>,
}

///
/// CargoMetadataPackage
///

#[derive(Clone, Debug, Deserialize)]
pub struct CargoMetadataPackage {
    pub name: String,
    #[serde(default)]
    pub version: String,
    pub manifest_path: PathBuf,
    pub metadata: Option<JsonValue>,
}

static CARGO_METADATA_NO_DEPS_CACHE: OnceLock<Mutex<HashMap<PathBuf, CargoMetadata>>> =
    OnceLock::new();

// Query cargo metadata for the selected workspace root.
pub fn cargo_metadata(
    workspace_root: &Path,
    include_deps: bool,
) -> Result<CargoMetadata, Box<dyn std::error::Error>> {
    let mut command = cargo_command();
    command.current_dir(workspace_root).args([
        "metadata",
        "--format-version=1",
        "--manifest-path",
        &workspace_root.join("Cargo.toml").display().to_string(),
    ]);
    if !include_deps {
        command.arg("--no-deps");
    }

    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    Ok(serde_json::from_slice(&output.stdout)?)
}

// Reuse one per-process no-deps Cargo metadata snapshot for manifest discovery.
pub fn cargo_metadata_no_deps_cached(
    workspace_root: &Path,
) -> Result<CargoMetadata, Box<dyn std::error::Error>> {
    let cache_key = workspace_root
        .canonicalize()
        .unwrap_or_else(|_| workspace_root.to_path_buf());
    let cache = CARGO_METADATA_NO_DEPS_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    {
        let cache = cache.lock().expect("cargo metadata cache lock poisoned");
        if let Some(metadata) = cache.get(&cache_key) {
            return Ok(metadata.clone());
        }
    }

    let metadata = cargo_metadata(workspace_root, false)?;
    let mut cache = cache.lock().expect("cargo metadata cache lock poisoned");
    cache.insert(cache_key, metadata.clone());
    Ok(metadata)
}
