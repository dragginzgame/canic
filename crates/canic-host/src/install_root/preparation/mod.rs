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
use super::plan_artifacts::validate_plan_artifacts_with_phase;
use super::timing::InstallTimingSummary;
use super::{clock::current_unix_timestamp_label, options::InstallRootOptions};
use crate::canister_build::{CurrentCanisterArtifactBuildOutput, WorkspaceBuildContext};
use crate::deployment_truth::{DeploymentCheckV1, DeploymentExecutionContextV1};
use std::{
    path::Path,
    time::{Duration, Instant},
};

pub(super) struct PreparedInstallTruth {
    pub(super) root_canister_id: String,
    pub(super) deployment_truth_check: DeploymentCheckV1,
    pub(super) timings: InstallTimingSummary,
    pub(super) build_outputs: Vec<CurrentCanisterArtifactBuildOutput>,
}

pub(super) fn prepare_install_deployment_truth(
    options: &InstallRootOptions,
    icp_root: &Path,
    config_path: &Path,
    deployment_name: &str,
    execution_context: &DeploymentExecutionContextV1,
    build_context: &WorkspaceBuildContext,
    install_snapshot: &ValidatedInstallSnapshot,
) -> Result<PreparedInstallTruth, Box<dyn std::error::Error>> {
    let mut timings = InstallTimingSummary::default();
    ensure_current_install_executor_capabilities(execution_context)?;
    ensure_icp_environment_ready(icp_root, &options.environment)?;
    let (root_canister_id, create_phase, create_duration) =
        resolve_root_canister_with_phase(options, icp_root, config_path, build_context)?;
    timings.create_canisters = create_duration;

    let (build_phase, build_duration, build_outputs) =
        build_install_targets_with_phase(options, build_context, icp_root, install_snapshot)?;
    timings.build_all = build_duration;

    let deployment_truth_check = run_install_deployment_truth_safety_gate(
        options,
        &build_context.workspace_root,
        icp_root,
        config_path,
        deployment_name,
        execution_context,
    )?;
    let receipt_scope = InstallReceiptScope {
        icp_root,
        environment: &options.environment,
        deployment_name,
        check: &deployment_truth_check,
        execution_context: Some(execution_context),
    };
    write_completed_install_phase_receipt(receipt_scope, create_phase)?;
    write_completed_install_phase_receipt(receipt_scope, build_phase)?;

    Ok(PreparedInstallTruth {
        root_canister_id,
        deployment_truth_check,
        timings,
        build_outputs,
    })
}

fn resolve_root_canister_with_phase(
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

fn build_install_targets_with_phase(
    options: &InstallRootOptions,
    build_context: &WorkspaceBuildContext,
    icp_root: &Path,
    install_snapshot: &ValidatedInstallSnapshot,
) -> Result<
    (
        CompletedInstallPhase,
        Duration,
        Vec<CurrentCanisterArtifactBuildOutput>,
    ),
    Box<dyn std::error::Error>,
> {
    if let Some(plan) = &options.deployment_plan_override {
        let (phase, duration) =
            validate_plan_artifacts_with_phase(plan, icp_root, &options.environment)?;
        return Ok((phase, duration, Vec::new()));
    }

    let complete_build = install_snapshot
        .complete_build
        .as_ref()
        .ok_or_else(|| "normal install is missing its complete-build snapshot".to_string())?;
    let operation = BuildInstallTargetsOperation::new(build_context, &complete_build.targets);
    let started_at = current_unix_timestamp_label()?;
    let started = Instant::now();
    let outputs = operation.execute()?;
    let duration = started.elapsed();
    let phase = CompletedInstallPhase {
        phase: InstallPhaseLabel::BUILD_ARTIFACTS,
        attempted_action: "build configured install targets",
        started_at,
        finished_at: Some(current_unix_timestamp_label()?),
        evidence: operation.evidence(),
        role_names: operation.role_names(),
    };
    Ok((phase, duration, outputs))
}
