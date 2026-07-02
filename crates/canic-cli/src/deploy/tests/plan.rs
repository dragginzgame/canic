use super::super::plan as deploy_plan;
use super::*;
use crate::test_support::TempDir;
use canic_host::install_root::{InstallState, RootVerificationStatus};
use serde_json::Value as JsonValue;
use std::{ffi::OsString, fs, path::PathBuf};

const SAMPLE_CONFIG: &str = r#"
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

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.user_hub]
kind = "service"
"#;

const MALFORMED_DESIRED_CONFIG: &str = r#"
controllers = ["not-a-principal"]

[fleet]
name = "demo"
"#;

const CONTROLLER_CONFIG: &str = r#"
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

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.user_hub]
kind = "service"
"#;

const POOL_CONFIG: &str = r#"
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
"#;

#[test]
fn deploy_plan_is_top_level_deploy_command() {
    let parsed = parse_subcommand(
        deploy_command(),
        [OsString::from("plan"), OsString::from("demo-local")],
    )
    .expect("parse deploy plan command")
    .expect("deploy plan command");

    assert_eq!(parsed.0, "plan");
    assert_eq!(parsed.1, vec![OsString::from("demo-local")]);

    let help = usage();
    assert!(help.contains("canic deploy plan demo"));
    assert!(help.contains("0.79 operator planning report"));
}

#[test]
fn deploy_plan_help_documents_no_mutation_contract() {
    let help = deploy_plan::usage();

    assert!(help.contains("canic deploy plan <deployment>"));
    assert!(help.contains("canic deploy plan demo-local --json"));
    assert!(help.contains("canic deploy plan demo-local --out deployment-plan.json"));
    assert!(help.contains("does not install, upgrade, create canisters"));
    assert!(help.contains("write deployment truth"));
    assert!(help.contains("installed deployment records"));
    assert!(help.contains("call live IC state"));
    assert!(help.contains("--out writes JSON only"));
    assert!(help.contains("fails if the requested path already exists"));
}

#[test]
fn deploy_plan_options_parse_supported_surface() {
    let options = deploy_plan::DeployPlanOptions::parse([
        OsString::from("demo-local"),
        OsString::from("--json"),
        OsString::from("--out"),
        OsString::from("deployment-plan.json"),
        OsString::from("--config"),
        OsString::from("fleets/demo/canic.toml"),
        OsString::from("--build-profile"),
        OsString::from("fast"),
        OsString::from(crate::cli::globals::INTERNAL_NETWORK_OPTION),
        OsString::from("local"),
    ])
    .expect("parse deploy plan options");

    assert_eq!(options.deployment, "demo-local");
    assert_eq!(options.network, "local");
    assert!(options.json);
    assert_eq!(options.out, Some(PathBuf::from("deployment-plan.json")));
    assert_eq!(
        options.config,
        Some(PathBuf::from("fleets/demo/canic.toml"))
    );
}

#[test]
fn deploy_plan_rejects_hard_cut_forms() {
    for args in [
        vec![],
        vec![OsString::from("--deployment"), OsString::from("demo-local")],
        vec![OsString::from("demo-local"), OsString::from("--apply")],
        vec![
            OsString::from("demo-local"),
            OsString::from("--write-truth"),
        ],
        vec![OsString::from("demo-local"), OsString::from("--evidence")],
        vec![
            OsString::from("demo-local"),
            OsString::from("--format"),
            OsString::from("json"),
        ],
        vec![
            OsString::from("demo-local"),
            OsString::from("--from-check"),
            OsString::from("deployment-check.json"),
        ],
        vec![
            OsString::from("demo-local"),
            OsString::from("--observe-local"),
        ],
        vec![
            OsString::from("demo-local"),
            OsString::from("--out"),
            OsString::from("deployment-plan.json"),
            OsString::from("--force"),
        ],
    ] {
        assert!(matches!(
            deploy_plan::DeployPlanOptions::parse(args),
            Err(DeployCommandError::Usage(_))
        ));
    }
}

