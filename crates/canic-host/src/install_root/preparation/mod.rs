use super::build_network::ensure_icp_environment_ready;
use super::build_snapshot::ValidatedInstallSnapshot;
use super::build_targets::{progress_bar, wasm_artifact_size};
use super::current_execution::{
    ensure_current_install_executor_capabilities, run_install_deployment_truth_safety_gate,
    run_install_early_authority_preflight,
};
use super::icp_context::InstallIcpContext;
use super::operations::{BuildInstallTargetsOperation, InstallPhaseLabel};
use super::output::{TerminalActivity, TerminalStyle};
use super::phase_receipts::CompletedInstallPhase;
use super::plan_artifacts::{PreparedPlanArtifacts, prepare_plan_artifacts_with_phase};
use super::reused_build::load_reused_install_build;
use super::timing::InstallTimingSummary;
use super::{clock::current_unix_timestamp_label, options::InstallRootOptions};
use crate::deployment_truth::{
    DeploymentCheckV1, DeploymentExecutionContextV1, DeploymentReceiptV1,
};
use crate::release_build::ReleaseBuildPlanState;
use crate::{
    canister_build::{
        CurrentCanisterArtifactBuildOutput, WorkspaceBuildContext,
        build_workspace_canister_artifact,
    },
    release_set::{CanicInfrastructureArtifactBuildOutput, CanicInfrastructureRole},
    table::{ColumnAlign, render_bordered_table},
};
use std::{
    path::Path,
    time::{Duration, Instant},
};

pub(super) struct PreparedInstallTruth {
    pub(super) deployment_truth_check: DeploymentCheckV1,
    pub(super) pre_activation_receipts: Vec<DeploymentReceiptV1>,
    pub(super) build_phase: CompletedInstallPhase,
    pub(super) timings: InstallTimingSummary,
    pub(super) build_outputs: Vec<CurrentCanisterArtifactBuildOutput>,
    pub(super) infrastructure_build_outputs: Vec<CanicInfrastructureArtifactBuildOutput>,
    pub(super) plan_artifacts: Option<PreparedPlanArtifacts>,
}

struct PreparedInstallBuild {
    phase: CompletedInstallPhase,
    configured_duration: Duration,
    infrastructure_duration: Duration,
    materialize_duration: Duration,
    reuse_duration: Duration,
    outputs: Vec<CurrentCanisterArtifactBuildOutput>,
    infrastructure_outputs: Vec<CanicInfrastructureArtifactBuildOutput>,
    plan_artifacts: Option<PreparedPlanArtifacts>,
}

pub(super) fn prepare_install_deployment_truth(
    options: &InstallRootOptions,
    icp: &InstallIcpContext,
    config_path: &Path,
    fleet_name: &str,
    execution_context: &DeploymentExecutionContextV1,
    build_context: &WorkspaceBuildContext,
    install_snapshot: &ValidatedInstallSnapshot,
) -> Result<PreparedInstallTruth, Box<dyn std::error::Error>> {
    let mut timings = InstallTimingSummary::default();
    let icp_root = icp.root();
    let preflight_started = Instant::now();
    ensure_current_install_executor_capabilities(execution_context)?;
    ensure_icp_environment_ready(icp)?;
    run_install_early_authority_preflight(
        options,
        &build_context.workspace_root,
        icp_root,
        config_path,
        fleet_name,
        execution_context,
    )?;
    timings.preflight = preflight_started.elapsed();
    let build =
        build_install_targets_with_phase(options, build_context, icp_root, install_snapshot)?;
    timings.build_configured = build.configured_duration;
    timings.build_infrastructure = build.infrastructure_duration;
    timings.materialize_artifacts = build.materialize_duration;
    timings.reuse_artifacts = build.reuse_duration;

    let post_build_gate_started = Instant::now();
    let safety_gate = run_install_deployment_truth_safety_gate(
        options,
        &build_context.workspace_root,
        icp_root,
        config_path,
        fleet_name,
        execution_context,
        build
            .plan_artifacts
            .as_ref()
            .map(PreparedPlanArtifacts::plan),
    )?;
    timings.post_build_gate = post_build_gate_started.elapsed();
    Ok(PreparedInstallTruth {
        deployment_truth_check: safety_gate.check,
        pre_activation_receipts: safety_gate.receipts,
        build_phase: build.phase,
        timings,
        build_outputs: build.outputs,
        infrastructure_build_outputs: build.infrastructure_outputs,
        plan_artifacts: build.plan_artifacts,
    })
}

