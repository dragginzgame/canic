use super::*;
use crate::cargo_metadata::{CargoMetadataDependencyKind, CargoMetadataNodeDependency};
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};

static FIXTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[test]
fn isolated_supported_role_workspace_is_accepted() {
    let fixture = FixtureWorkspace::materialize("supported");

    let validation = validate_declared_role_package(
        &fixture.root.join("canic.toml"),
        &CanisterRole::owned("app".to_string()),
        PackageValidationMode::Build,
    );
    assert!(
        matches!(validation, RolePackageValidation::Supported(_)),
        "unexpected validation: {validation:?}"
    );
}

#[test]
fn isolated_renamed_canic_workspace_is_rejected() {
    let fixture = FixtureWorkspace::materialize("renamed_canic");

    assert!(matches!(
        validate_declared_role_package(
            &fixture.root.join("canic.toml"),
            &CanisterRole::owned("app".to_string()),
            PackageValidationMode::Build,
        ),
        RolePackageValidation::Unsupported(RoleContractFinding::DependencyShapeUnsupported { .. })
    ));
}

#[test]
fn isolated_protected_sibling_workspace_reports_the_exact_path() {
    let fixture = FixtureWorkspace::materialize("protected_sibling");

    let RolePackageValidation::Unsupported(RoleContractFinding::DependencyShapeUnsupported {
        reason,
    }) = validate_declared_role_package(
        &fixture.root.join("canic.toml"),
        &CanisterRole::owned("app".to_string()),
        PackageValidationMode::Build,
    )
    else {
        panic!("protected sibling dependency must be rejected");
    };
    assert!(
        reason.contains("fixture_protected_sibling_role -> fixture_protected_helper -> canic-core"),
        "unexpected reason: {reason}"
    );
}

#[test]
fn isolated_role_workspace_rejects_missing_resolver_two() {
    let fixture = FixtureWorkspace::materialize("supported");
    fixture.rewrite("Cargo.toml", "resolver = \"2\"\n", "");

    let reason = fixture.rejection_reason();

    assert!(reason.contains("must declare resolver = \"2\""));
}

#[test]
fn isolated_role_workspace_rejects_unreviewed_resolver_three() {
    let fixture = FixtureWorkspace::materialize("supported");
    fixture.rewrite("Cargo.toml", "resolver = \"2\"", "resolver = \"3\"");

    let reason = fixture.rejection_reason();

    assert!(reason.contains("must declare resolver = \"2\""));
}

#[test]
fn isolated_role_workspace_rejects_enabled_workspace_defaults() {
    let fixture = FixtureWorkspace::materialize("supported");
    fixture.rewrite(
        "Cargo.toml",
        "default-features = false",
        "default-features = true",
    );

    let reason = fixture.rejection_reason();

    assert!(reason.contains("must disable Canic default features"));
}

#[test]
fn workspace_canic_declaration_never_owns_features() {
    let workspace = toml::from_str(
        r#"
[workspace]
resolver = "2"

[workspace.dependencies]
canic = { path = "canic", default-features = false, features = ["metrics"] }
"#,
    )
    .expect("workspace manifest");

    let Err(RoleContractFinding::DependencyShapeUnsupported { reason }) =
        validate_workspace_canic_declaration(&workspace)
    else {
        panic!("workspace-owned Canic features must be rejected");
    };
    assert!(reason.contains("workspace Canic dependency must not select features"));
}

#[test]
fn isolated_role_workspace_rejects_omitted_role_features() {
    let fixture = FixtureWorkspace::materialize("protected_sibling");
    fixture.rewrite(
        "role/Cargo.toml",
        "canic = { workspace = true, features = [\"metrics\"] }",
        "canic = { workspace = true }",
    );

    let reason = fixture.rejection_reason();

    assert!(
        reason.contains("must declare an explicit features array"),
        "unexpected reason: {reason}"
    );
}

#[test]
fn isolated_role_workspace_rejects_build_runtime_features() {
    let fixture = FixtureWorkspace::materialize("supported");
    fixture.rewrite(
        "role/Cargo.toml",
        "canic = { workspace = true, features = [] }",
        "canic = { workspace = true, features = [\"metrics\"] }",
    );

    let reason = fixture.rejection_reason();

    assert!(reason.contains("build dependency must be canonical"));
}

