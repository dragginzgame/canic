use super::build_network::ensure_icp_environment_ready;
use super::build_snapshot::ValidatedInstallSnapshot;
use super::current_execution::{
    ensure_current_install_executor_capabilities, run_install_deployment_truth_safety_gate,
};
use super::operations::InstallPhaseLabel;
use super::operations::{BuildInstallTargetsOperation, ResolveRootCanisterOperation};
use super::phase_receipts::{
    CompletedInstallPhase, InstallReceiptScope, write_completed_install_phase_receipt,
};
use super::plan_artifacts::{PreparedPlanArtifacts, prepare_plan_artifacts_with_phase};
use super::timing::InstallTimingSummary;
use super::{clock::current_unix_timestamp_label, options::InstallRootOptions};
use crate::deployment_truth::{
    DeploymentCheckV1, DeploymentExecutionContextV1, DeploymentReceiptV1,
};
use crate::{
    bootstrap_coordinator::build_bootstrap_fleet_coordinator_artifact,
    bootstrap_store::qualify_built_bootstrap_wasm_store_artifact,
    canister_build::{CurrentCanisterArtifactBuildOutput, WorkspaceBuildContext},
    release_set::{CanicInfrastructureArtifactBuildOutput, CanicInfrastructureRole},
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
    duration: Duration,
    outputs: Vec<CurrentCanisterArtifactBuildOutput>,
    infrastructure_outputs: Vec<CanicInfrastructureArtifactBuildOutput>,
    plan_artifacts: Option<PreparedPlanArtifacts>,
}

pub(super) fn prepare_install_deployment_truth(
    options: &InstallRootOptions,
    icp_root: &Path,
    config_path: &Path,
    fleet_name: &str,
    execution_context: &DeploymentExecutionContextV1,
    build_context: &WorkspaceBuildContext,
    install_snapshot: &ValidatedInstallSnapshot,
) -> Result<PreparedInstallTruth, Box<dyn std::error::Error>> {
    let mut timings = InstallTimingSummary::default();
    ensure_current_install_executor_capabilities(execution_context)?;
    ensure_icp_environment_ready(icp_root, &options.environment)?;
    let build =
        build_install_targets_with_phase(options, build_context, icp_root, install_snapshot)?;
    timings.build_all = build.duration;

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

pub(super) fn resolve_root_canister_with_phase(
    options: &InstallRootOptions,
    icp_root: &Path,
    config_path: &Path,
    build_context: &WorkspaceBuildContext,
) -> Result<(String, CompletedInstallPhase, Duration), Box<dyn std::error::Error>> {
    let operation = ResolveRootCanisterOperation::new(
        icp_root,
        &options.environment,
        &options.root_canister,
        config_path,
        build_context.local_replica.as_ref(),
    );
    let started_at = current_unix_timestamp_label()?;
    let started = Instant::now();
    let root_canister_id = operation.execute()?;
    let duration = started.elapsed();
    let phase = CompletedInstallPhase {
        phase: InstallPhaseLabel::RESOLVE_ROOT_CANISTER,
        attempted_action: "resolve or create root canister id",
        started_at,
        finished_at: Some(current_unix_timestamp_label()?),
        evidence: operation.evidence(&root_canister_id),
        role_names: Vec::new(),
    };
    Ok((root_canister_id, phase, duration))
}

pub(super) fn resolve_root_canister_after_manifest(
    receipt_scope: InstallReceiptScope<'_>,
    options: &InstallRootOptions,
    config_path: &Path,
    build_context: &WorkspaceBuildContext,
) -> Result<(String, Duration), Box<dyn std::error::Error>> {
    let (root_canister_id, phase, duration) = resolve_root_canister_with_phase(
        options,
        receipt_scope.icp_root,
        config_path,
        build_context,
    )?;
    write_completed_install_phase_receipt(receipt_scope, phase)?;
    Ok((root_canister_id, duration))
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
            duration,
            outputs: Vec::new(),
            infrastructure_outputs: Vec::new(),
            plan_artifacts: Some(plan_artifacts),
        });
    }

    let complete_build = install_snapshot
        .complete_build
        .as_ref()
        .ok_or_else(|| "normal install is missing its complete-build snapshot".to_string())?;
    let operation = BuildInstallTargetsOperation::new(build_context, &complete_build.targets);
    let started_at = current_unix_timestamp_label()?;
    let started = Instant::now();
    let outputs = operation.execute()?;
    let infrastructure_outputs =
        qualify_infrastructure_outputs(options, build_context, complete_build, &outputs)?;
    let duration = started.elapsed();
    let phase = CompletedInstallPhase {
        phase: InstallPhaseLabel::BUILD_ARTIFACTS,
        attempted_action: "build configured install targets",
        started_at,
        finished_at: Some(current_unix_timestamp_label()?),
        evidence: operation.evidence(),
        role_names: operation.role_names(),
    };
    Ok(PreparedInstallBuild {
        phase,
        duration,
        outputs,
        infrastructure_outputs,
        plan_artifacts: None,
    })
}

fn qualify_infrastructure_outputs(
    options: &InstallRootOptions,
    build_context: &WorkspaceBuildContext,
    complete_build: &super::build_snapshot::CompleteInstallBuildSnapshot,
    outputs: &[CurrentCanisterArtifactBuildOutput],
) -> Result<Vec<CanicInfrastructureArtifactBuildOutput>, Box<dyn std::error::Error>> {
    let release_build_id = build_context
        .release_build_id
        .ok_or("infrastructure build is missing its durable release-build identity")?;
    let root_target = complete_build
        .targets
        .iter()
        .find(|target| target.role == options.root_canister)
        .ok_or("complete install build has no Fleet Subnet Root target")?;
    let root_output = outputs
        .iter()
        .find(|output| output.role == options.root_canister)
        .ok_or("complete install build has no Fleet Subnet Root output")?;
    let coordinator =
        build_bootstrap_fleet_coordinator_artifact(&build_context.with_role("fleet_coordinator"))?;
    let wasm_store = qualify_built_bootstrap_wasm_store_artifact(build_context)?;

    Ok(vec![
        CanicInfrastructureArtifactBuildOutput {
            role: CanicInfrastructureRole::FleetCoordinator,
            package: coordinator.package_name,
            release_build_id,
            wasm_path: coordinator.wasm_path,
            wasm_gz_path: coordinator.wasm_gz_path,
        },
        CanicInfrastructureArtifactBuildOutput {
            role: CanicInfrastructureRole::FleetSubnetRoot,
            package: root_target.spec.package_name.clone(),
            release_build_id,
            wasm_path: root_output.output.wasm_path.clone(),
            wasm_gz_path: root_output.output.wasm_gz_path.clone(),
        },
        CanicInfrastructureArtifactBuildOutput {
            role: CanicInfrastructureRole::WasmStore,
            package: wasm_store.package_name,
            release_build_id,
            wasm_path: wasm_store.wasm_path,
            wasm_gz_path: wasm_store.wasm_gz_path,
        },
    ])
}
