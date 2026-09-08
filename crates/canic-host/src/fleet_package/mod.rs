//! Fleet canister build-package generation.
//!
//! Owns exact dependency selection and thin Cargo entrypoints for the three
//! infrastructure artifacts. Runtime behavior and artifact finalization remain
//! with their existing owners.

#[cfg(test)]
mod tests;

use crate::{
    canister_build::CanisterBuildProfile,
    cargo_metadata::{CargoMetadata, CargoMetadataPackage},
    durable_io::write_bytes,
};
use std::{
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const CANIC_FAMILY_CRATES: &[&str] = &["canic-control-plane", "canic-core", "canic-macros"];
const RELEASE_PROFILE: &[(&str, &str)] = &[
    ("opt-level", "\"z\""),
    ("lto", "true"),
    ("codegen-units", "1"),
    ("strip", "\"symbols\""),
    ("debug", "false"),
    ("panic", "\"abort\""),
    ("overflow-checks", "false"),
    ("incremental", "false"),
];
const FAST_PROFILE: &[(&str, &str)] = &[
    ("inherits", "\"release\""),
    ("lto", "false"),
    ("codegen-units", "16"),
    ("incremental", "false"),
];

/// Exact dependencies selected from the consuming workspace's Canic graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedWrapperDependencies {
    pub canic_version: String,
    pub candid_version: String,
    pub ic_cdk_version: String,
}

/// Role-specific inputs to the shared infrastructure package writer.
pub struct FleetPackageSpec<'a> {
    pub package: &'a str,
    pub crate_name: &'a str,
    pub app: &'a str,
    pub role: &'a str,
    pub features: &'a [&'a str],
    pub entrypoint: &'a str,
    pub build_script: Option<&'a str>,
}

pub fn manifest_path(config_path: &Path, package: &str) -> PathBuf {
    config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(".canic/generated")
        .join(package)
        .join("Cargo.toml")
}

pub fn materialize(
    manifest: &Path,
    workspace: &Path,
    canic_manifest: &Path,
    dependencies: &GeneratedWrapperDependencies,
    spec: &FleetPackageSpec<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let canic_root = canic_manifest
        .parent()
        .ok_or("Canic manifest has no parent")?;
    let mut document = serde_json::json!({
        "package": {
            "name": spec.package, "version": dependencies.canic_version,
            "edition": "2024", "publish": false,
            "metadata": { "canic": { "app": spec.app, "role": spec.role } },
        },
        "workspace": { "resolver": "2" },
        "lib": { "name": spec.crate_name, "crate-type": ["cdylib"] },
        "dependencies": {
            "canic": { "path": canic_root, "default-features": false, "features": spec.features },
            "candid": { "version": format!("={}", dependencies.candid_version), "default-features": false },
            "ic-cdk": { "version": format!("={}", dependencies.ic_cdk_version) },
        },
    });
    if spec.build_script.is_some() {
        document["build-dependencies"] = serde_json::json!({
            "canic": { "path": canic_root, "default-features": false, "features": [] },
        });
    }
    let mut source = toml::to_string(&toml::Value::try_from(document)?)?;
    render_infrastructure_profiles(&mut source);
    source.push_str(&dependency_patch_table(
        canic_manifest,
        &dependencies.canic_version,
    )?);
    let directory = manifest.parent().ok_or("Fleet manifest has no parent")?;
    fs::create_dir_all(directory.join("src"))?;
    write_if_changed(manifest, source.as_bytes())?;
    write_if_changed(&directory.join("src/lib.rs"), spec.entrypoint.as_bytes())?;
    if let Some(script) = spec.build_script {
        write_if_changed(&directory.join("build.rs"), script.as_bytes())?;
    }
    let lock = directory.join("Cargo.lock");
    if !lock.is_file() && workspace.join("Cargo.lock").is_file() {
        fs::copy(workspace.join("Cargo.lock"), lock)?;
    }
    Ok(())
}

fn write_if_changed(path: &Path, bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    if fs::read(path).ok().as_deref() != Some(bytes) {
        write_bytes(path, bytes)?;
    }
    Ok(())
}

pub fn append_infrastructure_profile_args(command: &mut Command, profile: CanisterBuildProfile) {
    match profile {
        CanisterBuildProfile::Debug => {}
        CanisterBuildProfile::Fast => {
            append_profile_config_args(command, "release", RELEASE_PROFILE);
            append_profile_config_args(command, "fast", FAST_PROFILE);
        }
        CanisterBuildProfile::Release => {
            append_profile_config_args(command, "release", RELEASE_PROFILE);
        }
    }
}

pub fn resolved_canic_package(
    metadata: &CargoMetadata,
) -> Result<&CargoMetadataPackage, Box<dyn std::error::Error>> {
    let matches = metadata
        .packages
        .iter()
        .filter(|package| package.name == "canic")
        .collect::<Vec<_>>();
    let [package] = matches.as_slice() else {
        return Err(format!(
            "built-in artifact source requires exactly one resolved 'canic' package; found {}",
            matches.len()
        )
        .into());
    };
    Ok(package)
}

