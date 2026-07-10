//! Module: role_contract::package
//!
//! Responsibility: validate the single supported wasm runtime Cargo package shape.
//! Does not own: arbitrary Cargo feature resolution or role allocation policy.
//! Boundary: rich metadata stays private; consumers receive compact evidence.

use crate::cargo_metadata::{
    CargoMetadata, CargoMetadataDependency, CargoMetadataNode, CargoMetadataNodeDependency,
    CargoMetadataPackage, cargo_metadata_for_manifest,
};
use canic_core::{
    bootstrap::parse_config_model,
    ids::CanisterRole,
    role_contract::{
        BuiltInRoleKind, CanicFeatureKey, RoleContractFinding,
        catalog::{default_features, feature_definitions, implied_features, validate_catalog},
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

const WASM_TARGET: &str = "wasm32-unknown-unknown";
const CANIC_PACKAGE: &str = "canic";
const CANIC_CORE_PACKAGE: &str = "canic-core";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageValidationMode {
    Build,
    Passive,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RolePackageEvidence {
    pub fleet: String,
    pub role: CanisterRole,
    pub role_package_name: String,
    pub role_package_id: String,
    pub role_manifest_path: PathBuf,
    pub canic_package_id: String,
    pub canic_version: String,
    pub canic_source: Option<String>,
    pub canic_manifest_path: PathBuf,
    pub dependency_key: String,
    pub default_features_enabled: bool,
    pub direct_features: BTreeSet<CanicFeatureKey>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RolePackageValidation {
    Supported(RolePackageEvidence),
    Unsupported(RoleContractFinding),
}

pub fn declared_role_manifest_path(
    config_path: &Path,
    role: &CanisterRole,
) -> Result<PathBuf, RoleContractFinding> {
    let config_source = fs::read_to_string(config_path)
        .map_err(|_| RoleContractFinding::PackageMissing { role: role.clone() })?;
    let config = parse_config_model(&config_source).map_err(|error| {
        RoleContractFinding::DependencyShapeUnsupported {
            reason: format!(
                "invalid role configuration {}: {error}",
                config_path.display()
            ),
        }
    })?;
    let declaration = config
        .roles
        .get(role)
        .ok_or_else(|| RoleContractFinding::RoleUnknown { role: role.clone() })?;
    let manifest_path = package_manifest_path(config_path, &declaration.package);
    if !manifest_path.is_file() {
        return Err(RoleContractFinding::PackageMissing { role: role.clone() });
    }
    Ok(manifest_path)
}

#[must_use]
pub fn validate_declared_role_package(
    config_path: &Path,
    role: &CanisterRole,
    mode: PackageValidationMode,
) -> RolePackageValidation {
    let Ok(config_source) = fs::read_to_string(config_path) else {
        return RolePackageValidation::Unsupported(RoleContractFinding::PackageMissing {
            role: role.clone(),
        });
    };
    let config = match parse_config_model(&config_source) {
        Ok(config) => config,
        Err(error) => {
            return unsupported_shape(format!(
                "invalid role configuration {}: {error}",
                config_path.display()
            ));
        }
    };
    let Some(declaration) = config.roles.get(role) else {
        return RolePackageValidation::Unsupported(RoleContractFinding::RoleUnknown {
            role: role.clone(),
        });
    };
    let Some(fleet) = config.fleet_name() else {
        return unsupported_shape(format!("missing [fleet].name in {}", config_path.display()));
    };
    let manifest_path = package_manifest_path(config_path, &declaration.package);
    if !manifest_path.is_file() {
        return RolePackageValidation::Unsupported(RoleContractFinding::PackageMissing {
            role: role.clone(),
        });
    }

    validate_package_manifest(&manifest_path, fleet, role, mode, false)
}

#[must_use]
pub fn validate_built_in_wasm_store_package(
    manifest_path: &Path,
    mode: PackageValidationMode,
) -> RolePackageValidation {
    if !manifest_path.is_file() {
        return RolePackageValidation::Unsupported(
            RoleContractFinding::BuiltInPackageUnavailable {
                role: BuiltInRoleKind::WasmStore,
            },
        );
    }

    validate_package_manifest(
        manifest_path,
        "wasm_store",
        &CanisterRole::WASM_STORE,
        mode,
        true,
    )
}

fn validate_package_manifest(
    manifest_path: &Path,
    expected_fleet: &str,
    expected_role: &CanisterRole,
    mode: PackageValidationMode,
    built_in: bool,
) -> RolePackageValidation {
    let metadata = match cargo_metadata_for_manifest(
        manifest_path,
        WASM_TARGET,
        mode == PackageValidationMode::Passive,
    ) {
        Ok(metadata) => metadata,
        Err(error) => {
            let finding = if built_in {
                RoleContractFinding::BuiltInPackageUnavailable {
                    role: BuiltInRoleKind::WasmStore,
                }
            } else {
                RoleContractFinding::DependencyShapeUnsupported {
                    reason: format!(
                        "unable to inspect the local wasm runtime graph for {}: {error}",
                        manifest_path.display()
                    ),
                }
            };
            return RolePackageValidation::Unsupported(finding);
        }
    };

    match validate_metadata_package(&metadata, manifest_path, expected_fleet, expected_role) {
        Ok(evidence) => RolePackageValidation::Supported(evidence),
        Err(finding) => RolePackageValidation::Unsupported(finding),
    }
}

fn validate_metadata_package(
    metadata: &CargoMetadata,
    manifest_path: &Path,
    expected_fleet: &str,
    expected_role: &CanisterRole,
) -> Result<RolePackageEvidence, RoleContractFinding> {
    validate_catalog()?;
    let selected = exact_manifest_package(metadata, manifest_path, expected_role)?;
    validate_package_metadata(selected, expected_fleet, expected_role)?;

    let direct_dependency = direct_canic_dependency(selected, expected_role)?;
    let dependency_key = direct_dependency
        .rename
        .as_deref()
        .unwrap_or(CANIC_PACKAGE)
        .to_string();
    reject_package_feature_forwarding(selected, &dependency_key)?;

    let package_by_id = metadata
        .packages
        .iter()
        .map(|package| (package.id.as_str(), package))
        .collect::<BTreeMap<_, _>>();
    let node_by_id = metadata
        .resolve
        .as_ref()
        .ok_or_else(|| unsupported_finding("cargo metadata omitted the resolved graph"))?
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    let selected_node = node_by_id
        .get(selected.id.as_str())
        .copied()
        .ok_or_else(|| unsupported_finding("selected role package is absent from the graph"))?;
    let dependency_edge_name = cargo_dependency_edge_name(&dependency_key);
    let direct_edges = normal_dependencies(selected_node)
        .filter(|dependency| dependency.name == dependency_edge_name)
        .collect::<Vec<_>>();
    let [direct_edge] = direct_edges.as_slice() else {
        return Err(unsupported_finding(
            "the direct Canic dependency did not resolve to exactly one wasm runtime edge",
        ));
    };
    let canic_package = package_by_id
        .get(direct_edge.pkg.as_str())
        .copied()
        .ok_or_else(|| unsupported_finding("resolved Canic package metadata is missing"))?;
    if canic_package.name != CANIC_PACKAGE {
        return Err(unsupported_finding(
            "the direct runtime dependency does not resolve to package `canic`",
        ));
    }

    validate_runtime_graph(selected_node, direct_edge, &package_by_id, &node_by_id)?;
    if canic_package.version != env!("CARGO_PKG_VERSION") {
        return Err(RoleContractFinding::CanicVersionMismatch {
            expected: env!("CARGO_PKG_VERSION").to_string(),
            actual: canic_package.version.clone(),
        });
    }
    validate_cargo_catalog_parity(canic_package, &package_by_id, &node_by_id)?;

    let mut default_features_enabled = direct_dependency.uses_default_features;
    let mut direct_features = BTreeSet::new();
    for feature_name in &direct_dependency.features {
        if feature_name == "default" {
            default_features_enabled = true;
            continue;
        }
        let Some(feature) = CanicFeatureKey::from_cargo_name(feature_name) else {
            return Err(RoleContractFinding::CargoCatalogDrift {
                reason: format!(
                    "direct Canic dependency enables unclassified public feature `{feature_name}`"
                ),
            });
        };
        direct_features.insert(feature);
    }

    Ok(RolePackageEvidence {
        fleet: expected_fleet.to_string(),
        role: expected_role.clone(),
        role_package_name: selected.name.clone(),
        role_package_id: selected.id.clone(),
        role_manifest_path: selected.manifest_path.clone(),
        canic_package_id: canic_package.id.clone(),
        canic_version: canic_package.version.clone(),
        canic_source: canic_package.source.clone(),
        canic_manifest_path: canic_package.manifest_path.clone(),
        dependency_key,
        default_features_enabled,
        direct_features,
    })
}

fn exact_manifest_package<'a>(
    metadata: &'a CargoMetadata,
    manifest_path: &Path,
    role: &CanisterRole,
) -> Result<&'a CargoMetadataPackage, RoleContractFinding> {
    let expected = manifest_path
        .canonicalize()
        .unwrap_or_else(|_| manifest_path.to_path_buf());
    let matches = metadata
        .packages
        .iter()
        .filter(|package| {
            package
                .manifest_path
                .canonicalize()
                .unwrap_or_else(|_| package.manifest_path.clone())
                == expected
        })
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [package] => Ok(*package),
        [] => Err(RoleContractFinding::PackageMissing { role: role.clone() }),
        _ => Err(RoleContractFinding::PackageAmbiguous { role: role.clone() }),
    }
}

fn validate_package_metadata(
    package: &CargoMetadataPackage,
    expected_fleet: &str,
    expected_role: &CanisterRole,
) -> Result<(), RoleContractFinding> {
    let canic = package
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.get("canic"));
    let actual_fleet = canic
        .and_then(|metadata| metadata.get("fleet"))
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string);
    let actual_role = canic
        .and_then(|metadata| metadata.get("role"))
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string);

    if actual_fleet.as_deref() == Some(expected_fleet)
        && actual_role.as_deref() == Some(expected_role.as_str())
    {
        return Ok(());
    }

    Err(RoleContractFinding::PackageMetadataMismatch {
        expected_fleet: expected_fleet.to_string(),
        expected_role: expected_role.clone(),
        actual_fleet,
        actual_role,
    })
}

