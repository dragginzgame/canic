use super::*;
use crate::test_support::temp_dir;

#[test]
fn bootstrap_wasm_store_rejects_competing_canic_packages() {
    let mut metadata = cargo_metadata_fixture(vec![package("canic", "canic@1", "0.98.2")]);
    metadata
        .packages
        .push(package("canic", "canic@2", "0.98.2"));

    assert!(resolved_canic_package(&metadata).is_err());
}

#[test]
fn packaged_patch_table_never_uses_another_cached_version() {
    let root = temp_dir("canic-generated-wasm-store-exact-siblings");
    let canic_manifest = root.join("registry/canic-0.98.2/Cargo.toml");
    let stale_core = root.join("registry/canic-core-0.98.1/Cargo.toml");
    let exact_macros = root.join("registry/canic-macros-0.98.2/Cargo.toml");
    for manifest in [&canic_manifest, &stale_core, &exact_macros] {
        fs::create_dir_all(manifest.parent().expect("manifest parent"))
            .expect("create package directory");
    }
    fs::write(
        &canic_manifest,
        "[package]\nname = \"canic\"\nversion = \"0.98.2\"\n",
    )
    .expect("write Canic manifest");
    fs::write(
        &stale_core,
        "[package]\nname = \"canic-core\"\nversion = \"0.98.1\"\n",
    )
    .expect("write stale core manifest");
    fs::write(
        &exact_macros,
        "[package]\nname = \"canic-macros\"\nversion = \"0.98.2\"\n",
    )
    .expect("write exact macros manifest");

    let patch_table =
        dependency_patch_table(&canic_manifest, "0.98.2").expect("render exact patch table");

    assert!(!patch_table.contains("canic-core-0.98.1"));
    assert!(patch_table.contains("canic-macros-0.98.2"));
    fs::remove_dir_all(root).expect("clean temp dir");
}

fn cargo_metadata_fixture(packages: Vec<CargoMetadataPackage>) -> CargoMetadata {
    CargoMetadata {
        packages,
        resolve: None,
        workspace_root: PathBuf::from("/workspace"),
    }
}

fn package(name: &str, id: &str, version: &str) -> CargoMetadataPackage {
    CargoMetadataPackage {
        id: id.to_string(),
        name: name.to_string(),
        version: version.to_string(),
        source: None,
        manifest_path: PathBuf::from(format!("/workspace/{name}/Cargo.toml")),
        metadata: None,
        dependencies: Vec::new(),
        features: std::collections::BTreeMap::new(),
        targets: Vec::new(),
    }
}