#[test]
fn isolated_role_workspace_rejects_unowned_build_dependency() {
    let fixture = FixtureWorkspace::materialize("supported");
    fixture.rewrite(
        "role/build.rs",
        "    canic::build!(\"../canic.toml\");\n",
        "",
    );

    let reason = fixture.rejection_reason();

    assert!(reason.contains("requires exactly one `canic::build!` invocation"));
}

#[test]
fn repository_canic_runtime_closure_matches_the_protected_catalog() {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let metadata = crate::cargo_metadata::cargo_metadata_catalog_for_manifest(
        &workspace.join("crates/canic/Cargo.toml"),
        true,
        true,
    )
    .expect("workspace Cargo catalog");
    let canic = metadata
        .packages
        .iter()
        .find(|package| {
            package.name == CANIC_PACKAGE
                && normalized_manifest_path(&package.manifest_path)
                    == normalized_manifest_path(&workspace.join("crates/canic/Cargo.toml"))
        })
        .expect("workspace Canic package");
    let nodes = metadata
        .resolve
        .as_ref()
        .expect("workspace Cargo resolve")
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    let packages = metadata
        .packages
        .iter()
        .map(|package| (package.id.as_str(), package))
        .collect::<BTreeMap<_, _>>();
    let mut reachable = BTreeSet::from([canic.id.clone()]);
    let mut frontier = vec![canic.id.clone()];
    while let Some(package_id) = frontier.pop() {
        let Some(node) = nodes.get(package_id.as_str()) else {
            continue;
        };
        for dependency in normal_dependencies(node) {
            if reachable.insert(dependency.pkg.clone()) {
                frontier.push(dependency.pkg.clone());
            }
        }
    }
    let crates_root = normalized_manifest_path(&workspace.join("crates"));
    let actual = reachable
        .iter()
        .filter_map(|package_id| packages.get(package_id.as_str()))
        .filter(|package| {
            package.name.starts_with("canic")
                && normalized_manifest_path(&package.manifest_path).starts_with(&crates_root)
        })
        .map(|package| package.name.as_str())
        .collect::<BTreeSet<_>>();
    let expected = PROTECTED_CANIC_PACKAGES
        .iter()
        .map(|package| package.name)
        .collect::<BTreeSet<_>>();

    assert_eq!(actual, expected);
    assert_eq!(expected.len(), PROTECTED_CANIC_PACKAGES.len());
    assert!(
        PROTECTED_CANIC_PACKAGES
            .iter()
            .all(|package| !package.reason.is_empty())
    );
}

#[test]
fn direct_dependency_rejects_a_renamed_canic_key() {
    let mut package = package("role", "role@1", "/tmp/role/Cargo.toml");
    package.dependencies.push(CargoMetadataDependency {
        name: CANIC_PACKAGE.to_string(),
        kind: None,
        rename: Some("framework".to_string()),
        optional: false,
        uses_default_features: false,
        features: vec!["metrics".to_string()],
        target: None,
    });

    assert!(matches!(
        direct_canic_dependency(&package, &CanisterRole::owned("app".to_string())),
        Err(RoleContractFinding::DependencyShapeUnsupported { .. })
    ));
}

#[test]
fn direct_dependency_rejects_optional_and_target_specific_shapes() {
    for (optional, target) in [
        (true, None),
        (false, Some("cfg(target_arch = \"wasm32\")".to_string())),
    ] {
        let mut package = package("role", "role@1", "/tmp/role/Cargo.toml");
        package.dependencies.push(CargoMetadataDependency {
            name: CANIC_PACKAGE.to_string(),
            kind: None,
            rename: None,
            optional,
            uses_default_features: true,
            features: Vec::new(),
            target,
        });

        assert!(matches!(
            direct_canic_dependency(&package, &CanisterRole::owned("app".to_string())),
            Err(RoleContractFinding::DependencyShapeUnsupported { .. })
        ));
    }
}

#[test]
fn package_feature_forwarding_is_rejected() {
    let mut package = package("role", "role@1", "/tmp/role/Cargo.toml");
    package.features.insert(
        "storage".to_string(),
        vec!["framework/blob-storage".to_string()],
    );

    assert!(matches!(
        reject_package_feature_forwarding(&package, "framework"),
        Err(RoleContractFinding::DependencyShapeUnsupported { .. })
    ));
}