fn direct_canic_dependency<'a>(
    package: &'a CargoMetadataPackage,
    role: &CanisterRole,
) -> Result<&'a CargoMetadataDependency, RoleContractFinding> {
    let dependencies = package
        .dependencies
        .iter()
        .filter(|dependency| dependency.name == CANIC_PACKAGE && dependency.kind.is_none())
        .collect::<Vec<_>>();
    let [dependency] = dependencies.as_slice() else {
        return if dependencies.is_empty() {
            Err(RoleContractFinding::RuntimeCanicDependencyMissing { role: role.clone() })
        } else {
            Err(unsupported_finding(
                "the role package declares more than one normal Canic dependency",
            ))
        };
    };
    if dependency.optional {
        return Err(unsupported_finding(
            "the direct normal Canic dependency must not be optional",
        ));
    }
    if dependency.target.is_some() {
        return Err(unsupported_finding(
            "the direct normal Canic dependency must be unconditional",
        ));
    }
    Ok(*dependency)
}

fn reject_package_feature_forwarding(
    package: &CargoMetadataPackage,
    dependency_key: &str,
) -> Result<(), RoleContractFinding> {
    let strong_prefix = format!("{dependency_key}/");
    let weak_prefix = format!("{dependency_key}?/");
    let optional_dependency = format!("dep:{dependency_key}");
    if package.features.values().flatten().any(|member| {
        member.starts_with(&strong_prefix)
            || member.starts_with(&weak_prefix)
            || member == &optional_dependency
    }) {
        return Err(unsupported_finding(
            "package features must not forward features into the Canic dependency",
        ));
    }
    Ok(())
}

