use crate::canister_build::CanisterBuildProfile;
use crate::deployment_truth::{
    ArtifactPromotionPlanV1, DeploymentCheckV1, DeploymentExecutionContextV1,
    DeploymentExecutorCapabilityV1, DeploymentPlanV1,
};
use crate::release_set::{
    configured_fleet_name, configured_install_targets, icp_root, load_root_release_set_manifest,
    workspace_root,
};
use config_selection::resolve_install_config_path;
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

mod artifact_promotion;
mod commands;
mod config_selection;
mod current_execution;
mod deployment_registration;
mod deployment_truth_gate;
mod execution_preflight;
mod operations;
mod phase_receipts;
mod plan_artifacts;
mod readiness;
mod receipt_io;
mod root_canister;
mod root_verification;
mod staging;
mod state;
mod truth_check;

use artifact_promotion::write_artifact_promotion_execution_receipt_for_install;
use commands::{
    BuildEnvGuard, ensure_icp_environment_ready, print_install_result_summary,
    print_install_timing_summary,
};
#[cfg(test)]
use commands::{
    add_create_root_target, add_icp_environment_target, add_local_root_create_cycles_arg,
    icp_canister_command_in_network, is_missing_canister_id_error, parse_created_canister_id,
};
pub use config_selection::{
    current_canic_project_root, discover_canic_config_choices, discover_canic_project_root_from,
    discover_project_canic_config_choices, project_fleet_roots,
};
#[cfg(test)]
use current_execution::current_install_executor_missing_capabilities;
use current_execution::{
    current_install_execution_context, ensure_current_install_executor_capabilities,
    run_install_deployment_truth_safety_gate,
};
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
#[cfg(test)]
use operations::EmitRootManifestOperation;
use operations::{
    BuildInstallTargetsOperation, EnsureRootCyclesOperation, InstallRootWasmOperation,
    ResolveRootCanisterOperation, ResumeBootstrapOperation, WaitRootReadyOperation,
};
#[cfg(test)]
use phase_receipts::install_deployment_truth_phase_receipt;
use phase_receipts::{
    CompletedInstallPhase, InstallReceiptScope, write_completed_install_phase_receipt,
};
use plan_artifacts::{
    emit_manifest_with_deployment_truth_receipt, root_wasm_for_install_plan,
    validate_plan_artifacts_with_phase,
};
pub use receipt_io::latest_deployment_truth_receipt_path_from_root;
#[cfg(test)]
use receipt_io::write_install_deployment_truth_receipt;
#[cfg(test)]
use root_verification::write_verified_root_state_if_unchanged;
use staging::StageReleaseSetOperation;
#[cfg(test)]
use staging::current_install_staging_evidence;
use state::{INSTALL_STATE_SCHEMA_VERSION, validate_state_name, write_install_state};
pub use state::{
    InstallState, RootVerificationStatus, read_named_deployment_install_state,
    read_named_deployment_install_state_from_root,
};
#[cfg(test)]
use state::{deployment_install_state_path, read_deployment_install_state};
#[cfg(test)]
use truth_check::current_install_deployment_truth_check_at;
use truth_check::validate_expected_fleet_name;
pub use truth_check::{check_install_deployment_truth, check_install_execution_preflight};

#[cfg(test)]
mod tests;

#[cfg(test)]
use crate::response_parse::parse_cycle_balance_response;
#[cfg(test)]
use commands::{parse_canister_id_json, render_install_timing_summary, root_init_args};
#[cfg(test)]
use config_selection::config_selection_error;
#[cfg(test)]
use readiness::{parse_bootstrap_status_value, parse_root_ready_value};
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

