use super::build_network::resolve_install_build_context;
use super::build_snapshot::{InstallBuildTarget, resolve_install_snapshot};
use super::commands::{
    icp_canister_command, icp_canister_create_command, icp_canister_install_binary_args_command,
    icp_e8s_text, read_created_canister, write_candid_args,
};
use super::config_selection::{
    config_selection_error, discover_canic_config_choices, discover_workspace_canic_config_choices,
    resolve_install_config_path, select_discovered_app_config_path,
};
use super::current_execution::{
    current_install_execution_context, current_install_execution_context_at_root,
    current_install_executor_missing_capabilities,
};
use super::deployment_truth_gate::{
    deployment_truth_display_summary, deployment_truth_findings_summary,
    enforce_install_deployment_truth_gate, install_deployment_truth_gate_lines,
    install_deployment_truth_gate_receipt,
};
use super::execution_preflight::current_install_execution_preflight_receipt;
use super::icp_context::InstallIcpContext;
use super::operations::{
    BuildInstallTargetsOperation, EmitRootManifestOperation, InstallPhaseLabel,
};
use super::output::render_install_timing_summary;
use super::phase_receipts::{
    CompletedInstallPhase, InstallReceiptScope, install_deployment_truth_phase_receipt,
    write_completed_install_phase_receipt,
};
use super::plan_artifacts::{
    PlanArtifactError, PreparedPlanArtifacts, prepare_plan_artifacts_with_phase,
};
use super::receipt_io::{
    install_deployment_truth_receipt_path, write_install_deployment_truth_receipt,
};
use super::timing::InstallTimingSummary;
use super::truth_check::current_install_deployment_truth_check_at;
use super::{
    InstallRootBlockKind, InstallRootBlockedError, InstallRootError, InstallRootOptions,
    InstallRootPhase, check_install_deployment_truth, check_install_execution_preflight,
    current_install_release_build, install_root, latest_deployment_truth_receipt_path_from_root,
    prepare_current_fresh_fleet_preflight, require_current_release_builder,
    root_registry_synchronization_operation_id, root_store_adoption_operation_id,
    root_store_bootstrap_operation_id,
};
use crate::canister_build::{
    CanisterArtifactBuildSpec, CanisterBuildProfile, WorkspaceBuildContext,
};
use crate::deployment_truth::{
    CanisterControlClassV1, DeploymentCheckV1, DeploymentExecutionContextV1,
    DeploymentExecutionPreflightStatusV1, DeploymentExecutionStatusV1, DeploymentExecutorBackendV1,
    DeploymentExecutorCapabilityV1, DeploymentReceiptV1, ObservationStatusV1, ObservedCanisterV1,
    SafetyFindingV1, SafetySeverityV1, SafetyStatusV1, artifact_gate_phase_receipt,
    artifact_gate_role_phase_receipts, compare_plan_to_inventory, safety_report_from_diff,
};
use crate::icp::LocalReplicaTarget;
use crate::test_support::{temp_dir, write_local_network_authority};
use crate::{
    release_build::{finalize_release_build_from_manifest, plan_release_build_for_profile},
    release_set::RootReleaseSetBuildSnapshot,
};
use canic_core::{
    dto::fleet_activation::FleetActivationIdentity,
    ids::{
        AppId, BuildNetwork, CanonicalNetworkId, FleetBinding, FleetId, FleetKey, ReleaseBuildId,
        ReleaseBuildNonce,
    },
};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

#[test]
fn root_install_phase_operation_ids_are_nonzero_distinct_and_stable() {
    let install_operation_id = [7; 32];
    let operation_ids = [
        root_store_adoption_operation_id(install_operation_id),
        root_store_bootstrap_operation_id(install_operation_id),
        root_registry_synchronization_operation_id(install_operation_id),
    ];
    assert!(
        operation_ids
            .iter()
            .all(|operation_id| *operation_id != [0; 32])
    );
    let unique = operation_ids
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(unique.len(), 3);
    assert_eq!(
        root_store_bootstrap_operation_id(install_operation_id),
        root_store_bootstrap_operation_id(install_operation_id)
    );
}

mod commands;
mod config_selection;
mod install_truth;

