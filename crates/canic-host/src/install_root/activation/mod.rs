use super::operations::{
    EnsureRootCyclesOperation, InstallPhaseLabel, InstallRootWasmOperation,
    ResumeBootstrapOperation, WaitRootReadyOperation,
};
use super::options::InstallRootOptions;
use super::output::print_install_timing_summary;
use super::phase_receipts::InstallReceiptScope;
use super::plan_artifacts::{PreparedPlanArtifacts, normal_install_root_wasm};
use super::staging::StageReleaseSetOperation;
use super::timing::InstallTimingSummary;
use crate::canister_build::WorkspaceBuildContext;
use crate::release_set::load_root_release_set_manifest;
use std::{path::Path, time::Instant};

pub(super) fn run_root_activation_phases(
    receipt_scope: InstallReceiptScope<'_>,
    options: &InstallRootOptions,
    root_canister_id: &str,
    manifest_path: &Path,
    total_started_at: Instant,
    build_context: &WorkspaceBuildContext,
    plan_artifacts: Option<&PreparedPlanArtifacts>,
) -> Result<InstallTimingSummary, Box<dyn std::error::Error>> {
    let mut timings = InstallTimingSummary::default();
    let root_wasm = match plan_artifacts {
        Some(artifacts) => artifacts.verified_root_wasm_path()?,
        None => normal_install_root_wasm(receipt_scope.icp_root, &options.root_build_target),
    };
    let install_operation = InstallRootWasmOperation::new(
        receipt_scope.icp_root,
        receipt_scope.environment,
        root_canister_id,
        root_wasm,
        build_context.local_replica.as_ref(),
    );
    timings.install_root = receipt_scope.run_operation(&install_operation)?;
    let pre_bootstrap_funding = EnsureRootCyclesOperation::new(
        receipt_scope.icp_root,
        receipt_scope.environment,
        root_canister_id,
        InstallPhaseLabel::FUND_ROOT_PRE_BOOTSTRAP,
        "ensure local root minimum cycles before bootstrap",
        "pre-bootstrap",
        build_context.local_replica.as_ref(),
    );
    timings.fund_root = receipt_scope.run_operation(&pre_bootstrap_funding)?;
    let manifest = load_root_release_set_manifest(manifest_path)?;
    let stage_operation = StageReleaseSetOperation::new(
        receipt_scope.icp_root,
        receipt_scope.environment,
        root_canister_id,
        manifest_path,
        manifest,
        build_context.local_replica.as_ref(),
    );
    timings.stage_release_set = receipt_scope.run_operation(&stage_operation)?;
    let resume_operation = ResumeBootstrapOperation::new(
        receipt_scope.icp_root,
        receipt_scope.environment,
        root_canister_id,
        build_context.local_replica.as_ref(),
    );
    timings.resume_bootstrap = receipt_scope.run_operation(&resume_operation)?;
    let wait_ready_operation = WaitRootReadyOperation::new(
        receipt_scope.icp_root,
        receipt_scope.environment,
        root_canister_id,
        options.ready_timeout_seconds,
        build_context.local_replica.as_ref(),
    );
    let wait_ready_result = receipt_scope.run_operation(&wait_ready_operation);
    match wait_ready_result {
        Ok(duration) => timings.wait_ready = duration,
        Err(err) => {
            print_install_timing_summary(&timings, total_started_at.elapsed());
            return Err(err);
        }
    }
    let post_ready_funding = EnsureRootCyclesOperation::new(
        receipt_scope.icp_root,
        receipt_scope.environment,
        root_canister_id,
        InstallPhaseLabel::FUND_ROOT_POST_READY,
        "ensure local root minimum cycles after ready",
        "post-ready",
        build_context.local_replica.as_ref(),
    );
    timings.finalize_root_funding = receipt_scope.run_operation(&post_ready_funding)?;
    Ok(timings)
}
