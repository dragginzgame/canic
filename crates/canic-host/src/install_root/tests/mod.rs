use super::build_environment::resolve_install_build_context;
use super::build_snapshot::InstallBuildTarget;
use super::commands::{
    add_create_root_target, add_icp_environment_target, icp_canister_command_in_network,
    root_init_args,
};
use super::config_selection::{
    config_selection_error, discover_canic_config_choices, discover_project_canic_config_choices,
    resolve_install_config_path,
};
use super::current_execution::{
    current_install_execution_context, current_install_executor_missing_capabilities,
};
use super::deployment_truth_gate::{
    enforce_install_deployment_truth_gate, install_deployment_truth_gate_lines,
    install_deployment_truth_gate_receipt,
};
use super::execution_preflight::write_current_install_execution_preflight_receipt;
use super::operations::{
    BuildInstallTargetsOperation, EmitRootManifestOperation, EnsureRootCyclesOperation,
    InstallPhaseLabel, InstallPhaseOperation, InstallRootWasmOperation,
    ResolveRootCanisterOperation, ResumeBootstrapOperation, WaitRootReadyOperation,
};
use super::output::render_install_timing_summary;
use super::phase_receipts::{
    CompletedInstallPhase, InstallReceiptScope, install_deployment_truth_phase_receipt,
    write_completed_install_phase_receipt,
};
use super::plan_artifacts::validate_plan_artifacts_with_phase;
use super::receipt_io::{
    install_deployment_truth_receipt_path, write_install_deployment_truth_receipt,
};
use super::root_cycles::add_local_root_create_cycles_arg;
use super::root_verification::write_verified_root_state_if_unchanged;
use super::staging::{StageReleaseSetOperation, current_install_staging_evidence};
use super::state::{
    INSTALL_STATE_SCHEMA_VERSION, InstallStateError, deployment_install_state_path,
    read_deployment_install_state, validate_network_name, validate_state_name, write_install_state,
};
use super::timing::InstallTimingSummary;
use super::truth_check::{current_install_deployment_truth_check_at, validate_expected_fleet_name};
use super::{
    InstallRootBlockKind, InstallRootBlockedError, InstallRootOptions, InstallState,
    RegisterDeploymentStateOptions, RootVerificationStatus, VerifyDeploymentRootOptions,
    check_install_deployment_truth, check_install_execution_preflight,
    latest_deployment_truth_receipt_path_from_root, register_deployment_state,
    verify_registered_deployment_root, write_artifact_promotion_execution_receipt_for_install,
    write_install_state_with_deployment_truth_receipt,
};
use crate::canister_build::{
    CanisterArtifactBuildSpec, CanisterBuildProfile, WorkspaceBuildContext,
};
use crate::deployment_truth::{
    ArtifactPromotionExecutionReceiptV1, ArtifactPromotionPlanRequest, ArtifactPromotionPlanV1,
    CanisterControlClassV1, DeploymentCheckV1, DeploymentExecutionContextV1,
    DeploymentExecutionPreflightStatusV1, DeploymentExecutionStatusV1, DeploymentExecutorBackendV1,
    DeploymentExecutorCapabilityV1, DeploymentReceiptV1, DeploymentRootObservationSourceV1,
    ObservationStatusV1, ObservedCanisterV1, PromotionArtifactIdentityReportRequest,
    PromotionArtifactLevelV1, PromotionPlanTransformRequest, RoleArtifactSourceKindV1,
    RoleArtifactSourceV1, RolePromotionInputV1, SafetyFindingV1, SafetySeverityV1, SafetyStatusV1,
    artifact_gate_phase_receipt, artifact_gate_role_phase_receipts, artifact_promotion_plan,
    compare_plan_to_inventory, promoted_deployment_plan_transform_from_inputs,
    promotion_artifact_identity_report_from_inputs, promotion_readiness_from_inputs,
    safety_report_from_diff, validate_deployment_root_verification_receipt,
};
use crate::icp::LocalReplicaTarget;
use crate::release_set::{ReleaseSetEntry, RootReleaseSetBuildSnapshot, RootReleaseSetManifest};
use crate::test_support::temp_dir;
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

mod commands;
mod config_selection;
mod install_truth;
mod state_root_verification;

#[test]
fn named_ic_environment_is_explicit_for_cargo_builds() {
    let root = temp_dir("canic-install-build-environment");
    fs::create_dir_all(&root).expect("create root");
    fs::write(
        root.join("icp.yaml"),
        "environments:\n  - name: staging\n    network: ic\n",
    )
    .expect("write icp yaml");

    let context = resolve_install_build_context(
        &root,
        &root,
        &root.join("canic.toml"),
        "staging",
        "root",
        Some(CanisterBuildProfile::Fast),
    )
    .expect("resolve build context");
    let mut command = std::process::Command::new("cargo");
    context.apply_to_command(&mut command);

    assert_eq!(context.environment, "staging");
    assert_eq!(context.build_network, "ic");
    assert!(command.get_envs().any(|(key, value)| {
        key == "ICP_ENVIRONMENT" && value.is_some_and(|value| value == "ic")
    }));
    fs::remove_dir_all(root).expect("remove temp root");
}

