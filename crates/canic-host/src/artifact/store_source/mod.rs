//! Resolve the standalone Store from the same locked Git source as its passive protocol.
use super::cargo_failure;
use crate::{Error, Result, supplier};
use ic_host_process::tool::{CompletedExecution, OutputLimit, OutputLimits, capture_group_command};
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    source: Option<String>,
    manifest_path: PathBuf,
}

pub(super) fn manifest(workspace: &Path) -> Result<PathBuf> {
    let output = capture_group_command(
        Command::new("cargo")
            .args([
                "metadata",
                "--locked",
                "--format-version",
                "1",
                "--manifest-path",
            ])
            .arg(workspace),
        OutputLimits {
            stdout: OutputLimit::Terminate(8 * 1024 * 1024),
            stderr: OutputLimit::Terminate(1024 * 1024),
            timeout: Some(Duration::from_secs(120)),
        },
    )
    .and_then(CompletedExecution::require_success)
    .and_then(CompletedExecution::require_complete)
    .map_err(cargo_failure)?;
    let metadata: Metadata = serde_json::from_slice(&output.stdout).map_err(supplier)?;
    let source = selected(metadata)?;
    let root = source.ancestors().nth(3).ok_or_else(|| {
        Error::Invalid("Store protocol must use the supplier's crates/package layout".into())
    })?;
    let manifest = root.join("Cargo.toml");
    if !root.join("Cargo.lock").is_file() || !root.join("crates/ic-wasm-store/Cargo.toml").is_file()
    {
        return Err(Error::Invalid(
            "Store source lacks its runtime or committed lockfile".into(),
        ));
    }
    Ok(manifest)
}

fn selected(metadata: Metadata) -> Result<PathBuf> {
    let mut packages = metadata
        .packages
        .into_iter()
        .filter(|p| p.name == "ic-wasm-store-protocol");
    let package = packages.next().ok_or_else(|| {
        Error::Invalid("workspace has no locked ic-wasm-store-protocol dependency".into())
    })?;
    if packages.next().is_some() {
        return Err(Error::Invalid(
            "workspace selects multiple Store protocol sources".into(),
        ));
    }
    let source = package.source.as_deref().unwrap_or_default();
    let pin = source
        .strip_prefix("git+https://github.com/dragginzgame/ic-wasm-store?rev=")
        .and_then(|s| s.split_once('#'));
    if !pin.is_some_and(|(requested, resolved)| {
        requested.len() == 40
            && requested.bytes().all(|b| b.is_ascii_hexdigit())
            && requested == resolved
    }) {
        return Err(Error::Invalid("Store build requires an exact ic-wasm-store Git revision; path and branch overrides are refused".into()));
    }
    Ok(package.manifest_path)
}

#[cfg(test)]
mod tests;
