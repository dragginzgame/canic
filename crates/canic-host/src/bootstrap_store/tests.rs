use super::*;
use crate::canister_build::CanisterBuildProfile;
use crate::test_support::temp_dir;

#[test]
fn generated_wasm_store_wrapper_satisfies_role_package_contract() {
    let root = temp_dir("canic-generated-wasm-store-contract");
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let context = WorkspaceBuildContext {
        role: WASM_STORE_ROLE.to_string(),
        profile: CanisterBuildProfile::Fast,
        environment: "local".into(),
        build_network: canic_core::ids::BuildNetwork::Local,
        workspace_root,
        icp_root: root.clone(),
        config_path: root.join("canic.toml"),
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: None,
    };
    let source = resolve_bootstrap_wasm_store_source(&context).expect("generate Store package");
    assert!(
        source
            .canonical_did_path
            .ends_with("canic/candid/wasm_store.did")
    );
    assert_eq!(source.package_version, env!("CARGO_PKG_VERSION"));
    let manifest: toml::Value =
        toml::from_str(&fs::read_to_string(&source.manifest_path).unwrap()).unwrap();
    assert_eq!(
        manifest["package"]["version"].as_str(),
        Some(env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(manifest["package"]["publish"].as_bool(), Some(false));
    assert_eq!(
        manifest["lib"]["crate-type"].as_array().unwrap(),
        &[toml::Value::String("cdylib".into())]
    );
    assert_eq!(
        manifest["dependencies"]["canic"]["default-features"].as_bool(),
        Some(false)
    );

    let validation =
        validate_built_in_wasm_store_package(&source.manifest_path, PackageValidationMode::Build);
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
    append_infrastructure_profile_args(&mut command, CanisterBuildProfile::Fast);
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
fn wasm_store_build_uses_the_locked_resolver() {
    let context = WorkspaceBuildContext {
        role: WASM_STORE_ROLE.to_string(),
        profile: CanisterBuildProfile::Fast,
        environment: "local".to_string(),
        build_network: canic_core::ids::BuildNetwork::Local,
        workspace_root: "/workspace".into(),
        icp_root: "/project".into(),
        config_path: "/workspace/apps/demo/canic.toml".into(),
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: None,
    };
    let command = wasm_store_cargo_build_command(
        &context,
        Path::new("/workspace/.canic/generated/canic-fleet-wasm-store/Cargo.toml"),
        false,
    );
    assert!(command.get_args().any(|argument| argument == "--locked"));
}

#[test]
fn wasm_store_declaration_build_uses_the_canonical_candid_environment() {
    let context = WorkspaceBuildContext {
        role: WASM_STORE_ROLE.to_string(),
        profile: CanisterBuildProfile::Fast,
        environment: "local".to_string(),
        build_network: canic_core::ids::BuildNetwork::Local,
        workspace_root: "/workspace".into(),
        icp_root: "/project".into(),
        config_path: "/workspace/apps/demo/canic.toml".into(),
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: None,
    };
    let command = wasm_store_cargo_build_command(
        &context,
        Path::new("/workspace/.canic/generated/canic-fleet-wasm-store/Cargo.toml"),
        true,
    );

    assert_eq!(
        command.get_args().next(),
        Some(std::ffi::OsStr::new("build"))
    );
    assert_eq!(
        command.get_envs().find(|(key, _)| {
            *key == std::ffi::OsStr::new(canic_core::role_contract::CANONICAL_CANDID_BUILD_ENV)
        }),
        Some((
            std::ffi::OsStr::new(canic_core::role_contract::CANONICAL_CANDID_BUILD_ENV),
            Some(std::ffi::OsStr::new("1")),
        ))
    );
}
