//! Module: canic_cli::medic::package
//!
//! Responsibility: inspect role-package metadata and resolved Canic dependency features.
//! Does not own: fleet configuration policy, report rendering, or Cargo manifest mutation.
//! Boundary: reads Cargo manifests and returns passive metadata to project medic checks.

use std::{
    fs,
    path::{Path, PathBuf},
};
use toml::Value as TomlValue;

const PACKAGE_MANIFEST_FILE: &str = "Cargo.toml";

pub(super) struct CanicPackageMetadata {
    pub(super) fleet: String,
    pub(super) role: String,
}

pub(super) fn role_package_manifest_path(config: &Path, package: &str) -> PathBuf {
    let package_path = PathBuf::from(package);
    let path = if package_path.is_absolute() {
        package_path
    } else {
        config
            .parent()
            .map_or_else(|| PathBuf::from(package), |parent| parent.join(package))
    };
    if path.file_name().and_then(|name| name.to_str()) == Some(PACKAGE_MANIFEST_FILE) {
        path
    } else {
        path.join(PACKAGE_MANIFEST_FILE)
    }
}

pub(super) fn canic_package_metadata(path: &Path) -> Result<CanicPackageMetadata, String> {
    let source = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let manifest = toml::from_str::<TomlValue>(&source)
        .map_err(|err| format!("invalid {}: {err}", path.display()))?;
    let fleet = manifest_string(&manifest, &["package", "metadata", "canic", "fleet"], path)?;
    let role = manifest_string(&manifest, &["package", "metadata", "canic", "role"], path)?;
    Ok(CanicPackageMetadata { fleet, role })
}

pub(super) fn canic_dependency_feature_snippet<'a>(
    features: impl IntoIterator<Item = &'a str>,
) -> String {
    let features = features
        .into_iter()
        .map(|feature| format!(r#""{feature}""#))
        .collect::<Vec<_>>()
        .join(", ");

    format!("[dependencies]\ncanic = {{ workspace = true, features = [{features}] }}")
}

fn manifest_string(
    manifest: &TomlValue,
    path: &[&str],
    manifest_path: &Path,
) -> Result<String, String> {
    let mut value = manifest;
    for segment in path {
        value = value
            .get(*segment)
            .ok_or_else(|| format!("missing {} in {}", path.join("."), manifest_path.display()))?;
    }
    value.as_str().map(ToString::to_string).ok_or_else(|| {
        format!(
            "{} must be a string in {}",
            path.join("."),
            manifest_path.display()
        )
    })
}