#[test]
fn deploy_plan_report_builds_from_config_without_installed_state() {
    let (_temp, workspace_root, icp_root) = temp_plan_workspace("canic-deploy-plan-report");
    write_artifact(&icp_root, "root", b"root-artifact");
    let options = deploy_plan::DeployPlanOptions::parse([
        OsString::from("demo-local"),
        OsString::from("--config"),
        OsString::from("fleets/demo/canic.toml"),
    ])
    .expect("parse deploy plan options");

    let report = deploy_plan::build_report(
        &options,
        &deploy_plan::DeployPlanRoots {
            workspace_root,
            icp_root,
        },
    );
    let json = serde_json::to_value(&report).expect("report should serialize");

    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["command"], "canic deploy plan");
    assert_eq!(json["target"], "demo-local");
    assert_eq!(json["status"], "warning");
    assert_eq!(json["comparison_status"], "not_available");
    assert_eq!(
        json["plan"]["deployment_identity"]["deployment_name"],
        "demo-local"
    );
    assert_eq!(json["plan"]["fleet_template"], "demo");
    assert_base_plan_verified_facts(&json);
    assert!(
        json["warnings"]
            .as_array()
            .expect("warnings")
            .iter()
            .any(|item| item["code"] == "observed_inventory_unavailable")
    );
    assert_proposed_operation_keys(
        &json,
        &[
            "future_apply_preview|create_canister|root|not_executed",
            "future_apply_preview|create_canister|user_hub|not_executed",
            "future_apply_preview|create_canister|wasm_store|not_executed",
            "future_apply_preview|install_wasm|root|not_executed",
            "future_apply_preview|install_wasm|user_hub|not_executed",
            "future_apply_preview|install_wasm|wasm_store|not_executed",
            "future_apply_preview|register_child|user_hub|not_executed",
            "future_apply_preview|register_child|wasm_store|not_executed",
            "future_apply_preview|register_root|root|not_executed",
            "future_apply_preview|verify_topology|demo-local|not_executed",
        ],
    );
    assert!(
        json["assumptions"]
            .as_array()
            .expect("assumptions")
            .iter()
            .all(|item| item["code"] != "local_state_root_canister_id")
    );
}

#[test]
fn deploy_plan_report_records_verified_installed_root_fact() {
    let (_temp, workspace_root, icp_root) = temp_plan_workspace("canic-deploy-plan-root-fact");
    write_artifact(&icp_root, "root", b"root-artifact");
    write_install_state(
        &icp_root,
        "local",
        sample_install_state("demo-local", "aaaaa-aa"),
    );
    let options = deploy_plan::DeployPlanOptions::parse([
        OsString::from("demo-local"),
        OsString::from("--config"),
        OsString::from("fleets/demo/canic.toml"),
    ])
    .expect("parse deploy plan options");

    let report = deploy_plan::build_report(
        &options,
        &deploy_plan::DeployPlanRoots {
            workspace_root,
            icp_root,
        },
    );
    let json = serde_json::to_value(&report).expect("report should serialize");

    assert_eq!(json["comparison_status"], "compared_with_warnings");
    assert_eq!(
        json["plan"]["trust_domain"]["root_trust_anchor"],
        "aaaaa-aa"
    );
    assert!(
        json["verified_facts"]
            .as_array()
            .expect("verified facts")
            .iter()
            .any(|item| item["code"] == "installed_root_canister_id_resolved"
                && item["source"] == "installed_deployment")
    );
    assert_verified_fact(
        &json,
        "root_trust_anchor_resolved",
        "demo-local",
        "installed_deployment",
    );
    assert!(
        json["warnings"]
            .as_array()
            .expect("warnings")
            .iter()
            .all(|item| item["code"] != "observed_inventory_unavailable")
    );
    assert!(
        json["proposed_operations"]
            .as_array()
            .expect("proposed operations")
            .iter()
            .any(|item| item["label"] == "upgrade_wasm" && item["subject"] == "root")
    );
}

