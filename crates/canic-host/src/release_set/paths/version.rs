use std::{fs, path::Path};
use toml::Value as TomlValue;

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
