//! Module: canic_cli::medic::package
//!
//! Responsibility: inspect role-package metadata and resolved Canic dependency features.
//! Does not own: fleet configuration policy, report rendering, or Cargo manifest mutation.
//! Boundary: reads Cargo manifests and returns passive metadata to project medic checks.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};
use toml::Value as TomlValue;

const PACKAGE_MANIFEST_FILE: &str = "Cargo.toml";

pub(super) struct CanicPackageMetadata {
    pub(super) fleet: String,
    pub(super) role: String,
    pub(super) canic_features: BTreeSet<String>,
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
    let canic_features = canic_dependency_features_for_manifest(path, &manifest);
    Ok(CanicPackageMetadata {
        fleet,
        role,
        canic_features,
    })
}

fn canic_dependency_features_for_manifest(path: &Path, manifest: &TomlValue) -> BTreeSet<String> {
    let mut features = canic_dependency_features(manifest);
    if canic_dependency_inherits_workspace(manifest)
        && let Some(manifest_dir) = path.parent()
    {
        features.extend(workspace_canic_dependency_features(manifest_dir));
    }
    features
}

fn canic_dependency_features(manifest: &TomlValue) -> BTreeSet<String> {
    manifest
        .get("dependencies")
        .and_then(|dependencies| dependencies.get("canic"))
        .map(toml_dependency_features)
        .unwrap_or_default()
}

fn canic_dependency_inherits_workspace(manifest: &TomlValue) -> bool {
    manifest
        .get("dependencies")
        .and_then(|dependencies| dependencies.get("canic"))
        .and_then(|canic| canic.get("workspace"))
        .and_then(TomlValue::as_bool)
        .unwrap_or(false)
}

fn workspace_canic_dependency_features(manifest_dir: &Path) -> BTreeSet<String> {
    for dir in manifest_dir.ancestors() {
        let manifest_path = dir.join(PACKAGE_MANIFEST_FILE);
        let Ok(manifest_source) = fs::read_to_string(&manifest_path) else {
            continue;
        };
        let Ok(manifest) = toml::from_str::<TomlValue>(&manifest_source) else {
            continue;
        };
        if let Some(features) = workspace_canic_dependency_features_from_manifest(&manifest) {
            return features;
        }
    }

    BTreeSet::new()
}

pub(super) fn workspace_canic_dependency_features_from_manifest(
    manifest: &TomlValue,
) -> Option<BTreeSet<String>> {
    manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(|dependencies| dependencies.get("canic"))
        .map(toml_dependency_features)
}

fn toml_dependency_features(dependency: &TomlValue) -> BTreeSet<String> {
    dependency
        .get("features")
        .and_then(TomlValue::as_array)
        .into_iter()
        .flatten()
        .filter_map(TomlValue::as_str)
        .map(ToString::to_string)
        .collect()
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
