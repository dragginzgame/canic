use crate::workspace_discovery::{
    discover_canister_manifest_from_metadata, discover_icp_root_from, discover_workspace_root_from,
    normalize_workspace_path,
};
use std::{
    fs,
    path::{Path, PathBuf},
};
use toml::Value as TomlValue;

use super::{
    CANISTERS_ROOT_RELATIVE, ROOT_CONFIG_FILE, ROOT_RELEASE_SET_MANIFEST_FILE,
    WORKSPACE_MANIFEST_RELATIVE,
};

// Resolve the downstream Cargo workspace root from the current directory,
// config hints, or an explicit override.
pub fn workspace_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Ok(path) = std::env::var("CANIC_WORKSPACE_ROOT") {
        return Ok(PathBuf::from(path).canonicalize()?);
    }

    if let Some(root) = std::env::var_os("CANIC_WORKSPACE_MANIFEST_PATH")
        .map(PathBuf::from)
        .and_then(|path| discover_workspace_root_from(&path))
    {
        return Ok(root);
    }

    if let Some(root) = std::env::var_os("CANIC_CONFIG_PATH")
        .map(PathBuf::from)
        .and_then(|path| discover_workspace_root_from(&path))
    {
        return Ok(root);
    }

    if let Some(root) = discover_workspace_root_from(&std::env::current_dir()?) {
        return Ok(root);
    }

    Ok(std::env::current_dir()?.canonicalize()?)
}

// Resolve the downstream ICP CLI/project root from the current directory or an
// explicit override.
pub fn icp_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Ok(path) = std::env::var("CANIC_ICP_ROOT") {
        return Ok(PathBuf::from(path).canonicalize()?);
    }

    let current_dir = std::env::current_dir()?.canonicalize()?;
    if let Some(root) = discover_icp_root_from(&current_dir) {
        return Ok(root);
    }

    if let Ok(path) = std::env::var("CANIC_WORKSPACE_ROOT") {
        let workspace_root = PathBuf::from(path).canonicalize()?;
        if let Some(root) = discover_icp_root_from(&workspace_root) {
            return Ok(root);
        }
        return Ok(workspace_root);
    }

    Ok(current_dir)
}

// Resolve the downstream Canic config path.
#[must_use]
pub fn config_path(workspace_root: &Path) -> PathBuf {
    std::env::var_os("CANIC_CONFIG_PATH").map_or_else(
        || canisters_root(workspace_root).join(ROOT_CONFIG_FILE),
        |path| normalize_workspace_path(workspace_root, PathBuf::from(path)),
    )
}

// Resolve the downstream canister-manifest root.
#[must_use]
pub fn canisters_root(workspace_root: &Path) -> PathBuf {
    if let Some(path) = std::env::var_os("CANIC_CANISTERS_ROOT") {
        return normalize_workspace_path(workspace_root, PathBuf::from(path));
    }

    if let Some(path) = std::env::var_os("CANIC_CONFIG_PATH") {
        let config_path = normalize_workspace_path(workspace_root, PathBuf::from(path));
        if let Some(parent) = config_path.parent() {
            return parent.to_path_buf();
        }
    }

    if let Some(manifest_path) = discover_canister_manifest_from_metadata(workspace_root, "root")
        && let Some(parent) = manifest_path.parent().and_then(Path::parent)
    {
        return parent.to_path_buf();
    }

    workspace_root.join(CANISTERS_ROOT_RELATIVE)
}

// Resolve the downstream root canister manifest path.
#[must_use]
pub fn root_manifest_path(workspace_root: &Path) -> PathBuf {
    std::env::var_os("CANIC_ROOT_MANIFEST_PATH").map_or_else(
        || {
            discover_canister_manifest_from_metadata(workspace_root, "root").unwrap_or_else(|| {
                canisters_root(workspace_root)
                    .join("root")
                    .join("Cargo.toml")
            })
        },
        |path| normalize_workspace_path(workspace_root, PathBuf::from(path)),
    )
}

// Resolve the downstream manifest path for one visible canister role.
#[must_use]
pub fn canister_manifest_path(workspace_root: &Path, canister_name: &str) -> PathBuf {
    discover_canister_manifest_from_metadata(workspace_root, canister_name).unwrap_or_else(|| {
        canisters_root(workspace_root)
            .join(canister_name)
            .join("Cargo.toml")
    })
}

// Resolve the downstream workspace manifest path.
#[must_use]
pub fn workspace_manifest_path(workspace_root: &Path) -> PathBuf {
    std::env::var_os("CANIC_WORKSPACE_MANIFEST_PATH").map_or_else(
        || workspace_root.join(WORKSPACE_MANIFEST_RELATIVE),
        |path| normalize_workspace_path(workspace_root, PathBuf::from(path)),
    )
}

// Render a path relative to the workspace root when possible.
#[must_use]
pub fn display_workspace_path(workspace_root: &Path, path: &Path) -> String {
    path.strip_prefix(workspace_root)
        .unwrap_or(path)
        .display()
        .to_string()
}

// Resolve the built artifact directory for the selected ICP environment.
pub fn resolve_artifact_root(
    icp_root: &Path,
    network: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let preferred = icp_root.join(".icp").join(network).join("canisters");
    if preferred.is_dir() {
        return Ok(preferred);
    }

    let local_artifact_root = icp_root.join(".icp/local/canisters");
    if local_artifact_root.is_dir() {
        return Ok(local_artifact_root);
    }

    Err(format!(
        "missing built ICP artifacts under {} or {}",
        preferred.display(),
        local_artifact_root.display()
    )
    .into())
}

// Return the canonical manifest path for the staged root release set.
pub fn root_release_set_manifest_path(
    artifact_root: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let manifest_path = artifact_root
        .join("root")
        .join(ROOT_RELEASE_SET_MANIFEST_FILE);

    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent)?;
    }

    Ok(manifest_path)
}

// Read the reference root canister version so staged release versions match the install.
pub fn load_root_package_version(
    root_manifest_path: &Path,
    workspace_manifest_path: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let manifest_source = fs::read_to_string(root_manifest_path)?;
    let manifest = toml::from_str::<TomlValue>(&manifest_source)?;
    let version_value = manifest
        .get("package")
        .and_then(TomlValue::as_table)
        .and_then(|package| package.get("version"))
        .ok_or_else(|| {
            format!(
                "missing package.version in {}",
                root_manifest_path.display()
            )
        })?;

    if let Some(version) = version_value.as_str() {
        return Ok(version.to_string());
    }

    if version_value
        .as_table()
        .and_then(|value| value.get("workspace"))
        .and_then(TomlValue::as_bool)
        == Some(true)
    {
        return load_workspace_package_version(workspace_manifest_path);
    }

    Err(format!(
        "unsupported package.version format in {}",
        root_manifest_path.display()
    )
    .into())
}

// Resolve the shared workspace package version used by reference canisters.
pub fn load_workspace_package_version(
    workspace_manifest_path: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let manifest_source = fs::read_to_string(workspace_manifest_path)?;
    let manifest = toml::from_str::<TomlValue>(&manifest_source)?;
    let version = manifest
        .get("workspace")
        .and_then(TomlValue::as_table)
        .and_then(|workspace| workspace.get("package"))
        .and_then(TomlValue::as_table)
        .and_then(|package| package.get("version"))
        .and_then(TomlValue::as_str)
        .ok_or_else(|| {
            format!(
                "missing workspace.package.version in {}",
                workspace_manifest_path.display()
            )
        })?;

    Ok(version.to_string())
}