fn validate_runtime_graph(
    selected_node: &CargoMetadataNode,
    direct_edge: &CargoMetadataNodeDependency,
    package_by_id: &BTreeMap<&str, &CargoMetadataPackage>,
    node_by_id: &BTreeMap<&str, &CargoMetadataNode>,
) -> Result<(), RoleContractFinding> {
    let reachable = reachable_normal_packages(selected_node, node_by_id);
    let mut canic_package_ids = reachable
        .iter()
        .filter_map(|package_id| package_by_id.get(package_id.as_str()))
        .filter(|package| package.name == CANIC_PACKAGE)
        .map(|package| package.id.clone())
        .collect::<BTreeSet<_>>();
    canic_package_ids.insert(direct_edge.pkg.clone());
    if canic_package_ids.len() != 1 {
        return Err(RoleContractFinding::MultipleCanicPackages {
            package_ids: canic_package_ids.into_iter().collect(),
        });
    }

    for dependency in normal_dependencies(selected_node) {
        if dependency.name == direct_edge.name && dependency.pkg == direct_edge.pkg {
            continue;
        }
        if dependency.pkg == direct_edge.pkg {
            return Err(unsupported_finding(
                "the role package has more than one normal runtime path to Canic",
            ));
        }
        let Some(node) = node_by_id.get(dependency.pkg.as_str()).copied() else {
            continue;
        };
        if reachable_normal_packages(node, node_by_id).contains(&direct_edge.pkg) {
            return Err(unsupported_finding(
                "a transitive normal runtime dependency also contributes Canic",
            ));
        }
    }

    Ok(())
}

