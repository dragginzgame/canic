use super::*;

#[test]
fn install_truth_check_uses_deployment_state_config_for_target_named_commands() {
    let root = temp_dir("canic-deploy-target-state-config");
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
    let state = sample_install_state(&root, "demo-local", "demo");
    write_install_state(&root, "local", &state).expect("write deployment state");
    let options = InstallRootOptions {
        root_canister: "root".to_string(),
        root_build_target: "root".to_string(),
        environment: "local".to_string(),
        fleet_name: "demo-local".to_string(),
        icp_root: Some(root.clone()),
        build_profile: Some(CanisterBuildProfile::Fast),
        config_path: None,
        expected_app: None,
        interactive_config_selection: false,
        deployment_plan_override: None,
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