#[test]
fn deploy_plan_report_marks_complete_installed_state_as_compared() {
    let (_temp, workspace_root, icp_root) = temp_plan_workspace("canic-deploy-plan-compared");
    write_complete_local_plan_inputs(&icp_root);
    write_install_state(
        &icp_root,
        "local",
        sample_install_state("demo-local", "aaaaa-aa"),
    );
    let options = deploy_plan::DeployPlanOptions::parse([
        OsString::from("demo-local"),
        OsString::from("--config"),
        OsString::from("fleets/demo/canic.toml"),
    ])
    .expect("parse deploy plan options");

    let report = deploy_plan::build_report(
        &options,
        &deploy_plan::DeployPlanRoots {
            workspace_root,
            icp_root,
        },
    );
    let json = serde_json::to_value(&report).expect("report should serialize");

    assert_eq!(json["status"], "planned");
    assert_eq!(json["comparison_status"], "compared");
    assert_eq!(json["blockers"], JsonValue::Array(vec![]));
    assert_eq!(json["warnings"], JsonValue::Array(vec![]));
    assert_eq!(json["assumptions"], JsonValue::Array(vec![]));
    assert_verified_fact(
        &json,
        "artifact_set_resolved",
        "demo-local",
        "deployment_plan_builder",
    );
    assert_verified_fact(
        &json,
        "deployment_manifest_resolved",
        "demo-local",
        "deployment_plan_builder",
    );
    assert_verified_fact(&json, "role_artifact_observed", "root", "local_observation");
    assert_verified_fact(
        &json,
        "role_artifact_observed",
        "user_hub",
        "local_observation",
    );
    assert_verified_fact(
        &json,
        "role_artifact_observed",
        "wasm_store",
        "local_observation",
    );
}

#[test]
fn deploy_plan_report_previews_pool_canister_creation() {
    let (_temp, workspace_root, icp_root) =
        temp_plan_workspace_with_config("canic-deploy-plan-pool-preview", POOL_CONFIG);
    write_artifact(&icp_root, "root", b"root-artifact");
    write_artifact(&icp_root, "user_hub", b"user-hub-artifact");
    write_artifact(&icp_root, "user_shard", b"user-shard-artifact");
    let options = deploy_plan::DeployPlanOptions::parse([
        OsString::from("demo-local"),
        OsString::from("--config"),
        OsString::from("fleets/demo/canic.toml"),
    ])
    .expect("parse deploy plan options");

    let report = deploy_plan::build_report(
        &options,
        &deploy_plan::DeployPlanRoots {
            workspace_root,
            icp_root,
        },
    );
    let json = serde_json::to_value(&report).expect("report should serialize");

    assert_eq!(json["plan"]["expected_pool"][0]["pool"], "user_shards");
    assert_eq!(json["plan"]["expected_pool"][0]["role"], "user_shard");
    assert_verified_fact(
        &json,
        "expected_pool_inventory_resolved",
        "demo-local",
        "deployment_plan_builder",
    );
    assert_proposed_operation(&json, "create_canister", "user_shards:user_shard");
    assert_proposed_operation(&json, "register_child", "user_shards:user_shard");
}

#[test]
fn deploy_plan_report_previews_controller_reconciliation() {
    let (_temp, workspace_root, icp_root) =
        temp_plan_workspace_with_config("canic-deploy-plan-controller-preview", CONTROLLER_CONFIG);
    write_artifact(&icp_root, "root", b"root-artifact");
    write_artifact(&icp_root, "user_hub", b"user-hub-artifact");
    let options = deploy_plan::DeployPlanOptions::parse([
        OsString::from("demo-local"),
        OsString::from("--config"),
        OsString::from("fleets/demo/canic.toml"),
    ])
    .expect("parse deploy plan options");

    let report = deploy_plan::build_report(
        &options,
        &deploy_plan::DeployPlanRoots {
            workspace_root,
            icp_root,
        },
    );
    let json = serde_json::to_value(&report).expect("report should serialize");

    assert_eq!(
        json["plan"]["authority_profile"]["expected_controllers"],
        serde_json::json!([
            "aaaaa-aa",
            "zbf4m-zw3nk-6owqc-qmluz-xhwxt-2pkky-xhjy2-kqxor-qzxsn-6d2bz-nae"
        ])
    );
    assert_verified_fact(
        &json,
        "expected_controller_set_resolved",
        "demo-local",
        "deployment_plan_builder",
    );
    assert!(
        json["proposed_operations"]
            .as_array()
            .expect("proposed operations")
            .iter()
            .any(|item| {
                item["phase"] == "future_apply_preview"
                    && item["label"] == "set_controllers"
                    && item["subject"] == "demo-local"
                    && item["status"] == "not_executed"
            })
    );
}

