use super::super::plan as deploy_plan;
use super::*;
use crate::test_support::TempDir;
use canic_core::{
    CANIC_WASM_CHUNK_BYTES,
    cdk::utils::hash::{sha256_hex, wasm_hash_hex},
    ids::{AppId, CanonicalNetworkId, FleetId},
};
use canic_host::fleet_catalog::FleetCatalogEntryV1;
use serde_json::Value as JsonValue;
use std::{ffi::OsString, fs, path::PathBuf};

const SAMPLE_CONFIG: &str = r#"
[app]
name = "demo"
init_mode = "enabled"


[roles.root]
kind = "root"
package = "root"

[roles.user_hub]
kind = "canister"
package = "user_hub"
[app.whitelist]



[component_specs.user_hub]
component_role = "user_hub"
maximum_instances = 1
"#;

const USER_HUB_ARTIFACT: &[u8] = b"user-hub-artifact";

const MALFORMED_DESIRED_CONFIG: &str = r#"
unknown = true

[app]
name = "demo"
"#;

const POOL_CONFIG: &str = r#"
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
    assert!(help.contains("Deploy commands are read-only"));
    assert!(help.contains("fresh Fleet creation uses `canic install`"));
}

#[test]
fn deploy_plan_help_documents_no_mutation_contract() {
    let help = deploy_plan::usage();

    assert!(help.contains("canic deploy plan <fleet> --app <app>"));
    assert!(help.contains("canic deploy plan demo-local --app demo --out deployment-plan.json"));
    assert!(help.contains("Read-only"));
    assert!(help.contains("deterministic local desired state"));
    assert!(help.contains("without contacting the IC"));
    assert!(help.contains("or authorizing mutation"));
    assert_eq!(help.matches("  canic deploy plan ").count(), 2);
}

#[test]
fn deploy_plan_options_parse_supported_surface() {
    let options = deploy_plan::DeployPlanOptions::parse([
        OsString::from("demo-local"),
        OsString::from("--app"),
        OsString::from("demo"),
        OsString::from("--json"),
        OsString::from("--out"),
        OsString::from("deployment-plan.json"),
        OsString::from("--config"),
        OsString::from("apps/demo/canic.toml"),
        OsString::from("--build-profile"),
        OsString::from("fast"),
        OsString::from(crate::cli::globals::INTERNAL_ENVIRONMENT_OPTION),
        OsString::from("local"),
    ])
    .expect("parse deploy plan options");

    assert_eq!(options.fleet, "demo-local");
    assert_eq!(options.environment, "local");
    assert!(options.json);
    assert_eq!(options.out, Some(PathBuf::from("deployment-plan.json")));
    assert_eq!(options.config, Some(PathBuf::from("apps/demo/canic.toml")));
}

#[test]
fn deploy_plan_options_reject_invalid_app_before_path_resolution() {
    let error = deploy_plan::DeployPlanOptions::parse([
        OsString::from("demo-local"),
        OsString::from("--app"),
        OsString::from("../../sentinel"),
    ])
    .expect_err("invalid App path identity must reject");

    assert!(matches!(error, DeployCommandError::Usage(_)));
    assert!(error.to_string().contains("invalid App name"));
}

