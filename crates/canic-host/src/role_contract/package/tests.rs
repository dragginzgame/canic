use super::*;
use crate::cargo_metadata::{CargoMetadataDependencyKind, CargoMetadataNodeDependency};
use serde_json::json;

#[test]
fn direct_dependency_accepts_the_supported_renamed_shape() {
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

    let dependency = direct_canic_dependency(&package, &CanisterRole::owned("app".to_string()))
        .expect("supported dependency");
    assert_eq!(dependency.rename.as_deref(), Some("framework"));
    assert!(!dependency.uses_default_features);
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
    let package_by_id = packages
        .iter()
        .map(|package| (package.id.as_str(), package))
        .collect::<BTreeMap<_, _>>();
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
    let node_by_id = nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<BTreeMap<_, _>>();

    assert!(matches!(
        validate_runtime_graph(&nodes[0], &nodes[0].deps[0], &package_by_id, &node_by_id),
        Err(RoleContractFinding::DependencyShapeUnsupported { .. })
    ));
}

#[test]
fn multiple_runtime_canic_packages_are_rejected() {
    let packages = [
        package("role", "role@1", "/tmp/role/Cargo.toml"),
        package("helper", "helper@1", "/tmp/helper/Cargo.toml"),
        package(CANIC_PACKAGE, "canic@1", "/tmp/canic-1/Cargo.toml"),
        package(CANIC_PACKAGE, "canic@2", "/tmp/canic-2/Cargo.toml"),
    ];
    let package_by_id = packages
        .iter()
        .map(|package| (package.id.as_str(), package))
        .collect::<BTreeMap<_, _>>();
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
    let node_by_id = nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<BTreeMap<_, _>>();

    assert!(matches!(
        validate_runtime_graph(&nodes[0], &nodes[0].deps[0], &package_by_id, &node_by_id),
        Err(RoleContractFinding::MultipleCanicPackages { .. })
    ));
}

#[test]
fn build_only_canic_path_does_not_enter_the_runtime_graph() {
    let packages = [
        package("role", "role@1", "/tmp/role/Cargo.toml"),
        package("helper", "helper@1", "/tmp/helper/Cargo.toml"),
        package(CANIC_PACKAGE, "canic@1", "/tmp/canic/Cargo.toml"),
    ];
    let package_by_id = packages
        .iter()
        .map(|package| (package.id.as_str(), package))
        .collect::<BTreeMap<_, _>>();
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
    let node_by_id = nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<BTreeMap<_, _>>();

    validate_runtime_graph(&nodes[0], &nodes[0].deps[0], &package_by_id, &node_by_id)
        .expect("build dependencies are not wasm runtime evidence");
}

#[test]
fn internal_pocketic_packages_are_validated_before_the_marker_is_granted() {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    validate_internal_test_wasm_packages(&workspace, &["sharding_root_stub", "canister_user_hub"])
        .expect("internal PocketIC package validation");
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