#[test]
fn transitive_runtime_canic_path_is_rejected() {
    let packages = [
        package("role", "role@1", "/tmp/role/Cargo.toml"),
        package("helper", "helper@1", "/tmp/helper/Cargo.toml"),
        package(CANIC_PACKAGE, "canic@1", "/tmp/canic/Cargo.toml"),
    ];
    let nodes = [
        node(
            "role@1",
            vec![
                normal_edge("canic", "canic@1"),
                normal_edge("helper", "helper@1"),
            ],
        ),
        node("helper@1", vec![normal_edge("canic", "canic@1")]),
        node("canic@1", Vec::new()),
    ];
    let graph = runtime_graph(&packages, &nodes);
    let direct_edge = graph.edges["role@1"]
        .iter()
        .find(|edge| edge.alias == "canic")
        .expect("direct Canic edge");

    assert!(matches!(
        validate_runtime_graph(&graph, direct_edge),
        Err(RoleContractFinding::DependencyShapeUnsupported { .. })
    ));
}

#[test]
fn selected_canic_features_accept_only_public_cargo_implications() {
    let mut packages = [
        package("role", "role@1", "/tmp/role/Cargo.toml"),
        package(CANIC_PACKAGE, "canic@1", "/tmp/canic/Cargo.toml"),
    ];
    packages[1].features.insert(
        "blob-storage-billing".to_string(),
        vec![
            "blob-storage".to_string(),
            "canic-core/blob-storage-billing".to_string(),
        ],
    );
    let nodes = [
        node("role@1", vec![normal_edge("canic", "canic@1")]),
        node("canic@1", Vec::new()),
    ];
    let mut graph = runtime_graph(&packages, &nodes);
    graph
        .packages
        .get_mut("canic@1")
        .expect("Canic graph package")
        .enabled_features = BTreeSet::from([
        "blob-storage".to_string(),
        "blob-storage-billing".to_string(),
    ]);
    let direct_edge = &graph.edges["role@1"][0];
    let declared = BTreeSet::from([CanicFeatureKey::BlobStorageBilling]);

    validate_selected_canic_features(&graph, direct_edge, &packages[1], &declared)
        .expect("public Cargo-implied blob storage feature");

    graph
        .packages
        .get_mut("canic@1")
        .expect("Canic graph package")
        .enabled_features
        .insert("metrics".to_string());
    assert!(
        validate_selected_canic_features(&graph, direct_edge, &packages[1], &declared).is_err()
    );
}

#[test]
fn alternate_protected_package_path_names_the_target_and_dependency_alias() {
    let packages = [
        package("role", "role@1", "/tmp/role/Cargo.toml"),
        package("helper", "helper@1", "/tmp/helper/Cargo.toml"),
        package(CANIC_PACKAGE, "canic@1", "/tmp/canic/Cargo.toml"),
        package(
            "canic-control-plane",
            "control@1",
            "/tmp/control/Cargo.toml",
        ),
    ];
    let nodes = [
        node(
            "role@1",
            vec![
                normal_edge("canic", "canic@1"),
                normal_edge("runtime_bridge", "helper@1"),
            ],
        ),
        node(
            "helper@1",
            vec![normal_edge("canic_control_plane", "control@1")],
        ),
        node("canic@1", Vec::new()),
        node("control@1", Vec::new()),
    ];
    let graph = runtime_graph(&packages, &nodes);
    let direct_edge = graph.edges["role@1"]
        .iter()
        .find(|edge| edge.alias == "canic")
        .expect("direct Canic edge");

    let Err(RoleContractFinding::DependencyShapeUnsupported { reason }) =
        validate_runtime_graph(&graph, direct_edge)
    else {
        panic!("protected control-plane path must be rejected");
    };
    assert!(reason.contains("protected package `canic-control-plane`"));
    assert!(reason.contains(&format!(
        "runtime_bridge (helper {})",
        env!("CARGO_PKG_VERSION")
    )));
}

#[test]
fn non_protected_dependency_cycle_is_bounded_and_accepted() {
    let packages = [
        package("role", "role@1", "/tmp/role/Cargo.toml"),
        package("helper", "helper@1", "/tmp/helper/Cargo.toml"),
        package("adapter", "adapter@1", "/tmp/adapter/Cargo.toml"),
        package(CANIC_PACKAGE, "canic@1", "/tmp/canic/Cargo.toml"),
    ];
    let nodes = [
        node(
            "role@1",
            vec![
                normal_edge("canic", "canic@1"),
                normal_edge("helper", "helper@1"),
            ],
        ),
        node("helper@1", vec![normal_edge("adapter", "adapter@1")]),
        node("adapter@1", vec![normal_edge("helper", "helper@1")]),
        node("canic@1", Vec::new()),
    ];
    let graph = runtime_graph(&packages, &nodes);
    let direct_edge = graph.edges["role@1"]
        .iter()
        .find(|edge| edge.alias == "canic")
        .expect("direct Canic edge");

    validate_runtime_graph(&graph, direct_edge).expect("non-protected cycle remains bounded");
}

