use super::super::*;

#[test]
fn local_check_builds_plan_inventory_diff_and_report() {
    let temp = TempWorkspace::new("canic-host-local-check");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("canic.toml"), SAMPLE_CONFIG).expect("write config");
    write_artifact(&icp_root, "root", b"root-artifact");
    write_release_set_manifest(&icp_root);

    let check = check_local_deployment(&LocalDeploymentCheckRequest {
        deployment_name: "demo".to_string(),
        network: "local".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        runtime_variant: "local".to_string(),
        build_profile: "fast".to_string(),
    })
    .expect("check local deployment");

    assert_eq!(check.schema_version, DEPLOYMENT_TRUTH_SCHEMA_VERSION);
    assert_eq!(check.check_id, "local:local:demo:check");
    assert_eq!(check.plan.plan_id, "local:local:demo:plan");
    assert_eq!(check.inventory.inventory_id, "local:local:demo");
    assert_eq!(check.diff.resume_safety.status, check.report.status);
    assert!(
        check
            .diff
            .hard_failures
            .iter()
            .any(|finding| finding.code == "artifact_missing")
    );
    assert_eq!(check.report.status, SafetyStatusV1::Blocked);
}

#[test]
fn local_inventory_collects_configured_roles_and_artifacts_without_live_queries() {
    let temp = TempWorkspace::new("canic-host-local-inventory");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("canic.toml"), SAMPLE_CONFIG).expect("write config");

    let artifact_path = icp_root
        .join(".icp")
        .join("local")
        .join("canisters")
        .join("root")
        .join("root.wasm.gz");
    fs::create_dir_all(artifact_path.parent().expect("artifact parent"))
        .expect("create artifact dir");
    fs::write(&artifact_path, b"artifact").expect("write artifact");
    write_release_set_manifest(&icp_root);

    let inventory = collect_local_deployment_inventory(&LocalInventoryRequest {
        deployment_name: "demo".to_string(),
        network: "local".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
        observed_at: "2026-05-21T00:00:00Z".to_string(),
    })
    .expect("collect inventory");

    assert_eq!(inventory.schema_version, DEPLOYMENT_TRUTH_SCHEMA_VERSION);
    assert_eq!(inventory.inventory_id, "local:local:demo");
    assert_sha256_len(inventory.local_config.raw_config_sha256.as_ref());
    assert_sha256_len(
        inventory
            .local_config
            .canonical_embedded_config_sha256
            .as_ref(),
    );
    let observed_identity = inventory.observed_identity.as_ref().expect("identity");
    assert_sha256_len(observed_identity.deployment_manifest_digest.as_ref());
    assert_sha256_len(observed_identity.canonical_runtime_config_digest.as_ref());
    assert_sha256_len(observed_identity.role_topology_hash.as_ref());
    assert_sha256_len(observed_identity.artifact_set_digest.as_ref());
    assert_sha256_len(observed_identity.pool_identity_set_digest.as_ref());
    assert_eq!(inventory.observed_artifacts.len(), 1);
    assert_eq!(inventory.observed_artifacts[0].role, "root");
    assert_eq!(inventory.observed_artifacts[0].payload_size_bytes, Some(8));
    assert_eq!(
        inventory.observed_artifacts[0].file_sha256_source,
        Some(ArtifactDigestSourceV1::ObservedFileDigest)
    );
    assert_sha256_len(inventory.observed_artifacts[0].file_sha256.as_ref());
    assert!(
        inventory
            .unresolved_observations
            .iter()
            .any(|gap| gap.key == "local_artifacts.user_hub")
    );
    assert!(
        inventory
            .observed_artifacts
            .iter()
            .all(|artifact| artifact.role != "store")
    );
    assert!(
        inventory
            .unresolved_observations
            .iter()
            .all(|gap| gap.key != "local_artifacts.store")
    );
}

#[test]
fn local_inventory_records_explicit_root_evidence_for_deployment_target() {
    let temp = TempWorkspace::new("canic-host-local-root-evidence");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("canic.toml"), SAMPLE_CONFIG).expect("write config");
    write_deployment_state_json(&icp_root, "local", sample_install_state("prod", "aaaaa-aa"));

    let inventory = collect_local_deployment_inventory(&LocalInventoryRequest {
        deployment_name: "prod".to_string(),
        network: "local".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
        observed_at: "2026-05-27T00:00:00Z".to_string(),
    })
    .expect("collect inventory");

    let observed_identity = inventory.observed_identity.as_ref().expect("identity");
    assert_eq!(observed_identity.deployment_name, "prod");
    assert_eq!(
        observed_identity.root_principal.as_deref(),
        Some("aaaaa-aa")
    );

    let observed_root = inventory.observed_root.as_ref().expect("root evidence");
    assert_eq!(observed_root.deployment_name, "prod");
    assert_eq!(observed_root.network, "local");
    assert_eq!(observed_root.fleet_template, "demo");
    assert_eq!(observed_root.root_principal, "aaaaa-aa");
    assert_eq!(observed_root.observed_canister_id, "aaaaa-aa");
    assert_eq!(
        observed_root.observation_source,
        DeploymentRootObservationSourceV1::LocalDeploymentState
    );
    assert_eq!(
        observed_root.control_class,
        CanisterControlClassV1::UnknownUnsafe
    );
    assert_eq!(
        observed_root.role_assignment_source.as_deref(),
        Some("local_install_state")
    );
}