#[test]
fn deploy_plan_report_blocks_unverified_installed_root_state() {
    let (_temp, workspace_root, icp_root) =
        temp_plan_workspace("canic-deploy-plan-unverified-root");
    let mut state = sample_install_state("demo-local", "aaaaa-aa");
    state.root_verification = RootVerificationStatus::NotVerified;
    write_install_state(&icp_root, "local", state);
    let options = deploy_plan::DeployPlanOptions::parse([
        OsString::from("demo-local"),
        OsString::from("--config"),
        OsString::from("fleets/demo/canic.toml"),
    ])
    .expect("parse deploy plan options");

    let report = deploy_plan::build_report(
        &options,
        &deploy_plan::DeployPlanRoots {
            workspace_root,
            icp_root,
        },
    );
    let json = serde_json::to_value(&report).expect("report should serialize");

    assert_eq!(json["status"], "blocked");
    assert_eq!(json["comparison_status"], "not_requested");
    assert!(
        json["blockers"]
            .as_array()
            .expect("blockers")
            .iter()
            .any(
                |item| item["code"] == "local_state_unverified_root_canister_id"
                    && item["category"] == "observation"
            )
    );
    assert!(
        json["verified_facts"]
            .as_array()
            .expect("verified facts")
            .iter()
            .all(|item| item["code"] != "installed_root_canister_id_resolved")
    );
    assert!(
        json["warnings"]
            .as_array()
            .expect("warnings")
            .iter()
            .all(|item| item["code"] != "local_state_unverified_root_canister_id")
    );
}

#[test]
fn deploy_plan_report_marks_installed_network_mismatch_as_drift() {
    let (_temp, workspace_root, icp_root) = temp_plan_workspace("canic-deploy-plan-network-drift");
    let mut state = sample_install_state("demo-local", "aaaaa-aa");
    state.network = "mainnet".to_string();
    write_install_state(&icp_root, "local", state);
    let options = deploy_plan::DeployPlanOptions::parse([
        OsString::from("demo-local"),
        OsString::from("--config"),
        OsString::from("fleets/demo/canic.toml"),
    ])
    .expect("parse deploy plan options");

    let report = deploy_plan::build_report(
        &options,
        &deploy_plan::DeployPlanRoots {
            workspace_root,
            icp_root,
        },
    );
    let json = serde_json::to_value(&report).expect("report should serialize");

    assert_eq!(json["status"], "warning");
    assert_eq!(json["comparison_status"], "compared_with_drift");
    assert!(
        json["warnings"]
            .as_array()
            .expect("warnings")
            .iter()
            .any(|item| item["code"] == "observed_inventory_drift")
    );
    assert!(
        json["warnings"]
            .as_array()
            .expect("warnings")
            .iter()
            .all(|item| item["code"] != "observed_inventory_unavailable")
    );
}

#[test]
fn deploy_plan_report_blocks_unresolved_config_target() {
    let temp = TempDir::new("canic-deploy-plan-missing-config");
    let workspace_root = temp.join("workspace");
    let icp_root = temp.join("icp");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    fs::create_dir_all(&icp_root).expect("create icp root");
    let options = deploy_plan::DeployPlanOptions::parse([OsString::from("missing")])
        .expect("parse deploy plan options");

    let report = deploy_plan::build_report(
        &options,
        &deploy_plan::DeployPlanRoots {
            workspace_root,
            icp_root,
        },
    );
    let json = serde_json::to_value(&report).expect("report should serialize");

    assert_eq!(json["status"], "blocked");
    assert_eq!(json["comparison_status"], "not_requested");
    assert_eq!(json["blockers"][0]["code"], "deployment_target_unresolved");
    assert_eq!(json["verified_facts"], JsonValue::Array(vec![]));
    assert!(matches!(
        deploy_plan::command_exit_result(&report),
        Err(DeployCommandError::PlanBlocked(_))
    ));
}

