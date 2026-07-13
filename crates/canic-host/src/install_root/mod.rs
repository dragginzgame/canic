use crate::{
    canister_build::cache::DefaultCanisterBuildCacheCleanup,
    release_set::{icp_root, workspace_root},
};
use config_selection::resolve_install_config_path;
use std::{path::PathBuf, time::Instant};
use thiserror::Error as ThisError;

mod activation;
mod artifact_promotion;
mod build_environment;
mod build_targets;
mod capabilities;
mod clock;
mod commands;
mod config_selection;
mod current_execution;
mod deployment_registration;
mod deployment_truth_gate;
mod execution_preflight;
mod identity;
mod install_state;
mod operations;
mod options;
mod output;
mod phase_receipts;
mod plan_artifacts;
mod preparation;
mod readiness;
mod receipt_io;
mod root_canister;
mod root_cycles;
mod root_verification;
mod staging;
mod state;
mod timing;
mod truth_check;

use activation::run_root_activation_phases;
use artifact_promotion::write_artifact_promotion_execution_receipt_for_install;
use build_environment::resolve_install_build_context;
pub use config_selection::{
    current_canic_project_root, discover_canic_config_choices, discover_canic_project_root_from,
    discover_project_canic_config_choices, project_fleet_roots,
};
use current_execution::current_install_execution_context;
pub use deployment_registration::{
    RegisterDeploymentStateOptions, VerifyDeploymentRootOptions, register_deployment_state,
    verify_registered_deployment_root,
};
use identity::resolve_install_identity;
use install_state::{build_install_state, write_install_state_with_deployment_truth_receipt};
pub use options::InstallRootOptions;
use output::{print_install_result_summary, print_install_timing_summary};
use phase_receipts::InstallReceiptScope;
use plan_artifacts::emit_manifest_with_deployment_truth_receipt;
use preparation::prepare_install_deployment_truth;
pub use receipt_io::latest_deployment_truth_receipt_path_from_root;
pub use state::{
    InstallState, InstallStateError, RootVerificationStatus, read_named_deployment_install_state,
    read_named_deployment_install_state_from_root,
};
pub(crate) use state::{decode_install_state, validate_network_name};
use timing::InstallTimingSummary as CurrentInstallTimingSummary;
pub use truth_check::{check_install_deployment_truth, check_install_execution_preflight};

#[cfg(test)]
mod tests;

///
/// InstallRootBlockKind
///
/// Machine-readable reason that a fresh root install stopped before mutation.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstallRootBlockKind {
    DeploymentExecutionPreflight,
    DeploymentTruth,
}

///
/// InstallRootBlockedError
///
/// Typed install block retained through the host/CLI error boundary.
///

#[derive(Debug, ThisError)]
#[error("{message}")]
pub struct InstallRootBlockedError {
    kind: InstallRootBlockKind,
    message: String,
}

impl InstallRootBlockedError {
    pub(super) const fn new(kind: InstallRootBlockKind, message: String) -> Self {
        Self { kind, message }
    }

    #[must_use]
    pub const fn kind(&self) -> InstallRootBlockKind {
        self.kind
    }
}

/// Discover installable Canic config choices under the current workspace.
pub fn discover_current_canic_config_choices() -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let project_root = current_canic_project_root()?;
    let choices = config_selection::discover_workspace_canic_config_choices(&project_root)?;
    if !choices.is_empty() {
        return Ok(choices);
    }

    if let Ok(icp_root) = icp_root()
        && icp_root != project_root
    {
        return config_selection::discover_workspace_canic_config_choices(&icp_root);
    }

    Ok(choices)
}

// Execute the local thin-root install flow against an already running replica.
pub fn install_root(options: InstallRootOptions) -> Result<(), Box<dyn std::error::Error>> {
    let workspace_root = workspace_root()?;
    let _build_cache_cleanup = DefaultCanisterBuildCacheCleanup::for_install(&workspace_root);
    let icp_root = match &options.icp_root {
        Some(path) => path.canonicalize()?,
        None => icp_root()?,
    };
    let config_path = resolve_install_config_path(
        &icp_root,
        options.config_path.as_deref(),
        options.interactive_config_selection,
    )?;
    let build_context =
        current_install_build_context(&workspace_root, &icp_root, &config_path, &options)?;
    let (fleet_name, deployment_name) = resolve_install_identity(&options, &config_path)?;
    let total_started_at = Instant::now();
    let mut timings = CurrentInstallTimingSummary::default();
    let network = options.network.as_str();
    let execution_context = current_install_execution_context(&workspace_root, &icp_root, network);

    println!("Installing deployment {deployment_name}");
    println!("Fleet template {fleet_name}");
    println!();
    let prepared = prepare_install_deployment_truth(
        &options,
        &workspace_root,
        &icp_root,
        &config_path,
        &deployment_name,
        &execution_context,
        &build_context,
    )?;
    timings.create_canisters = prepared.timings.create_canisters;
    timings.build_all = prepared.timings.build_all;

    let (manifest_path, emit_manifest_duration) = emit_manifest_with_deployment_truth_receipt(
        &workspace_root,
        &icp_root,
        &options,
        &config_path,
        &deployment_name,
        &prepared.deployment_truth_check,
        &execution_context,
    )?;
    timings.emit_manifest = emit_manifest_duration;
    let activation_timings = run_root_activation_phases(
        InstallReceiptScope {
            icp_root: &icp_root,
            network,
            deployment_name: &deployment_name,
            check: &prepared.deployment_truth_check,
            execution_context: Some(&execution_context),
        },
        &options,
        &prepared.root_canister_id,
        &manifest_path,
        total_started_at,
        &build_context,
    )?;
    timings.install_root = activation_timings.install_root;
    timings.fund_root = activation_timings.fund_root;
    timings.stage_release_set = activation_timings.stage_release_set;
    timings.resume_bootstrap = activation_timings.resume_bootstrap;
    timings.wait_ready = activation_timings.wait_ready;
    timings.finalize_root_funding = activation_timings.finalize_root_funding;

    print_install_timing_summary(&timings, total_started_at.elapsed());
    let state = build_install_state(
        &options,
        &workspace_root,
        &icp_root,
        &config_path,
        &manifest_path,
        (&deployment_name, &fleet_name),
        &prepared.root_canister_id,
    )?;
    let state_path = write_install_state_with_deployment_truth_receipt(
        InstallReceiptScope {
            icp_root: &icp_root,
            network,
            deployment_name: &deployment_name,
            check: &prepared.deployment_truth_check,
            execution_context: Some(&execution_context),
        },
        &options.network,
        &state,
    )?;
    write_artifact_promotion_execution_receipt_for_install(
        &options,
        &icp_root,
        network,
        &deployment_name,
        &prepared.deployment_truth_check,
        &execution_context,
    )?;
    print_install_result_summary(
        &options.network,
        &state.deployment_name,
        &state.fleet_template,
        &state_path,
    );
    Ok(())
}

fn current_install_build_context(
    workspace_root: &std::path::Path,
    icp_root: &std::path::Path,
    config_path: &std::path::Path,
    options: &InstallRootOptions,
) -> Result<crate::canister_build::WorkspaceBuildContext, Box<dyn std::error::Error>> {
    resolve_install_build_context(
        workspace_root,
        icp_root,
        config_path,
        &options.network,
        &options.root_build_target,
        options.build_profile,
    )
}
