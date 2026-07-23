use super::super::*;

#[test]
fn local_inventory_reports_missing_config_as_observation_gap() {
    let temp = TempWorkspace::new("canic-host-local-inventory-missing-config");

    let inventory = collect_local_deployment_inventory(&LocalInventoryRequest {
        deployment_name: "demo".to_string(),
        environment: "local".to_string(),
        artifact_environment: "local".to_string(),
        workspace_root: temp.path().join("workspace"),
        icp_root: temp.path().join("icp"),
        config_path: None,
        observed_at: "2026-05-21T00:00:00Z".to_string(),
    })
    .expect("collect inventory");

    assert_eq!(inventory.inventory_id, "local:local:demo");
    assert!(
        inventory
            .unresolved_observations
            .iter()
            .any(|gap| gap.key == "local_config.fleet_name")
    );
    assert!(
        inventory
            .unresolved_observations
            .iter()
            .any(|gap| gap.key == "local_config.roles")
    );
}

#[test]
fn local_artifact_manifest_collects_roles_and_release_set_hashes() {
    let temp = TempWorkspace::new("canic-host-local-artifact-manifest");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("apps");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("canic.toml"), SAMPLE_CONFIG).expect("write config");
    write_artifact(&icp_root, "root", b"root-artifact");
    write_artifact(&icp_root, "wasm_store", b"wasm-store-artifact");
    write_artifact(&icp_root, "user_hub", b"user-hub-artifact");
    write_release_set_manifest(&icp_root);

    let manifest = collect_local_role_artifact_manifest(&LocalArtifactManifestRequest {
        environment: "local".to_string(),
        artifact_environment: "local".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
    });

    assert_eq!(manifest.manifest_id, "local:local:demo:artifacts");
    assert_eq!(manifest.role_artifacts.len(), 3);
    assert!(
        manifest
            .role_artifacts
            .iter()
            .all(|artifact| artifact.role != "store")
    );
    let wasm_store = manifest
        .role_artifacts
        .iter()
        .find(|artifact| artifact.role == "wasm_store")
        .expect("wasm_store artifact");
    assert_eq!(wasm_store.source, ArtifactSourceV1::WasmStore);
    assert_eq!(
        wasm_store.observed_wasm_gz_file_sha256_source,
        Some(ArtifactDigestSourceV1::ObservedFileDigest)
    );
    let user_hub = manifest
        .role_artifacts
        .iter()
        .find(|artifact| artifact.role == "user_hub")
        .expect("user_hub artifact");
    assert_eq!(
        user_hub.wasm_gz_sha256.as_deref(),
        Some(RELEASE_SET_USER_HUB_SHA256)
    );
    assert_eq!(
        user_hub.wasm_gz_sha256_source,
        Some(ArtifactDigestSourceV1::ReleaseSetManifest)
    );
    assert_eq!(user_hub.wasm_gz_size_bytes, Some(17));
    assert_eq!(
        user_hub.observed_wasm_gz_file_sha256_source,
        Some(ArtifactDigestSourceV1::ObservedFileDigest)
    );
    assert_eq!(
        user_hub
            .observed_wasm_gz_file_sha256
            .as_ref()
            .map(String::len),
        Some(64)
    );
    let root = manifest
        .role_artifacts
        .iter()
        .find(|artifact| artifact.role == "root")
        .expect("root artifact");
    assert_eq!(root.wasm_gz_sha256, None);
    assert_eq!(root.wasm_gz_sha256_source, None);
    assert!(manifest.unresolved_artifacts.is_empty());
}

