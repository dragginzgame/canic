use super::super::install as deploy_install;
use super::fixtures::*;
use super::*;
use canic_host::canister_build::CanisterBuildProfile;

fn install_required_args() -> Vec<OsString> {
    vec![
        OsString::from("demo-local"),
        OsString::from("--plan"),
        OsString::from("promoted-plan.json"),
    ]
}

#[test]
fn deploy_install_command_dispatches_plan_install() {
    let mut args = vec![OsString::from("install")];
    args.extend(install_required_args());
    let parsed = parse_subcommand(deploy_command(), args)
        .expect("parse deploy install")
        .expect("install command");

    assert_eq!(parsed.0, "install");
    assert_eq!(parsed.1, install_required_args());

    let options =
        deploy_install::DeployInstallPlanOptions::parse(parsed.1).expect("parse install plan");
    assert_eq!(options.deployment, "demo-local");
    assert_eq!(options.plan, PathBuf::from("promoted-plan.json"));
}

#[test]
fn deploy_install_plan_builds_current_install_options_with_plan_override() {
    let mut identity = sample_deployment_identity();
    identity.deployment_name = "demo-local".to_string();
    let plan = sample_deployment_plan(identity);
    let input = deploy_install::DeployInstallPlanInput {
        deployment_plan: plan,
    };
    let options = deploy_install::DeployInstallPlanOptions {
        deployment: "demo-local".to_string(),
        plan: PathBuf::from("promoted-plan.json"),
        environment: "local".to_string(),
        profile: Some(CanisterBuildProfile::Fast),
    }
    .into_install_root_options(input, Some(PathBuf::from("/tmp/icp")));

    assert_eq!(options.root_canister, "aaaaa-aa");
    assert_eq!(options.root_build_target, "root");
    assert_eq!(options.environment, "local");
    assert_eq!(options.fleet_name, "demo-local");
    assert_eq!(options.build_profile, Some(CanisterBuildProfile::Fast));
    assert_eq!(options.config_path.as_deref(), Some("apps/demo/canic.toml"));
    assert_eq!(options.expected_app.as_deref(), Some("demo"));
    assert!(options.deployment_plan_override.is_some());
}

#[test]
fn deploy_install_plan_reader_accepts_raw_deployment_plan() {
    let path = temp_json_path("deploy-install-raw-plan.json");
    let plan = sample_deployment_plan(sample_deployment_identity());
    fs::write(&path, serde_json::to_vec(&plan).expect("encode plan")).expect("write plan");

    let decoded = deploy_install::read_plan(&path).expect("decode deployment plan");

    assert_eq!(decoded.deployment_plan.plan_id, "plan-1");
    fs::remove_file(path).expect("clean temp plan");
}

#[test]
fn deploy_install_plan_reader_accepts_ready_promotion_envelope() {
    let path = temp_json_path("deploy-install-ready-promotion-plan.json");
    let plan = sample_artifact_promotion_plan();
    fs::write(&path, serde_json::to_vec(&plan).expect("encode plan")).expect("write plan");

    let decoded = deploy_install::read_plan(&path).expect("decode promotion plan");

    assert_eq!(decoded.deployment_plan.plan_id, "promoted-plan-1");
    fs::remove_file(path).expect("clean temp plan");
}

#[test]
fn deploy_install_plan_reader_rejects_blocked_promotion_envelope() {
    let path = temp_json_path("deploy-install-blocked-promotion-plan.json");
    let plan = sample_blocked_artifact_promotion_plan();
    fs::write(&path, serde_json::to_vec(&plan).expect("encode plan")).expect("write plan");

    let result = deploy_install::read_plan(&path);

    std::assert_matches!(result, Err(DeployCommandError::Blocked(_)));
    fs::remove_file(path).expect("clean temp plan");
}