#[test]
fn deploy_plan_report_builds_from_config_without_fleet_catalog_entry() {
    let (_temp, workspace_root, icp_root) = temp_plan_workspace("canic-deploy-plan-report");
    write_artifact(&icp_root, "root", b"root-artifact");
    let options = deploy_plan::DeployPlanOptions::parse([
        OsString::from("demo-local"),
        OsString::from("--app"),
        OsString::from("demo"),
        OsString::from("--config"),
        OsString::from("apps/demo/canic.toml"),
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
    assert_eq!(json["fleet"], "demo-local");
    assert_eq!(json["app"], "demo");
    assert_eq!(json["status"], "warning");
    assert_eq!(json["comparison_status"], "not_available");
    assert_eq!(
        json["plan"]["deployment_identity"]["fleet_name"],
        "demo-local"
    );
    assert_eq!(json["plan"]["deployment_identity"]["app"], "demo");
    assert_base_plan_verified_facts(&json);
    assert!(
        json["warnings"]
            .as_array()
            .expect("warnings")
            .iter()
            .any(|item| item["code"] == "observed_inventory_unavailable")
    );
    assert_next_action(
        &json,
        "run canic build or provide a build profile with resolved artifacts",
    );
    assert_proposed_operation_keys(
        &json,
        &[
            "future_apply_preview|create_canister|fleet_coordinator|not_executed",
            "future_apply_preview|create_canister|root|not_executed",
            "future_apply_preview|create_canister|user_hub|not_executed",
            "future_apply_preview|create_canister|wasm_store|not_executed",
            "future_apply_preview|install_wasm|fleet_coordinator|not_executed",
            "future_apply_preview|install_wasm|root|not_executed",
            "future_apply_preview|install_wasm|user_hub|not_executed",
            "future_apply_preview|install_wasm|wasm_store|not_executed",
            "future_apply_preview|register_child|fleet_coordinator|not_executed",
            "future_apply_preview|register_child|user_hub|not_executed",
            "future_apply_preview|register_child|wasm_store|not_executed",
            "future_apply_preview|register_root|root|not_executed",
            "future_apply_preview|upload_artifact|fleet_coordinator|not_executed",
            "future_apply_preview|upload_artifact|root|not_executed",
            "future_apply_preview|upload_artifact|user_hub|not_executed",
            "future_apply_preview|upload_artifact|wasm_store|not_executed",
            "future_apply_preview|verify_topology|demo-local|not_executed",
        ],
    );
}

#[test]
fn deploy_plan_catalog_identity_does_not_invent_one_root_fact() {
    let (_temp, workspace_root, icp_root) =
        temp_plan_workspace("canic-deploy-plan-coordinator-catalog");
    write_artifact(&icp_root, "root", b"root-artifact");
    write_fleet_catalog(
        &icp_root,
        "local",
        sample_fleet_catalog_entry("demo-local", "rrkah-fqaaa-aaaaa-aaaaq-cai"),
    );
    let options = deploy_plan::DeployPlanOptions::parse([
        OsString::from("demo-local"),
        OsString::from("--app"),
        OsString::from("demo"),
        OsString::from("--config"),
        OsString::from("apps/demo/canic.toml"),
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

    assert_eq!(json["comparison_status"], "not_requested");
    assert_eq!(
        json["plan"]["trust_domain"]["root_trust_anchor"],
        JsonValue::Null
    );
    assert!(
        json["verified_facts"]
            .as_array()
            .expect("verified facts")
            .iter()
            .all(|item| item["code"] != "installed_root_canister_id_resolved")
    );
    assert!(
        json["verified_facts"]
            .as_array()
            .expect("verified facts")
            .iter()
            .all(|item| item["code"] != "root_trust_anchor_resolved")
    );
}

#[test]
fn deploy_plan_report_keeps_complete_inputs_planned_without_root_comparison() {
    let (_temp, workspace_root, icp_root) = temp_plan_workspace("canic-deploy-plan-compared");
    write_complete_local_plan_inputs(&icp_root);
    write_fleet_catalog(
        &icp_root,
        "local",
        sample_fleet_catalog_entry("demo-local", "rrkah-fqaaa-aaaaa-aaaaq-cai"),
    );
    let options = deploy_plan::DeployPlanOptions::parse([
        OsString::from("demo-local"),
        OsString::from("--app"),
        OsString::from("demo"),
        OsString::from("--config"),
        OsString::from("apps/demo/canic.toml"),
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
        json["status"], "planned",
        "complete local plan unexpectedly emitted diagnostics: {json:#}"
    );
    assert_eq!(json["comparison_status"], "not_requested");
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
    assert_verified_fact(
        &json,
        "role_artifact_observed",
        "fleet_coordinator",
        "local_observation",
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
        OsString::from("--app"),
        OsString::from("demo"),
        OsString::from("--config"),
        OsString::from("apps/demo/canic.toml"),
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
fn deploy_plan_report_blocks_unresolved_config_target() {
    let temp = TempDir::new("canic-deploy-plan-missing-config");
    let workspace_root = temp.join("workspace");
    let icp_root = temp.join("icp");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    fs::create_dir_all(&icp_root).expect("create icp root");
    let options = deploy_plan::DeployPlanOptions::parse([
        OsString::from("missing"),
        OsString::from("--app"),
        OsString::from("demo"),
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
    assert_eq!(json["blockers"][0]["code"], "app_unresolved");
    assert_eq!(json["verified_facts"], JsonValue::Array(vec![]));
    assert!(matches!(
        deploy_plan::command_exit_result(&report),
        Err(DeployCommandError::PlanBlocked(_))
    ));
}

#[test]
fn deploy_plan_report_blocks_invalid_fleet_name() {
    let (_temp, workspace_root, icp_root) = temp_plan_workspace("canic-deploy-plan-invalid-target");
    let options = deploy_plan::DeployPlanOptions::parse([
        OsString::from("demo/local"),
        OsString::from("--app"),
        OsString::from("demo"),
        OsString::from("--config"),
        OsString::from("apps/demo/canic.toml"),
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
    assert_eq!(json["blockers"][0]["code"], "fleet_name_invalid");
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
        OsString::from("--app"),
        OsString::from("demo"),
        OsString::from("--config"),
        OsString::from("apps/demo/canic.toml"),
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
            .any(|item| item["code"] == "fleet_app_resolved")
    );
    assert_verified_fact(
        &json,
        "authority_profile_resolved",
        "demo-local",
        "deployment_plan_builder",
    );
    assert_verified_fact(
        &json,
        "expected_controller_set_resolved",
        "demo-local",
        "deployment_plan_builder",
    );
    assert_no_verified_fact(&json, "expected_canister_inventory_resolved");
    assert!(
        json["blockers"]
            .as_array()
            .expect("blockers")
            .iter()
            .any(|item| item["code"] == "local_config_roles")
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
        OsString::from("--app"),
        OsString::from("demo"),
        OsString::from("--config"),
        OsString::from("apps/demo/canic.toml"),
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
}

#[test]
fn deploy_plan_out_does_not_create_parent_directories() {
    let (_temp, workspace_root, icp_root) = temp_plan_workspace("canic-deploy-plan-out-parent");
    let report_dir = workspace_root.join("missing-reports");
    let out = report_dir.join("deployment-plan.json");
    let options = deploy_plan::DeployPlanOptions::parse([
        OsString::from("demo-local"),
        OsString::from("--app"),
        OsString::from("demo"),
        OsString::from("--config"),
        OsString::from("apps/demo/canic.toml"),
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
        OsString::from("--app"),
        OsString::from("demo"),
        OsString::from("--config"),
        OsString::from("apps/demo/canic.toml"),
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
    assert_no_deploy_plan_safety_claims(&json);
}

#[test]
fn deploy_plan_json_renderer_uses_contract_field_order() {
    let (_temp, workspace_root, icp_root) = temp_plan_workspace("canic-deploy-plan-json-order");
    let options = deploy_plan::DeployPlanOptions::parse([
        OsString::from("demo-local"),
        OsString::from("--app"),
        OsString::from("demo"),
        OsString::from("--config"),
        OsString::from("apps/demo/canic.toml"),
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
            "fleet",
            "app",
            "environment",
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
    write_artifact(&icp_root, "root", b"root-artifact");
    let options = deploy_plan::DeployPlanOptions::parse([
        OsString::from("demo-local"),
        OsString::from("--app"),
        OsString::from("demo"),
        OsString::from("--config"),
        OsString::from("apps/demo/canic.toml"),
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
    assert!(text.contains("future apply preview (proposed operation labels; not executed)"));
    assert!(text.contains(
        "phase: future_apply_preview label: upload_artifact subject: root status: not_executed"
    ));
    assert!(text.contains(
        "phase: future_apply_preview label: verify_topology subject: demo-local status: not_executed"
    ));
    assert!(text.contains("run canic build or provide a build profile with resolved artifacts"));
    assert!(text.contains("source: app_config"));
    assert!(text.contains("source: deployment_plan_builder"));
    assert!(text.contains("source: fleet_catalog"));
    assert_no_deploy_plan_safety_claims(&text);
}

fn temp_plan_workspace(prefix: &str) -> (TempDir, PathBuf, PathBuf) {
    temp_plan_workspace_with_config(prefix, SAMPLE_CONFIG)
}

fn temp_plan_workspace_with_config(prefix: &str, config: &str) -> (TempDir, PathBuf, PathBuf) {
    let temp = TempDir::new(prefix);
    let workspace_root = temp.join("workspace");
    let icp_root = temp.join("icp");
    let config_dir = workspace_root.join("apps").join("demo");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::create_dir_all(&icp_root).expect("create icp root");
    fs::write(config_dir.join("canic.toml"), config).expect("write config");
    (temp, workspace_root, icp_root)
}

fn write_fleet_catalog(
    icp_root: &std::path::Path,
    environment: &str,
    mut fleet: FleetCatalogEntryV1,
) {
    let root_key = test_local_root_key();
    let network = CanonicalNetworkId::from_der_root_trust_anchor(&root_key)
        .expect("canonical local network ID");
    fleet.canonical_network_id = network;
    let authority = icp_root
        .join(".canic")
        .join("networks")
        .join(network.to_string());
    fs::create_dir_all(authority.join("trust")).expect("create network authority");
    fs::write(authority.join("trust/root-key.der"), &root_key).expect("write root key");
    fs::write(
        authority.join("enrollment.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "root_key_digest": sha256_hex(&root_key),
            "enrolled_at": 1,
            "source_profile": environment,
        }))
        .expect("encode enrollment"),
    )
    .expect("write enrollment");
    let profile = icp_root
        .join(".canic")
        .join("environment-profiles")
        .join(environment)
        .join("network.json");
    fs::create_dir_all(profile.parent().expect("profile parent")).expect("create profile dir");
    fs::write(
        profile,
        serde_json::to_vec_pretty(&serde_json::json!({
            "canonical_network_id": network,
        }))
        .expect("encode profile"),
    )
    .expect("write profile");
    let path = icp_root
        .join(".canic")
        .join("networks")
        .join(network.to_string())
        .join("fleets/catalog.json");
    fs::create_dir_all(path.parent().expect("catalog parent")).expect("create catalog dir");
    fs::write(
        path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version": 1,
            "canonical_network_id": network,
            "entries": [fleet],
        }))
        .expect("encode Fleet catalog"),
    )
    .expect("write Fleet catalog");
}

fn test_local_root_key() -> Vec<u8> {
    let mut root_key = vec![
        0x30, 0x81, 0x82, 0x30, 0x1d, 0x06, 0x0d, 0x2b, 0x06, 0x01, 0x04, 0x01, 0x82, 0xdc, 0x7c,
        0x05, 0x03, 0x01, 0x02, 0x01, 0x06, 0x0c, 0x2b, 0x06, 0x01, 0x04, 0x01, 0x82, 0xdc, 0x7c,
        0x05, 0x03, 0x02, 0x01, 0x03, 0x61, 0x00,
    ];
    root_key.extend_from_slice(&[9; 96]);
    root_key
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
    write_artifact(icp_root, "fleet_coordinator", b"fleet-coordinator-artifact");
    write_artifact(icp_root, "root", b"root-artifact");
    write_artifact(icp_root, "wasm_store", b"wasm-store-artifact");
    write_artifact(icp_root, "user_hub", USER_HUB_ARTIFACT);
    write_release_set_manifest(icp_root);
}

fn write_release_set_manifest(icp_root: &std::path::Path) {
    let path = icp_root
        .join(".icp")
        .join("local")
        .join("canisters")
        .join("root")
        .join("root.release-set.json");
    let user_hub_hash = wasm_hash_hex(USER_HUB_ARTIFACT);
    let candid_hash = sha256_hex(b"user-hub-candid");
    let protocol_profile_digest = sha256_hex(b"user-hub-protocol-profile");
    let manifest = serde_json::json!({
        "release_version": "0.79.0",
        "entries": [{
            "role": "user_hub",
            "template_id": "embedded:user_hub",
            "artifact_relative_path": ".icp/local/canisters/user_hub/user_hub.wasm.gz",
            "candid_sha256_hex": candid_hash,
            "protocol_profile_digest_hex": protocol_profile_digest,
            "payload_size_bytes": USER_HUB_ARTIFACT.len(),
            "payload_sha256_hex": user_hub_hash,
            "chunk_size_bytes": CANIC_WASM_CHUNK_BYTES,
            "chunk_sha256_hex": [user_hub_hash]
        }]
    });
    fs::create_dir_all(path.parent().expect("manifest parent")).expect("create manifest dir");
    fs::write(
        path,
        serde_json::to_vec_pretty(&manifest).expect("encode manifest"),
    )
    .expect("write manifest");
}

fn sample_fleet_catalog_entry(
    fleet_name: &str,
    coordinator_principal: &str,
) -> FleetCatalogEntryV1 {
    FleetCatalogEntryV1 {
        canonical_network_id: CanonicalNetworkId::ic_mainnet(),
        fleet_id: FleetId::from_generated_bytes([9; 32]),
        fleet_name: fleet_name.parse().expect("Fleet name"),
        app: AppId::from("demo"),
        environment: "local".to_string(),
        deployed_at_unix_secs: 1,
        release_build_id: "01".repeat(32).parse().expect("release build"),
        coordinator_principal: coordinator_principal.to_string(),
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

fn assert_next_action(report: &JsonValue, expected: &str) {
    assert!(
        report["next_actions"]
            .as_array()
            .expect("next actions")
            .iter()
            .any(|item| item == expected),
        "missing next action {expected}: {:#}",
        report["next_actions"]
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
        ("build_profile_resolved", "demo-local", "build_profile"),
        ("config_path_resolved", "demo-local", "deployment_config"),
        ("fleet_app_resolved", "demo-local", "app_config"),
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
        ("app_resolved", "demo", "app_config"),
        (
            "environment_resolved",
            "demo-local",
            "deployment_plan_builder",
        ),
        (
            "pool_identity_set_resolved",
            "demo-local",
            "deployment_plan_builder",
        ),
        ("plan_id_resolved", "demo-local", "deployment_plan_builder"),
        (
            "planner_version_resolved",
            "demo-local",
            "deployment_plan_builder",
        ),
        ("role_artifact_observed", "root", "local_observation"),
        (
            "role_topology_resolved",
            "demo-local",
            "deployment_plan_builder",
        ),
        (
            "runtime_variant_resolved",
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

fn assert_no_deploy_plan_safety_claims(rendered: &str) {
    for phrase in [
        "DeploymentPlanReport",
        "EvidenceEnvelope",
        "authorization to mutate",
        "deployment is safe",
        "deployment truth",
        "ready to apply",
        "ready_for_apply",
        "ready_to_apply",
        "safe to deploy",
        "safe_to_deploy",
        "will apply",
        "will create",
        "will install",
        "will mutate",
        "will register",
        "will set",
        "will upgrade",
        "will upload",
    ] {
        assert!(
            !rendered.contains(phrase),
            "deploy-plan output must not contain safety/evidence/truth claim {phrase:?}: {rendered}"
        );
    }
}
