use crate::canister_build::CanisterBuildProfile;
use crate::deployment_truth::{
    ArtifactPromotionPlanV1, DeploymentExecutorCapabilityV1, DeploymentPlanV1,
};
use crate::release_set::{configured_fleet_name, icp_root, workspace_root};
use config_selection::resolve_install_config_path;
use std::{
    path::{Path, PathBuf},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

mod activation;
mod artifact_promotion;
mod build_environment;
mod build_targets;
mod commands;
mod config_selection;
mod current_execution;
mod deployment_registration;
mod deployment_truth_gate;
mod execution_preflight;
mod install_state;
mod operations;
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
use build_environment::BuildEnvGuard;
#[cfg(test)]
use commands::{
    add_create_root_target, add_icp_environment_target, icp_canister_command_in_network,
    is_missing_canister_id_error, parse_created_canister_id,
};
pub use config_selection::{
    current_canic_project_root, discover_canic_config_choices, discover_canic_project_root_from,
    discover_project_canic_config_choices, project_fleet_roots,
};
use current_execution::current_install_execution_context;
#[cfg(test)]
use current_execution::current_install_executor_missing_capabilities;
pub use deployment_registration::{
    RegisterDeploymentStateOptions, VerifyDeploymentRootOptions, register_deployment_state,
    verify_registered_deployment_root,
};
#[cfg(test)]
use deployment_truth_gate::{
    enforce_install_deployment_truth_gate, install_deployment_truth_gate_lines,
    install_deployment_truth_gate_receipt,
};
#[cfg(test)]
use execution_preflight::write_current_install_execution_preflight_receipt;
use install_state::{build_install_state, write_install_state_with_deployment_truth_receipt};
#[cfg(test)]
use operations::EmitRootManifestOperation;
#[cfg(test)]
use operations::{
    BuildInstallTargetsOperation, EnsureRootCyclesOperation, InstallRootWasmOperation,
    ResolveRootCanisterOperation, ResumeBootstrapOperation, WaitRootReadyOperation,
};
#[cfg(test)]
use output::render_install_timing_summary;
use output::{print_install_result_summary, print_install_timing_summary};
use phase_receipts::InstallReceiptScope;
#[cfg(test)]
use phase_receipts::install_deployment_truth_phase_receipt;
#[cfg(test)]
use phase_receipts::{CompletedInstallPhase, write_completed_install_phase_receipt};
use plan_artifacts::emit_manifest_with_deployment_truth_receipt;
#[cfg(test)]
use plan_artifacts::validate_plan_artifacts_with_phase;
use preparation::prepare_install_deployment_truth;
pub use receipt_io::latest_deployment_truth_receipt_path_from_root;
#[cfg(test)]
use receipt_io::write_install_deployment_truth_receipt;
#[cfg(test)]
use root_cycles::add_local_root_create_cycles_arg;
#[cfg(test)]
use root_verification::write_verified_root_state_if_unchanged;
#[cfg(test)]
use staging::StageReleaseSetOperation;
#[cfg(test)]
use staging::current_install_staging_evidence;
use state::validate_state_name;
#[cfg(test)]
use state::{INSTALL_STATE_SCHEMA_VERSION, write_install_state};
pub use state::{
    InstallState, RootVerificationStatus, read_named_deployment_install_state,
    read_named_deployment_install_state_from_root,
};
#[cfg(test)]
use state::{deployment_install_state_path, read_deployment_install_state};
#[cfg(test)]
use timing::InstallTimingSummary;
use timing::InstallTimingSummary as CurrentInstallTimingSummary;
#[cfg(test)]
use truth_check::current_install_deployment_truth_check_at;
use truth_check::validate_expected_fleet_name;
pub use truth_check::{check_install_deployment_truth, check_install_execution_preflight};

#[cfg(test)]
mod tests;

#[cfg(test)]
use commands::{parse_canister_id_json, root_init_args};
#[cfg(test)]
use config_selection::config_selection_error;
#[cfg(test)]
use readiness::parse_bootstrap_status_value;
#[cfg(test)]
use receipt_io::install_deployment_truth_receipt_path;
#[cfg(test)]
use state::legacy_fleet_install_state_path;

///
/// InstallRootOptions
///

#[derive(Clone, Debug)]
pub struct InstallRootOptions {
    pub root_canister: String,
    pub root_build_target: String,
    pub network: String,
    pub deployment_name: Option<String>,
    pub icp_root: Option<PathBuf>,
    pub build_profile: Option<CanisterBuildProfile>,
    pub ready_timeout_seconds: u64,
    pub config_path: Option<String>,
    pub expected_fleet: Option<String>,
    pub interactive_config_selection: bool,
    pub deployment_plan_override: Option<DeploymentPlanV1>,
    pub artifact_promotion_plan_override: Option<ArtifactPromotionPlanV1>,
}

const CURRENT_INSTALL_REQUIRED_CAPABILITIES: &[DeploymentExecutorCapabilityV1] = &[
    DeploymentExecutorCapabilityV1::CreateCanister,
    DeploymentExecutorCapabilityV1::InstallCode,
    DeploymentExecutorCapabilityV1::Call,
    DeploymentExecutorCapabilityV1::Query,
    DeploymentExecutorCapabilityV1::StageArtifact,
];

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
    let icp_root = match &options.icp_root {
        Some(path) => path.canonicalize()?,
        None => icp_root()?,
    };
    let config_path = resolve_install_config_path(
        &icp_root,
        options.config_path.as_deref(),
        options.interactive_config_selection,
    )?;
    let _install_env = BuildEnvGuard::apply(&options.network, &config_path, &icp_root);
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

fn resolve_install_identity(
    options: &InstallRootOptions,
    config_path: &Path,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let fleet_name = configured_fleet_name(config_path)?;
    validate_expected_fleet_name(options.expected_fleet.as_deref(), &fleet_name, config_path)?;
    validate_state_name(&fleet_name)?;
    let deployment_name = options
        .deployment_name
        .clone()
        .unwrap_or_else(|| fleet_name.clone());
    validate_state_name(&deployment_name)?;
    Ok((fleet_name, deployment_name))
}

// Read the current host clock as a unix timestamp for install state.
fn current_unix_secs() -> Result<u64, Box<dyn std::error::Error>> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

fn current_unix_timestamp_label() -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!("unix:{}", current_unix_secs()?))
}
