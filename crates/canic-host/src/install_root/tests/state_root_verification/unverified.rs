use super::*;

#[test]
fn unverified_registered_root_is_not_used_as_plan_authority() {
    let root = temp_dir("canic-register-unverified-plan");
    let workspace_root = root.join("workspace");
    let icp_root = root.join("icp");
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(
        config_dir.join("canic.toml"),
        r#"
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[roles.project_registry]
kind = "canister"
package = "project_registry"

[roles.oracle_pokemon]
kind = "canister"
package = "oracle_pokemon"

[roles.user_hub]
kind = "canister"
package = "user_hub"

[roles.user_shard]
kind = "canister"
package = "user_shard"

[roles.scale_hub]
kind = "canister"
package = "scale_hub"

[roles.scale_replica]
kind = "canister"
package = "scale"

[roles.role_baseline]
kind = "canister"
package = "role_baseline"

[roles.worker]
kind = "canister"
package = "worker"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"
"#,
    )
    .expect("write config");
    register_deployment_state(RegisterDeploymentStateOptions {
        deployment_name: "demo-local".to_string(),
        fleet_template: "demo".to_string(),
        root_canister_id: "uxrrr-q7777-77774-qaaaq-cai".to_string(),
        network: "local".to_string(),
        allow_unverified: true,
        icp_root: Some(icp_root.clone()),
        workspace_root: Some(workspace_root.clone()),
    })
    .expect("register deployment state");

    let plan = crate::deployment_truth::build_local_deployment_plan(
        &crate::deployment_truth::LocalDeploymentPlanRequest {
            deployment_name: "demo-local".to_string(),
            network: "local".to_string(),
            workspace_root,
            icp_root,
            config_path: None,
            runtime_variant: "local".to_string(),
            build_profile: "fast".to_string(),
        },
    );

    assert_eq!(plan.trust_domain.root_trust_anchor, None);
    assert!(plan.unresolved_assumptions.iter().any(|assumption| {
        assumption.key == "local_state.unverified_root_canister_id"
            && assumption
                .description
                .contains("root verification is NotVerified")
    }));

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn unverified_registered_root_blocks_install_truth_gate() {
    let root = temp_dir("canic-register-unverified-gate");
    let workspace_root = root.join("workspace");
    let icp_root = root.join("icp");
    let config_path = workspace_root.join("fleets/demo/canic.toml");
    fs::create_dir_all(config_path.parent().expect("config parent")).expect("create config dir");
    fs::write(
        &config_path,
        r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[roles.project_registry]
kind = "canister"
package = "project_registry"

[roles.oracle_pokemon]
kind = "canister"
package = "oracle_pokemon"

[roles.user_hub]
kind = "canister"
package = "user_hub"

[roles.user_shard]
kind = "canister"
package = "user_shard"

[roles.scale_hub]
kind = "canister"
package = "scale_hub"

[roles.scale_replica]
kind = "canister"
package = "scale"

[roles.role_baseline]
kind = "canister"
package = "role_baseline"

[roles.worker]
kind = "canister"
package = "worker"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"
"#,
    )
    .expect("write config");
    write_wasm_gz_artifact(&icp_root, "root", b"root-artifact");
    register_deployment_state(RegisterDeploymentStateOptions {
        deployment_name: "demo-local".to_string(),
        fleet_template: "demo".to_string(),
        root_canister_id: "uxrrr-q7777-77774-qaaaq-cai".to_string(),
        network: "local".to_string(),
        allow_unverified: true,
        icp_root: Some(icp_root.clone()),
        workspace_root: Some(workspace_root.clone()),
    })
    .expect("register deployment state");
    let options = InstallRootOptions {
        root_canister: "root".to_string(),
        root_build_target: "root".to_string(),
        network: "local".to_string(),
        deployment_name: Some("demo-local".to_string()),
        icp_root: Some(icp_root.clone()),
        build_profile: Some(CanisterBuildProfile::Fast),
        ready_timeout_seconds: 30,
        config_path: Some(config_path.display().to_string()),
        expected_fleet: Some("demo".to_string()),
        interactive_config_selection: false,
        deployment_plan_override: None,
        artifact_promotion_plan_override: None,
    };

    let check = current_install_deployment_truth_check_at(
        &options,
        &workspace_root,
        &icp_root,
        &config_path,
        "demo-local",
        "2026-05-22T00:00:00Z".to_string(),
    )
    .expect("deployment truth check");
    let err = enforce_install_deployment_truth_gate(&check)
        .expect_err("unverified registered root must block mutation");

    assert!(check.report.hard_failures.iter().any(|finding| {
        finding.code == "unverified_deployment_root"
            && finding.subject.as_deref() == Some("local_state.unverified_root_canister_id")
    }));
    let blocked = err
        .downcast_ref::<InstallRootBlockedError>()
        .expect("deployment-truth gate should retain its typed reason");
    assert_eq!(blocked.kind(), InstallRootBlockKind::DeploymentTruth);

    fs::remove_dir_all(root).expect("clean temp dir");
}
