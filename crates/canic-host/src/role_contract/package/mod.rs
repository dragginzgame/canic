//! Module: role_contract::package
//!
//! Responsibility: validate the single supported wasm runtime Cargo package shape.
//! Does not own: arbitrary Cargo feature resolution or role allocation policy.
//! Boundary: rich metadata stays private; consumers receive compact evidence.

mod graph;
#[cfg(test)]
mod tests;

use self::graph::{CargoGraphEdge, CargoGraphEvidence, TREE_FORMAT, correlate_package_tree};
use crate::cargo_metadata::{
    CargoMetadata, CargoMetadataDependency, CargoMetadataNode, CargoMetadataNodeDependency,
    CargoMetadataPackage, cargo_metadata, cargo_metadata_catalog_for_manifest,
    cargo_metadata_for_manifest, cargo_tree_for_package,
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
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    path::{Path, PathBuf},
};

const WASM_TARGET: &str = "wasm32-unknown-unknown";
const CANIC_PACKAGE: &str = "canic";
const CANIC_CORE_PACKAGE: &str = "canic-core";
struct ProtectedCanicPackage {
    name: &'static str,
    reason: &'static str,
}

const PROTECTED_CANIC_PACKAGES: &[ProtectedCanicPackage] = &[
    ProtectedCanicPackage {
        name: "canic",
        reason: "public runtime facade and role feature authority",
    },
    ProtectedCanicPackage {
        name: "canic-core",
        reason: "runtime, state, lifecycle, authentication, and feature implementation",
    },
    ProtectedCanicPackage {
        name: "canic-control-plane",
        reason: "root and Wasm-store runtime implementation",
    },
    ProtectedCanicPackage {
        name: "canic-macros",
        reason: "compile-time framework coupling",
    },
];

struct ValidatedRoleDeclaration<'a> {
    package: &'a CargoMetadataPackage,
    direct_dependency: &'a CargoMetadataDependency,
    dependency_key: String,
}

///
/// PackageValidationMode
///
/// Cargo resolution fidelity used by the host-owned role evidence producer.
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageValidationMode {
    /// Development evidence that may resolve and update dependencies.
    Build,
    /// Build evidence that must use the existing lockfile but may fetch dependencies.
    LockedBuild,
    /// Read-only evidence that must use the existing lockfile and local cache.
    Passive,
}

impl PackageValidationMode {
    const fn locked(self) -> bool {
        !matches!(self, Self::Build)
    }

    const fn offline(self) -> bool {
        matches!(self, Self::Passive)
    }
}

///
/// RoleCargoGraphEvidence
///
/// Supported role package evidence consumed by build, Medic, and state resolution.
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleCargoGraphEvidence {
    pub fleet: String,
    pub role: CanisterRole,
    pub role_package_name: String,
    pub role_manifest_path: PathBuf,
    pub canic_version: String,
    pub canic_manifest_path: PathBuf,
    pub default_features_enabled: bool,
    pub direct_features: BTreeSet<CanicFeatureKey>,
}

///
/// RolePackageValidation
///
/// Fail-closed result of collecting and checking one role's Cargo graph.
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RolePackageValidation {
    Supported(RoleCargoGraphEvidence),
    Unsupported(RoleContractFinding),
}

