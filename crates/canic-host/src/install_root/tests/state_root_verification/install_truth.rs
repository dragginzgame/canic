use super::*;

#[test]
fn install_truth_check_uses_deployment_state_config_for_target_named_commands() {
    let root = temp_dir("canic-deploy-target-state-config");
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
"#,
    )
    .expect("write config");
    write_wasm_gz_artifact(&root, "root", b"root-artifact");
    let state = sample_install_state(&root, "demo-local", "demo");
    write_install_state(&root, "local", &state).expect("write deployment state");
    let options = InstallRootOptions {
        root_canister: "root".to_string(),
        root_build_target: "root".to_string(),
        environment: "local".to_string(),
        deployment_name: Some("demo-local".to_string()),
        icp_root: Some(root.clone()),
        build_profile: Some(CanisterBuildProfile::Fast),
        ready_timeout_seconds: 30,
        config_path: None,
        expected_fleet: None,
        interactive_config_selection: false,
        deployment_plan_override: None,
        artifact_promotion_plan_override: None,
    };

    let check = check_install_deployment_truth(&options, "2026-05-22T00:00:00Z")
        .expect("deployment truth check");

    assert_eq!(check.plan.deployment_identity.deployment_name, "demo-local");
    assert_eq!(check.plan.fleet_template, "demo");
    assert_eq!(
        check.plan.trust_domain.root_trust_anchor.as_deref(),
        Some("uxrrr-q7777-77774-qaaaq-cai")
    );

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_state_write_receipt_records_local_state_path() {
    let (root, check) = demo_install_deployment_truth_check("canic-install-state-receipt");
    let state = sample_install_state(&root, "demo", "demo");
    let execution_context = current_install_execution_context(&root, &root, "local");
    let scope = InstallReceiptScope {
        icp_root: &root,
        environment: "local",
        deployment_name: "demo",
        check: &check,
        execution_context: Some(&execution_context),
    };

    let state_path = write_install_state_with_deployment_truth_receipt(scope, "local", &state)
        .expect("write install state and receipt");
    let receipt_dir = root.join(".canic/local/deployment-receipts/demo");
    let receipt = fs::read_dir(&receipt_dir)
        .expect("read receipts")
        .map(|entry| {
            let path = entry.expect("receipt entry").path();
            serde_json::from_slice::<DeploymentReceiptV1>(
                &fs::read(path).expect("read receipt JSON"),
            )
            .expect("decode receipt")
        })
        .find(|receipt| receipt.operation_id.ends_with(":write_install_state"))
        .expect("write install state receipt");

    assert_eq!(state_path, root.join(".canic/local/deployments/demo.json"));
    assert_eq!(
        receipt.operation_status,
        DeploymentExecutionStatusV1::Complete
    );
    assert_eq!(receipt.phase_receipts[0].phase, "write_install_state");
    assert!(
        receipt.phase_receipts[0]
            .verified_postcondition
            .evidence
            .contains(&format!("install_state:{}", state_path.display()))
    );
    assert!(
        receipt.phase_receipts[0]
            .verified_postcondition
            .evidence
            .contains(&"deployment:demo".to_string())
    );
    assert!(
        receipt.phase_receipts[0]
            .verified_postcondition
            .evidence
            .contains(&"fleet_template:demo".to_string())
    );

    fs::remove_dir_all(root).expect("clean temp dir");
}
