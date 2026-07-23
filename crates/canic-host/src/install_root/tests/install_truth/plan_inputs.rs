use super::*;

#[test]
fn install_truth_artifact_gate_blocks_missing_built_artifacts() {
    let root = temp_dir("canic-install-truth-artifact-gate");
    let config_path = root.join("fleets/demo/canic.toml");
    fs::create_dir_all(config_path.parent().expect("config parent")).expect("create config dir");
    fs::write(
        &config_path,
        r#"
controllers = []
[services.fleet]
roles = []

[app]
name = "demo"
init_mode = "enabled"


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
[app.whitelist]

[subnets.default.canisters.root]
kind = "root"

[subnets.default.canisters.user_hub]
kind = "service"
"#,
    )
    .expect("write config");
    write_wasm_gz_artifact(&root, "root", b"root-artifact");
    write_wasm_gz_artifact(&root, "wasm_store", b"wasm-store-artifact");

    let options = local_demo_install_options(&root);

    let check = current_install_deployment_truth_check_at(
        &options,
        &root,
        &root,
        &config_path,
        "demo",
        "2026-05-22T00:00:00Z".to_string(),
    )
    .expect("deployment truth check");

    assert!(
        check
            .report
            .hard_failures
            .iter()
            .any(|finding| finding.code == "artifact_missing"
                && finding.subject.as_deref() == Some("user_hub"))
    );
    assert!(enforce_install_deployment_truth_gate(&check).is_err());

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_check_uses_supplied_deployment_plan_override() {
    let (root, mut check) = demo_install_deployment_truth_check(
        "canic-install-truth-supplied-deployment-plan-override",
    );
    check.plan.plan_id = "promoted-plan-1".to_string();
    let config_path = root.join("fleets/demo/canic.toml");
    let options = InstallRootOptions {
        root_canister: "root".to_string(),
        root_build_target: "root".to_string(),
        environment: "local".to_string(),
        deployment_name: None,
        icp_root: Some(root.clone()),
        build_profile: Some(CanisterBuildProfile::Fast),
        ready_timeout_seconds: 30,
        config_path: Some("fleets/demo/canic.toml".to_string()),
        expected_fleet: Some("demo".to_string()),
        interactive_config_selection: false,
        deployment_plan_override: Some(check.plan),
        artifact_promotion_plan_override: None,
    };

    let supplied_check = current_install_deployment_truth_check_at(
        &options,
        &root,
        &root,
        &config_path,
        "demo",
        "2026-05-22T00:00:00Z".to_string(),
    )
    .expect("deployment truth check");

    assert_eq!(supplied_check.plan.plan_id, "promoted-plan-1");
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_check_rejects_supplied_plan_environment_mismatch() {
    let (root, mut check) =
        demo_install_deployment_truth_check("canic-install-truth-plan-environment-mismatch");
    check.plan.deployment_identity.environment = "ic".to_string();
    let config_path = root.join("fleets/demo/canic.toml");
    let options = InstallRootOptions {
        root_canister: "root".to_string(),
        root_build_target: "root".to_string(),
        environment: "local".to_string(),
        deployment_name: None,
        icp_root: Some(root.clone()),
        build_profile: Some(CanisterBuildProfile::Fast),
        ready_timeout_seconds: 30,
        config_path: Some("fleets/demo/canic.toml".to_string()),
        expected_fleet: Some("demo".to_string()),
        interactive_config_selection: false,
        deployment_plan_override: Some(check.plan),
        artifact_promotion_plan_override: None,
    };

    current_install_deployment_truth_check_at(
        &options,
        &root,
        &root,
        &config_path,
        "demo",
        "2026-05-22T00:00:00Z".to_string(),
    )
    .expect_err("environment mismatch should fail");

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_check_rejects_supplied_plan_deployment_target_mismatch() {
    let (root, mut check) =
        demo_install_deployment_truth_check("canic-install-truth-plan-target-mismatch");
    check.plan.deployment_identity.deployment_name = "prod".to_string();
    let config_path = root.join("fleets/demo/canic.toml");
    let options = InstallRootOptions {
        root_canister: "root".to_string(),
        root_build_target: "root".to_string(),
        environment: "local".to_string(),
        deployment_name: None,
        icp_root: Some(root.clone()),
        build_profile: Some(CanisterBuildProfile::Fast),
        ready_timeout_seconds: 30,
        config_path: Some("fleets/demo/canic.toml".to_string()),
        expected_fleet: Some("demo".to_string()),
        interactive_config_selection: false,
        deployment_plan_override: Some(check.plan),
        artifact_promotion_plan_override: None,
    };

    current_install_deployment_truth_check_at(
        &options,
        &root,
        &root,
        &config_path,
        "demo",
        "2026-05-22T00:00:00Z".to_string(),
    )
    .expect_err("deployment target mismatch should fail");

    fs::remove_dir_all(root).expect("clean temp dir");
}