pub fn resolved_wrapper_dependencies(
    metadata: &CargoMetadata,
    canic_package: &CargoMetadataPackage,
) -> Result<GeneratedWrapperDependencies, Box<dyn std::error::Error>> {
    let canic_core = resolved_normal_dependency(metadata, canic_package, "canic-core")?;
    let candid = resolved_normal_dependency(metadata, canic_core, "candid")?;
    let ic_cdk = resolved_normal_dependency(metadata, canic_core, "ic-cdk")?;
    Ok(GeneratedWrapperDependencies {
        canic_version: canic_package.version.clone(),
        candid_version: candid.version.clone(),
        ic_cdk_version: ic_cdk.version.clone(),
    })
}

fn resolved_normal_dependency<'a>(
    metadata: &'a CargoMetadata,
    parent: &CargoMetadataPackage,
    dependency_name: &str,
) -> Result<&'a CargoMetadataPackage, Box<dyn std::error::Error>> {
    let resolve = metadata
        .resolve
        .as_ref()
        .ok_or("Fleet package cargo metadata omitted the resolved dependency graph")?;
    let node = resolve
        .nodes
        .iter()
        .find(|node| node.id == parent.id)
        .ok_or_else(|| {
            format!(
                "Fleet package cargo metadata omitted the graph node for {}",
                parent.name
            )
        })?;
    let matches = node
        .deps
        .iter()
        .filter(|dependency| dependency.dep_kinds.iter().any(|kind| kind.kind.is_none()))
        .filter_map(|dependency| {
            metadata
                .packages
                .iter()
                .find(|package| package.id == dependency.pkg)
        })
        .filter(|package| package.name == dependency_name)
        .collect::<Vec<_>>();
    let [package] = matches.as_slice() else {
        return Err(format!(
            "Fleet package requires exactly one resolved normal {dependency_name} dependency from {}; found {}",
            parent.name,
            matches.len()
        )
        .into());
    };
    Ok(package)
}

// Generate the `[patch.crates-io]` table for sibling packaged Canic crates.
pub fn dependency_patch_table(
    canic_manifest_path: &Path,
    canic_version: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let canic_root = canic_manifest_path
        .parent()
        .expect("canic manifest path must have parent");
    let sibling_root = canic_root.parent().expect("canic root must have parent");
    let registry_version = registry_package_version_suffix(canic_manifest_path, "canic")
        .filter(|version| *version == canic_version);
    let mut rendered = String::new();

    for crate_name in CANIC_FAMILY_CRATES {
        let sibling_dir = registry_version.map_or_else(
            || (*crate_name).to_string(),
            |version| format!("{crate_name}-{version}"),
        );
        let manifest_path = sibling_root.join(sibling_dir).join("Cargo.toml");

        if !manifest_path.is_file() {
            continue;
        }
        require_package_manifest_identity(&manifest_path, crate_name, canic_version)?;

        let crate_root = manifest_path
            .parent()
            .expect("manifest path must have parent");
        let _ = writeln!(
            rendered,
            "{crate_name} = {{ path = \"{}\" }}",
            crate_root.display()
        );
    }

    if rendered.is_empty() {
        Ok(String::new())
    } else {
        Ok(format!("[patch.crates-io]\n{rendered}"))
    }
}

pub fn require_package_manifest_identity(
    manifest_path: &Path,
    expected_name: &str,
    expected_version: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(manifest_path)?;
    let manifest = toml::from_str::<toml::Value>(&source)?;
    let package = manifest
        .get("package")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| format!("package table missing from {}", manifest_path.display()))?;
    let name = package
        .get("name")
        .and_then(toml::Value::as_str)
        .unwrap_or_default();
    let version = package.get("version");
    let version_matches = version
        .and_then(toml::Value::as_str)
        .is_some_and(|version| version == expected_version)
        || version
            .and_then(toml::Value::as_table)
            .and_then(|version| version.get("workspace"))
            .and_then(toml::Value::as_bool)
            == Some(true);
    if name != expected_name || !version_matches {
        let observed_version = version
            .and_then(toml::Value::as_str)
            .unwrap_or("<not an exact or workspace version>");
        return Err(format!(
            "built-in artifact sibling {} must be package {expected_name} {expected_version}; found {name} {observed_version}",
            manifest_path.display()
        )
        .into());
    }
    Ok(())
}

pub fn registry_package_version_suffix<'a>(
    manifest_path: &'a Path,
    crate_name: &str,
) -> Option<&'a str> {
    let parent_name = manifest_path.parent()?.file_name()?.to_str()?;
    parent_name.strip_prefix(&format!("{crate_name}-"))
}

pub fn render_profile(output: &mut String, profile: &str, settings: &[(&str, &str)]) {
    let _ = writeln!(output, "\n[profile.{profile}]");
    for (key, value) in settings {
        let _ = writeln!(output, "{key} = {value}");
    }
}

pub fn render_infrastructure_profiles(output: &mut String) {
    render_profile(output, "release", RELEASE_PROFILE);
    render_profile(output, "fast", FAST_PROFILE);
}

pub fn append_profile_config_args(command: &mut Command, profile: &str, settings: &[(&str, &str)]) {
    for (key, value) in settings {
        command
            .arg("--config")
            .arg(format!("profile.{profile}.{key}={value}"));
    }
}
