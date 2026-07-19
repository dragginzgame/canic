use super::*;
use canic_core::ids::BuildNetwork;

#[test]
fn current_install_staging_evidence_records_release_set_transport_facts() {
    let manifest = RootReleaseSetManifest {
        release_version: "0.43.4".to_string(),
        entries: vec![ReleaseSetEntry {
            role: "user_hub".to_string(),
            template_id: "embedded:user_hub".to_string(),
            artifact_relative_path: "local/canisters/user_hub/user_hub.wasm.gz".to_string(),
            payload_size_bytes: 42,
            payload_sha256_hex: "payload-hash".to_string(),
            chunk_size_bytes: 1_048_576,
            chunk_sha256_hex: vec!["chunk-a".to_string(), "chunk-b".to_string()],
        }],
    };

    let evidence = current_install_staging_evidence(
        "aaaaa-aa",
        Path::new("/workspace/.icp/local/canisters/root.release-set.json"),
        &manifest,
    );

    assert!(evidence.contains(&"root_canister:aaaaa-aa".to_string()));
    assert!(evidence.contains(&"release_version:0.43.4".to_string()));
    assert!(evidence.contains(&"staging_receipts:1".to_string()));
    assert!(evidence.contains(&"staging_role:user_hub".to_string()));
    assert!(evidence.contains(&"staging_transport:WasmStore".to_string()));
    assert!(evidence.contains(&"staging_chunks_prepared:2".to_string()));
    assert!(evidence.contains(&"staging_chunks_published:2".to_string()));
    assert!(evidence.contains(&"staging_postcondition:Observed".to_string()));
    assert!(evidence.contains(&"staging_wasm_store:root:aaaaa-aa:bootstrap".to_string()));
}

#[test]
fn resolve_root_canister_operation_owns_current_install_evidence() {
    let operation = ResolveRootCanisterOperation::new(
        Path::new("/workspace/.icp"),
        "local",
        "root",
        Path::new("/workspace/fleets/demo/canic.toml"),
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
            package_manifest_path: PathBuf::from("/workspace/fleets")
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
fn stage_release_set_operation_owns_current_install_staging_evidence() {
    let manifest = RootReleaseSetManifest {
        release_version: "0.43.6".to_string(),
        entries: vec![ReleaseSetEntry {
            role: "root".to_string(),
            template_id: "embedded:root".to_string(),
            artifact_relative_path: "local/canisters/root/root.wasm.gz".to_string(),
            payload_size_bytes: 84,
            payload_sha256_hex: "payload-hash".to_string(),
            chunk_size_bytes: 1_048_576,
            chunk_sha256_hex: vec!["chunk-a".to_string()],
        }],
    };
    let operation = StageReleaseSetOperation::new(
        Path::new("/workspace/.icp"),
        "local",
        "aaaaa-aa",
        Path::new("/workspace/.icp/local/canisters/root.release-set.json"),
        manifest,
        None,
    );

    let evidence = operation.evidence();

    assert!(evidence.contains(&"root_canister:aaaaa-aa".to_string()));
    assert!(evidence.contains(&"release_version:0.43.6".to_string()));
    assert!(evidence.contains(&"staging_role:root".to_string()));
    assert!(evidence.contains(&"staging_transport:WasmStore".to_string()));
    assert!(evidence.contains(&"staging_chunks_prepared:1".to_string()));
    assert!(evidence.contains(&"staging_chunks_published:1".to_string()));
}

#[test]
fn install_root_wasm_operation_owns_current_install_evidence() {
    let operation = InstallRootWasmOperation::new(
        Path::new("/workspace/.icp"),
        "local",
        "aaaaa-aa",
        PathBuf::from("/workspace/.icp/local/canisters/root/root.wasm"),
        None,
    );

    let evidence = operation.evidence();

    assert!(evidence.contains(&"root_canister:aaaaa-aa".to_string()));
    assert!(
        evidence.contains(&"root_wasm:/workspace/.icp/local/canisters/root/root.wasm".to_string())
    );
}

#[test]
fn ensure_root_cycles_operation_owns_current_install_evidence() {
    let operation = EnsureRootCyclesOperation::new(
        Path::new("/workspace/.icp"),
        "local",
        "aaaaa-aa",
        InstallPhaseLabel::FUND_ROOT_PRE_BOOTSTRAP,
        "ensure local root minimum cycles before bootstrap",
        "pre-bootstrap",
        None,
    );

    let evidence = operation.evidence();

    assert!(evidence.contains(&"root_canister:aaaaa-aa".to_string()));
    assert!(evidence.contains(&"minimum_cycles:100000000000000".to_string()));
    assert!(evidence.contains(&"funding_phase:pre-bootstrap".to_string()));
}

#[test]
fn resume_bootstrap_operation_owns_current_install_evidence() {
    let operation =
        ResumeBootstrapOperation::new(Path::new("/workspace/.icp"), "local", "aaaaa-aa", None);

    let evidence = operation.evidence();

    assert_eq!(evidence, ["root_canister:aaaaa-aa"]);
}

#[test]
fn wait_root_ready_operation_owns_current_install_evidence() {
    let operation =
        WaitRootReadyOperation::new(Path::new("/workspace/.icp"), "local", "aaaaa-aa", 30, None);

    let evidence = operation.evidence();

    assert!(evidence.contains(&"root_canister:aaaaa-aa".to_string()));
    assert!(evidence.contains(&"timeout_seconds:30".to_string()));
}

fn test_build_context() -> WorkspaceBuildContext {
    WorkspaceBuildContext {
        role: "root".to_string(),
        profile: CanisterBuildProfile::Fast,
        environment: "local".to_string(),
        build_network: BuildNetwork::Local,
        workspace_root: PathBuf::from("/workspace"),
        icp_root: PathBuf::from("/workspace/.icp"),
        config_path: PathBuf::from("/workspace/fleets/demo/canic.toml"),
        local_replica: None,
        refresh_canonical_wasm_store_did: false,
    }
}

#[test]
fn current_install_activation_phases_use_operation_runner() {
    let activation = include_str!("../../activation/mod.rs");

    for operation in [
        "install_operation",
        "pre_bootstrap_funding",
        "stage_operation",
        "resume_operation",
        "wait_ready_operation",
        "post_ready_funding",
    ] {
        assert!(
            activation.contains(&format!("run_operation(&{operation})")),
            "activation phase must run through operation runner: {operation}"
        );
    }
    assert!(
        !activation.contains("run_phase("),
        "activation phases must not manually wire receipt_scope.run_phase"
    );
}
