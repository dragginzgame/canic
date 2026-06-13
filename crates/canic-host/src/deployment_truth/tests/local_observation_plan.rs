use super::*;

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

#[test]
fn live_root_status_observation_maps_status_controllers_and_module_hash() {
    let state = sample_install_state("demo", "aaaaa-aa");
    let report = IcpCanisterStatusReport {
        id: "aaaaa-aa".to_string(),
        name: Some("root".to_string()),
        status: "Running".to_string(),
        settings: Some(IcpCanisterStatusSettings {
            controllers: vec!["aaaaa-aa".to_string()],
            compute_allocation: Some("0".to_string()),
            memory_allocation: None,
            freezing_threshold: None,
            reserved_cycles_limit: None,
            wasm_memory_limit: None,
            wasm_memory_threshold: None,
            log_memory_limit: None,
        }),
        module_hash: Some("0xABCD".to_string()),
        memory_size: None,
        cycles: None,
        reserved_cycles: None,
        idle_cycles_burned_per_day: None,
    };

    let observed = observed_root_from_status(&state, &report);

    assert_eq!(observed.canister_id, "aaaaa-aa");
    assert_eq!(
        observed.control_class,
        CanisterControlClassV1::DeploymentControlled
    );
    assert_eq!(observed.controllers, vec!["aaaaa-aa"]);
    assert_eq!(observed.module_hash.as_deref(), Some("abcd"));
    assert_eq!(observed.status.as_deref(), Some("Running"));
    assert_eq!(
        observed.role_assignment_source.as_deref(),
        Some("icp_canister_status")
    );
}

#[test]
fn registry_entries_map_configured_pool_roles_to_observed_pool() {
    let mut gaps = Vec::new();
    let entries = vec![
        RegistryEntry {
            pid: "root-id".to_string(),
            role: Some("root".to_string()),
            kind: None,
            parent_pid: None,
            module_hash: None,
        },
        RegistryEntry {
            pid: "shard-id".to_string(),
            role: Some("user_shard".to_string()),
            kind: None,
            parent_pid: Some("user_hub-id".to_string()),
            module_hash: Some("module".to_string()),
        },
        RegistryEntry {
            pid: "user_hub-id".to_string(),
            role: Some("user_hub".to_string()),
            kind: None,
            parent_pid: Some("root-id".to_string()),
            module_hash: None,
        },
    ];
    let expectations = vec![ConfiguredPoolExpectation {
        pool: "user_shards".to_string(),
        canister_role: "user_shard".to_string(),
    }];

    let observed = registry_entries_to_observed_pool("root-id", &entries, &expectations, &mut gaps);

    assert_eq!(
        observed,
        vec![ObservedPoolCanisterV1 {
            pool: "user_shards".to_string(),
            canister_id: "shard-id".to_string(),
            role: Some("user_shard".to_string()),
            control_class: CanisterControlClassV1::CanicManagedPool,
        }]
    );
    assert!(gaps.is_empty());
}

#[test]
fn registry_entries_map_roles_to_observed_canisters_without_controller_authority() {
    let entries = vec![
        RegistryEntry {
            pid: "root-id".to_string(),
            role: Some("root".to_string()),
            kind: None,
            parent_pid: None,
            module_hash: None,
        },
        RegistryEntry {
            pid: "user_hub-id".to_string(),
            role: Some("user_hub".to_string()),
            kind: None,
            parent_pid: Some("root-id".to_string()),
            module_hash: Some("0xABCDEF".to_string()),
        },
    ];

    let observed = registry_entries_to_observed_canisters("root-id", &entries);

    assert_eq!(observed.len(), 1);
    assert_eq!(observed[0].canister_id, "user_hub-id");
    assert_eq!(observed[0].role.as_deref(), Some("user_hub"));
    assert_eq!(
        observed[0].control_class,
        CanisterControlClassV1::CanicManagedPool
    );
    assert!(observed[0].controllers.is_empty());
    assert_eq!(observed[0].module_hash.as_deref(), Some("abcdef"));
    assert_eq!(
        observed[0].role_assignment_source.as_deref(),
        Some("subnet_registry")
    );
}