fn source_section<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let start_index = source.find(start).expect("source section start exists");
    let end_index = source[start_index..]
        .find(end)
        .map(|offset| start_index + offset)
        .expect("source section end exists");
    &source[start_index..end_index]
}

fn assert_before(source: &str, before: &str, after: &str) {
    let before_index = source.find(before).expect("before marker exists");
    let after_index = source.find(after).expect("after marker exists");
    assert!(
        before_index < after_index,
        "`{before}` must appear before `{after}`"
    );
}

fn sample_install_state(root: &Path, deployment_name: &str, fleet_template: &str) -> InstallState {
    InstallState {
        schema_version: INSTALL_STATE_SCHEMA_VERSION,
        deployment_name: deployment_name.to_string(),
        fleet_template: fleet_template.to_string(),
        created_at_unix_secs: 42,
        updated_at_unix_secs: 42,
        network: "local".to_string(),
        root_target: "root".to_string(),
        root_canister_id: "uxrrr-q7777-77774-qaaaq-cai".to_string(),
        root_verification: RootVerificationStatus::Verified,
        root_build_target: "root".to_string(),
        workspace_root: root.display().to_string(),
        icp_root: root.display().to_string(),
        config_path: root
            .join(format!("fleets/{fleet_template}/canic.toml"))
            .display()
            .to_string(),
        release_set_manifest_path: root
            .join(".icp/local/canisters/root/root.release-set.json")
            .display()
            .to_string(),
    }
}

fn write_temp_workspace_config(config_source: &str) -> PathBuf {
    let root = temp_dir("canic-install-test");
    fs::create_dir_all(root.join("fleets")).expect("temp fleets dir must be created");
    fs::write(root.join("fleets/canic.toml"), config_source)
        .expect("temp canic.toml must be written");
    root
}

fn demo_config_source(attached: &str) -> String {
    format!(
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

[roles.worker]
kind = "canister"
package = "worker"

[app]
init_mode = "enabled"
[app.whitelist]

{attached}
"#
    )
}

fn local_demo_install_options(root: &Path) -> InstallRootOptions {
    InstallRootOptions {
        root_canister: "root".to_string(),
        root_build_target: "root".to_string(),
        network: "local".to_string(),
        deployment_name: None,
        icp_root: Some(root.to_path_buf()),
        build_profile: Some(CanisterBuildProfile::Fast),
        ready_timeout_seconds: 30,
        config_path: Some("fleets/demo/canic.toml".to_string()),
        expected_fleet: Some("demo".to_string()),
        interactive_config_selection: false,
        deployment_plan_override: None,
        artifact_promotion_plan_override: None,
    }
}

fn write_demo_root_only_config(config_path: &Path) {
    fs::create_dir_all(config_path.parent().expect("config parent")).expect("create config dir");
    fs::write(
        config_path,
        r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"
"#,
    )
    .expect("write config");
}

fn write_wasm_gz_artifact(root: &Path, role: &str, bytes: &[u8]) {
    let path = root
        .join(".icp/local/canisters")
        .join(role)
        .join(format!("{role}.wasm.gz"));
    fs::create_dir_all(path.parent().expect("artifact parent")).expect("create artifact dir");
    fs::write(path, bytes).expect("write artifact");
}