fn reachable_normal_packages(
    start: &CargoMetadataNode,
    node_by_id: &BTreeMap<&str, &CargoMetadataNode>,
) -> BTreeSet<String> {
    let mut reachable = BTreeSet::new();
    let mut frontier = normal_dependencies(start)
        .map(|dependency| dependency.pkg.clone())
        .collect::<Vec<_>>();
    while let Some(package_id) = frontier.pop() {
        if !reachable.insert(package_id.clone()) {
            continue;
        }
        if let Some(node) = node_by_id.get(package_id.as_str()) {
            frontier.extend(normal_dependencies(node).map(|dependency| dependency.pkg.clone()));
        }
    }
    reachable
}

fn normal_dependencies(
    node: &CargoMetadataNode,
) -> impl Iterator<Item = &CargoMetadataNodeDependency> {
    node.deps
        .iter()
        .filter(|dependency| dependency.dep_kinds.iter().any(|kind| kind.kind.is_none()))
}

fn validate_cargo_catalog_parity(
    canic_package: &CargoMetadataPackage,
    package_by_id: &BTreeMap<&str, &CargoMetadataPackage>,
    node_by_id: &BTreeMap<&str, &CargoMetadataNode>,
) -> Result<(), RoleContractFinding> {
    let cargo_public_features = canic_package
        .features
        .keys()
        .filter(|name| name.as_str() != "default")
        .cloned()
        .collect::<BTreeSet<_>>();
    let catalog_public_features = feature_definitions()
        .iter()
        .map(|definition| definition.cargo_name.to_string())
        .collect::<BTreeSet<_>>();
    if cargo_public_features != catalog_public_features {
        return Err(RoleContractFinding::CargoCatalogDrift {
            reason: "resolved Canic public features differ from the role-contract catalog"
                .to_string(),
        });
    }

    let cargo_defaults = canic_package
        .features
        .get("default")
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .collect::<BTreeSet<_>>();
    let catalog_defaults = default_features()
        .iter()
        .map(|feature| feature.cargo_name().to_string())
        .collect::<BTreeSet<_>>();
    if cargo_defaults != catalog_defaults {
        return Err(RoleContractFinding::CargoCatalogDrift {
            reason: "resolved Canic default features differ from the role-contract catalog"
                .to_string(),
        });
    }

    let canic_node = node_by_id
        .get(canic_package.id.as_str())
        .copied()
        .ok_or_else(|| cargo_catalog_drift("resolved Canic graph node is missing"))?;
    let core_dependency = canic_package
        .dependencies
        .iter()
        .find(|dependency| {
            dependency.name == CANIC_CORE_PACKAGE
                && dependency.kind.is_none()
                && dependency.target.is_none()
        })
        .ok_or_else(|| cargo_catalog_drift("resolved Canic core dependency is missing"))?;
    let core_key = core_dependency
        .rename
        .as_deref()
        .unwrap_or(CANIC_CORE_PACKAGE);
    let core_edge_name = cargo_dependency_edge_name(core_key);
    let core_edge = normal_dependencies(canic_node)
        .find(|dependency| dependency.name == core_edge_name)
        .ok_or_else(|| cargo_catalog_drift("resolved Canic core graph edge is missing"))?;
    let core_package = package_by_id
        .get(core_edge.pkg.as_str())
        .copied()
        .ok_or_else(|| cargo_catalog_drift("resolved Canic core package is missing"))?;

    let cargo_implications = cargo_public_implications(
        &canic_package.features,
        &core_package.features,
        core_key,
        &cargo_public_features,
    );
    let catalog_implications = CanicFeatureKey::ALL
        .iter()
        .flat_map(|feature| {
            implied_features(*feature).map(|implied| {
                (
                    feature.cargo_name().to_string(),
                    implied.cargo_name().to_string(),
                )
            })
        })
        .collect::<BTreeSet<_>>();
    if cargo_implications != catalog_implications {
        return Err(RoleContractFinding::CargoCatalogDrift {
            reason:
                "resolved Canic public feature implications differ from the role-contract catalog"
                    .to_string(),
        });
    }

    Ok(())
}