#[test]
fn local_artifact_manifest_requires_selected_environment_artifact_root() {
    let temp = TempWorkspace::new("canic-host-local-artifact-manifest-environment-root");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("apps");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("canic.toml"), SAMPLE_CONFIG).expect("write config");
    write_artifact(&icp_root, "root", b"root-artifact");

    let manifest = collect_local_role_artifact_manifest(&LocalArtifactManifestRequest {
        environment: "ic".to_string(),
        artifact_environment: "ic".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
    });

    assert_eq!(manifest.artifact_root, None);
    assert_eq!(manifest.role_artifacts.len(), 3);
    assert!(manifest.role_artifacts.iter().all(|artifact| {
        artifact
            .wasm_gz_path
            .as_deref()
            .is_some_and(|path| path.contains(".icp/ic/canisters"))
    }));
    assert!(
        manifest
            .unresolved_artifacts
            .iter()
            .any(|gap| gap.key == "local_artifacts.root"
                && gap.description.contains(".icp/ic/canisters"))
    );
    assert!(
        manifest
            .unresolved_artifacts
            .iter()
            .all(|gap| gap.key != "local_artifacts.environment_fallback")
    );
}

#[test]
fn local_deployment_check_rejects_missing_exact_artifact_root() {
    let temp = TempWorkspace::new("canic-host-local-check-missing-artifact-root");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("apps");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("canic.toml"), SAMPLE_CONFIG).expect("write config");

    let check = check_local_deployment(&LocalDeploymentCheckRequest {
        deployment_name: "demo".to_string(),
        environment: "ic".to_string(),
        artifact_environment: "ic".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
        observed_at: "2026-07-18T00:00:00Z".to_string(),
        runtime_variant: "ic".to_string(),
        build_profile: "release".to_string(),
    })
    .expect("check local deployment");

    assert_eq!(check.plan.role_artifacts.len(), 3);
    assert!(check.inventory.observed_artifacts.is_empty());
    assert!(check.report.hard_failures.iter().any(|finding| {
        finding.code == "artifact_missing" && finding.subject.as_deref() == Some("root")
    }));
}

#[test]
fn local_deployment_check_separates_target_environment_from_artifact_environment() {
    let temp = TempWorkspace::new("canic-host-local-check-explicit-artifact-environment");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("apps");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("canic.toml"), SAMPLE_CONFIG).expect("write config");
    write_artifact(&icp_root, "root", b"root-artifact");
    write_artifact(&icp_root, "wasm_store", b"wasm-store-artifact");
    write_artifact(&icp_root, "user_hub", b"user-hub-artifact");
    write_release_set_manifest(&icp_root);

    let check = check_local_deployment(&LocalDeploymentCheckRequest {
        deployment_name: "demo".to_string(),
        environment: "staging".to_string(),
        artifact_environment: "local".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
        observed_at: "2026-07-18T00:00:00Z".to_string(),
        runtime_variant: "staging".to_string(),
        build_profile: "release".to_string(),
    })
    .expect("check local deployment");

    assert_eq!(check.plan.deployment_identity.environment, "staging");
    assert_eq!(check.plan.role_artifacts.len(), 3);
    assert_eq!(check.inventory.observed_artifacts.len(), 3);
    assert!(check.report.hard_failures.iter().all(|finding| {
        !matches!(
            finding.code.as_str(),
            "artifact_missing" | "artifact_file_digest_mismatch"
        )
    }));
}

#[test]
fn local_artifact_manifest_records_missing_artifacts_as_gaps() {
    let temp = TempWorkspace::new("canic-host-local-artifact-manifest-missing");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("apps");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("canic.toml"), SAMPLE_CONFIG).expect("write config");
    write_artifact(&icp_root, "root", b"root-artifact");

    let manifest = collect_local_role_artifact_manifest(&LocalArtifactManifestRequest {
        environment: "local".to_string(),
        artifact_environment: "local".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
    });

    assert!(
        manifest
            .unresolved_artifacts
            .iter()
            .any(|gap| gap.key == "local_artifacts.release_set_manifest")
    );
    assert!(
        manifest
            .unresolved_artifacts
            .iter()
            .any(|gap| gap.key == "local_artifacts.user_hub")
    );
    assert!(
        manifest
            .unresolved_artifacts
            .iter()
            .any(|gap| gap.key == "local_artifacts.wasm_store")
    );
    assert!(
        manifest
            .unresolved_artifacts
            .iter()
            .all(|gap| gap.key != "local_artifacts.store")
    );
}
