//! Cargo evidence recipes for Canic discovery and role admission.
//!
//! Canic selects features, budgets and graph interpretation. IC Build owns
//! command rendering, Host owns bounded execution, and Cargo owns metadata decoding.

#[cfg(test)]
mod tests;

use crate::cargo_command;
use ic_build::cargo::observation::{
    CargoMetadataSpec, CargoTreeSpec, decode_metadata, schema::Metadata,
};
use ic_build::cargo::selection::CargoFeatureSelection;
use ic_host_process::tool::{
    CompletedExecution, OutputLimit, OutputLimits, ToolError, capture_command,
};
use std::{
    error::Error as StdError,
    fmt,
    path::{Path, PathBuf},
};

/// Cargo's bounded failure output remains available at the consumer diagnostic boundary.
#[derive(Debug)]
struct MetadataObservationError {
    manifest: PathBuf,
    source: ToolError,
}

impl fmt::Display for MetadataObservationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Cargo metadata for {} failed: {}",
            self.manifest.display(),
            self.source
        )?;
        if let Some(execution) = self.source.execution_error()
            && !execution.evidence.stderr.is_empty()
        {
            write!(
                formatter,
                "\n{}",
                String::from_utf8_lossy(&execution.evidence.stderr)
            )?;
        }
        Ok(())
    }
}

impl StdError for MetadataObservationError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.source)
    }
}

const fn limits() -> OutputLimits {
    OutputLimits {
        stdout: OutputLimit::Terminate(16 * 1024 * 1024),
        stderr: OutputLimit::Terminate(1024 * 1024),
        timeout: None,
    }
}

pub fn cargo_metadata(
    workspace_root: &Path,
    include_deps: bool,
) -> Result<Metadata, Box<dyn std::error::Error>> {
    let spec = CargoMetadataSpec {
        manifest_path: &workspace_root.join("Cargo.toml"),
        include_dependencies: include_deps,
        filter_platform: None,
        locked: true,
        offline: false,
        features: &CargoFeatureSelection::default(),
    };
    capture_metadata(&spec)
}

pub fn cargo_metadata_for_manifest(
    manifest_path: &Path,
    filter_platform: &str,
    locked: bool,
    offline: bool,
    features: &CargoFeatureSelection,
) -> Result<Metadata, Box<dyn std::error::Error>> {
    let spec = CargoMetadataSpec {
        manifest_path,
        include_dependencies: true,
        filter_platform: Some(filter_platform),
        locked,
        offline,
        features,
    };
    capture_metadata(&spec)
}

pub fn cargo_metadata_catalog_for_manifest(
    manifest_path: &Path,
    locked: bool,
    offline: bool,
    features: &CargoFeatureSelection,
) -> Result<Metadata, Box<dyn std::error::Error>> {
    let spec = CargoMetadataSpec {
        manifest_path,
        include_dependencies: true,
        filter_platform: None,
        locked,
        offline,
        features,
    };
    capture_metadata(&spec)
}

fn capture_metadata(spec: &CargoMetadataSpec<'_>) -> Result<Metadata, Box<dyn std::error::Error>> {
    let output = capture_command(&mut spec.command(cargo_command())?, limits())
        .and_then(CompletedExecution::require_success)
        .and_then(CompletedExecution::require_complete)
        .map_err(|source| MetadataObservationError {
            manifest: spec.manifest_path.to_path_buf(),
            source,
        })?;
    Ok(decode_metadata(&output.stdout)?)
}

pub fn cargo_tree_for_package(
    manifest_path: &Path,
    package_spec: &str,
    filter_platform: &str,
    locked: bool,
    offline: bool,
    features: &CargoFeatureSelection,
    format: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let spec = CargoTreeSpec {
        manifest_path,
        packages: &[package_spec],
        target: filter_platform,
        edges: "normal",
        format,
        locked,
        offline,
        features,
    };
    let output = capture_command(&mut spec.command(cargo_command())?, limits())?
        .require_success()?
        .require_complete()?;
    Ok(String::from_utf8(output.stdout)?)
}