#[test]
fn multiple_runtime_canic_packages_are_rejected() {
    let packages = [
        package("role", "role@1", "/tmp/role/Cargo.toml"),
        package("helper", "helper@1", "/tmp/helper/Cargo.toml"),
        package(CANIC_PACKAGE, "canic@1", "/tmp/canic-1/Cargo.toml"),
        package(CANIC_PACKAGE, "canic@2", "/tmp/canic-2/Cargo.toml"),
    ];
    let nodes = [
        node(
            "role@1",
            vec![
                normal_edge("canic", "canic@1"),
                normal_edge("helper", "helper@1"),
            ],
        ),
        node("helper@1", vec![normal_edge("canic", "canic@2")]),
        node("canic@1", Vec::new()),
        node("canic@2", Vec::new()),
    ];
    let graph = runtime_graph(&packages, &nodes);
    let direct_edge = graph.edges["role@1"]
        .iter()
        .find(|edge| edge.alias == "canic")
        .expect("direct Canic edge");

    let Err(RoleContractFinding::MultipleCanicPackages { packages }) =
        validate_runtime_graph(&graph, direct_edge)
    else {
        panic!("multiple Canic packages must be rejected");
    };
    assert_eq!(packages.len(), 2);
    assert!(packages.iter().all(|package| !package.contains("/tmp")));
    assert!(packages.iter().all(|package| !package.contains('@')));
}

#[test]
fn build_only_canic_path_does_not_enter_the_runtime_graph() {
    let packages = [
        package("role", "role@1", "/tmp/role/Cargo.toml"),
        package("helper", "helper@1", "/tmp/helper/Cargo.toml"),
        package(CANIC_PACKAGE, "canic@1", "/tmp/canic/Cargo.toml"),
    ];
    let nodes = [
        node(
            "role@1",
            vec![
                normal_edge("canic", "canic@1"),
                normal_edge("helper", "helper@1"),
            ],
        ),
        node("helper@1", vec![build_edge("canic", "canic@1")]),
        node("canic@1", Vec::new()),
    ];
    let graph = runtime_graph(&packages, &nodes);
    let direct_edge = graph.edges["role@1"]
        .iter()
        .find(|edge| edge.alias == "canic")
        .expect("direct Canic edge");

    validate_runtime_graph(&graph, direct_edge)
        .expect("build dependencies are not wasm runtime evidence");
}

#[test]
fn internal_pocketic_packages_are_validated_before_the_marker_is_granted() {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let lockfile = workspace.join("Cargo.lock");
    let lockfile_before = fs::read(&lockfile).expect("read workspace lockfile");

    validate_internal_test_wasm_packages(&workspace, &["sharding_root_stub", "canister_user_hub"])
        .expect("internal PocketIC package validation");

    assert_eq!(
        fs::read(lockfile).expect("reread workspace lockfile"),
        lockfile_before,
        "locked internal build validation must not refresh the workspace lockfile"
    );
}

#[test]
fn built_in_wasm_store_uses_the_canonical_role_graph_contract() {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let validation = validate_built_in_wasm_store_package(
        &workspace.join("crates/canic-wasm-store/Cargo.toml"),
        PackageValidationMode::LockedBuild,
    );
    let RolePackageValidation::Supported(evidence) = validation else {
        panic!("unexpected built-in validation: {validation:?}");
    };

    assert_eq!(evidence.role, CanisterRole::WASM_STORE);
    assert!(!evidence.default_features_enabled);
    assert_eq!(
        evidence.direct_features,
        BTreeSet::from([CanicFeatureKey::Metrics, CanicFeatureKey::WasmStoreCanister,])
    );
}

fn package(name: &str, id: &str, manifest_path: &str) -> CargoMetadataPackage {
    CargoMetadataPackage {
        id: id.to_string(),
        name: name.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        source: None,
        manifest_path: PathBuf::from(manifest_path),
        metadata: Some(json!({})),
        dependencies: Vec::new(),
        features: BTreeMap::new(),
        targets: vec![crate::cargo_metadata::CargoMetadataTarget {
            name: name.replace('-', "_"),
            kind: vec!["lib".to_string()],
            src_path: PathBuf::from(manifest_path).with_file_name("src/lib.rs"),
        }],
    }
}