fn demo_install_deployment_truth_check(root_name: &str) -> (PathBuf, DeploymentCheckV1) {
    let root = temp_dir(root_name);
    let config_path = root.join("fleets/demo/canic.toml");
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
    write_wasm_gz_artifact(&root, "root", b"root-artifact");

    let options = InstallRootOptions {
        root_canister: "root".to_string(),
        root_build_target: "root".to_string(),
        network: "local".to_string(),
        deployment_name: None,
        icp_root: Some(root.clone()),
        build_profile: Some(CanisterBuildProfile::Fast),
        ready_timeout_seconds: 30,
        config_path: Some("fleets/demo/canic.toml".to_string()),
        expected_fleet: Some("demo".to_string()),
        interactive_config_selection: false,
        deployment_plan_override: None,
        artifact_promotion_plan_override: None,
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
    (root, check)
}

fn demo_unverified_registered_root_check(root_name: &str) -> (PathBuf, DeploymentCheckV1) {
    let root = temp_dir(root_name);
    let config_path = root.join("fleets/demo/canic.toml");
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
    write_wasm_gz_artifact(&root, "root", b"root-artifact");
    write_wasm_gz_artifact(&root, "wasm_store", b"wasm-store-artifact");
    let mut state = sample_install_state(&root, "demo-local", "demo");
    state.root_verification = RootVerificationStatus::NotVerified;
    write_install_state(&root, "local", &state).expect("write unverified state");

    let check = demo_registered_root_check_from_state(&root);
    (root, check)
}

fn demo_registered_root_check_from_state(root: &Path) -> DeploymentCheckV1 {
    let config_path = root.join("fleets/demo/canic.toml");
    let options = InstallRootOptions {
        root_canister: "root".to_string(),
        root_build_target: "root".to_string(),
        network: "local".to_string(),
        deployment_name: Some("demo-local".to_string()),
        icp_root: Some(root.to_path_buf()),
        build_profile: Some(CanisterBuildProfile::Fast),
        ready_timeout_seconds: 30,
        config_path: Some("fleets/demo/canic.toml".to_string()),
        expected_fleet: Some("demo".to_string()),
        interactive_config_selection: false,
        deployment_plan_override: None,
        artifact_promotion_plan_override: None,
    };
    let mut check = current_install_deployment_truth_check_at(
        &options,
        root,
        root,
        &config_path,
        "demo-local",
        "2026-05-27T00:00:00Z".to_string(),
    )
    .expect("deployment truth check");
    let observed_root = check
        .inventory
        .observed_root
        .as_mut()
        .expect("observed root");
    observed_root.observation_source = DeploymentRootObservationSourceV1::IcpCanisterStatus;
    observed_root.control_class = CanisterControlClassV1::DeploymentControlled;
    observed_root.role_assignment_source = Some("icp_canister_status".to_string());
    for observed_canister in &mut check.inventory.observed_canisters {
        if observed_canister.role.as_deref() == Some("root") {
            observed_canister.control_class = CanisterControlClassV1::DeploymentControlled;
        }
    }
    check.diff = compare_plan_to_inventory(&check.plan, &check.inventory);
    check.report = safety_report_from_diff("report-1", Some("diff-1".to_string()), &check.diff);
    check
}

fn sample_artifact_promotion_plan_for_install(
    check: &DeploymentCheckV1,
) -> ArtifactPromotionPlanV1 {
    let input = sample_role_promotion_input_for_install(check);
    let readiness = promotion_readiness_from_inputs(
        "promotion-readiness-1",
        &check.plan,
        std::slice::from_ref(&input),
    );
    let artifact_identity_report =
        promotion_artifact_identity_report_from_inputs(PromotionArtifactIdentityReportRequest {
            report_id: "promotion-artifact-identity-1".to_string(),
            inputs: vec![input.clone()],
        })
        .expect("sample promotion artifact identity report");
    let transform =
        promoted_deployment_plan_transform_from_inputs(&PromotionPlanTransformRequest {
            promoted_plan_id: check.plan.plan_id.clone(),
            target_plan: check.plan.clone(),
            inputs: vec![input],
        })
        .expect("sample promotion transform");

    artifact_promotion_plan(ArtifactPromotionPlanRequest {
        plan_id: "artifact-promotion-plan-1".to_string(),
        generated_at: "2026-05-26T00:00:00Z".to_string(),
        readiness,
        artifact_identity_report,
        transform,
        target_execution_lineage: None,
    })
    .expect("sample artifact promotion plan")
}

fn sample_role_promotion_input_for_install(check: &DeploymentCheckV1) -> RolePromotionInputV1 {
    let artifact = check
        .plan
        .role_artifacts
        .iter()
        .find(|artifact| artifact.role == "root")
        .expect("root artifact");
    RolePromotionInputV1 {
        role: "root".to_string(),
        promotion_level: PromotionArtifactLevelV1::SealedWasm,
        source: RoleArtifactSourceV1 {
            role: "root".to_string(),
            kind: RoleArtifactSourceKindV1::LocalWasmGz,
            locator: artifact.wasm_gz_path.clone(),
            previous_receipt_kind: None,
            previous_receipt_lineage_digest: None,
            expected_wasm_sha256: artifact.wasm_sha256.clone(),
            expected_wasm_gz_sha256: artifact
                .wasm_gz_sha256
                .clone()
                .or_else(|| artifact.observed_wasm_gz_file_sha256.clone()),
            expected_candid_sha256: artifact.candid_sha256.clone(),
            expected_canonical_embedded_config_sha256: artifact
                .canonical_embedded_config_sha256
                .clone(),
        },
        require_byte_identical_wasm: true,
        require_target_embedded_config: true,
        target_store_has_artifact: Some(true),
    }
}

fn sample_sha256(seed: &str) -> String {
    seed.repeat(64)
}
