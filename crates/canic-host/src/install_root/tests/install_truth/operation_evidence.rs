use super::*;
use canic_core::ids::BuildNetwork;

#[test]
fn build_install_targets_operation_owns_current_install_evidence() {
    let context = test_build_context();
    let targets = [build_target("root"), build_target("wasm_store")];
    let operation = BuildInstallTargetsOperation::new(&context, &targets);

    assert_eq!(
        operation.evidence(),
        [
            "build_target:fleet_coordinator",
            "build_target:root",
            "build_target:wasm_store",
        ]
    );
    assert_eq!(
        operation.role_names(),
        ["fleet_coordinator", "root", "wasm_store"]
    );
}

#[test]
fn emit_root_manifest_operation_owns_current_install_evidence() {
    let snapshot = RootReleaseSetBuildSnapshot {
        icp_root: PathBuf::from("/workspace"),
        manifest_path: PathBuf::from("/workspace/.icp/local/canisters/root/root.release-set.json"),
        release_version: "0.91.0".to_string(),
        targets: Vec::new(),
    };
    let _operation = EmitRootManifestOperation::new(&snapshot, &[]);

    let evidence = EmitRootManifestOperation::evidence(Path::new(
        "/workspace/.icp/local/canisters/root.release-set.json",
    ));

    assert_eq!(
        evidence,
        ["manifest_path:/workspace/.icp/local/canisters/root.release-set.json"]
    );
}

fn build_target(role: &str) -> InstallBuildTarget {
    let artifact_root = PathBuf::from("/workspace/.icp/local/canisters").join(role);
    InstallBuildTarget {
        role: role.to_string(),
        spec: CanisterArtifactBuildSpec {
            role: role.to_string(),
            package_name: format!("canister_{role}"),
            package_version: "0.101.51".to_string(),
            package_manifest_path: PathBuf::from("/workspace/apps")
                .join(role)
                .join("Cargo.toml"),
            cargo_workspace_root: PathBuf::from("/workspace"),
            wasm_path: artifact_root.join(format!("{role}.wasm")),
            wasm_gz_path: artifact_root.join(format!("{role}.wasm.gz")),
            did_path: artifact_root.join(format!("{role}.did")),
            artifact_root,
        },
    }
}

fn test_build_context() -> WorkspaceBuildContext {
    WorkspaceBuildContext {
        role: "root".to_string(),
        profile: CanisterBuildProfile::Fast,
        environment: "local".to_string(),
        build_network: BuildNetwork::Local,
        workspace_root: PathBuf::from("/workspace"),
        icp_root: PathBuf::from("/workspace/.icp"),
        config_path: PathBuf::from("/workspace/apps/demo/canic.toml"),
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: None,
    }
}
