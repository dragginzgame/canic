use super::*;
use canic_core::ids::BuildNetwork;

#[test]
fn resolve_root_canister_operation_owns_current_install_evidence() {
    let operation = ResolveRootCanisterOperation::new(
        Path::new("/workspace/.icp"),
        "local",
        "root",
        Path::new("/workspace/apps/demo/canic.toml"),
        None,
    );

    let evidence = operation.evidence("aaaaa-aa");

    assert_eq!(evidence, ["root_target:root", "root_canister:aaaaa-aa"]);
}

#[test]
fn build_install_targets_operation_owns_current_install_evidence() {
    let context = test_build_context();
    let targets = [build_target("root"), build_target("wasm_store")];
    let operation = BuildInstallTargetsOperation::new(&context, &targets);

    assert_eq!(
        operation.evidence(),
        ["build_target:root", "build_target:wasm_store"]
    );
    assert_eq!(operation.role_names(), ["root", "wasm_store"]);
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
            package_manifest_path: PathBuf::from("/workspace/apps")
                .join(role)
                .join("Cargo.toml"),
            wasm_path: artifact_root.join(format!("{role}.wasm")),
            wasm_gz_path: artifact_root.join(format!("{role}.wasm.gz")),
            did_path: artifact_root.join(format!("{role}.did")),
            artifact_root,
        },
    }
}

#[test]
fn install_root_wasm_operation_owns_current_install_evidence() {
    let root = temp_dir("canic-install-root-operation-evidence");
    fs::create_dir_all(&root).expect("create temp root");
    let root_wasm = root.join("root.wasm");
    fs::write(&root_wasm, b"root wasm").expect("write root Wasm");
    let operation = InstallRootWasmOperation::new(
        &root,
        "local",
        "aaaaa-aa",
        root_wasm.clone(),
        &sample_fleet_activation_identity(),
        None,
    )
    .expect("prepare root install operation");

    let evidence = operation.evidence();

    assert!(evidence.contains(&"root_canister:aaaaa-aa".to_string()));
    assert!(evidence.contains(&format!("root_wasm:{}", root_wasm.display())));
    assert!(evidence.iter().any(|item| {
        item.strip_prefix("expected_module_hash:")
            .is_some_and(|hash| hash.len() == 64)
    }));

    fs::remove_dir_all(root).expect("remove temp root");
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
        refresh_canonical_wasm_store_did: false,
        release_build_id: None,
    }
}

#[test]
fn current_install_activation_records_verified_root_before_advancing_the_journal() {
    let activation = include_str!("../../activation/mod.rs");

    assert_before(
        activation,
        "run_operation_with_receipt(&install_operation, Some(root_canister_id))",
        "admit_root_install_receipt(&completed_root_install.receipt_path)",
    );
    assert_before(
        activation,
        "admit_root_install_receipt(&completed_root_install.receipt_path)",
        "record_root_installed(receipt_scope.icp_root, activation, &receipt)",
    );
}