pub fn declared_role_manifest_path(
    config_path: &Path,
    role: &CanisterRole,
) -> Result<PathBuf, RoleContractFinding> {
    let config_source = fs::read_to_string(config_path)
        .map_err(|_| RoleContractFinding::PackageMissing { role: role.clone() })?;
    let config = parse_config_model(&config_source).map_err(|_| {
        RoleContractFinding::DependencyShapeUnsupported {
            reason: "invalid role configuration".to_string(),
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
    let Ok(config) = parse_config_model(&config_source) else {
        return unsupported_shape("invalid role configuration".to_string());
    };
    validate_declared_role_package_from_config(config_path, &config, role, mode)
}

#[must_use]
pub fn validate_declared_role_package_from_config(
    config_path: &Path,
    config: &canic_core::bootstrap::compiled::ConfigModel,
    role: &CanisterRole,
    mode: PackageValidationMode,
) -> RolePackageValidation {
    let Some(declaration) = config.roles.get(role) else {
        return RolePackageValidation::Unsupported(RoleContractFinding::RoleUnknown {
            role: role.clone(),
        });
    };
    let Some(fleet) = config.fleet_name() else {
        return unsupported_shape("role configuration is missing [fleet].name".to_string());
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

/// Validate every Canic role package before an internal PocketIC Cargo build.
///
/// Packages without Canic role metadata are test stubs and are ignored. The
/// private build marker may be granted only after this function succeeds.
#[doc(hidden)]
pub fn validate_internal_test_wasm_packages(
    workspace_root: &Path,
    package_names: &[&str],
) -> Result<(), RoleContractFinding> {
    let metadata = cargo_metadata(workspace_root, false)
        .map_err(|_| unsupported_finding("unable to inspect internal test package metadata"))?;

    for package_name in package_names {
        let matches = metadata
            .packages
            .iter()
            .filter(|package| package.name == *package_name)
            .collect::<Vec<_>>();
        let [package] = matches.as_slice() else {
            return Err(unsupported_finding(format!(
                "internal test package `{package_name}` did not resolve exactly once"
            )));
        };
        let Some((fleet, role)) = package_canic_identity(package)? else {
            continue;
        };
        let config_path = package_role_config_path(package, &fleet, &role).ok_or_else(|| {
            unsupported_finding(format!(
                "internal test package `{package_name}` has no matching ancestor canic.toml"
            ))
        })?;
        let evidence = match validate_declared_role_package(
            &config_path,
            &role,
            PackageValidationMode::LockedBuild,
        ) {
            RolePackageValidation::Supported(evidence) => evidence,
            RolePackageValidation::Unsupported(finding) => return Err(finding),
        };
        if evidence.role_package_name != *package_name
            || normalized_manifest_path(&evidence.role_manifest_path)
                != normalized_manifest_path(&package.manifest_path)
        {
            return Err(unsupported_finding(format!(
                "internal test package `{package_name}` does not match the package selected by its role configuration"
            )));
        }
        match super::resolve_declared_role_package_contract(&config_path, &evidence) {
            canic_core::role_contract::RoleContractResolution::Resolved { .. } => {}
            canic_core::role_contract::RoleContractResolution::Rejected { errors } => {
                return Err(errors
                    .into_iter()
                    .next()
                    .expect("rejected role contract has a blocking finding"));
            }
        }
    }

    Ok(())
}

fn package_canic_identity(
    package: &CargoMetadataPackage,
) -> Result<Option<(String, CanisterRole)>, RoleContractFinding> {
    let canic = package
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.get("canic"));
    let fleet = canic
        .and_then(|metadata| metadata.get("fleet"))
        .and_then(serde_json::Value::as_str);
    let role = canic
        .and_then(|metadata| metadata.get("role"))
        .and_then(serde_json::Value::as_str);
    match (canic, fleet, role) {
        (None, _, _) => Ok(None),
        (Some(_), Some(fleet), Some(role)) => Ok(Some((
            fleet.to_string(),
            CanisterRole::owned(role.to_string()),
        ))),
        (Some(_), _, _) => Err(unsupported_finding(format!(
            "internal test package `{}` has incomplete Canic package metadata",
            package.name
        ))),
    }
}

fn package_role_config_path(
    package: &CargoMetadataPackage,
    expected_fleet: &str,
    expected_role: &CanisterRole,
) -> Option<PathBuf> {
    let manifest_dir = package.manifest_path.parent()?;
    for ancestor in manifest_dir.ancestors() {
        let candidate = ancestor.join("canic.toml");
        let Ok(source) = fs::read_to_string(&candidate) else {
            continue;
        };
        let Ok(config) = parse_config_model(&source) else {
            continue;
        };
        if config.fleet_name() != Some(expected_fleet) {
            continue;
        }
        let Some(declaration) = config.roles.get(expected_role) else {
            continue;
        };
        if normalized_manifest_path(&package_manifest_path(&candidate, &declaration.package))
            == normalized_manifest_path(&package.manifest_path)
        {
            return Some(candidate);
        }
    }
    None
}

fn normalized_manifest_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn validate_package_manifest(
    manifest_path: &Path,
    expected_fleet: &str,
    expected_role: &CanisterRole,
    mode: PackageValidationMode,
    built_in: bool,
) -> RolePackageValidation {
    let Ok(metadata) =
        cargo_metadata_for_manifest(manifest_path, WASM_TARGET, mode.locked(), mode.offline())
    else {
        let finding = if built_in {
            RoleContractFinding::BuiltInPackageUnavailable {
                role: BuiltInRoleKind::WasmStore,
            }
        } else {
            RoleContractFinding::DependencyShapeUnsupported {
                reason:
                    "unable to inspect the local wasm runtime graph for the selected role package"
                        .to_string(),
            }
        };
        return RolePackageValidation::Unsupported(finding);
    };

    let selected = match exact_manifest_package(&metadata, manifest_path, expected_role) {
        Ok(selected) => selected,
        Err(finding) => return RolePackageValidation::Unsupported(finding),
    };
    let declaration =
        match validate_role_declaration(&metadata, selected, expected_fleet, expected_role) {
            Ok(declaration) => declaration,
            Err(finding) => return RolePackageValidation::Unsupported(finding),
        };
    if let Err(finding) = validate_catalog() {
        return RolePackageValidation::Unsupported(finding);
    }
    let Ok(catalog) =
        cargo_metadata_catalog_for_manifest(manifest_path, mode.locked(), mode.offline())
    else {
        return unsupported_shape(
            "unable to inspect the Cargo package catalog for the selected role package".to_string(),
        );
    };
    let Ok(tree) = cargo_tree_for_package(
        manifest_path,
        &declaration.package.id,
        WASM_TARGET,
        mode.locked(),
        mode.offline(),
        TREE_FORMAT,
    ) else {
        return unsupported_shape(
            "unable to inspect the package-selected wasm runtime graph for the selected role package"
                .to_string(),
        );
    };
    let graph = match correlate_package_tree(&catalog, &metadata, declaration.package, &tree) {
        Ok(graph) => graph,
        Err(reason) => return unsupported_shape(reason),
    };

    match validate_resolved_package(
        &metadata,
        &graph,
        &declaration,
        expected_fleet,
        expected_role,
    ) {
        Ok(evidence) => RolePackageValidation::Supported(evidence),
        Err(finding) => RolePackageValidation::Unsupported(finding),
    }
}

fn validate_role_declaration<'a>(
    metadata: &CargoMetadata,
    package: &'a CargoMetadataPackage,
    expected_fleet: &str,
    expected_role: &CanisterRole,
) -> Result<ValidatedRoleDeclaration<'a>, RoleContractFinding> {
    validate_package_metadata(package, expected_fleet, expected_role)?;

    let direct_dependency = direct_canic_dependency(package, expected_role)?;
    let dependency_key = direct_dependency
        .rename
        .as_deref()
        .unwrap_or(CANIC_PACKAGE)
        .to_string();
    reject_package_feature_forwarding(package, &dependency_key)?;
    validate_cargo_declarations(metadata, package, direct_dependency)?;

    Ok(ValidatedRoleDeclaration {
        package,
        direct_dependency,
        dependency_key,
    })
}

fn validate_resolved_package(
    metadata: &CargoMetadata,
    graph: &CargoGraphEvidence,
    declaration: &ValidatedRoleDeclaration<'_>,
    expected_fleet: &str,
    expected_role: &CanisterRole,
) -> Result<RoleCargoGraphEvidence, RoleContractFinding> {
    let selected = declaration.package;
    let direct_dependency = declaration.direct_dependency;
    let dependency_key = &declaration.dependency_key;

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
    let dependency_edge_name = cargo_dependency_edge_name(dependency_key);
    let direct_edges = graph
        .edges
        .get(&selected.id)
        .into_iter()
        .flatten()
        .filter(|dependency| dependency.alias == dependency_edge_name)
        .collect::<Vec<_>>();
    let [direct_edge] = direct_edges.as_slice() else {
        return Err(unsupported_finding(
            "the direct Canic dependency did not resolve to exactly one wasm runtime edge",
        ));
    };
    let canic_package = package_by_id
        .get(direct_edge.package_id.as_str())
        .copied()
        .ok_or_else(|| unsupported_finding("resolved Canic package metadata is missing"))?;
    if canic_package.name != CANIC_PACKAGE {
        return Err(unsupported_finding(
            "the direct runtime dependency does not resolve to package `canic`",
        ));
    }

    validate_runtime_graph(graph, direct_edge)?;
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
    validate_selected_canic_features(graph, direct_edge, canic_package, &direct_features)?;

    Ok(RoleCargoGraphEvidence {
        fleet: expected_fleet.to_string(),
        role: expected_role.clone(),
        role_package_name: selected.name.clone(),
        role_manifest_path: selected.manifest_path.clone(),
        canic_version: canic_package.version.clone(),
        canic_manifest_path: canic_package.manifest_path.clone(),
        default_features_enabled,
        direct_features,
    })
}

fn validate_selected_canic_features(
    graph: &CargoGraphEvidence,
    direct_edge: &CargoGraphEdge,
    canic_package: &CargoMetadataPackage,
    direct_features: &BTreeSet<CanicFeatureKey>,
) -> Result<(), RoleContractFinding> {
    let canic = graph
        .packages
        .get(&direct_edge.package_id)
        .ok_or_else(|| unsupported_finding("selected Canic graph evidence is missing"))?;
    let actual = canic
        .enabled_features
        .iter()
        .map(|feature| {
            CanicFeatureKey::from_cargo_name(feature).ok_or_else(|| {
                unsupported_finding(format!(
                    "package-selected Canic graph enables unclassified feature `{feature}`"
                ))
            })
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    let expected = selected_canic_cargo_feature_closure(canic_package, direct_features);
    if actual != expected {
        return Err(unsupported_finding(
            "package-selected Canic features do not match the canonical role declaration and public Cargo implication closure",
        ));
    }
    Ok(())
}

fn selected_canic_cargo_feature_closure(
    canic_package: &CargoMetadataPackage,
    direct_features: &BTreeSet<CanicFeatureKey>,
) -> BTreeSet<CanicFeatureKey> {
    let mut selected = direct_features.clone();
    let mut frontier = direct_features.iter().copied().collect::<Vec<_>>();
    while let Some(feature) = frontier.pop() {
        for member in canic_package
            .features
            .get(feature.cargo_name())
            .into_iter()
            .flatten()
        {
            let Some(implied) = CanicFeatureKey::from_cargo_name(member) else {
                continue;
            };
            if selected.insert(implied) {
                frontier.push(implied);
            }
        }
    }
    selected
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
    if dependency.rename.is_some() {
        return Err(unsupported_finding(
            "the direct normal Canic dependency key must be exactly `canic`",
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

fn validate_cargo_declarations(
    metadata: &CargoMetadata,
    package: &CargoMetadataPackage,
    normal_dependency: &CargoMetadataDependency,
) -> Result<(), RoleContractFinding> {
    let workspace_manifest = metadata.workspace_root.join("Cargo.toml");
    let workspace_document = read_cargo_document(&workspace_manifest)?;
    let resolver = workspace_document
        .get("workspace")
        .and_then(|workspace| workspace.get("resolver"))
        .or_else(|| {
            workspace_document
                .get("package")
                .and_then(|package| package.get("resolver"))
        })
        .and_then(toml::Value::as_str);
    if resolver != Some("2") {
        return Err(unsupported_finding(
            "the top-level Cargo workspace or package must declare resolver = \"2\"",
        ));
    }
    validate_workspace_canic_declaration(&workspace_document)?;

    let role_document = read_cargo_document(&package.manifest_path)?;
    let role_dependency = cargo_dependency_value(&role_document, "dependencies", CANIC_PACKAGE)
        .ok_or_else(|| {
            unsupported_finding(
                "the role manifest must declare the normal Canic dependency under key `canic`",
            )
        })?;
    let role_dependency = dependency_table(role_dependency, "normal Canic dependency")?;
    let role_features = explicit_feature_array(role_dependency, "normal Canic dependency")?;
    if role_features.iter().collect::<BTreeSet<_>>().len() != role_features.len() {
        return Err(unsupported_finding(
            "the normal Canic dependency feature list contains duplicates",
        ));
    }
    validate_dependency_source(
        role_dependency,
        &workspace_document,
        normal_dependency,
        "normal Canic dependency",
    )?;
    let metadata_features = normal_dependency
        .features
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if role_features.iter().copied().collect::<BTreeSet<_>>() != metadata_features {
        return Err(unsupported_finding(
            "the role manifest Canic features do not match Cargo dependency evidence",
        ));
    }
    let build_dependencies = package
        .dependencies
        .iter()
        .filter(|dependency| dependency.name == CANIC_PACKAGE)
        .filter(|dependency| dependency.kind.as_deref() == Some("build"))
        .collect::<Vec<_>>();
    if let Some(dependency) = package.dependencies.iter().find(|dependency| {
        dependency.kind.as_deref() == Some("build")
            && dependency.name != CANIC_PACKAGE
            && protected_canic_package(&dependency.name).is_some()
    }) {
        return Err(unsupported_finding(format!(
            "the role build graph must not depend directly on protected package `{}`",
            dependency.name
        )));
    }
    let [build_dependency] = build_dependencies.as_slice() else {
        return Err(unsupported_finding(
            "the role package must declare exactly one build dependency on `canic`",
        ));
    };
    if build_dependency.rename.is_some()
        || build_dependency.optional
        || build_dependency.target.is_some()
        || !build_dependency.features.is_empty()
    {
        return Err(unsupported_finding(
            "the Canic build dependency must be canonical, unconditional, non-optional, and feature-empty",
        ));
    }
    let build_value = cargo_dependency_value(&role_document, "build-dependencies", CANIC_PACKAGE)
        .ok_or_else(|| {
        unsupported_finding("the role manifest omits its Canic build dependency")
    })?;
    let build_table = dependency_table(build_value, "Canic build dependency")?;
    if let Some(features) = build_table.get("features") {
        let features = features.as_array().ok_or_else(|| {
            unsupported_finding("the Canic build dependency features must be an array")
        })?;
        if !features.is_empty() {
            return Err(unsupported_finding(
                "the Canic build dependency must not select runtime features",
            ));
        }
    }
    validate_dependency_source(
        build_table,
        &workspace_document,
        build_dependency,
        "Canic build dependency",
    )?;
    validate_build_script_purpose(package)?;

    Ok(())
}

fn read_cargo_document(path: &Path) -> Result<toml::Value, RoleContractFinding> {
    let source = fs::read_to_string(path)
        .map_err(|_| unsupported_finding("unable to read selected Cargo manifest"))?;
    toml::from_str(&source)
        .map_err(|_| unsupported_finding("unable to parse selected Cargo manifest"))
}

fn cargo_dependency_value<'a>(
    document: &'a toml::Value,
    section: &str,
    dependency: &str,
) -> Option<&'a toml::Value> {
    document.get(section)?.get(dependency)
}

fn dependency_table<'a>(
    value: &'a toml::Value,
    label: &str,
) -> Result<&'a toml::map::Map<String, toml::Value>, RoleContractFinding> {
    value.as_table().ok_or_else(|| {
        unsupported_finding(format!("the {label} must use an explicit dependency table"))
    })
}

fn explicit_feature_array<'a>(
    dependency: &'a toml::map::Map<String, toml::Value>,
    label: &str,
) -> Result<Vec<&'a str>, RoleContractFinding> {
    dependency
        .get("features")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| {
            unsupported_finding(format!(
                "the {label} must declare an explicit features array"
            ))
        })?
        .iter()
        .map(|feature| {
            feature.as_str().ok_or_else(|| {
                unsupported_finding(format!(
                    "the {label} features array must contain only strings"
                ))
            })
        })
        .collect()
}

fn validate_dependency_source(
    dependency: &toml::map::Map<String, toml::Value>,
    workspace_document: &toml::Value,
    metadata_dependency: &CargoMetadataDependency,
    label: &str,
) -> Result<(), RoleContractFinding> {
    let effective = if dependency.get("workspace").and_then(toml::Value::as_bool) == Some(true) {
        workspace_canic_dependency(workspace_document)?.ok_or_else(|| {
            unsupported_finding(
                "the workspace-inherited Canic dependency has no workspace declaration",
            )
        })?
    } else {
        dependency
    };

    if effective
        .get("default-features")
        .and_then(toml::Value::as_bool)
        != Some(false)
        || metadata_dependency.uses_default_features
    {
        return Err(unsupported_finding(format!(
            "the {label} must disable Canic default features"
        )));
    }
    Ok(())
}

fn validate_workspace_canic_declaration(
    workspace_document: &toml::Value,
) -> Result<(), RoleContractFinding> {
    let Some(dependency) = workspace_canic_dependency(workspace_document)? else {
        return Ok(());
    };
    if let Some(features) = dependency.get("features") {
        let features = features
            .as_array()
            .ok_or_else(|| unsupported_finding("the workspace Canic features must be an array"))?;
        if !features.is_empty() {
            return Err(unsupported_finding(
                "the workspace Canic dependency must not select features",
            ));
        }
    }
    if dependency
        .get("default-features")
        .and_then(toml::Value::as_bool)
        != Some(false)
    {
        return Err(unsupported_finding(
            "the workspace Canic dependency must disable Canic default features",
        ));
    }
    Ok(())
}

fn workspace_canic_dependency(
    workspace_document: &toml::Value,
) -> Result<Option<&toml::map::Map<String, toml::Value>>, RoleContractFinding> {
    let Some(value) = workspace_document
        .get("workspace")
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(|dependencies| dependencies.get(CANIC_PACKAGE))
    else {
        return Ok(None);
    };
    value.as_table().map(Some).ok_or_else(|| {
        unsupported_finding("the workspace Canic dependency must use an explicit dependency table")
    })
}

fn validate_build_script_purpose(
    package: &CargoMetadataPackage,
) -> Result<(), RoleContractFinding> {
    let build_targets = package
        .targets
        .iter()
        .filter(|target| target.kind.iter().any(|kind| kind == "custom-build"))
        .collect::<Vec<_>>();
    let [build_target] = build_targets.as_slice() else {
        return Err(unsupported_finding(
            "the Canic build dependency requires exactly one package build script",
        ));
    };
    let source = fs::read_to_string(&build_target.src_path)
        .map_err(|_| unsupported_finding("unable to read Canic role build script"))?;
    let build_macro_calls = source
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("canic::build!(") && line.ends_with(");"))
        .count();
    if build_macro_calls != 1 {
        return Err(unsupported_finding(
            "the Canic build dependency requires exactly one `canic::build!` invocation",
        ));
    }
    Ok(())
}

