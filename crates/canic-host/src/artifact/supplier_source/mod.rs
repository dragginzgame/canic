//! Resolve locked supplier sources for artifact builds.

#[cfg(test)]
mod tests;

use super::cargo_failure;
use crate::{Error, Result, supplier};
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

const REGISTRY: &str = "registry+https://github.com/rust-lang/crates.io-index";

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
}

#[derive(Deserialize)]
struct Package {
    id: String,
    name: String,
    version: String,
    source: Option<String>,
    manifest_path: PathBuf,
}

pub(crate) fn manifest(workspace_manifest: &Path, name: &str) -> Result<PathBuf> {
    if !matches!(name, "canic-root" | "ic-wasm-store") {
        return Ok(workspace_manifest.into());
    }
    let workspace = metadata(workspace_manifest)?;
    if name == "canic-root" {
        return root_manifest(workspace_manifest, &workspace);
    }
    let runtime = selected(&workspace)?;
    if !runtime.manifest_path.with_file_name("Cargo.lock").is_file() {
        return Err(Error::Invalid(
            "Store archive lacks its packaged lockfile".into(),
        ));
    }
    // The standalone build uses the archive's lock, not the consumer's graph.
    // Refuse a protocol mismatch instead of silently resolving another version.
    let build = metadata(&runtime.manifest_path)?;
    let protocol = package(&build, "ic-wasm-store-protocol")?;
    if protocol.source.as_deref() != Some(REGISTRY) || protocol.version != runtime.version {
        return Err(Error::Invalid(
            "Store archive lock selects a different protocol".into(),
        ));
    }
    Ok(runtime.manifest_path.clone())
}

fn root_manifest(workspace_manifest: &Path, workspace: &Metadata) -> Result<PathBuf> {
    let root = package(workspace, "canic-root")?;
    if workspace.workspace_members.contains(&root.id) {
        return Ok(workspace_manifest.into());
    }
    if root.source.as_deref() != Some(REGISTRY)
        || !root.manifest_path.with_file_name("Cargo.lock").is_file()
    {
        return Err(Error::Invalid("Root build requires a workspace member or a locked crates.io Root with its packaged Cargo.lock".into()));
    }
    Ok(root.manifest_path.clone())
}

fn metadata(manifest: &Path) -> Result<Metadata> {
    let output = crate::process::capture(
        Command::new("cargo")
            .args([
                "metadata",
                "--locked",
                "--format-version",
                "1",
                "--manifest-path",
            ])
            .arg(manifest),
        8 * 1024 * 1024,
        1024 * 1024,
        Duration::from_secs(120),
    )
    .map_err(cargo_failure)?;
    serde_json::from_slice(&output).map_err(supplier)
}

fn package<'a>(metadata: &'a Metadata, name: &str) -> Result<&'a Package> {
    let mut packages = metadata.packages.iter().filter(|p| p.name == name);
    let package = packages
        .next()
        .ok_or_else(|| Error::Invalid(format!("workspace has no locked {name} dependency")))?;
    if packages.next().is_some() {
        return Err(Error::Invalid(format!(
            "workspace selects multiple {name} sources"
        )));
    }
    Ok(package)
}

fn selected(metadata: &Metadata) -> Result<&Package> {
    let runtime = package(metadata, "ic-wasm-store")?;
    let protocol = package(metadata, "ic-wasm-store-protocol")?;
    if runtime.source.as_deref() != Some(REGISTRY)
        || protocol.source.as_deref() != Some(REGISTRY)
        || runtime.version != protocol.version
    {
        return Err(Error::Invalid(
            "Store build requires matching locked crates.io runtime and protocol versions; source overrides are refused".into(),
        ));
    }
    Ok(runtime)
}
