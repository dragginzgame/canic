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

    let wrapper_root = ensure_generated_wasm_store_wrapper(&root, &root, &canic_manifest)
        .expect("generate wrapper");
    let manifest = fs::read_to_string(wrapper_root.join("Cargo.toml"))
        .expect("read generated wrapper manifest");

    assert!(manifest.contains("features = [\"wasm-store-canister\"]"));
    assert!(!manifest.contains("features = [\"control-plane\"]"));
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
    let canic_manifest = workspace_root.join("crates/canic/Cargo.toml");
    let wrapper_root = ensure_generated_wasm_store_wrapper(&root, &workspace_root, &canic_manifest)
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
}
