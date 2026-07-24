use super::*;

#[test]
fn install_truth_gate_lines_include_warning_codes() {
    let root = temp_dir("canic-install-truth-warning-lines");
    let config_path = root.join("apps/demo/canic.toml");
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
    write_wasm_gz_artifact(&root, "wasm_store", b"wasm-store-artifact");

    let options = InstallRootOptions {
        root_canister: "root".to_string(),
        root_build_target: "root".to_string(),
        environment: "local".to_string(),
        fleet_name: "demo".to_string(),
        icp_root: Some(root.clone()),
        build_profile: Some(CanisterBuildProfile::Fast),
        config_path: Some("apps/demo/canic.toml".to_string()),
        expected_app: Some("demo".to_string()),
        interactive_config_selection: false,
        deployment_plan_override: None,
    };

    let mut check = current_install_deployment_truth_check_at(
        &options,
        &root,
        &root,
        &config_path,
        "demo",
        "2026-05-22T00:00:00Z".to_string(),
    )
    .expect("deployment truth check");
    check.report.warnings.push(SafetyFindingV1 {
        code: "observation_gap".to_string(),
        message: "live root status was not observed".to_string(),
        severity: SafetySeverityV1::Warning,
        subject: Some("live_canister_status.root".to_string()),
    });

    let receipt = install_deployment_truth_gate_receipt(
        &check,
        "start".to_string(),
        vec![artifact_gate_phase_receipt(
            &check,
            "start",
            Some("finish".into()),
        )],
        artifact_gate_role_phase_receipts(&check),
    );
    let lines = install_deployment_truth_gate_lines(&check, &receipt);

    assert!(lines.iter().any(|line| {
        line.contains("Deployment truth receipt:") && line.contains("status=Complete")
    }));
    assert!(lines.iter().any(|line| line.contains(
        "Deployment truth warning: inventory:observation_gap:live_canister_status.root"
    )));
    assert!(lines.iter().any(|line| {
        line.contains("Deployment truth role receipt: phase=materialize_artifacts role=root")
    }));

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_gate_lines_distinguish_plan_assumptions() {
    let root = temp_dir("canic-install-truth-plan-assumption-lines");
    let config_path = root.join("apps/demo/canic.toml");
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

    let options = InstallRootOptions {
        root_canister: "root".to_string(),
        root_build_target: "root".to_string(),
        environment: "local".to_string(),
        fleet_name: "demo".to_string(),
        icp_root: Some(root.clone()),
        build_profile: Some(CanisterBuildProfile::Fast),
        config_path: Some("apps/demo/canic.toml".to_string()),
        expected_app: Some("demo".to_string()),
        interactive_config_selection: false,
        deployment_plan_override: None,
    };

    let check = current_install_deployment_truth_check_at(
        &options,
        &root,
        &root,
        &config_path,
        "demo",
        "2026-05-22T00:00:00Z".to_string(),
    )
    .expect("deployment truth check");
    let receipt = install_deployment_truth_gate_receipt(
        &check,
        "start".to_string(),
        vec![artifact_gate_phase_receipt(
            &check,
            "start",
            Some("finish".into()),
        )],
        artifact_gate_role_phase_receipts(&check),
    );
    let lines = install_deployment_truth_gate_lines(&check, &receipt);

    assert!(lines.iter().any(|line| {
        line.contains(
            "Deployment truth warning: plan:plan_assumption:local_state.root_canister_id.missing",
        )
    }));

    fs::remove_dir_all(root).expect("clean temp dir");
}
