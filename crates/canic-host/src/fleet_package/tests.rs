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
#[test]
fn sibling_configurations_preserve_each_others_generated_packages() {
    let root = temp_dir("generated-package-config-isolation");
    let canic_manifest = root.join("crates/canic/Cargo.toml");
    fs::create_dir_all(canic_manifest.parent().unwrap()).unwrap();
    fs::write(
        &canic_manifest,
        "[package]\nname = 'canic'\nversion = '0.1.0'\n",
    )
    .unwrap();
    fs::write(root.join("Cargo.lock"), "version = 4\n").unwrap();
    let dependencies = GeneratedWrapperDependencies {
        canic_version: "0.1.0".into(),
        candid_version: "0.10.0".into(),
        ic_cdk_version: "0.20.0".into(),
    };
    for package in [
        "canic-fleet-root",
        "canic-fleet-coordinator",
        "canic-fleet-wasm-store",
    ] {
        let first = manifest_path(&root.join("configs/first.toml"), package);
        let second = manifest_path(&root.join("configs/second.toml"), package);
        let write = |manifest: &Path, features: &[&str]| {
            materialize(
                manifest,
                &root,
                &canic_manifest,
                &dependencies,
                &FleetPackageSpec {
                    package,
                    crate_name: "fixture",
                    app: "test",
                    role: "root",
                    features,
                    entrypoint: "// fixture entrypoint\n",
                    build_script: Some("fn main() {}\n"),
                },
            )
            .unwrap();
        };
        let read = |manifest: &Path| {
            ["Cargo.toml", "Cargo.lock", "src/lib.rs", "build.rs"]
                .map(|name| fs::read(manifest.parent().unwrap().join(name)).unwrap())
        };
        write(&first, &["control-plane", "auth-chain-key-root-sign"]);
        let retained = read(&first);
        // Reproduce a second worker preparing a different Root feature contract
        // while the first worker has already frozen its Cargo inputs.
        write(&second, &["control-plane"]);
        assert_eq!(read(&first), retained);
        assert_ne!(first, second);
        assert_ne!(read(&first)[0], read(&second)[0]);
        write(&first, &["control-plane", "auth-chain-key-root-sign"]);
        assert_eq!(read(&first), retained);
    }
    fs::remove_dir_all(root).unwrap();
}
