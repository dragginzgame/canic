//! Canonical Fleet Subnet Root build-package materialization.
//!
//! Binds the Canic-owned entrypoint to the selected App configuration and exact
//! Canic dependency. Runtime lifecycle and capability policy remain their owners.

#[cfg(test)]
mod tests;

use crate::{
    cargo_metadata::cargo_metadata_catalog_for_manifest,
    fleet_package::{
        self, FleetPackageSpec, resolved_canic_package, resolved_wrapper_dependencies,
    },
    role_contract::PackageValidationMode,
};
use canic_core::{
    bootstrap::compiled::ConfigModel,
    ids::CanisterRole,
    role_contract::{RoleContractFinding, required_features_for_role},
};
use std::path::{Path, PathBuf};

pub const PACKAGE: &str = "canic-fleet-root";
const ENTRYPOINT: &str = "canic::start_fleet_root!();\ncanic::finish!();\n";
const BUILD_SCRIPT: &str = "fn main() { canic::build!(\"canic.toml\"); }\n";

pub fn manifest_path(config_path: &Path) -> PathBuf {
    fleet_package::manifest_path(config_path, PACKAGE)
}

pub fn materialize(
    config_path: &Path,
    config: &ConfigModel,
    mode: PackageValidationMode,
) -> Result<PathBuf, RoleContractFinding> {
    materialize_package(config_path, config, mode).map_err(|error| {
        RoleContractFinding::DependencyShapeUnsupported {
            reason: format!("canonical Fleet Subnet Root package: {error}"),
        }
    })
}

fn materialize_package(
    config_path: &Path,
    config: &ConfigModel,
    mode: PackageValidationMode,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let manifest = manifest_path(config_path);
    if mode != PackageValidationMode::Build {
        return Ok(manifest);
    }
    let cargo_root = config_path
        .ancestors()
        .skip(1)
        .find(|directory| directory.join("Cargo.toml").is_file())
        .ok_or("configuration has no Cargo manifest")?;
    let metadata =
        cargo_metadata_catalog_for_manifest(&cargo_root.join("Cargo.toml"), false, false)?;
    let workspace = &metadata.workspace_root;
    let canic = resolved_canic_package(&metadata)?;
    let dependencies = resolved_wrapper_dependencies(&metadata, canic)?;
    let features = required_features_for_role(config, &CanisterRole::ROOT)
        .map_err(|finding| format!("Root capability contract: {finding:?}"))?
        .into_iter()
        .map(|required| required.feature.cargo_name())
        .collect::<Vec<_>>();
    fleet_package::materialize(
        &manifest,
        workspace,
        &canic.manifest_path,
        &dependencies,
        &FleetPackageSpec {
            package: PACKAGE,
            crate_name: "canic_fleet_root",
            app: config.app_id().as_str(),
            role: "root",
            features: &features,
            entrypoint: ENTRYPOINT,
            build_script: Some(BUILD_SCRIPT),
        },
    )?;
    Ok(manifest)
}
