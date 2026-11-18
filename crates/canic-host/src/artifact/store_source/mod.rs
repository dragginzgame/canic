//! Resolve the registry Store artifact input without linking its runtime into Host.

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
}

#[derive(Deserialize)]
struct Package {
    name: String,
    version: String,
    source: Option<String>,
    manifest_path: PathBuf,
}

pub(super) fn manifest(workspace: &Path) -> Result<PathBuf> {
    let workspace = metadata(workspace)?;
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