///
/// InstallTimingSummary
///

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct InstallTimingSummary {
    create_canisters: Duration,
    build_all: Duration,
    emit_manifest: Duration,
    install_root: Duration,
    fund_root: Duration,
    stage_release_set: Duration,
    resume_bootstrap: Duration,
    wait_ready: Duration,
    finalize_root_funding: Duration,
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
    let mut timings = InstallTimingSummary::default();
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

struct PreparedInstallTruth {
    root_canister_id: String,
    deployment_truth_check: DeploymentCheckV1,
    timings: InstallTimingSummary,
}

fn prepare_install_deployment_truth(
    options: &InstallRootOptions,
    workspace_root: &Path,
    icp_root: &Path,
    config_path: &Path,
    deployment_name: &str,
    execution_context: &DeploymentExecutionContextV1,
) -> Result<PreparedInstallTruth, Box<dyn std::error::Error>> {
    let mut timings = InstallTimingSummary::default();
    ensure_current_install_executor_capabilities(execution_context)?;
    ensure_icp_environment_ready(icp_root, &options.network)?;
    let (root_canister_id, create_phase, create_duration) =
        resolve_root_canister_with_phase(options, icp_root, config_path)?;
    timings.create_canisters = create_duration;

    let (build_phase, build_duration) =
        build_install_targets_with_phase(options, icp_root, config_path)?;
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
) -> Result<(String, CompletedInstallPhase, Duration), Box<dyn std::error::Error>> {
    let operation = ResolveRootCanisterOperation::new(
        icp_root,
        &options.network,
        &options.root_canister,
        config_path,
    );
    let started_at = current_unix_timestamp_label()?;
    let started = Instant::now();
    let root_canister_id = operation.execute()?;
    let duration = started.elapsed();
    let phase = CompletedInstallPhase {
        phase: "resolve_root_canister",
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
    icp_root: &Path,
    config_path: &Path,
) -> Result<(CompletedInstallPhase, Duration), Box<dyn std::error::Error>> {
    if let Some(plan) = &options.deployment_plan_override {
        return validate_plan_artifacts_with_phase(plan, icp_root, &options.network);
    }

    let build_targets = configured_install_targets(config_path, &options.root_build_target)?;
    let operation = BuildInstallTargetsOperation::new(
        &options.network,
        build_targets,
        options.build_profile,
        config_path,
        icp_root,
    );
    let started_at = current_unix_timestamp_label()?;
    let started = Instant::now();
    operation.execute()?;
    let duration = started.elapsed();
    let phase = CompletedInstallPhase {
        phase: "build_artifacts",
        attempted_action: "build configured install targets",
        started_at,
        finished_at: Some(current_unix_timestamp_label()?),
        evidence: operation.evidence(),
        role_names: operation.role_names(),
    };
    Ok((phase, duration))
}

fn run_root_activation_phases(
    receipt_scope: InstallReceiptScope<'_>,
    options: &InstallRootOptions,
    root_canister_id: &str,
    manifest_path: &Path,
    total_started_at: Instant,
) -> Result<InstallTimingSummary, Box<dyn std::error::Error>> {
    let mut timings = InstallTimingSummary::default();
    let root_wasm = root_wasm_for_install_plan(
        receipt_scope.icp_root,
        receipt_scope.network,
        &options.root_build_target,
        options.deployment_plan_override.as_ref(),
    )?;
    let install_operation = InstallRootWasmOperation::new(
        receipt_scope.icp_root,
        receipt_scope.network,
        root_canister_id,
        root_wasm,
    );
    timings.install_root = receipt_scope.run_operation(&install_operation)?;
    let pre_bootstrap_funding = EnsureRootCyclesOperation::new(
        receipt_scope.icp_root,
        receipt_scope.network,
        root_canister_id,
        "fund_root_pre_bootstrap",
        "ensure local root minimum cycles before bootstrap",
        "pre-bootstrap",
    );
    timings.fund_root = receipt_scope.run_operation(&pre_bootstrap_funding)?;
    let manifest = load_root_release_set_manifest(manifest_path)?;
    let stage_operation = StageReleaseSetOperation::new(
        receipt_scope.icp_root,
        receipt_scope.network,
        root_canister_id,
        manifest_path,
        manifest,
    );
    timings.stage_release_set = receipt_scope.run_operation(&stage_operation)?;
    let resume_operation = ResumeBootstrapOperation::new(receipt_scope.network, root_canister_id);
    timings.resume_bootstrap = receipt_scope.run_operation(&resume_operation)?;
    let wait_ready_operation = WaitRootReadyOperation::new(
        receipt_scope.network,
        root_canister_id,
        options.ready_timeout_seconds,
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
        receipt_scope.network,
        root_canister_id,
        "fund_root_post_ready",
        "ensure local root minimum cycles after ready",
        "post-ready",
    );
    timings.finalize_root_funding = receipt_scope.run_operation(&post_ready_funding)?;
    Ok(timings)
}

fn write_install_state_with_deployment_truth_receipt(
    receipt_scope: InstallReceiptScope<'_>,
    network: &str,
    state: &InstallState,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let started_at = current_unix_timestamp_label()?;
    let state_path = write_install_state(receipt_scope.icp_root, network, state)?;
    let completed = CompletedInstallPhase {
        phase: "write_install_state",
        attempted_action: "write local install state",
        started_at,
        finished_at: Some(current_unix_timestamp_label()?),
        evidence: vec![
            format!("install_state:{}", state_path.display()),
            format!("deployment:{}", state.deployment_name),
            format!("fleet_template:{}", state.fleet_template),
            format!("root_canister:{}", state.root_canister_id),
        ],
        role_names: Vec::new(),
    };
    write_completed_install_phase_receipt(receipt_scope, completed)?;
    Ok(state_path)
}

// Build the persisted project-local install state from a completed install.
fn build_install_state(
    options: &InstallRootOptions,
    workspace_root: &Path,
    icp_root: &Path,
    config_path: &Path,
    release_set_manifest_path: &Path,
    identity: (&str, &str),
    root_canister_id: &str,
) -> Result<InstallState, Box<dyn std::error::Error>> {
    let (deployment_name, fleet_name) = identity;
    let timestamp = current_unix_secs()?;
    Ok(InstallState {
        schema_version: INSTALL_STATE_SCHEMA_VERSION,
        deployment_name: deployment_name.to_string(),
        fleet_template: fleet_name.to_string(),
        created_at_unix_secs: timestamp,
        updated_at_unix_secs: timestamp,
        network: options.network.clone(),
        root_target: options.root_canister.clone(),
        root_canister_id: root_canister_id.to_string(),
        root_verification: RootVerificationStatus::Verified,
        root_build_target: options.root_build_target.clone(),
        workspace_root: workspace_root.display().to_string(),
        icp_root: icp_root.display().to_string(),
        config_path: config_path.display().to_string(),
        release_set_manifest_path: release_set_manifest_path.display().to_string(),
    })
}

// Read the current host clock as a unix timestamp for install state.
fn current_unix_secs() -> Result<u64, Box<dyn std::error::Error>> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

fn current_unix_timestamp_label() -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!("unix:{}", current_unix_secs()?))
}