fn runtime_graph(
    packages: &[CargoMetadataPackage],
    nodes: &[CargoMetadataNode],
) -> CargoGraphEvidence {
    CargoGraphEvidence {
        selected_package_id: "role@1".to_string(),
        workspace_root: PathBuf::from("/tmp"),
        packages: packages
            .iter()
            .map(|package| {
                (
                    package.id.clone(),
                    graph::CargoGraphPackage {
                        name: package.name.clone(),
                        version: package.version.clone(),
                        source: package.source.clone(),
                        manifest_path: package.manifest_path.clone(),
                        enabled_features: BTreeSet::new(),
                    },
                )
            })
            .collect(),
        edges: nodes
            .iter()
            .filter_map(|node| {
                let edges = node
                    .deps
                    .iter()
                    .filter(|dependency| {
                        dependency.dep_kinds.iter().any(|kind| kind.kind.is_none())
                    })
                    .map(|dependency| CargoGraphEdge {
                        alias: dependency.name.clone(),
                        package_id: dependency.pkg.clone(),
                    })
                    .collect::<Vec<_>>();
                (!edges.is_empty()).then(|| (node.id.clone(), edges))
            })
            .collect(),
    }
}

fn node(id: &str, deps: Vec<CargoMetadataNodeDependency>) -> CargoMetadataNode {
    CargoMetadataNode {
        id: id.to_string(),
        deps,
    }
}

fn normal_edge(name: &str, package_id: &str) -> CargoMetadataNodeDependency {
    edge(name, package_id, None)
}

fn build_edge(name: &str, package_id: &str) -> CargoMetadataNodeDependency {
    edge(name, package_id, Some("build"))
}

fn edge(name: &str, package_id: &str, kind: Option<&str>) -> CargoMetadataNodeDependency {
    CargoMetadataNodeDependency {
        name: name.to_string(),
        pkg: package_id.to_string(),
        dep_kinds: vec![CargoMetadataDependencyKind {
            kind: kind.map(ToString::to_string),
        }],
    }
}

struct FixtureWorkspace {
    root: PathBuf,
}

impl FixtureWorkspace {
    fn materialize(name: &str) -> Self {
        let sequence = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "canic-role-contract-fixture-{}-{sequence}-{name}",
            std::process::id()
        ));
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/role_contract")
            .join(name);
        copy_fixture_directory(&source, &root).expect("copy fixture workspace");

        let canic_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("crates directory")
            .join("canic");
        let canic_core_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("crates directory")
            .join("canic-core");
        rewrite_fixture_manifests(&root, &canic_path, &canic_core_path)
            .expect("rewrite fixture dependency paths");
        Self { root }
    }

    fn rewrite(&self, relative_path: &str, from: &str, to: &str) {
        let path = self.root.join(relative_path);
        let source = fs::read_to_string(&path).expect("read fixture source");
        assert!(
            source.contains(from),
            "fixture source {} does not contain expected text",
            path.display()
        );
        fs::write(path, source.replacen(from, to, 1)).expect("rewrite fixture source");
    }

    fn rejection_reason(&self) -> String {
        let validation = validate_declared_role_package(
            &self.root.join("canic.toml"),
            &CanisterRole::owned("app".to_string()),
            PackageValidationMode::Build,
        );
        let RolePackageValidation::Unsupported(RoleContractFinding::DependencyShapeUnsupported {
            reason,
        }) = validation
        else {
            panic!("expected dependency-shape rejection, got {validation:?}");
        };
        reason
    }
}

impl Drop for FixtureWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn copy_fixture_directory(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_fixture_directory(&source_path, &destination_path)?;
        } else {
            fs::copy(source_path, destination_path)?;
        }
    }
    Ok(())
}

fn rewrite_fixture_manifests(
    root: &Path,
    canic_path: &Path,
    canic_core_path: &Path,
) -> std::io::Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            rewrite_fixture_manifests(&path, canic_path, canic_core_path)?;
            continue;
        }
        if entry.file_name() != "Cargo.toml" {
            continue;
        }
        let source = fs::read_to_string(&path)?;
        let source = source
            .replace(
                "../../../../../canic-core",
                &canic_core_path.display().to_string(),
            )
            .replace("../../../../../../canic", &canic_path.display().to_string())
            .replace("../../../../../canic", &canic_path.display().to_string());
        fs::write(path, source)?;
    }
    Ok(())
}
