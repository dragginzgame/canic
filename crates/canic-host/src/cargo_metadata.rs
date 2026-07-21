//! Module: cargo_metadata
//!
//! Responsibility: execute and decode bounded host-side Cargo evidence commands.
//! Does not own: role dependency policy or workspace discovery decisions.
//! Boundary: callers receive typed metadata or raw package-selected tree output.

use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::{
    collections::{BTreeMap, HashMap},
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
    #[serde(default)]
    pub resolve: Option<CargoMetadataResolve>,
    #[serde(default)]
    pub workspace_root: PathBuf,
}

///
/// CargoMetadataPackage
///

#[derive(Clone, Debug, Deserialize)]
pub struct CargoMetadataPackage {
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub source: Option<String>,
    pub manifest_path: PathBuf,
    pub metadata: Option<JsonValue>,
    #[serde(default)]
    pub dependencies: Vec<CargoMetadataDependency>,
    #[serde(default)]
    pub features: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub targets: Vec<CargoMetadataTarget>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CargoMetadataTarget {
    pub name: String,
    #[serde(default)]
    pub kind: Vec<String>,
    #[serde(default)]
    pub src_path: PathBuf,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CargoMetadataDependency {
    pub name: String,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub rename: Option<String>,
    #[serde(default)]
    pub optional: bool,
    #[serde(default = "default_true")]
    pub uses_default_features: bool,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub target: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CargoMetadataResolve {
    pub nodes: Vec<CargoMetadataNode>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CargoMetadataNode {
    pub id: String,
    #[serde(default)]
    pub deps: Vec<CargoMetadataNodeDependency>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CargoMetadataNodeDependency {
    pub name: String,
    pub pkg: String,
    #[serde(default)]
    pub dep_kinds: Vec<CargoMetadataDependencyKind>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CargoMetadataDependencyKind {
    #[serde(default)]
    pub kind: Option<String>,
}

static CARGO_METADATA_NO_DEPS_CACHE: OnceLock<Mutex<HashMap<PathBuf, CargoMetadata>>> =
    OnceLock::new();

// Query cargo metadata for the selected workspace root.
pub fn cargo_metadata(
    workspace_root: &Path,
    include_deps: bool,
) -> Result<CargoMetadata, Box<dyn std::error::Error>> {
    let manifest_path = workspace_root.join("Cargo.toml");
    let mut command = cargo_metadata_command(&manifest_path);
    if !include_deps {
        command.arg("--no-deps");
    }

    run_cargo_metadata(command)
}

pub fn cargo_metadata_for_manifest(
    manifest_path: &Path,
    filter_platform: &str,
    locked_offline: bool,
) -> Result<CargoMetadata, Box<dyn std::error::Error>> {
    let mut command = cargo_metadata_command(manifest_path);
    command.args(["--filter-platform", filter_platform]);
    if locked_offline {
        command.args(["--locked", "--offline"]);
    }

    run_cargo_metadata(command)
}

/// Query the complete Cargo package catalog without treating its union graph
/// as role-specific activation evidence.
pub fn cargo_metadata_catalog_for_manifest(
    manifest_path: &Path,
    locked_offline: bool,
) -> Result<CargoMetadata, Box<dyn std::error::Error>> {
    let mut command = cargo_metadata_command(manifest_path);
    if locked_offline {
        command.args(["--locked", "--offline"]);
    }

    run_cargo_metadata(command)
}

/// Query one package-selected normal dependency tree for the requested target.
pub fn cargo_tree_for_package(
    manifest_path: &Path,
    package_spec: &str,
    filter_platform: &str,
    locked_offline: bool,
    format: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut command = cargo_command();
    command
        .current_dir(manifest_path.parent().unwrap_or_else(|| Path::new(".")))
        .args([
            "tree",
            "--manifest-path",
            &manifest_path.display().to_string(),
            "--package",
            package_spec,
            "--target",
            filter_platform,
            "--edges",
            "normal",
            "--prefix",
            "depth",
            "--no-dedupe",
            "--charset",
            "ascii",
            "--format",
            format,
        ]);
    if locked_offline {
        command.args(["--locked", "--offline"]);
    }

    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "cargo tree failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    Ok(String::from_utf8(output.stdout)?)
}

fn cargo_metadata_command(manifest_path: &Path) -> std::process::Command {
    let mut command = cargo_command();
    command
        .current_dir(manifest_path.parent().unwrap_or_else(|| Path::new(".")))
        .args([
            "metadata",
            "--format-version=1",
            "--manifest-path",
            &manifest_path.display().to_string(),
        ]);
    command
}

fn run_cargo_metadata(
    mut command: std::process::Command,
) -> Result<CargoMetadata, Box<dyn std::error::Error>> {
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

const fn default_true() -> bool {
    true
}

// Reuse one per-process no-deps Cargo metadata snapshot for manifest discovery.
pub fn cargo_metadata_no_deps_cached(
    workspace_root: &Path,
) -> Result<CargoMetadata, Box<dyn std::error::Error>> {
    let cache_key = workspace_root.canonicalize()?;
    let cache = CARGO_METADATA_NO_DEPS_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    {
        let cache = cache.lock().expect("cargo metadata cache lock poisoned");
        if let Some(metadata) = cache.get(&cache_key) {
            return Ok(metadata.clone());
        }
    }

    let metadata = cargo_metadata(&cache_key, false)?;
    {
        let mut cache = cache.lock().expect("cargo metadata cache lock poisoned");
        cache.insert(cache_key, metadata.clone());
    }
    Ok(metadata)
}
