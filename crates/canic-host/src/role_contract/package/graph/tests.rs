use super::*;
use ic_build::cargo::observation::schema::{DepKindInfo, Node, Resolve, Target};

use serde_json::json;

#[test]
fn package_tree_correlation_reconstructs_normal_edges_and_features() {
    let metadata = metadata();
    let selected = &metadata.packages[0];
    let separator = TREE_FIELD_SEPARATOR;
    let tree = format!(
        "0role v1.0.0 (/workspace/role){separator}role{separator}\
         \n1canic v1.0.0 (/workspace/canic){separator}canic{separator}sharding\
         \n1domain v1.0.0 (/workspace/domain){separator}domain{separator}"
    );

    let evidence =
        correlate_package_tree(&metadata, &metadata, selected, &tree).expect("valid graph");

    assert_eq!(evidence.selected_package_id, "role@1");
    assert_eq!(
        evidence.edges["role@1"],
        vec![
            CargoGraphEdge {
                alias: "canic".to_string(),
                package_id: "canic@1".to_string(),
            },
            CargoGraphEdge {
                alias: "domain".to_string(),
                package_id: "domain@1".to_string(),
            },
        ]
    );
    assert_eq!(
        evidence.packages["canic@1"].enabled_features,
        BTreeSet::from(["sharding".to_string()])
    );
}

#[test]
fn package_tree_correlation_rejects_ambiguous_aliases() {
    let mut metadata = metadata();
    metadata.resolve.as_mut().expect("resolve").nodes[0]
        .deps
        .push(normal_edge("framework", "canic@1"));
    let selected = &metadata.packages[0];
    let separator = TREE_FIELD_SEPARATOR;
    let tree = format!(
        "0role v1.0.0 (/workspace/role){separator}role{separator}\
         \n1canic v1.0.0 (/workspace/canic){separator}canic{separator}sharding"
    );

    assert!(
        correlate_package_tree(&metadata, &metadata, selected, &tree)
            .expect_err("ambiguous edge")
            .contains("ambiguous normal dependency aliases")
    );
}

#[test]
fn package_tree_correlation_rejects_depth_gaps() {
    let metadata = metadata();
    let selected = &metadata.packages[0];
    let separator = TREE_FIELD_SEPARATOR;
    let tree = format!(
        "0role v1.0.0 (/workspace/role){separator}role{separator}\
         \n2canic v1.0.0 (/workspace/canic){separator}canic{separator}sharding"
    );

    assert!(
        correlate_package_tree(&metadata, &metadata, selected, &tree)
            .expect_err("depth gap")
            .contains("skips a parent depth")
    );
}

#[test]
fn target_filtered_metadata_excludes_tree_only_packages() {
    let catalog = metadata();
    let mut target_metadata = metadata();
    target_metadata.resolve.as_mut().expect("resolve").nodes[0]
        .deps
        .retain(|dependency| dependency.pkg.repr != "domain@1");
    let selected = &catalog.packages[0];
    let separator = TREE_FIELD_SEPARATOR;
    let tree = format!(
        "0role v1.0.0 (/workspace/role){separator}role{separator}\
         \n1canic v1.0.0 (/workspace/canic){separator}canic{separator}sharding\
         \n1domain v1.0.0 (/workspace/domain){separator}domain{separator}"
    );

    let evidence = correlate_package_tree(&catalog, &target_metadata, selected, &tree)
        .expect("target intersection");

    assert!(!evidence.packages.contains_key("domain@1"));
    assert_eq!(
        evidence.edges["role@1"],
        vec![CargoGraphEdge {
            alias: "canic".to_string(),
            package_id: "canic@1".to_string(),
        }]
    );
}

fn metadata() -> Metadata {
    serde_json::from_value::<Metadata>(
        json!({"workspace_members": [], "target_directory": "/workspace/target", "version": 1,
            "packages": vec![
                package("role", "role@1", "/workspace/role/Cargo.toml"),
                package("canic", "canic@1", "/workspace/canic/Cargo.toml"),
                package("domain", "domain@1", "/workspace/domain/Cargo.toml"),
            ],
            "resolve": Some(serde_json::from_value::<Resolve>(json!({
                "nodes": vec![
                    serde_json::from_value::<Node>(json!({"dependencies": [],
                        "id": "role@1".to_string(),
                        "deps": vec![
                            normal_edge("canic", "canic@1"),
                            normal_edge("domain", "domain@1"),
                        ],
                    })).unwrap(),
                    serde_json::from_value::<Node>(json!({"dependencies": [],
                        "id": "canic@1".to_string(),
                        "deps": [],
                    })).unwrap(),
                    serde_json::from_value::<Node>(json!({"dependencies": [],
                        "id": "domain@1".to_string(),
                        "deps": [],
                    })).unwrap(),
                ],
            })).unwrap()),
            "workspace_root": PathBuf::from("/workspace"),
        }),
    )
    .unwrap()
}

fn package(name: &str, id: &str, manifest_path: &str) -> Package {
    serde_json::from_value::<Package>(json!({
        "id": id.to_string(),
        "name": name.to_string(),
        "version": "1.0.0".to_string(),
        "source": null,
        "manifest_path": PathBuf::from(manifest_path),
        "metadata": Some(json!({})),
        "dependencies": [],
        "features": {},
        "targets": vec![serde_json::from_value::<Target>(json!({
            "name": name.replace('-', "_"),
            "kind": vec!["lib".to_string()],
            "src_path": PathBuf::from(manifest_path).with_file_name("src/lib.rs"),
        })).unwrap()],
    }))
    .unwrap()
}

fn normal_edge(name: &str, package_id: &str) -> NodeDep {
    serde_json::from_value::<NodeDep>(json!({
        "name": name.to_string(),
        "pkg": package_id.to_string(),
        "dep_kinds": vec![serde_json::from_value::<DepKindInfo>(json!({ "kind": null })).unwrap()],
    }))
    .unwrap()
}
