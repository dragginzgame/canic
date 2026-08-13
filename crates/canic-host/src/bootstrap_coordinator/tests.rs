use super::*;
use crate::test_support::temp_dir;
use canic_core::{ids::BuildNetwork, role_contract::CanicFeatureKey};

#[test]
fn canonical_fleet_coordinator_package_satisfies_its_runtime_only_contract() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest_path = workspace_root.join("crates/canic-fleet-coordinator/Cargo.toml");
    let validation =
        validate_built_in_fleet_coordinator_package(&manifest_path, PackageValidationMode::Build);
    let RolePackageValidation::Supported(evidence) = validation else {
        panic!("canonical package should satisfy the package contract: {validation:?}");
    };

    assert_eq!(evidence.role_package_name, CANONICAL_PACKAGE_NAME);
    assert_eq!(
        evidence.direct_features,
        std::collections::BTreeSet::from([CanicFeatureKey::FleetCoordinatorCanister])
    );
}

#[test]
fn workspace_resolution_prefers_the_canonical_fleet_coordinator_package() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let metadata = cargo_metadata(&workspace_root, true).expect("resolve workspace metadata");
    let canic_package = resolved_canic_package(&metadata).expect("resolve exact Canic package");
    let source = resolve_canonical_fleet_coordinator_source(&metadata, canic_package)
        .expect("resolve canonical Coordinator source")
        .expect("canonical Coordinator package");

    assert_eq!(source.package_name, CANONICAL_PACKAGE_NAME);
    assert!(
        source
            .manifest_path
            .ends_with("crates/canic-fleet-coordinator/Cargo.toml")
    );
    assert!(source.canonical_did_path.as_deref().is_some_and(|path| {
        path.ends_with("crates/canic-fleet-coordinator/fleet_coordinator.did")
    }));
}

#[test]
fn local_coordinator_build_exports_candid_in_the_selected_leaf_pass() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut context = WorkspaceBuildContext {
        role: FLEET_COORDINATOR_ROLE.to_string(),
        profile: CanisterBuildProfile::Fast,
        environment: "local".to_string(),
        build_network: BuildNetwork::Local,
        workspace_root: workspace_root.clone(),
        icp_root: workspace_root,
        config_path: PathBuf::from("canic.toml"),
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: None,
    };
    let manifest = Path::new("/workspace/coordinator/Cargo.toml");

    let local = coordinator_cargo_build_command(&context, manifest)
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(local.first().map(String::as_str), Some("rustc"));
    assert!(
        local
            .windows(2)
            .any(|args| args == ["--cfg", "canic_export_candid"])
    );
    assert!(local.contains(&"--check-cfg=cfg(canic_export_candid)".to_string()));

    context.build_network = BuildNetwork::Ic;
    let ic = coordinator_cargo_build_command(&context, manifest)
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(ic.first().map(String::as_str), Some("build"));
    assert!(!ic.contains(&"canic_export_candid".to_string()));
}

#[test]
fn generated_fleet_coordinator_wrapper_satisfies_its_runtime_only_contract() {
    let root = temp_dir("canic-generated-fleet-coordinator-contract");
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let context = WorkspaceBuildContext {
        role: FLEET_COORDINATOR_ROLE.to_string(),
        profile: CanisterBuildProfile::Fast,
        environment: "test".to_string(),
        build_network: BuildNetwork::Ic,
        workspace_root: workspace_root.clone(),
        icp_root: root.clone(),
        config_path: workspace_root.join("canic.toml"),
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: None,
    };

    let manifest_path = ensure_generated_wrapper(&context).expect("generate Coordinator wrapper");
    let manifest = fs::read_to_string(&manifest_path).expect("read generated manifest");
    let runtime = fs::read_to_string(
        manifest_path
            .parent()
            .expect("wrapper manifest parent")
            .join("src/lib.rs"),
    )
    .expect("read generated runtime");
    assert!(manifest.contains("app = \"fleet_coordinator\""));
    assert!(manifest.contains("role = \"fleet_coordinator\""));
    assert!(manifest.contains("default-features = false"));
    assert!(manifest.contains("features = [\"fleet-coordinator-canister\"]"));
    assert!(!manifest.contains("[build-dependencies]"));
    assert_eq!(
        runtime,
        "canic::start_fleet_coordinator!();\ncanic::finish!();\n"
    );

    let validation =
        validate_built_in_fleet_coordinator_package(&manifest_path, PackageValidationMode::Build);
    let RolePackageValidation::Supported(evidence) = validation else {
        panic!("generated wrapper should satisfy the package contract: {validation:?}");
    };
    assert_eq!(
        evidence.direct_features,
        std::collections::BTreeSet::from([CanicFeatureKey::FleetCoordinatorCanister])
    );

    fs::remove_dir_all(root).expect("clean temp dir");
}