fn build_install_targets_with_phase(
    options: &InstallRootOptions,
    build_context: &WorkspaceBuildContext,
    icp_root: &Path,
    install_snapshot: &ValidatedInstallSnapshot,
) -> Result<PreparedInstallBuild, Box<dyn std::error::Error>> {
    if let Some(plan) = &options.deployment_plan_override {
        let (plan_artifacts, phase, duration) =
            prepare_plan_artifacts_with_phase(plan, icp_root, &options.environment)?;
        return Ok(PreparedInstallBuild {
            phase,
            configured_duration: Duration::ZERO,
            infrastructure_duration: Duration::ZERO,
            materialize_duration: duration,
            reuse_duration: Duration::ZERO,
            outputs: Vec::new(),
            infrastructure_outputs: Vec::new(),
            plan_artifacts: Some(plan_artifacts),
        });
    }

    let complete_build = install_snapshot
        .complete_build
        .as_ref()
        .ok_or_else(|| "normal install is missing its complete-build snapshot".to_string())?;
    let release_build = install_snapshot
        .release_build
        .as_ref()
        .ok_or_else(|| "normal install is missing its release-build authority".to_string())?;
    if matches!(
        release_build.record.state,
        ReleaseBuildPlanState::Finalized { .. }
    ) {
        let started_at = current_unix_timestamp_label()?;
        let started = Instant::now();
        let reused = load_reused_install_build(
            icp_root,
            complete_build,
            release_build.record.release_build_id,
        )?;
        let duration = started.elapsed();
        TerminalStyle::detected().print_section(
            "Build artifacts ready",
            &format!(
                "reused finalized release {} in {:.2}s",
                release_build.record.release_build_id,
                duration.as_secs_f64()
            ),
        );
        println!();
        let role_names = reused
            .outputs
            .iter()
            .map(|output| output.role.clone())
            .chain(["fleet_coordinator".to_string(), "wasm_store".to_string()])
            .collect();
        return Ok(PreparedInstallBuild {
            phase: CompletedInstallPhase {
                phase: InstallPhaseLabel::BUILD_ARTIFACTS,
                attempted_action: "reuse finalized release-build artifacts",
                started_at,
                finished_at: Some(current_unix_timestamp_label()?),
                evidence: vec![format!(
                    "finalized_release_build:{}",
                    release_build.record.release_build_id
                )],
                role_names,
            },
            configured_duration: Duration::ZERO,
            infrastructure_duration: Duration::ZERO,
            materialize_duration: Duration::ZERO,
            reuse_duration: duration,
            outputs: reused.outputs,
            infrastructure_outputs: reused.infrastructure_outputs,
            plan_artifacts: None,
        });
    }
    let operation = BuildInstallTargetsOperation::new(build_context, &complete_build.targets);
    let started_at = current_unix_timestamp_label()?;
    let configured = operation.execute()?;
    let (infrastructure_outputs, infrastructure_duration) =
        qualify_infrastructure_outputs(options, build_context, &configured.outputs)?;
    let phase = CompletedInstallPhase {
        phase: InstallPhaseLabel::BUILD_ARTIFACTS,
        attempted_action: "build configured and built-in infrastructure targets",
        started_at,
        finished_at: Some(current_unix_timestamp_label()?),
        evidence: operation.evidence(),
        role_names: operation.role_names(),
    };
    Ok(PreparedInstallBuild {
        phase,
        configured_duration: configured.duration,
        infrastructure_duration,
        materialize_duration: Duration::ZERO,
        reuse_duration: Duration::ZERO,
        outputs: configured.outputs,
        infrastructure_outputs,
        plan_artifacts: None,
    })
}

