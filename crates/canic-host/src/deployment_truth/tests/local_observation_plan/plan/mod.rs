use super::super::*;

#[test]
fn local_plan_uses_configured_roles_and_local_artifact_manifest() {
    let temp = TempWorkspace::new("canic-host-local-plan");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("apps");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("canic.toml"), SAMPLE_CONFIG).expect("write config");
    write_local_network_authority(&icp_root, "local");
    write_artifact(&icp_root, "fleet_coordinator", b"coordinator-artifact");
    write_artifact(&icp_root, "root", b"root-artifact");
    write_artifact(&icp_root, "wasm_store", b"wasm-store-artifact");
    write_artifact(&icp_root, "user_hub", b"user-hub-artifact");
    write_release_set_manifest(&icp_root);

    let plan = build_local_deployment_plan(&LocalDeploymentPlanRequest {
        fleet_name: "demo-local".to_string(),
        app: "demo".to_string(),
        environment: "local".to_string(),
        artifact_environment: "local".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
        runtime_variant: "local".to_string(),
        build_profile: "fast".to_string(),
    });

    assert_eq!(plan.plan_id, "local:local:demo-local:plan");
    assert_eq!(plan.deployment_identity.fleet_name, "demo-local");
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
    assert_eq!(plan.deployment_identity.app, "demo");
    assert_eq!(plan.runtime_variant, "local");
    assert_eq!(plan.role_artifacts.len(), 4);
    assert!(
        plan.role_artifacts
            .iter()
            .all(|artifact| artifact.build_profile == "fast")
    );
    assert_plan_has_built_in_infrastructure_artifacts(&plan);
    assert_plan_has_user_hub_release_artifact(&plan);
    assert!(plan.authority_profile.expected_controllers.is_empty());
    assert_eq!(
        plan.expected_canisters
            .iter()
            .map(|canister| canister.role.as_str())
            .collect::<Vec<_>>(),
        vec!["fleet_coordinator", "root", "wasm_store", "user_hub"]
    );
    assert_plan_excludes_declared_only_store(&plan);
    assert!(
        plan.unresolved_assumptions
            .iter()
            .any(|assumption| assumption.has_kind(DeploymentAssumptionKindV1::FleetCatalogMissing))
    );
}

#[test]
fn local_plan_uses_catalog_fleet_identity_without_inventing_one_root() {
    let temp = TempWorkspace::new("canic-host-local-plan-root-state");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("apps");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("canic.toml"), SAMPLE_CONFIG).expect("write config");
    write_artifact(&icp_root, "fleet_coordinator", b"coordinator-artifact");
    write_artifact(&icp_root, "root", b"root-artifact");
    write_artifact(&icp_root, "wasm_store", b"wasm-store-artifact");
    write_artifact(&icp_root, "user_hub", b"user-hub-artifact");
    write_release_set_manifest(&icp_root);
    write_fleet_catalog_json(
        &icp_root,
        "local",
        sample_fleet_catalog_entry("demo-local", "rrkah-fqaaa-aaaaa-aaaaq-cai"),
    );

    let plan = build_local_deployment_plan(&LocalDeploymentPlanRequest {
        fleet_name: "demo-local".to_string(),
        app: "demo".to_string(),
        environment: "local".to_string(),
        artifact_environment: "local".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
        runtime_variant: "local".to_string(),
        build_profile: "fast".to_string(),
    });

    assert_eq!(
        plan.deployment_identity.fleet_id,
        Some(canic_core::ids::FleetId::from_generated_bytes([7; 32]))
    );
    assert_eq!(plan.deployment_identity.root_principal, None);
    assert_eq!(plan.trust_domain.root_trust_anchor, None);
    assert!(
        plan.expected_canisters
            .iter()
            .any(|canister| canister.role == "root" && canister.canister_id.is_none())
    );
    assert!(plan.unresolved_assumptions.is_empty());
}

#[test]
fn local_plan_uses_configured_pools_as_expected_pool_identities() {
    let temp = TempWorkspace::new("canic-host-local-plan-pools");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("apps");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(
        config_dir.join("canic.toml"),
        r#"
[app]
name = "demo"
init_mode = "enabled"


[roles.root]
kind = "root"
package = "root"

[roles.user_hub]
kind = "canister"
package = "user_hub"

[roles.user_shard]
kind = "canister"
package = "user_shard"
[app.whitelist]



[component_specs.user_hub]
component_role = "user_hub"
maximum_instances = 1

[component_specs.user_hub.sharding.pools.user_shards]
canister_role = "user_shard"
policy.capacity = 100
policy.max_shards = 4

[component_specs.user_hub.children.user_shard]
kind = "shard"

[component_specs.user_hub.spawn_grants.user_hub.user_shard]
maximum_instances_per_parent = 20_000
"#,
    )
    .expect("write config");
    write_artifact(&icp_root, "root", b"root-artifact");
    write_artifact(&icp_root, "user_hub", b"user-hub-artifact");
    write_artifact(&icp_root, "user_shard", b"user-shard-artifact");

    let plan = build_local_deployment_plan(&LocalDeploymentPlanRequest {
        fleet_name: "demo-local".to_string(),
        app: "demo".to_string(),
        environment: "local".to_string(),
        artifact_environment: "local".to_string(),
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