fn validate_runtime_graph(
    graph: &CargoGraphEvidence,
    direct_edge: &CargoGraphEdge,
) -> Result<(), RoleContractFinding> {
    let mut canic_package_ids = graph
        .packages
        .iter()
        .filter(|(_, package)| package.name == CANIC_PACKAGE)
        .map(|(package_id, _)| package_id.clone())
        .collect::<BTreeSet<_>>();
    canic_package_ids.insert(direct_edge.package_id.clone());
    if canic_package_ids.len() != 1 {
        return Err(RoleContractFinding::MultipleCanicPackages {
            packages: canic_package_ids
                .iter()
                .filter_map(|package_id| graph.packages.get(package_id))
                .map(|package| normalized_package_description(graph, package))
                .collect(),
        });
    }

    let mut protected_paths = Vec::new();
    for dependency in graph
        .edges
        .get(&graph.selected_package_id)
        .into_iter()
        .flatten()
    {
        if dependency == direct_edge {
            continue;
        }
        if dependency.package_id == direct_edge.package_id {
            return Err(unsupported_finding(
                "the role package has more than one normal runtime path to Canic",
            ));
        }
        if let Some(path) = shortest_protected_path(graph, dependency) {
            protected_paths.push(path);
        }
    }

    protected_paths.sort_by(|left, right| {
        left.edges.len().cmp(&right.edges.len()).then_with(|| {
            left.edges
                .iter()
                .map(|edge| graph_edge_sort_key(graph, edge))
                .cmp(
                    right
                        .edges
                        .iter()
                        .map(|edge| graph_edge_sort_key(graph, edge)),
                )
        })
    });
    if let Some(path) = protected_paths.first() {
        return Err(unsupported_finding(render_protected_path(graph, path)));
    }

    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DependencyPathEvidence {
    edges: Vec<CargoGraphEdge>,
    target_reason: &'static str,
}

fn shortest_protected_path(
    graph: &CargoGraphEvidence,
    first_edge: &CargoGraphEdge,
) -> Option<DependencyPathEvidence> {
    let mut queue = VecDeque::from([(first_edge.clone(), vec![first_edge.clone()])]);
    let mut visited = BTreeSet::new();

    while let Some((edge, path)) = queue.pop_front() {
        if !visited.insert(edge.package_id.clone()) {
            continue;
        }
        let package = graph.packages.get(&edge.package_id)?;
        if let Some(protected) = protected_canic_package(&package.name) {
            return Some(DependencyPathEvidence {
                edges: path,
                target_reason: protected.reason,
            });
        }

        let mut children = graph
            .edges
            .get(&edge.package_id)
            .cloned()
            .unwrap_or_default();
        children.sort_by(|left, right| {
            graph_edge_sort_key(graph, left).cmp(&graph_edge_sort_key(graph, right))
        });
        for child in children {
            if visited.contains(&child.package_id) {
                continue;
            }
            let mut child_path = path.clone();
            child_path.push(child.clone());
            queue.push_back((child, child_path));
        }
    }

    None
}

fn graph_edge_sort_key(
    graph: &CargoGraphEvidence,
    edge: &CargoGraphEdge,
) -> (String, String, String, String, String) {
    let Some(package) = graph.packages.get(&edge.package_id) else {
        return (
            edge.alias.clone(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        );
    };
    (
        edge.alias.clone(),
        package.name.clone(),
        package.version.clone(),
        normalized_source_kind(graph, package).to_string(),
        normalized_workspace_path(graph, package),
    )
}

fn normalized_source_kind(
    graph: &CargoGraphEvidence,
    package: &graph::CargoGraphPackage,
) -> &'static str {
    match package.source.as_deref() {
        None if normalized_workspace_path(graph, package).is_empty() => "external_path",
        None => "workspace_path",
        Some(source) if source.starts_with("registry+") => "registry",
        Some(source) if source.starts_with("git+") => "git",
        Some(_) => "other",
    }
}

fn normalized_workspace_path(
    graph: &CargoGraphEvidence,
    package: &graph::CargoGraphPackage,
) -> String {
    let workspace_root = graph
        .workspace_root
        .canonicalize()
        .unwrap_or_else(|_| graph.workspace_root.clone());
    let manifest_path = package
        .manifest_path
        .canonicalize()
        .unwrap_or_else(|_| package.manifest_path.clone());
    let Some(package_directory) = manifest_path.parent() else {
        return String::new();
    };
    let Ok(relative) = package_directory.strip_prefix(workspace_root) else {
        return String::new();
    };
    relative
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/")
}

fn normalized_package_description(
    graph: &CargoGraphEvidence,
    package: &graph::CargoGraphPackage,
) -> String {
    let source_kind = normalized_source_kind(graph, package);
    let path = normalized_workspace_path(graph, package);
    if path.is_empty() {
        format!("{} {} ({source_kind})", package.name, package.version)
    } else {
        format!(
            "{} {} ({source_kind}:{path})",
            package.name, package.version
        )
    }
}

fn protected_canic_package(name: &str) -> Option<&'static ProtectedCanicPackage> {
    PROTECTED_CANIC_PACKAGES
        .iter()
        .find(|package| package.name == name)
}

fn render_protected_path(graph: &CargoGraphEvidence, path: &DependencyPathEvidence) -> String {
    let role = graph
        .packages
        .get(&graph.selected_package_id)
        .map_or("role package", |package| package.name.as_str());
    let mut rendered = vec![role.to_string()];
    for edge in &path.edges {
        let Some(package) = graph.packages.get(&edge.package_id) else {
            continue;
        };
        if edge.alias == cargo_dependency_edge_name(&package.name) {
            rendered.push(package.name.clone());
        } else {
            rendered.push(format!(
                "{} ({} {})",
                edge.alias, package.name, package.version
            ));
        }
    }
    let target = path
        .edges
        .last()
        .and_then(|edge| graph.packages.get(&edge.package_id))
        .map_or("protected Canic package", |package| package.name.as_str());
    format!(
        "role package `{role}` reaches protected package `{target}` outside its direct Canic subtree: {}; protected ownership: {}",
        rendered.join(" -> "),
        path.target_reason
    )
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