fn qualify_infrastructure_outputs(
    options: &InstallRootOptions,
    build_context: &WorkspaceBuildContext,
    outputs: &[CurrentCanisterArtifactBuildOutput],
) -> Result<(Vec<CanicInfrastructureArtifactBuildOutput>, Duration), Box<dyn std::error::Error>> {
    let infrastructure_started_at = Instant::now();
    let release_build_id = build_context
        .release_build_id
        .ok_or("infrastructure build is missing its durable release-build identity")?;
    let root_output = outputs
        .iter()
        .find(|output| output.role == options.root_canister)
        .ok_or("complete install build has no Fleet Subnet Root output")?;
    let style = TerminalStyle::detected();
    style.print_section("Build infrastructure Wasm", "2 built-in canisters");
    let coordinator_started_at = Instant::now();
    let coordinator_activity =
        TerminalActivity::start(format!("{}  fleet_coordinator", progress_bar(1, 2, 12)));
    let coordinator =
        build_workspace_canister_artifact(&build_context.with_role("fleet_coordinator"))?;
    coordinator_activity.finish();
    let coordinator_elapsed = coordinator_started_at.elapsed();

    let wasm_store_started_at = Instant::now();
    let wasm_store_activity =
        TerminalActivity::start(format!("{}  wasm_store", progress_bar(2, 2, 12)));
    let wasm_store = build_workspace_canister_artifact(&build_context.with_role("wasm_store"))?;
    wasm_store_activity.finish();
    let wasm_store_elapsed = wasm_store_started_at.elapsed();

    let rows = [
        [
            "fleet_coordinator".to_string(),
            style.success("done"),
            wasm_artifact_size(&coordinator.wasm_path, &coordinator.wasm_gz_path)?,
            format!("{:.2}s", coordinator_elapsed.as_secs_f64()),
        ],
        [
            "wasm_store".to_string(),
            style.success("done"),
            wasm_artifact_size(&wasm_store.wasm_path, &wasm_store.wasm_gz_path)?,
            format!("{:.2}s", wasm_store_elapsed.as_secs_f64()),
        ],
    ];
    println!(
        "{}",
        render_bordered_table(
            &["CANISTER", "STATUS", "WASM", "ELAPSED"],
            &rows,
            &[
                ColumnAlign::Left,
                ColumnAlign::Left,
                ColumnAlign::Right,
                ColumnAlign::Right,
            ],
        )
    );
    println!();

    let outputs = vec![
        CanicInfrastructureArtifactBuildOutput {
            role: CanicInfrastructureRole::FleetCoordinator,
            package: coordinator.package_name,
            release_build_id,
            wasm_path: coordinator.wasm_path,
            wasm_gz_path: coordinator.wasm_gz_path,
            candid_sha256: coordinator.candid_sha256,
            protocol_profile_digest: coordinator.protocol_profile_digest,
        },
        CanicInfrastructureArtifactBuildOutput {
            role: CanicInfrastructureRole::FleetSubnetRoot,
            package: root_output.output.package_name.clone(),
            release_build_id,
            wasm_path: root_output.output.wasm_path.clone(),
            wasm_gz_path: root_output.output.wasm_gz_path.clone(),
            candid_sha256: root_output.output.candid_sha256,
            protocol_profile_digest: root_output.output.protocol_profile_digest,
        },
        CanicInfrastructureArtifactBuildOutput {
            role: CanicInfrastructureRole::WasmStore,
            package: wasm_store.package_name,
            release_build_id,
            wasm_path: wasm_store.wasm_path,
            wasm_gz_path: wasm_store.wasm_gz_path,
            candid_sha256: wasm_store.candid_sha256,
            protocol_profile_digest: wasm_store.protocol_profile_digest,
        },
    ];
    Ok((outputs, infrastructure_started_at.elapsed()))
}
