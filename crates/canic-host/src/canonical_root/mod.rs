//! Canonical Fleet Subnet Root build-package materialization.
//!
//! Binds the Canic-owned entrypoint to the selected App configuration and exact
//! Canic dependency. Runtime lifecycle and capability policy remain their owners.

#[cfg(test)]
mod tests;

use crate::{
    bootstrap_store::{
        generated_wasm_store_wrapper_patch_table, render_infrastructure_profiles,
        resolved_canic_package, resolved_wrapper_dependencies,
    },
    cargo_metadata::cargo_metadata_catalog_for_manifest,
    durable_io::write_bytes,
    role_contract::PackageValidationMode,
};
use canic_core::{
    bootstrap::compiled::ConfigModel,
    ids::CanisterRole,
    role_contract::{RoleContractFinding, required_features_for_role},
};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub const PACKAGE: &str = "canic-fleet-root";
const ENTRYPOINT: &str = "canic::start_fleet_root!();\ncanic::finish!();\n";
const BUILD_SCRIPT: &str = "fn main() { canic::build!(\"canic.toml\"); }\n";

pub fn manifest_path(config_path: &Path) -> PathBuf {
    config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(".canic/generated/canic-fleet-root/Cargo.toml")
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
    let canic_root = canic
        .manifest_path
        .parent()
        .ok_or("Canic manifest has no parent")?;
    let features = required_features_for_role(config, &CanisterRole::ROOT)
        .map_err(|finding| format!("Root capability contract: {finding:?}"))?
        .into_iter()
        .map(|required| required.feature.cargo_name())
        .collect::<Vec<_>>();
    let document = serde_json::json!({
        "package": {
            "name": PACKAGE, "version": canic.version, "edition": "2024", "publish": false,
            "metadata": { "canic": { "app": config.app_id().as_str(), "role": "root" } },
        },
        "workspace": { "resolver": "2" },
        "lib": { "crate-type": ["cdylib"] },
        "dependencies": {
            "canic": { "path": canic_root, "default-features": false, "features": features },
            "candid": { "version": format!("={}", dependencies.candid_version), "default-features": false },
            "ic-cdk": { "version": format!("={}", dependencies.ic_cdk_version) },
        },
        "build-dependencies": {
            "canic": { "path": canic_root, "default-features": false, "features": [] },
        },
    });
    let mut source = toml::to_string(&toml::Value::try_from(document)?)?;
    render_infrastructure_profiles(&mut source);
    source.push_str(&generated_wasm_store_wrapper_patch_table(
        &canic.manifest_path,
        &canic.version,
    )?);
    let directory = manifest.parent().ok_or("Root manifest has no parent")?;
    fs::create_dir_all(directory.join("src"))?;
    for (path, bytes) in [
        (manifest.clone(), source.as_bytes()),
        (directory.join("src/lib.rs"), ENTRYPOINT.as_bytes()),
        (directory.join("build.rs"), BUILD_SCRIPT.as_bytes()),
    ] {
        if fs::read(&path).ok().as_deref() != Some(bytes) {
            write_bytes(&path, bytes)?;
        }
    }
    let lock = directory.join("Cargo.lock");
    if !lock.is_file() && workspace.join("Cargo.lock").is_file() {
        fs::copy(workspace.join("Cargo.lock"), lock)?;
    }
    Ok(manifest)
}
