use super::*;
use crate::test_support::temp_dir;
use canic_core::{ids::BuildNetwork, role_contract::CanicFeatureKey};

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
        refresh_canonical_wasm_store_did: false,
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