#[test]
fn deploy_plan_report_blocks_invalid_deployment_target_name() {
    let (_temp, workspace_root, icp_root) = temp_plan_workspace("canic-deploy-plan-invalid-target");
    let options = deploy_plan::DeployPlanOptions::parse([
        OsString::from("demo/local"),
        OsString::from("--config"),
        OsString::from("fleets/demo/canic.toml"),
    ])
    .expect("parse deploy plan options");

    let report = deploy_plan::build_report(
        &options,
        &deploy_plan::DeployPlanRoots {
            workspace_root,
            icp_root,
        },
    );
    let json = serde_json::to_value(&report).expect("report should serialize");

    assert_eq!(json["status"], "blocked");
    assert_eq!(json["comparison_status"], "not_requested");
    assert_eq!(json["blockers"][0]["code"], "deployment_target_invalid");
    assert_eq!(json["blockers"][0]["source"], "cli_arg");
    assert_eq!(json["verified_facts"], JsonValue::Array(vec![]));
}

#[test]
fn deploy_plan_report_blocks_malformed_desired_config() {
    let (temp, workspace_root, icp_root) = temp_plan_workspace_with_config(
        "canic-deploy-plan-malformed-config",
        MALFORMED_DESIRED_CONFIG,
    );
    let options = deploy_plan::DeployPlanOptions::parse([
        OsString::from("demo-local"),
        OsString::from("--config"),
        OsString::from("fleets/demo/canic.toml"),
    ])
    .expect("parse deploy plan options");

    let report = deploy_plan::build_report(
        &options,
        &deploy_plan::DeployPlanRoots {
            workspace_root,
            icp_root,
        },
    );
    let json = serde_json::to_value(&report).expect("report should serialize");

    assert_eq!(json["status"], "blocked");
    assert_eq!(json["comparison_status"], "not_requested");
    assert!(
        json["verified_facts"]
            .as_array()
            .expect("verified facts")
            .iter()
            .any(|item| item["code"] == "deployment_target_resolved")
    );
    assert_no_verified_fact(&json, "authority_profile_resolved");
    assert_no_verified_fact(&json, "expected_controller_set_resolved");
    assert_no_verified_fact(&json, "expected_canister_inventory_resolved");
    assert!(
        json["blockers"]
            .as_array()
            .expect("blockers")
            .iter()
            .any(|item| item["code"] == "local_config_controllers")
    );
    assert!(
        json["assumptions"]
            .as_array()
            .expect("assumptions")
            .iter()
            .all(|item| !item["code"]
                .as_str()
                .unwrap_or_default()
                .starts_with("local_config_"))
    );
    assert!(matches!(
        deploy_plan::command_exit_result(&report),
        Err(DeployCommandError::PlanBlocked(_))
    ));

    drop(temp);
}

#[test]
fn deploy_plan_json_out_is_create_new_and_json_only() {
    let (_temp, workspace_root, icp_root) = temp_plan_workspace("canic-deploy-plan-out");
    let out = workspace_root.join("reports").join("deployment-plan.json");
    fs::create_dir_all(out.parent().expect("report parent")).expect("create report parent");
    let options = deploy_plan::DeployPlanOptions::parse([
        OsString::from("demo-local"),
        OsString::from("--config"),
        OsString::from("fleets/demo/canic.toml"),
        OsString::from("--out"),
        OsString::from(out.as_os_str()),
    ])
    .expect("parse deploy plan options");
    let report = deploy_plan::build_report(
        &options,
        &deploy_plan::DeployPlanRoots {
            workspace_root,
            icp_root,
        },
    );

    deploy_plan::write_report(&options, &report).expect("write report");
    let written = fs::read_to_string(&out).expect("read report");
    let json: JsonValue = serde_json::from_str(&written).expect("out should be json");
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["command"], "canic deploy plan");
    assert_eq!(
        written,
        format!(
            "{}\n",
            deploy_plan::render_json(&report).expect("render report json")
        )
    );
    assert!(!written.contains("Deployment plan"));
    assert!(!written.contains("status:"));

    let err = deploy_plan::write_report(&options, &report)
        .expect_err("--out must not overwrite an existing report");
    assert!(matches!(err, DeployCommandError::PlanOutput(_)));
    assert_eq!(err.exit_code(), 2);
    assert!(err.to_string().contains("File exists") || err.to_string().contains("exists"));
}

