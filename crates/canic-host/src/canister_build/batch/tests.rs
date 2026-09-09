use super::*;
use std::fs;

#[test]
fn real_cargo_resolution_batches_peers_and_splits_feature_growth() {
    let root = crate::test_support::temp_dir("build-feature-batches");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("Cargo.toml"), "[workspace]\nmembers=[\"shared\",\"exclusive\",\"left\",\"peer\",\"right\"]\nresolver=\"3\"\n").unwrap();
    for (name, extra) in [
        (
            "shared",
            "[features]\ndefault=[\"base\"]\nbase=[]\nextra=[]\n",
        ),
        ("exclusive", ""),
        ("left", "[dependencies]\nshared={path=\"../shared\"}\n"),
        (
            "peer",
            "[dependencies]\nshared={path=\"../shared\"}\nexclusive={path=\"../exclusive\"}\n",
        ),
        (
            "right",
            "[dependencies]\nshared={path=\"../shared\",features=[\"extra\"]}\n",
        ),
    ] {
        let package = root.join(name);
        fs::create_dir_all(package.join("src")).unwrap();
        fs::write(
            package.join("Cargo.toml"),
            format!("[package]\nname=\"{name}\"\nversion=\"0.1.0\"\nedition=\"2024\"\n{extra}"),
        )
        .unwrap();
        fs::write(package.join("src/lib.rs"), "pub fn fixture() {}\n").unwrap();
    }
    fs::write(root.join("Cargo.lock"), "version=4\n[[package]]\nname=\"exclusive\"\nversion=\"0.1.0\"\n[[package]]\nname=\"left\"\nversion=\"0.1.0\"\ndependencies=[\"shared\"]\n[[package]]\nname=\"peer\"\nversion=\"0.1.0\"\ndependencies=[\"exclusive\",\"shared\"]\n[[package]]\nname=\"right\"\nversion=\"0.1.0\"\ndependencies=[\"shared\"]\n[[package]]\nname=\"shared\"\nversion=\"0.1.0\"\n").unwrap();
    let context = WorkspaceBuildContext {
        role: "left".into(),
        profile: crate::canister_build::CanisterBuildProfile::Fast,
        environment: "local".into(),
        build_network: canic_core::ids::BuildNetwork::Local,
        workspace_root: root.clone(),
        icp_root: root.clone(),
        config_path: root.join("Cargo.toml"),
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: None,
    };
    let packages = ["left", "peer", "right"].map(str::to_string);
    let groups = compatible_package_groups(&context, &root, &packages).unwrap();
    assert_eq!(groups, [vec!["left", "peer"], vec!["right"]]);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn shared_feature_growth_rejects_a_batch_but_disjoint_dependencies_do_not() {
    let left = "0left v1.0.0\t\n1shared v1.0.0\tbase\n";
    let right = "0right v1.0.0\t\n1shared v1.0.0\tbase\n1extra v1.0.0\tunique\n";
    let isolated = parse_trees(&format!("{left}\n{right}")).unwrap();
    let packages = ["left".to_string(), "right".to_string()];
    assert!(preserves_individual_trees(&isolated, &isolated, &packages));
    let changed = parse_trees(&format!(
        "{}\n{right}",
        left.replace("\tbase", "\tbase,extra")
    ))
    .unwrap();
    assert!(!preserves_individual_trees(&isolated, &changed, &packages));
    let omitted = parse_trees(left).unwrap();
    assert!(!preserves_individual_trees(&isolated, &omitted, &packages));
    assert!(parse_trees(&format!("{left}{left}")).is_err());
    assert!(parse_trees("1shared v1.0.0\tbase\n").is_err());
}

#[test]
fn feature_assignment_order_is_retained_for_host_and_wasm_instances() {
    let isolated =
        parse_trees("0role v1.0.0\t\n1shared v1.0.0\twasm\n1shared v1.0.0\thost\n").unwrap();
    let swapped =
        parse_trees("0role v1.0.0\t\n1shared v1.0.0\thost\n1shared v1.0.0\twasm\n").unwrap();
    assert!(!preserves_individual_trees(
        &isolated,
        &swapped,
        &["role".to_string()]
    ));
}