#[test]
fn registry_observation_can_be_enriched_with_live_status() {
    let mut observed = registry_entries_to_observed_canisters(
        "root-id",
        &[RegistryEntry {
            pid: "user_hub-id".to_string(),
            role: Some("user_hub".to_string()),
            kind: None,
            parent_pid: Some("root-id".to_string()),
            module_hash: Some("stale".to_string()),
        }],
    )
    .pop()
    .expect("registry observation");
    let report = IcpCanisterStatusReport {
        id: "user_hub-id".to_string(),
        name: Some("user_hub".to_string()),
        status: "Running".to_string(),
        settings: Some(IcpCanisterStatusSettings {
            controllers: vec!["root-id".to_string()],
            compute_allocation: Some("0".to_string()),
            memory_allocation: None,
            freezing_threshold: None,
            reserved_cycles_limit: None,
            wasm_memory_limit: None,
            wasm_memory_threshold: None,
            log_memory_limit: None,
        }),
        module_hash: Some("0xCAFE".to_string()),
        memory_size: None,
        cycles: None,
        reserved_cycles: None,
        idle_cycles_burned_per_day: None,
    };

    apply_live_status_to_registry_observation(&mut observed, &report);

    assert_eq!(
        observed.control_class,
        CanisterControlClassV1::CanicManagedPool
    );
    assert_eq!(observed.controllers, vec!["root-id"]);
    assert_eq!(observed.module_hash.as_deref(), Some("cafe"));
    assert_eq!(observed.status.as_deref(), Some("Running"));
    assert_eq!(
        observed.role_assignment_source.as_deref(),
        Some("subnet_registry+icp_canister_status")
    );
}

#[test]
fn observed_pool_control_uses_enriched_canister_status() {
    let mut observed_pool = vec![ObservedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: "shard-id".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    }];
    let observed_canisters = vec![ObservedCanisterV1 {
        canister_id: "shard-id".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::UnknownUnsafe,
        controllers: vec!["external-controller".to_string()],
        module_hash: Some("module".to_string()),
        status: Some("Running".to_string()),
        root_trust_anchor: Some("root-id".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("subnet_registry+icp_canister_status".to_string()),
    }];

    apply_canister_control_to_observed_pool(&mut observed_pool, &observed_canisters);

    assert_eq!(
        observed_pool[0].control_class,
        CanisterControlClassV1::UnknownUnsafe
    );
}

#[test]
fn registry_entries_report_ambiguous_pool_role_mapping() {
    let mut gaps = Vec::new();
    let entries = vec![RegistryEntry {
        pid: "worker-id".to_string(),
        role: Some("worker".to_string()),
        kind: None,
        parent_pid: Some("root-id".to_string()),
        module_hash: None,
    }];
    let expectations = vec![
        ConfiguredPoolExpectation {
            pool: "workers_a".to_string(),
            canister_role: "worker".to_string(),
        },
        ConfiguredPoolExpectation {
            pool: "workers_b".to_string(),
            canister_role: "worker".to_string(),
        },
    ];

    let observed = registry_entries_to_observed_pool("root-id", &entries, &expectations, &mut gaps);

    assert!(observed.is_empty());
    assert!(
        gaps.iter()
            .any(|gap| gap.key == "live_subnet_registry.pool.worker")
    );
}