fn cargo_public_implications(
    canic_features: &BTreeMap<String, Vec<String>>,
    core_features: &BTreeMap<String, Vec<String>>,
    core_dependency_key: &str,
    public_features: &BTreeSet<String>,
) -> BTreeSet<(String, String)> {
    let mut implications = BTreeSet::new();
    let core_prefix = format!("{core_dependency_key}/");

    for feature in public_features {
        for member in canic_features.get(feature).into_iter().flatten() {
            if public_features.contains(member) {
                implications.insert((feature.clone(), member.clone()));
                continue;
            }
            let Some(core_feature) = member.strip_prefix(&core_prefix) else {
                continue;
            };
            for core_member in core_features.get(core_feature).into_iter().flatten() {
                if public_features.contains(core_member) {
                    implications.insert((feature.clone(), core_member.clone()));
                }
            }
        }
    }

    implications
}

fn package_manifest_path(config_path: &Path, package: &str) -> PathBuf {
    let package_path = PathBuf::from(package);
    let path = if package_path.is_absolute() {
        package_path
    } else {
        config_path
            .parent()
            .map_or_else(|| PathBuf::from(package), |parent| parent.join(package))
    };
    if path.file_name().and_then(|name| name.to_str()) == Some("Cargo.toml") {
        path
    } else {
        path.join("Cargo.toml")
    }
}

fn cargo_dependency_edge_name(dependency_key: &str) -> String {
    dependency_key.replace('-', "_")
}

const fn unsupported_shape(reason: String) -> RolePackageValidation {
    RolePackageValidation::Unsupported(RoleContractFinding::DependencyShapeUnsupported { reason })
}

fn unsupported_finding(reason: impl Into<String>) -> RoleContractFinding {
    RoleContractFinding::DependencyShapeUnsupported {
        reason: reason.into(),
    }
}

fn cargo_catalog_drift(reason: impl Into<String>) -> RoleContractFinding {
    RoleContractFinding::CargoCatalogDrift {
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests;