#[test]
fn deploy_plan_out_does_not_create_parent_directories() {
    let (_temp, workspace_root, icp_root) = temp_plan_workspace("canic-deploy-plan-out-parent");
    let report_dir = workspace_root.join("missing-reports");
    let out = report_dir.join("deployment-plan.json");
    let options = deploy_plan::DeployPlanOptions::parse([
        OsString::from("demo-local"),
        OsString::from("--config"),
        OsString::from("fleets/demo/canic.toml"),
        OsString::from("--out"),
        OsString::from(out.as_os_str()),
    ])
    .expect("parse deploy plan options");
    let report = deploy_plan::build_report(
        &options,
        &deploy_plan::DeployPlanRoots {
            workspace_root,
            icp_root,
        },
    );

    let err = deploy_plan::write_report(&options, &report)
        .expect_err("--out must not create parent directories");
    assert!(matches!(err, DeployCommandError::PlanOutput(_)));
    assert_eq!(err.exit_code(), 2);
    assert!(!report_dir.exists());
}

#[test]
fn deploy_plan_json_renderer_is_report_only() {
    let (_temp, workspace_root, icp_root) = temp_plan_workspace("canic-deploy-plan-json-render");
    let options = deploy_plan::DeployPlanOptions::parse([
        OsString::from("demo-local"),
        OsString::from("--config"),
        OsString::from("fleets/demo/canic.toml"),
    ])
    .expect("parse deploy plan options");
    let report = deploy_plan::build_report(
        &options,
        &deploy_plan::DeployPlanRoots {
            workspace_root,
            icp_root,
        },
    );

    let json = deploy_plan::render_json(&report).expect("render report json");
    let parsed: JsonValue = serde_json::from_str(&json).expect("json payload should parse");

    assert_eq!(parsed["schema_version"], 1);
    assert_eq!(parsed["command"], "canic deploy plan");
    assert!(!json.contains("Deployment plan"));
    assert!(!json.contains("next actions"));
    assert!(!json.contains("ready_to_apply"));
    assert!(!json.contains("deployment is safe"));
}

#[test]
fn deploy_plan_json_renderer_uses_contract_field_order() {
    let (_temp, workspace_root, icp_root) = temp_plan_workspace("canic-deploy-plan-json-order");
    let options = deploy_plan::DeployPlanOptions::parse([
        OsString::from("demo-local"),
        OsString::from("--config"),
        OsString::from("fleets/demo/canic.toml"),
    ])
    .expect("parse deploy plan options");
    let report = deploy_plan::build_report(
        &options,
        &deploy_plan::DeployPlanRoots {
            workspace_root,
            icp_root,
        },
    );

    let json = deploy_plan::render_json(&report).expect("render report json");

    assert_top_level_json_field_order(
        &json,
        &[
            "schema_version",
            "command",
            "target",
            "network",
            "build_profile",
            "config_path",
            "status",
            "comparison_status",
            "plan",
            "blockers",
            "warnings",
            "assumptions",
            "verified_facts",
            "proposed_operations",
            "next_actions",
        ],
    );
}

