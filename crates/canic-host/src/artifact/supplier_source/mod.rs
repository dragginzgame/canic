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

#[derive(Debug)]
pub(crate) struct BuildSource {
    pub(crate) manifest: PathBuf,
    directory: PathBuf,
    toolchain: Option<PathBuf>,
}

impl BuildSource {
    fn workspace(manifest: &Path) -> Result<Self> {
        let manifest = manifest.canonicalize().map_err(supplier)?;
        let directory = manifest
            .parent()
            .ok_or_else(|| Error::Invalid("manifest has no parent".into()))?
            .into();
        Ok(Self {
            manifest,
            directory,
            toolchain: None,
        })
    }

    fn archive(&self, manifest: &Path) -> Result<Self> {
        let sysroot = crate::process::capture(
            Command::new("rustc")
                .current_dir(&self.directory)
                .args(["--print", "sysroot"]),
            65536,
            65536,
            Duration::from_secs(30),
        )
        .map_err(cargo_failure)?;
        let sysroot = Path::new(std::str::from_utf8(&sysroot).map_err(supplier)?.trim_end());
        if !sysroot.is_absolute() {
            return Err(Error::Invalid(
                "expected an absolute Rust toolchain sysroot".into(),
            ));
        }
        // Discover no consumer-local Cargo configuration, even for a nested registry cache.
        let directory = manifest
            .ancestors()
            .last()
            .filter(|p| p.is_absolute())
            .ok_or_else(|| Error::Invalid("expected an absolute archive manifest".into()))?
            .into();
        Ok(Self {
            manifest: manifest.into(),
            directory,
            toolchain: Some(sysroot.into()),
        })
    }

    pub(crate) fn cargo(&self) -> Command {
        let mut command = Command::new("cargo");
        command.current_dir(&self.directory);
        if let Some(toolchain) = &self.toolchain {
            command.env("RUSTUP_TOOLCHAIN", toolchain);
        }
        command
    }

    fn metadata(&self) -> Result<Metadata> {
        let output = crate::process::capture(
            self.cargo()
                .args([
                    "metadata",
                    "--locked",
                    "--format-version",
                    "1",
                    "--manifest-path",
                ])
                .arg(&self.manifest),
            8 * 1024 * 1024,
            1024 * 1024,
            Duration::from_secs(120),
        )
        .map_err(cargo_failure)?;
        serde_json::from_slice(&output).map_err(supplier)
    }
}

pub(crate) fn manifest(workspace_manifest: &Path, name: &str) -> Result<BuildSource> {
    let consumer = BuildSource::workspace(workspace_manifest)?;
    if !matches!(name, "canic-root" | "ic-wasm-store") {
        return Ok(consumer);
    }
    let workspace = consumer.metadata()?;
    if name != "ic-wasm-store" {
        return canic_manifest(consumer, &workspace, name);
    }
    let runtime = selected(&workspace)?;
    if !runtime.manifest_path.with_file_name("Cargo.lock").is_file() {
        return Err(Error::Invalid(
            "Store archive lacks its packaged lockfile".into(),
        ));
    }
    // The standalone build uses the archive's lock, not the consumer's graph.
    // Refuse a protocol mismatch instead of silently resolving another version.
    let archive = consumer.archive(&runtime.manifest_path)?;
    let build = archive.metadata()?;
    let protocol = package(&build, "ic-wasm-store-protocol")?;
    if protocol.source.as_deref() != Some(REGISTRY) || protocol.version != runtime.version {
        return Err(Error::Invalid(
            "Store archive lock selects a different protocol".into(),
        ));
    }
    Ok(archive)
}

fn canic_manifest(
    mut consumer: BuildSource,
    workspace: &Metadata,
    name: &str,
) -> Result<BuildSource> {
    let root = package(workspace, name)?;
    if workspace.workspace_members.contains(&root.id) {
        return Ok(consumer);
    }
    // A source starter selects both packages from the same Canic checkout.
    // Build the selected Root under that checkout’s locked workspace.
    if root.source.is_none() {
        let runtime = package(workspace, "canic")?;
        if runtime.source.is_none() && root.version == runtime.version {
            consumer.manifest.clone_from(&root.manifest_path);
            let owner = consumer.metadata()?;
            if owner.workspace_members.contains(&root.id)
                && owner.workspace_members.contains(&runtime.id)
            {
                return Ok(consumer);
            }
        }
    }
    if root.source.as_deref() != Some(REGISTRY)
        || !root.manifest_path.with_file_name("Cargo.lock").is_file()
    {
        return Err(Error::Invalid(format!(
            "{name} requires a workspace member, a package from the selected Canic workspace, or a locked registry archive"
        )));
    }
    consumer.archive(&root.manifest_path)
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
