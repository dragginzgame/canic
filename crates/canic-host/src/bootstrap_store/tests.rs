use super::*;
use crate::test_support::temp_dir;

#[test]
fn generated_wasm_store_wrapper_enables_wasm_store_canister_feature() {
    let root = temp_dir("canic-generated-wasm-store-wrapper");
    let canic_manifest = root.join("registry/canic-0.35.5/Cargo.toml");
    fs::create_dir_all(canic_manifest.parent().expect("canic manifest parent"))
        .expect("create canic package dir");
    fs::write(
        &canic_manifest,
        "[package]\nname = \"canic\"\nversion = \"0.35.5\"\n",
    )
    .expect("write canic manifest");

    let dependencies = test_wrapper_dependencies();
    let wrapper_root =
        ensure_generated_wasm_store_wrapper(&root, &root, &canic_manifest, &dependencies)
            .expect("generate wrapper");
    let manifest = fs::read_to_string(wrapper_root.join("Cargo.toml"))
        .expect("read generated wrapper manifest");

    assert!(manifest.contains("resolver = \"2\""));
    assert!(manifest.contains("default-features = false"));
    assert!(manifest.contains("features = [\"metrics\", \"wasm-store-canister\"]"));
    assert!(manifest.contains("features = []"));
    assert!(!manifest.contains("features = [\"control-plane\"]"));
    assert!(manifest.contains("ic-cdk = \"=0.20.2\""));
    assert!(manifest.contains("candid = { version = \"=0.10.32\""));
    assert!(manifest.contains("[profile.fast]"));
    assert!(manifest.contains("incremental = false"));
    assert!(!manifest.contains("incremental = true"));
    assert!(manifest.contains("canic = { path = "));
    assert!(manifest.contains("[package.metadata.canic]"));
    assert!(manifest.contains("fleet = \"wasm_store\""));
    assert!(manifest.contains("role = \"wasm_store\""));
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn generated_wasm_store_wrapper_satisfies_role_package_contract() {
    let root = temp_dir("canic-generated-wasm-store-contract");
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let metadata = cargo_metadata(&workspace_root, true).expect("resolve workspace metadata");
    let canic_package = resolved_canic_package(&metadata).expect("resolve exact Canic package");
    let dependencies =
        resolved_wrapper_dependencies(&metadata, canic_package).expect("resolve wrapper deps");
    let wrapper_root = ensure_generated_wasm_store_wrapper(
        &root,
        &workspace_root,
        &canic_package.manifest_path,
        &dependencies,
    )
    .expect("generate wrapper");

    let validation = validate_built_in_wasm_store_package(
        &wrapper_root.join("Cargo.toml"),
        PackageValidationMode::Build,
    );
    let RolePackageValidation::Supported(evidence) = validation else {
        panic!("generated wrapper should satisfy the package contract: {validation:?}");
    };
    assert!(
        evidence
            .direct_features
            .contains(&canic_core::role_contract::CanicFeatureKey::WasmStoreCanister)
    );

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn wasm_store_fast_profile_config_defines_standalone_profile() {
    let mut command = Command::new("cargo");
    append_wasm_store_profile_config_args(&mut command, CanisterBuildProfile::Fast);
    let args: Vec<String> = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();

    assert!(args.contains(&"profile.release.opt-level=\"z\"".to_string()));
    assert!(args.contains(&"profile.release.panic=\"abort\"".to_string()));
    assert!(args.contains(&"profile.fast.inherits=\"release\"".to_string()));
    assert!(args.contains(&"profile.fast.lto=false".to_string()));
    assert!(args.contains(&"profile.fast.codegen-units=16".to_string()));
    assert!(args.contains(&"profile.fast.incremental=false".to_string()));
}

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

    let patch_table = generated_wasm_store_wrapper_patch_table(&canic_manifest, "0.98.2")
        .expect("render exact patch table");

    assert!(!patch_table.contains("canic-core-0.98.1"));
    assert!(patch_table.contains("canic-macros-0.98.2"));
    fs::remove_dir_all(root).expect("clean temp dir");
}

fn test_wrapper_dependencies() -> GeneratedWrapperDependencies {
    GeneratedWrapperDependencies {
        canic_version: "0.35.5".to_string(),
        candid_version: "0.10.32".to_string(),
        ic_cdk_version: "0.20.2".to_string(),
    }
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