#[test]
fn deploy_plan_text_avoids_apply_safety_claims() {
    let (_temp, workspace_root, icp_root) = temp_plan_workspace("canic-deploy-plan-text");
    let options = deploy_plan::DeployPlanOptions::parse([
        OsString::from("demo-local"),
        OsString::from("--config"),
        OsString::from("fleets/demo/canic.toml"),
    ])
    .expect("parse deploy plan options");
    let report = deploy_plan::build_report(
        &options,
        &deploy_plan::DeployPlanRoots {
            workspace_root,
            icp_root,
        },
    );
    let text = deploy_plan::render_text(&report);

    assert!(text.contains("Deployment plan"));
    assert!(text.contains("schema_version: 1"));
    assert!(text.contains("command: canic deploy plan"));
    assert!(text.contains("future apply preview"));
    assert!(text.contains("label: verify_topology subject: demo-local status: not_executed"));
    assert!(text.contains("source: fleet_config"));
    assert!(text.contains("source: deployment_plan_builder"));
    assert!(text.contains("source: installed_deployment"));
    assert!(!text.contains("ready_to_apply"));
    assert!(!text.contains("deployment is safe"));
    assert!(!text.contains("will create"));
    assert!(!text.contains("will install"));
}

fn temp_plan_workspace(prefix: &str) -> (TempDir, PathBuf, PathBuf) {
    temp_plan_workspace_with_config(prefix, SAMPLE_CONFIG)
}

fn temp_plan_workspace_with_config(prefix: &str, config: &str) -> (TempDir, PathBuf, PathBuf) {
    let temp = TempDir::new(prefix);
    let workspace_root = temp.join("workspace");
    let icp_root = temp.join("icp");
    let config_dir = workspace_root.join("fleets").join("demo");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::create_dir_all(&icp_root).expect("create icp root");
    fs::write(config_dir.join("canic.toml"), config).expect("write config");
    (temp, workspace_root, icp_root)
}

fn write_install_state(icp_root: &std::path::Path, network: &str, state: InstallState) {
    let path = icp_root
        .join(".canic")
        .join(network)
        .join("deployments")
        .join(format!("{}.json", state.deployment_name));
    fs::create_dir_all(path.parent().expect("state parent")).expect("create state dir");
    fs::write(
        path,
        serde_json::to_vec_pretty(&state).expect("encode install state"),
    )
    .expect("write install state");
}

fn write_artifact(icp_root: &std::path::Path, role: &str, bytes: &[u8]) {
    let path = icp_root
        .join(".icp")
        .join("local")
        .join("canisters")
        .join(role)
        .join(format!("{role}.wasm.gz"));
    fs::create_dir_all(path.parent().expect("artifact parent")).expect("create artifact dir");
    fs::write(path, bytes).expect("write artifact");
}

fn write_complete_local_plan_inputs(icp_root: &std::path::Path) {
    write_artifact(icp_root, "root", b"root-artifact");
    write_artifact(icp_root, "wasm_store", b"wasm-store-artifact");
    write_artifact(icp_root, "user_hub", b"user-hub-artifact");
    write_release_set_manifest(icp_root);
}

fn write_release_set_manifest(icp_root: &std::path::Path) {
    let path = icp_root
        .join(".icp")
        .join("local")
        .join("canisters")
        .join("root")
        .join("root.release-set.json");
    let manifest = serde_json::json!({
        "release_version": "0.79.0",
        "entries": [{
            "role": "user_hub",
            "template_id": "embedded:user_hub",
            "artifact_relative_path": ".icp/local/canisters/user_hub/user_hub.wasm.gz",
            "payload_size_bytes": 17,
            "payload_sha256_hex": "user-hub-hash",
            "chunk_size_bytes": 1_048_576,
            "chunk_sha256_hex": ["user-hub-hash"]
        }]
    });
    fs::create_dir_all(path.parent().expect("manifest parent")).expect("create manifest dir");
    fs::write(
        path,
        serde_json::to_vec_pretty(&manifest).expect("encode manifest"),
    )
    .expect("write manifest");
}