#[test]
fn local_inventory_reports_missing_config_as_observation_gap() {
    let temp = TempWorkspace::new("canic-host-local-inventory-missing-config");

    let inventory = collect_local_deployment_inventory(&LocalInventoryRequest {
        deployment_name: "demo".to_string(),
        network: "local".to_string(),
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
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("canic.toml"), SAMPLE_CONFIG).expect("write config");
    write_artifact(&icp_root, "root", b"root-artifact");
    write_artifact(&icp_root, "wasm_store", b"wasm-store-artifact");
    write_artifact(&icp_root, "user_hub", b"user-hub-artifact");
    write_release_set_manifest(&icp_root);

    let manifest = collect_local_role_artifact_manifest(&LocalArtifactManifestRequest {
        network: "local".to_string(),
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
    assert_eq!(user_hub.wasm_gz_sha256.as_deref(), Some("user-hub-hash"));
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
fn local_artifact_manifest_reports_network_artifact_fallback() {
    let temp = TempWorkspace::new("canic-host-local-artifact-manifest-fallback");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("canic.toml"), SAMPLE_CONFIG).expect("write config");
    write_artifact(&icp_root, "root", b"root-artifact");

    let manifest = collect_local_role_artifact_manifest(&LocalArtifactManifestRequest {
        network: "ic".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
    });

    assert!(
        manifest
            .unresolved_artifacts
            .iter()
            .any(|gap| gap.key == "local_artifacts.network_fallback")
    );
}

#[test]
fn local_artifact_manifest_records_missing_artifacts_as_gaps() {
    let temp = TempWorkspace::new("canic-host-local-artifact-manifest-missing");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("canic.toml"), SAMPLE_CONFIG).expect("write config");
    write_artifact(&icp_root, "root", b"root-artifact");

    let manifest = collect_local_role_artifact_manifest(&LocalArtifactManifestRequest {
        network: "local".to_string(),
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

#[test]
fn local_plan_uses_configured_roles_and_local_artifact_manifest() {
    let temp = TempWorkspace::new("canic-host-local-plan");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("canic.toml"), SAMPLE_CONFIG).expect("write config");
    write_artifact(&icp_root, "root", b"root-artifact");
    write_artifact(&icp_root, "wasm_store", b"wasm-store-artifact");
    write_artifact(&icp_root, "user_hub", b"user-hub-artifact");
    write_release_set_manifest(&icp_root);

    let plan = build_local_deployment_plan(&LocalDeploymentPlanRequest {
        deployment_name: "demo-local".to_string(),
        network: "local".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
        runtime_variant: "local".to_string(),
        build_profile: "fast".to_string(),
    });

    assert_eq!(plan.plan_id, "local:local:demo-local:plan");
    assert_eq!(plan.deployment_identity.deployment_name, "demo-local");
    assert_eq!(
        plan.deployment_identity
            .deployment_manifest_digest
            .as_ref()
            .map(String::len),
        Some(64)
    );
    assert_eq!(
        plan.deployment_identity
            .canonical_runtime_config_digest
            .as_ref()
            .map(String::len),
        Some(64)
    );
    assert_eq!(
        plan.deployment_identity
            .authority_profile_hash
            .as_ref()
            .map(String::len),
        Some(64)
    );
    assert_eq!(
        plan.deployment_identity
            .role_topology_hash
            .as_ref()
            .map(String::len),
        Some(64)
    );
    assert_eq!(
        plan.deployment_identity
            .artifact_set_digest
            .as_ref()
            .map(String::len),
        Some(64)
    );
    assert_eq!(
        plan.deployment_identity
            .pool_identity_set_digest
            .as_ref()
            .map(String::len),
        Some(64)
    );
    assert_eq!(
        plan.role_artifacts[0]
            .raw_config_sha256
            .as_ref()
            .map(String::len),
        Some(64)
    );
    assert_eq!(plan.fleet_template, "demo");
    assert_eq!(plan.runtime_variant, "local");
    assert_eq!(plan.role_artifacts.len(), 3);
    assert!(
        plan.role_artifacts
            .iter()
            .all(|artifact| artifact.build_profile == "fast")
    );
    assert_plan_has_implicit_wasm_store_artifact(&plan);
    assert_plan_has_user_hub_release_artifact(&plan);
    assert_eq!(
        plan.expected_canisters
            .iter()
            .map(|canister| canister.role.as_str())
            .collect::<Vec<_>>(),
        vec!["root", "wasm_store", "user_hub"]
    );
    assert_plan_excludes_declared_only_store(&plan);
    let root_assumption = plan
        .unresolved_assumptions
        .iter()
        .find(|assumption| assumption.key == "local_state.root_canister_id")
        .expect("missing root identity should be recorded");
    assert!(root_assumption.description.contains("--allow-unverified"));
}

#[test]
fn local_plan_uses_configured_controllers_as_expected_authority() {
    let temp = TempWorkspace::new("canic-host-local-plan-controllers");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(
        config_dir.join("canic.toml"),
        r#"
controllers = [
  "zbf4m-zw3nk-6owqc-qmluz-xhwxt-2pkky-xhjy2-kqxor-qzxsn-6d2bz-nae",
  "aaaaa-aa",
]
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.user_hub]
kind = "canister"
package = "user_hub"

[roles.user_shard]
kind = "canister"
package = "user_shard"

[roles.store]
kind = "canister"
package = "store"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"
"#,
    )
    .expect("write config");
    write_artifact(&icp_root, "root", b"root-artifact");

    let plan = build_local_deployment_plan(&LocalDeploymentPlanRequest {
        deployment_name: "demo-local".to_string(),
        network: "local".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
        runtime_variant: "local".to_string(),
        build_profile: "fast".to_string(),
    });

    assert_eq!(
        plan.authority_profile.expected_controllers,
        vec![
            "aaaaa-aa".to_string(),
            "zbf4m-zw3nk-6owqc-qmluz-xhwxt-2pkky-xhjy2-kqxor-qzxsn-6d2bz-nae".to_string(),
        ]
    );
    assert!(plan.authority_profile.staging_controllers.is_empty());
    assert!(plan.authority_profile.emergency_controllers.is_empty());
    let root_assumption = plan
        .unresolved_assumptions
        .iter()
        .find(|assumption| assumption.key == "local_state.root_canister_id")
        .expect("missing root identity should be recorded");
    assert!(root_assumption.description.contains("--allow-unverified"));
}

#[test]
fn local_plan_uses_install_state_root_as_expected_canister() {
    let temp = TempWorkspace::new("canic-host-local-plan-root-state");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("canic.toml"), SAMPLE_CONFIG).expect("write config");
    write_artifact(&icp_root, "root", b"root-artifact");
    write_artifact(&icp_root, "wasm_store", b"wasm-store-artifact");
    write_artifact(&icp_root, "user_hub", b"user-hub-artifact");
    write_release_set_manifest(&icp_root);
    let state_path = icp_root.join(".canic/local/deployments/demo-local.json");
    fs::create_dir_all(state_path.parent().expect("state parent")).expect("create state dir");
    fs::write(
        state_path,
        serde_json::to_vec_pretty(&sample_install_state("demo-local", "aaaaa-aa"))
            .expect("encode state"),
    )
    .expect("write install state");

    let plan = build_local_deployment_plan(&LocalDeploymentPlanRequest {
        deployment_name: "demo-local".to_string(),
        network: "local".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
        runtime_variant: "local".to_string(),
        build_profile: "fast".to_string(),
    });

    assert_eq!(
        plan.deployment_identity.root_principal.as_deref(),
        Some("aaaaa-aa")
    );
    assert_eq!(
        plan.trust_domain.root_trust_anchor.as_deref(),
        Some("aaaaa-aa")
    );
    assert!(
        plan.expected_canisters
            .iter()
            .any(|canister| canister.role == "root"
                && canister.canister_id.as_deref() == Some("aaaaa-aa"))
    );
    assert!(plan.unresolved_assumptions.is_empty());
}

#[test]
fn local_plan_uses_configured_pools_as_expected_pool_identities() {
    let temp = TempWorkspace::new("canic-host-local-plan-pools");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(
        config_dir.join("canic.toml"),
        r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.user_hub]
kind = "canister"
package = "user_hub"

[roles.user_shard]
kind = "canister"
package = "user_shard"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.user_hub]
kind = "service"

[subnets.prime.canisters.user_hub.sharding.pools.user_shards]
canister_role = "user_shard"
policy.capacity = 100
policy.max_shards = 4

[subnets.prime.canisters.user_shard]
kind = "shard"
"#,
    )
    .expect("write config");
    write_artifact(&icp_root, "root", b"root-artifact");
    write_artifact(&icp_root, "user_hub", b"user-hub-artifact");
    write_artifact(&icp_root, "user_shard", b"user-shard-artifact");

    let plan = build_local_deployment_plan(&LocalDeploymentPlanRequest {
        deployment_name: "demo-local".to_string(),
        network: "local".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
        runtime_variant: "local".to_string(),
        build_profile: "fast".to_string(),
    });

    assert_eq!(
        plan.expected_pool,
        vec![ExpectedPoolCanisterV1 {
            pool: "user_shards".to_string(),
            canister_id: None,
            role: Some("user_shard".to_string()),
        }]
    );
    let inventory = sample_matching_inventory();
    let diff = compare_plan_to_inventory(&plan, &inventory);
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "pool_canister_unobserved"
                && finding.subject.as_deref() == Some("user_shards:user_shard"))
    );
}