#[test]
fn current_install_reuses_the_existing_session_release_build_before_rebuilding() {
    let root = temp_dir("current-install-release-recovery");
    fs::create_dir_all(&root).expect("create temp root");
    fs::write(
        root.join("icp.yaml"),
        "environments:\n  - name: proof\n    network: ic\n",
    )
    .expect("write ICP project");
    let first = current_install_release_build(&root, "proof", "primary", "demo", None, None)
        .expect("plan first release build");
    let release_build_id = first.record.release_build_id;
    let manifest = root.join("release-set.json");
    fs::write(&manifest, [7; 32]).expect("write release-set authority");
    let finalized = finalize_release_build_from_manifest(&root, release_build_id, &manifest)
        .expect("finalize release build");
    super::fleet_install_session::plan_fleet_install_session(
        super::fleet_install_session::PlanFleetInstallSessionRequest {
            root: &root,
            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
            fleet_name: "primary".parse().expect("Fleet name"),
            app: "demo".into(),
            finalized_release_build: &finalized,
            decision_release_build_id: None,
            fresh_fleet_plan_digest:
                "abababababababababababababababababababababababababababababababab",
        },
    )
    .expect("publish Fleet install session");

    let recovered = current_install_release_build(&root, "proof", "primary", "demo", None, None)
        .expect("recover release build before rebuilding");

    assert_eq!(recovered.record, finalized.record);
    assert_eq!(
        fs::read_dir(root.join(".canic/release-builds"))
            .expect("read release-build directory")
            .count(),
        1
    );
    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn current_install_reuses_only_the_requested_finalized_profile() {
    let root = temp_dir("current-install-explicit-release-build");
    fs::create_dir_all(&root).expect("create temp root");
    fs::write(
        root.join("icp.yaml"),
        "environments:\n  - name: proof\n    network: ic\n",
    )
    .expect("write ICP project");
    let planned = plan_release_build_for_profile(&root, CanisterBuildProfile::Fast)
        .expect("plan fast release build");
    let release_build_id = planned.record.release_build_id;
    let manifest = root.join("release-set.json");
    fs::write(&manifest, [8; 32]).expect("write release-set authority");
    let finalized = finalize_release_build_from_manifest(&root, release_build_id, &manifest)
        .expect("finalize release build");

    let selected = current_install_release_build(
        &root,
        "proof",
        "secondary",
        "demo",
        Some(release_build_id),
        None,
    )
    .expect("select finalized release build");

    assert_eq!(selected.record, finalized.record);
    assert!(
        require_current_release_builder("0.0.0").is_err(),
        "a finalized build from a different Canic release must be rejected"
    );
    assert!(
        current_install_release_build(
            &root,
            "proof",
            "secondary",
            "demo",
            Some(release_build_id),
            Some(CanisterBuildProfile::Release),
        )
        .is_err(),
        "a conflicting requested profile must fail before a build"
    );
    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn public_install_error_preserves_phase_and_typed_source() {
    let error = InstallRootError::new(
        InstallRootPhase::Activation,
        std::io::Error::other("activation failed"),
    );

    assert_eq!(error.phase(), InstallRootPhase::Activation);
    assert!(
        std::error::Error::source(&error)
            .and_then(|source| source.downcast_ref::<std::io::Error>())
            .is_some()
    );
}

#[test]
fn invalid_fleet_input_rejects_before_release_build_allocation() {
    let root = temp_dir("current-install-invalid-fleet-input");
    fs::create_dir_all(&root).expect("create temp root");
    write_demo_root_only_config(&root.join("apps/demo/canic.toml"));
    let input_path = root.join("fleet-input.toml");
    fs::write(&input_path, "schema_version = 2\n").expect("write invalid Fleet input");
    let mut options = local_demo_install_options(&root);
    options.fleet_install_input_path = Some(input_path);

    let error = install_root(options).expect_err("invalid Fleet input must reject");

    assert_eq!(error.phase(), InstallRootPhase::Planning);
    assert!(!root.join(".canic/release-builds").exists());
    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn invalid_fresh_fleet_topology_rejects_before_release_build_allocation() {
    let root = temp_dir("current-install-invalid-fleet-topology");
    fs::create_dir_all(&root).expect("create temp root");
    write_demo_root_only_config(&root.join("apps/demo/canic.toml"));
    let input_path = root.join("fleet-input.toml");
    fs::write(&input_path, invalid_root_only_fleet_input())
        .expect("write invalid topology Fleet input");
    let mut options = local_demo_install_options(&root);
    options.fleet_install_input_path = Some(input_path);

    let error = install_root(options).expect_err("unknown Component admission must reject");

    assert_eq!(error.phase(), InstallRootPhase::Planning);
    assert!(!root.join(".canic/release-builds").exists());
    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn expected_plan_digest_mismatch_rejects_before_release_build_allocation() {
    let root = temp_dir("current-install-plan-digest-mismatch");
    fs::create_dir_all(&root).expect("create temp root");
    write_demo_single_component_config(&root.join("apps/demo/canic.toml"));
    let input_path = root.join("fleet-input.toml");
    fs::write(&input_path, valid_single_component_fleet_input()).expect("write valid Fleet input");
    let mut options = local_demo_install_options(&root);
    options.fleet_install_input_path = Some(input_path);
    options.expected_fresh_fleet_plan_digest = Some("00".repeat(32));

    let error = install_root(options).expect_err("changed plan digest must reject");

    assert_eq!(error.phase(), InstallRootPhase::Planning);
    assert!(!root.join(".canic/release-builds").exists());
    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn install_recompiles_the_exact_plan_digest_and_rejects_changed_balance_evidence() {
    let root = temp_dir("current-install-plan-parity");
    fs::create_dir_all(&root).expect("create temp root");
    fs::write(root.join("Cargo.lock"), "# test lock\n").expect("write test Cargo.lock");
    fs::write(root.join("Cargo.toml"), "[workspace]\n").expect("write test Cargo.toml");
    let config_path = root.join("apps/demo/canic.toml");
    write_demo_single_component_config(&config_path);
    let input_path = root.join("fleet-input.toml");
    fs::write(&input_path, valid_single_component_fleet_input()).expect("write valid Fleet input");
    let mut options = local_demo_install_options(&root);
    options.fleet_install_input_path = Some(input_path.clone());

    let first = prepare_current_fresh_fleet_preflight(&root, &root, &config_path, &options)
        .expect("compile first install decision");
    options.expected_fresh_fleet_plan_digest = Some(first.plan.plan_digest.clone());
    let exact = prepare_current_fresh_fleet_preflight(&root, &root, &config_path, &options)
        .expect("exact decision digest should be reusable");
    assert_eq!(exact.plan, first.plan);
    assert!(!root.join(".canic/release-builds").exists());

    let changed_input =
        valid_single_component_fleet_input().replace("cycles = \"100T\"", "cycles = \"101T\"");
    fs::write(&input_path, changed_input).expect("change balance observation");
    let error = prepare_current_fresh_fleet_preflight(&root, &root, &config_path, &options)
        .expect_err("changed balance must change and reject the expected digest");
    assert_eq!(error.phase(), InstallRootPhase::Planning);
    assert!(!root.join(".canic/release-builds").exists());
    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn named_ic_environment_is_explicit_for_cargo_builds() {
    let root = temp_dir("canic-install-build-environment");
    fs::create_dir_all(&root).expect("create root");
    fs::write(
        root.join("icp.yaml"),
        "environments:\n  - name: staging\n    network: ic\n",
    )
    .expect("write icp yaml");

    let icp = InstallIcpContext::new("/opt/icp", &root, "staging");
    let context = resolve_install_build_context(
        &root,
        &root.join("canic.toml"),
        &icp,
        "root",
        Some(CanisterBuildProfile::Fast),
    )
    .expect("resolve build context");
    let mut command = std::process::Command::new("cargo");
    context.apply_to_command(&mut command);

    assert_eq!(context.environment, "staging");
    assert_eq!(context.build_network, BuildNetwork::Ic);
    assert!(command.get_envs().any(|(key, value)| {
        key == "ICP_ENVIRONMENT" && value.is_some_and(|value| value == "ic")
    }));
    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn install_snapshot_accepts_multiple_flat_component_specs() {
    let root = temp_dir("canic-install-multiple-components");
    let config_path = root.join("canic.toml");
    fs::create_dir_all(&root).expect("create root");
    fs::write(
        &config_path,
        r#"
[app]
name = "demo"
init_mode = "enabled"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[roles.worker]
kind = "canister"
package = "worker"

[component_specs.default]
component_role = "app"
maximum_instances = 1

[component_specs.secondary]
component_role = "worker"
maximum_instances = 2
"#,
    )
    .expect("write config");
    let context = WorkspaceBuildContext {
        role: "root".to_string(),
        profile: CanisterBuildProfile::Fast,
        environment: "ic".to_string(),
        build_network: BuildNetwork::Ic,
        workspace_root: root.clone(),
        icp_root: root.clone(),
        config_path,
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: None,
    };

    let snapshot = resolve_install_snapshot(&context, "root", true)
        .expect("deployment-plan install should accept multiple Components");
    assert_eq!(snapshot.app_id, "demo");

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

fn demo_config_source(attached: &str) -> String {
    format!(
        r#"
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

[roles.worker]
kind = "canister"
package = "worker"
[app.whitelist]

{attached}
"#
    )
}

fn local_demo_install_options(root: &Path) -> InstallRootOptions {
    write_local_network_authority(root, "local");
    InstallRootOptions {
        root_canister: "root".to_string(),
        root_build_target: "root".to_string(),
        icp_executable: "icp".to_string(),
        environment: "local".to_string(),
        fleet_name: "demo".to_string(),
        icp_root: Some(root.to_path_buf()),
        build_profile: Some(CanisterBuildProfile::Fast),
        release_build_id: None,
        config_path: Some("apps/demo/canic.toml".to_string()),
        fleet_install_input_path: None,
        expected_fresh_fleet_plan_digest: None,
        admitted_fresh_fleet_plan_digest: None,
        expected_app: Some("demo".to_string()),
        interactive_config_selection: false,
        deployment_plan_override: None,
    }
}

fn write_demo_root_only_config(config_path: &Path) {
    fs::create_dir_all(config_path.parent().expect("config parent")).expect("create config dir");
    fs::write(
        config_path,
        r#"
[app]
name = "demo"
init_mode = "enabled"


[roles.root]
kind = "root"
package = "root"
[app.whitelist]


"#,
    )
    .expect("write config");
}

fn write_demo_single_component_config(config_path: &Path) {
    fs::create_dir_all(config_path.parent().expect("config parent")).expect("create config dir");
    fs::write(
        config_path,
        r#"
[app]
name = "demo"
init_mode = "enabled"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"
[app.whitelist]

[component_specs.app]
component_role = "app"
maximum_instances = 1
"#,
    )
    .expect("write config");
}

fn invalid_root_only_fleet_input() -> &'static str {
    r#"schema_version = 1

[operator]
principal = "ryjl3-tyaaa-aaaaa-aaaba-cai"
funding_account = "test-operator"
source = "test_fixture"
observed_at_unix_secs = 1782432100
valid_until_unix_secs = 4102444800

[operator.balance]
kind = "cycles"
cycles = "100T"

[coordinator.subnet]
kind = "explicit"
subnet = "pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae"

[coordinator.creation_funding]
kind = "cycles"
cycles = "2T"

[[fleet_subnet_roots]]
placement_subnet = "pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae"

[fleet_subnet_roots.component_admissions]
unknown = 1

[fleet_subnet_roots.limits]
maximum_component_instances = 1
maximum_registry_bytes = 4194304
maximum_wasm_store_bytes = 40000000
maximum_group_placements = 0

[fleet_subnet_roots.limits.cycles_funding]
window_secs = 3600
maximum_cycles = "10T"

[fleet_subnet_roots.canister_pool]
minimum_size = 1
maximum_size = 1
canister_cycles = "1T"
imports = []

[fleet_subnet_roots.root_creation_funding]
kind = "cycles"
cycles = "2T"

[fleet_subnet_roots.wasm_store_creation_funding]
kind = "cycles"
cycles = "2T"
"#
}

fn valid_single_component_fleet_input() -> String {
    invalid_root_only_fleet_input().replace("unknown = 1", "app = 1")
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
    write_local_network_authority(&root, "local");
    let config_path = root.join("apps/demo/canic.toml");
    fs::create_dir_all(config_path.parent().expect("config parent")).expect("create config dir");
    fs::write(
        &config_path,
        r#"
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


"#,
    )
    .expect("write config");
    write_wasm_gz_artifact(&root, "root", b"root-artifact");

    let options = InstallRootOptions {
        root_canister: "root".to_string(),
        root_build_target: "root".to_string(),
        icp_executable: "icp".to_string(),
        environment: "local".to_string(),
        fleet_name: "demo".to_string(),
        icp_root: Some(root.clone()),
        build_profile: Some(CanisterBuildProfile::Fast),
        release_build_id: None,
        config_path: Some("apps/demo/canic.toml".to_string()),
        fleet_install_input_path: None,
        expected_fresh_fleet_plan_digest: None,
        admitted_fresh_fleet_plan_digest: None,
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
    (root, check)
}

fn sample_fleet_activation_identity() -> FleetActivationIdentity {
    FleetActivationIdentity {
        fleet: FleetBinding {
            fleet: sample_fleet_key(),
            app: AppId::from("demo"),
        },
        operation_id: [8; 32],
        release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([9; 32])),
    }
}

fn sample_fleet_key() -> FleetKey {
    FleetKey {
        canonical_network_id: CanonicalNetworkId::ic_mainnet(),
        fleet_id: FleetId::from_generated_bytes([7; 32]),
    }
}

fn sample_fleet_receipt_dir(root: &Path) -> PathBuf {
    let fleet = sample_fleet_key();
    root.join(".canic")
        .join("networks")
        .join(fleet.canonical_network_id.to_string())
        .join("fleets")
        .join(fleet.fleet_id.to_string())
        .join("deployment-receipts")
}
