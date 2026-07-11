use super::build_environment::ensure_icp_environment_ready;
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
use crate::canister_build::WorkspaceBuildContext;
use crate::deployment_truth::{DeploymentCheckV1, DeploymentExecutionContextV1};
use crate::release_set::configured_install_targets;
use std::{
    path::Path,
    time::{Duration, Instant},
};

pub(super) struct PreparedInstallTruth {
    pub(super) root_canister_id: String,
    pub(super) deployment_truth_check: DeploymentCheckV1,
    pub(super) timings: InstallTimingSummary,
}

pub(super) fn prepare_install_deployment_truth(
    options: &InstallRootOptions,
    workspace_root: &Path,
    icp_root: &Path,
    config_path: &Path,
    deployment_name: &str,
    execution_context: &DeploymentExecutionContextV1,
    build_context: &WorkspaceBuildContext,
) -> Result<PreparedInstallTruth, Box<dyn std::error::Error>> {
    let mut timings = InstallTimingSummary::default();
    ensure_current_install_executor_capabilities(execution_context)?;
    ensure_icp_environment_ready(icp_root, &options.network)?;
    let (root_canister_id, create_phase, create_duration) =
        resolve_root_canister_with_phase(options, icp_root, config_path, build_context)?;
    timings.create_canisters = create_duration;

    let (build_phase, build_duration) =
        build_install_targets_with_phase(options, build_context, icp_root, config_path)?;
    timings.build_all = build_duration;

    let deployment_truth_check = run_install_deployment_truth_safety_gate(
        options,
        workspace_root,
        icp_root,
        config_path,
        deployment_name,
        execution_context,
    )?;
    let receipt_scope = InstallReceiptScope {
        icp_root,
        network: &options.network,
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
        &options.network,
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
    config_path: &Path,
) -> Result<(CompletedInstallPhase, Duration), Box<dyn std::error::Error>> {
    if let Some(plan) = &options.deployment_plan_override {
        return validate_plan_artifacts_with_phase(plan, icp_root, &options.network);
    }

    let build_targets = configured_install_targets(config_path, &options.root_build_target)?;
    let operation = BuildInstallTargetsOperation::new(build_context, build_targets);
    let started_at = current_unix_timestamp_label()?;
    let started = Instant::now();
    operation.execute()?;
    let duration = started.elapsed();
    let phase = CompletedInstallPhase {
        phase: InstallPhaseLabel::BUILD_ARTIFACTS,
        attempted_action: "build configured install targets",
        started_at,
        finished_at: Some(current_unix_timestamp_label()?),
        evidence: operation.evidence(),
        role_names: operation.role_names(),
    };
    Ok((phase, duration))
}