fn sample_install_state(deployment_name: &str, root_canister_id: &str) -> InstallState {
    InstallState {
        schema_version: 2,
        deployment_name: deployment_name.to_string(),
        fleet_template: "demo".to_string(),
        created_at_unix_secs: 1,
        updated_at_unix_secs: 1,
        network: "local".to_string(),
        root_target: "root".to_string(),
        root_canister_id: root_canister_id.to_string(),
        root_verification: RootVerificationStatus::Verified,
        root_build_target: "root".to_string(),
        workspace_root: "/workspace".to_string(),
        icp_root: "/workspace".to_string(),
        config_path: "fleets/demo/canic.toml".to_string(),
        release_set_manifest_path: ".icp/local/canisters/root/release-set.json".to_string(),
    }
}

fn assert_verified_fact(report: &JsonValue, code: &str, subject: &str, source: &str) {
    assert!(
        report["verified_facts"]
            .as_array()
            .expect("verified facts")
            .iter()
            .any(|item| {
                item["code"] == code && item["subject"] == subject && item["source"] == source
            }),
        "missing verified fact {code} for {subject} from {source}: {:#}",
        report["verified_facts"]
    );
}

fn assert_no_verified_fact(report: &JsonValue, code: &str) {
    assert!(
        report["verified_facts"]
            .as_array()
            .expect("verified facts")
            .iter()
            .all(|item| item["code"] != code),
        "unexpected verified fact {code}: {:#}",
        report["verified_facts"]
    );
}

fn assert_proposed_operation(report: &JsonValue, label: &str, subject: &str) {
    assert!(
        report["proposed_operations"]
            .as_array()
            .expect("proposed operations")
            .iter()
            .any(|item| {
                item["phase"] == "future_apply_preview"
                    && item["label"] == label
                    && item["subject"] == subject
                    && item["status"] == "not_executed"
            }),
        "missing proposed operation {label} for {subject}: {:#}",
        report["proposed_operations"]
    );
}

fn assert_proposed_operation_keys(report: &JsonValue, expected: &[&str]) {
    let actual = report["proposed_operations"]
        .as_array()
        .expect("proposed operations")
        .iter()
        .map(proposed_operation_key)
        .collect::<Vec<_>>();

    assert_eq!(actual, expected, "proposed operation keys");
}

fn proposed_operation_key(item: &JsonValue) -> String {
    format!(
        "{}|{}|{}|{}",
        item["phase"].as_str().unwrap_or_default(),
        item["label"].as_str().unwrap_or_default(),
        item["subject"].as_str().unwrap_or_default(),
        item["status"].as_str().unwrap_or_default()
    )
}

fn assert_base_plan_verified_facts(report: &JsonValue) {
    assert_no_verified_fact(report, "artifact_set_resolved");
    for (code, subject, source) in [
        (
            "authority_profile_resolved",
            "demo-local",
            "deployment_plan_builder",
        ),
        (
            "canonical_runtime_config_resolved",
            "demo-local",
            "deployment_config",
        ),
        ("deployment_target_resolved", "demo-local", "fleet_config"),
        (
            "expected_controller_set_resolved",
            "demo-local",
            "deployment_plan_builder",
        ),
        (
            "expected_canister_inventory_resolved",
            "demo-local",
            "deployment_plan_builder",
        ),
        (
            "expected_role_artifact_inventory_resolved",
            "demo-local",
            "deployment_plan_builder",
        ),
        (
            "expected_pool_inventory_resolved",
            "demo-local",
            "deployment_plan_builder",
        ),
        ("fleet_template_resolved", "demo-local", "fleet_config"),
        (
            "pool_identity_set_resolved",
            "demo-local",
            "deployment_plan_builder",
        ),
        ("role_artifact_observed", "root", "local_observation"),
        (
            "role_topology_resolved",
            "demo-local",
            "deployment_plan_builder",
        ),
    ] {
        assert_verified_fact(report, code, subject, source);
    }
}

fn assert_top_level_json_field_order(json: &str, fields: &[&str]) {
    let mut last = 0;
    for field in fields {
        let pattern = format!("\n  \"{field}\"");
        let position = json
            .find(&pattern)
            .unwrap_or_else(|| panic!("missing top-level JSON field {field}: {json}"));
        assert!(
            position >= last,
            "top-level JSON field {field} appeared out of order"
        );
        last = position;
    }
}
