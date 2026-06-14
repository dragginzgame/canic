use crate::canister_build::CanisterBuildProfile;
use crate::deployment_truth::{
    ArtifactPromotionExecutionReceiptRequest, ArtifactPromotionPlanV1,
    ArtifactPromotionProvenanceReportRequest, CurrentCliDeploymentExecutor, DeploymentCheckV1,
    DeploymentCommandResultV1, DeploymentExecutionContextV1, DeploymentExecutionPreflightV1,
    DeploymentExecutionStatusV1, DeploymentExecutor, DeploymentExecutorCapabilityV1,
    DeploymentPlanV1, DeploymentReceiptV1, DeploymentRootVerificationEvidenceStatusV1,
    DeploymentRootVerificationReceiptV1, DeploymentRootVerificationRequestV1,
    DeploymentRootVerificationSourceV1, DeploymentRootVerificationStateV1,
    LocalDeploymentCheckRequest, LocalInventoryRequest, ObservationStatusV1,
    artifact_gate_phase_receipt, artifact_gate_role_phase_receipts,
    artifact_promotion_execution_receipt, artifact_promotion_provenance_report,
    check_local_deployment, collect_local_deployment_inventory, compare_plan_to_inventory,
    deployment_execution_preflight_from_check, deployment_receipt_from_check_with_status,
    deployment_root_verification_report_from_check, missing_executor_capabilities, phase_receipt,
    safety_report_from_diff, validate_deployment_execution_preflight_for_check,
    validate_deployment_root_verification_report,
};
use crate::release_set::{
    LOCAL_ROOT_MIN_READY_CYCLES, RootReleaseSetManifest, configured_fleet_name,
    configured_install_targets, emit_root_release_set_manifest_with_config, icp_root,
    load_root_release_set_manifest, resolve_artifact_root, resume_root_bootstrap, workspace_root,
};
use canic_core::cdk::utils::hash::wasm_hash_hex;
use canic_core::{CANIC_WASM_CHUNK_BYTES, cdk::types::Principal};
use config_selection::resolve_install_config_path;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

mod commands;
mod config_selection;
mod deployment_truth_gate;
mod execution_preflight;
mod phase_receipts;
mod readiness;
mod receipt_io;
mod root_verification;
mod staging;
mod state;

use commands::{
    BuildEnvGuard, add_create_root_target, add_icp_environment_target,
    add_local_root_create_cycles_arg, ensure_icp_environment_ready, ensure_local_root_min_cycles,
    icp_canister_command_in_network, icp_command_in_network, icp_command_on_network,
    is_missing_canister_id_error, parse_created_canister_id, planned_build_artifact_root,
    print_install_result_summary, print_install_timing_summary, root_init_args,
    run_canic_build_targets, run_command, run_command_stdout,
};
pub use config_selection::{
    current_canic_project_root, discover_canic_config_choices, discover_canic_project_root_from,
    discover_project_canic_config_choices, project_fleet_roots,
};
#[cfg(test)]
use deployment_truth_gate::install_deployment_truth_gate_lines;
use deployment_truth_gate::{
    enforce_install_deployment_truth_gate, install_deployment_truth_gate_receipt,
    print_install_deployment_truth_gate,
};
use execution_preflight::write_current_install_execution_preflight_receipt;
use phase_receipts::{
    CompletedInstallPhase, InstallReceiptScope, completed_phase_role_receipt,
    install_deployment_truth_phase_receipt, receipt_with_execution_context,
    write_completed_install_phase_receipt,
};
use readiness::wait_for_root_ready;
pub use receipt_io::latest_deployment_truth_receipt_path_from_root;
use receipt_io::{
    write_artifact_promotion_execution_receipt, write_install_deployment_truth_receipt,
};
use root_verification::{
    RootVerificationReceiptInput, deployment_root_verification_state, file_sha256_hex,
    root_verification_receipt_from_report, verified_root_state_transition,
    write_verified_root_state_if_unchanged,
};
use staging::StageReleaseSetOperation;
#[cfg(test)]
use staging::current_install_staging_evidence;
use state::{
    INSTALL_STATE_SCHEMA_VERSION, deployment_install_state_path, read_deployment_install_state,
    validate_network_name, validate_state_name, write_install_state,
};
pub use state::{
    InstallState, RootVerificationStatus, read_named_deployment_install_state,
    read_named_deployment_install_state_from_root,
};

#[cfg(test)]
mod tests;

#[cfg(test)]
use crate::response_parse::parse_cycle_balance_response;
#[cfg(test)]
use commands::{parse_canister_id_json, render_install_timing_summary};
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
/// RegisterDeploymentStateOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegisterDeploymentStateOptions {
    pub deployment_name: String,
    pub fleet_template: String,
    pub root_canister_id: String,
    pub network: String,
    pub allow_unverified: bool,
    pub icp_root: Option<PathBuf>,
    pub workspace_root: Option<PathBuf>,
}

///
/// VerifyDeploymentRootOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifyDeploymentRootOptions {
    pub deployment_name: String,
    pub network: String,
    pub deployment_check: DeploymentCheckV1,
    pub verified_at_unix_secs: Option<u64>,
    pub icp_root: Option<PathBuf>,
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

/// Register minimal local deployment-target state for an existing root canister.
///
/// Registration is an explicit operator recovery path after the 0.46 hard cut.
/// It does not migrate legacy fleet state, verify live inventory, copy receipts,
/// or claim artifact/controller truth.
pub fn register_deployment_state(
    options: RegisterDeploymentStateOptions,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    validate_state_name(&options.deployment_name)?;
    validate_state_name(&options.fleet_template)?;
    validate_network_name(&options.network)?;
    if !options.allow_unverified {
        return Err(
            "deployment registration requires explicit unverified-root acknowledgement; pass --allow-unverified"
                .into(),
        );
    }
    Principal::from_text(&options.root_canister_id).map_err(|err| {
        format!(
            "invalid root principal for deployment {}: {err}",
            options.deployment_name
        )
    })?;

    let workspace_root = match options.workspace_root {
        Some(path) => path,
        None => workspace_root()?,
    };
    let icp_root = match options.icp_root {
        Some(path) => path,
        None => icp_root()?,
    };
    let artifact_root = resolve_artifact_root(&icp_root, &options.network).unwrap_or_else(|_| {
        icp_root
            .join(".icp")
            .join(&options.network)
            .join("canisters")
    });
    let release_set_manifest_path =
        crate::release_set::root_release_set_manifest_path(&artifact_root)
            .unwrap_or_else(|_| artifact_root.join("root").join("root.release-set.json"));
    let timestamp = current_unix_secs()?;
    let state = InstallState {
        schema_version: INSTALL_STATE_SCHEMA_VERSION,
        deployment_name: options.deployment_name,
        fleet_template: options.fleet_template.clone(),
        created_at_unix_secs: timestamp,
        updated_at_unix_secs: timestamp,
        network: options.network.clone(),
        root_target: options.root_canister_id.clone(),
        root_canister_id: options.root_canister_id,
        root_verification: RootVerificationStatus::NotVerified,
        root_build_target: "root".to_string(),
        workspace_root: workspace_root.display().to_string(),
        icp_root: icp_root.display().to_string(),
        config_path: workspace_root
            .join("fleets")
            .join(&options.fleet_template)
            .join("canic.toml")
            .display()
            .to_string(),
        release_set_manifest_path: release_set_manifest_path.display().to_string(),
    };

    write_install_state(&icp_root, &options.network, &state)
}

/// Promote an explicitly registered deployment root from `not_verified` to
/// `verified` using bound deployment-truth evidence.
pub fn verify_registered_deployment_root(
    options: VerifyDeploymentRootOptions,
) -> Result<DeploymentRootVerificationReceiptV1, Box<dyn std::error::Error>> {
    validate_state_name(&options.deployment_name)?;
    validate_network_name(&options.network)?;
    let verified_at_unix_secs = match options.verified_at_unix_secs {
        Some(value) => value,
        None => current_unix_secs()?,
    };
    let icp_root = match options.icp_root {
        Some(path) => path,
        None => icp_root()?,
    };
    let state_path =
        deployment_install_state_path(&icp_root, &options.network, &options.deployment_name);
    let state =
        read_deployment_install_state(&icp_root, &options.network, &options.deployment_name)?
            .ok_or_else(|| {
                format!(
                    "no local deployment state exists for {}; run canic deploy register first",
                    options.deployment_name
                )
            })?;
    let state_fleet_template = state.fleet_template.clone();
    let state_root_canister_id = state.root_canister_id.clone();
    let local_state_digest_before = file_sha256_hex(&state_path)?;
    let previous_root_verification = deployment_root_verification_state(&state.root_verification);
    let report =
        deployment_root_verification_report_from_check(DeploymentRootVerificationRequestV1 {
            report_id: format!(
                "local:{}:{}:root-verification-report",
                options.network, options.deployment_name
            ),
            requested_at: format!("unix:{verified_at_unix_secs}"),
            deployment_name: options.deployment_name.clone(),
            network: options.network.clone(),
            expected_fleet_template: state.fleet_template.clone(),
            expected_root_principal: state.root_canister_id.clone(),
            current_root_verification: previous_root_verification,
            source: DeploymentRootVerificationSourceV1::DeploymentTruthCheck,
            deployment_check: options.deployment_check,
        });
    validate_deployment_root_verification_report(&report)?;
    if report.evidence_status != DeploymentRootVerificationEvidenceStatusV1::EvidenceSatisfied {
        return Err(format!(
            "deployment root verification failed for {}: {} blocker(s)",
            options.deployment_name,
            report.blockers.len()
        )
        .into());
    }
    let state_transition = verified_root_state_transition(previous_root_verification);
    let local_state_digest_after = match previous_root_verification {
        DeploymentRootVerificationStateV1::NotVerified => {
            let mut verified_state = state;
            verified_state.root_verification = RootVerificationStatus::Verified;
            verified_state.updated_at_unix_secs = verified_at_unix_secs;
            write_verified_root_state_if_unchanged(
                &icp_root,
                &options.network,
                &verified_state,
                &local_state_digest_before,
            )?
        }
        DeploymentRootVerificationStateV1::Verified => file_sha256_hex(&state_path)?,
    };

    root_verification_receipt_from_report(RootVerificationReceiptInput {
        deployment_name: options.deployment_name,
        network: options.network,
        fleet_template: state_fleet_template,
        root_principal: state_root_canister_id,
        previous_root_verification,
        state_transition,
        report,
        verified_at_unix_secs,
        local_state_path: state_path.display().to_string(),
        local_state_digest_before,
        local_state_digest_after,
    })
}

struct PreparedInstallTruth {
    root_canister_id: String,
    deployment_truth_check: DeploymentCheckV1,
    timings: InstallTimingSummary,
}

struct CurrentInstallTruthInputs {
    workspace_root: PathBuf,
    icp_root: PathBuf,
    config_path: PathBuf,
    deployment_name: String,
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

fn validate_plan_artifacts_with_phase(
    plan: &DeploymentPlanV1,
    icp_root: &Path,
    network: &str,
) -> Result<(CompletedInstallPhase, Duration), Box<dyn std::error::Error>> {
    let started_at = current_unix_timestamp_label()?;
    let started = Instant::now();
    validate_plan_artifact_paths(plan, icp_root, network)?;
    let duration = started.elapsed();
    let role_names = plan
        .role_artifacts
        .iter()
        .map(|artifact| artifact.role.clone())
        .collect::<Vec<_>>();
    let phase = CompletedInstallPhase {
        phase: "materialize_artifacts",
        attempted_action: "validate supplied deployment plan artifacts",
        started_at,
        finished_at: Some(current_unix_timestamp_label()?),
        evidence: vec![format!("deployment_plan:{}", plan.plan_id)],
        role_names,
    };
    Ok((phase, duration))
}

fn validate_plan_artifact_paths(
    plan: &DeploymentPlanV1,
    icp_root: &Path,
    network: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let artifact_root = resolve_artifact_root(icp_root, network)?;
    let root_artifact = plan
        .role_artifacts
        .iter()
        .find(|artifact| artifact.role == "root")
        .ok_or_else(|| "deployment plan is missing root role artifact".to_string())?;
    let root_wasm = plan_role_wasm_path(icp_root, &artifact_root, root_artifact);
    if !root_wasm.is_file() {
        return Err(format!(
            "deployment plan root wasm artifact does not exist: {}",
            root_wasm.display()
        )
        .into());
    }

    for artifact in plan_release_role_artifacts(plan) {
        let wasm_gz = plan_role_wasm_gz_path(icp_root, &artifact_root, artifact);
        if !wasm_gz.is_file() {
            return Err(format!(
                "deployment plan role {} wasm.gz artifact does not exist: {}",
                artifact.role,
                wasm_gz.display()
            )
            .into());
        }
    }
    Ok(())
}

fn emit_manifest_with_deployment_truth_receipt(
    workspace_root: &Path,
    icp_root: &Path,
    options: &InstallRootOptions,
    config_path: &Path,
    deployment_name: &str,
    deployment_truth_check: &DeploymentCheckV1,
    execution_context: &DeploymentExecutionContextV1,
) -> Result<(PathBuf, Duration), Box<dyn std::error::Error>> {
    let operation =
        EmitRootManifestOperation::new(workspace_root, icp_root, &options.network, config_path);
    let emit_manifest_started_at_label = current_unix_timestamp_label()?;
    let emit_manifest_started_at = Instant::now();
    let manifest_path = if let Some(plan) = &options.deployment_plan_override {
        emit_root_release_set_manifest_from_plan(icp_root, &options.network, plan)?
    } else {
        operation.execute()?
    };
    let emit_manifest_duration = emit_manifest_started_at.elapsed();
    let emit_manifest_receipt = receipt_with_execution_context(
        install_deployment_truth_phase_receipt(
            deployment_truth_check,
            "emit_manifest",
            emit_manifest_started_at_label,
            Some(current_unix_timestamp_label()?),
            "emit root release-set manifest",
            crate::deployment_truth::ObservationStatusV1::Observed,
            EmitRootManifestOperation::evidence(&manifest_path),
        ),
        execution_context,
    );
    let emit_manifest_receipt_path = write_install_deployment_truth_receipt(
        icp_root,
        &options.network,
        deployment_name,
        &emit_manifest_receipt,
    )?;
    println!(
        "Deployment truth receipt JSON: {}",
        emit_manifest_receipt_path.display()
    );
    Ok((manifest_path, emit_manifest_duration))
}

fn emit_root_release_set_manifest_from_plan(
    icp_root: &Path,
    network: &str,
    plan: &DeploymentPlanV1,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let artifact_root = resolve_artifact_root(icp_root, network)?;
    let manifest_path = crate::release_set::root_release_set_manifest_path(&artifact_root)?;
    let entries = plan_release_role_artifacts(plan)
        .map(|artifact| release_set_entry_from_plan_artifact(icp_root, &artifact_root, artifact))
        .collect::<Result<Vec<_>, _>>()?;
    let manifest = RootReleaseSetManifest {
        release_version: plan
            .deployment_identity
            .canic_version
            .clone()
            .unwrap_or_else(|| plan.plan_id.clone()),
        entries,
    };

    fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;
    Ok(manifest_path)
}

fn release_set_entry_from_plan_artifact(
    icp_root: &Path,
    artifact_root: &Path,
    artifact: &crate::deployment_truth::RoleArtifactV1,
) -> Result<crate::release_set::ReleaseSetEntry, Box<dyn std::error::Error>> {
    let artifact_path = plan_role_wasm_gz_path(icp_root, artifact_root, artifact);
    let artifact_relative_path = artifact_path
        .strip_prefix(icp_root)
        .map_err(|_| {
            format!(
                "deployment plan artifact {} is not under ICP root {}",
                artifact_path.display(),
                icp_root.display()
            )
        })?
        .to_string_lossy()
        .to_string();
    let wasm_module = fs::read(&artifact_path)?;
    let chunk_hashes = wasm_module
        .chunks(CANIC_WASM_CHUNK_BYTES)
        .map(wasm_hash_hex)
        .collect::<Vec<_>>();

    Ok(crate::release_set::ReleaseSetEntry {
        role: artifact.role.clone(),
        template_id: format!("embedded:{}", artifact.role),
        artifact_relative_path,
        payload_size_bytes: wasm_module.len() as u64,
        payload_sha256_hex: wasm_hash_hex(&wasm_module),
        chunk_size_bytes: CANIC_WASM_CHUNK_BYTES as u64,
        chunk_sha256_hex: chunk_hashes,
    })
}

fn plan_release_role_artifacts(
    plan: &DeploymentPlanV1,
) -> impl Iterator<Item = &crate::deployment_truth::RoleArtifactV1> {
    plan.role_artifacts
        .iter()
        .filter(|artifact| !matches!(artifact.role.as_str(), "root" | "wasm_store"))
}

fn plan_role_wasm_path(
    icp_root: &Path,
    artifact_root: &Path,
    artifact: &crate::deployment_truth::RoleArtifactV1,
) -> PathBuf {
    artifact.wasm_path.as_ref().map_or_else(
        || {
            artifact_root
                .join(&artifact.role)
                .join(format!("{}.wasm", artifact.role))
        },
        |path| plan_artifact_path(icp_root, path),
    )
}

fn plan_role_wasm_gz_path(
    icp_root: &Path,
    artifact_root: &Path,
    artifact: &crate::deployment_truth::RoleArtifactV1,
) -> PathBuf {
    artifact.wasm_gz_path.as_ref().map_or_else(
        || {
            artifact_root
                .join(&artifact.role)
                .join(format!("{}.wasm.gz", artifact.role))
        },
        |path| plan_artifact_path(icp_root, path),
    )
}

fn plan_artifact_path(icp_root: &Path, path: &str) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        path
    } else {
        icp_root.join(path)
    }
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

fn root_wasm_for_install_plan(
    icp_root: &Path,
    network: &str,
    root_build_target: &str,
    plan: Option<&DeploymentPlanV1>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let artifact_root = resolve_artifact_root(icp_root, network)?;
    if let Some(plan) = plan {
        let root_artifact = plan
            .role_artifacts
            .iter()
            .find(|artifact| artifact.role == "root")
            .ok_or_else(|| "deployment plan is missing root role artifact".to_string())?;
        return Ok(plan_role_wasm_path(icp_root, &artifact_root, root_artifact));
    }

    Ok(artifact_root
        .join(root_build_target)
        .join(format!("{root_build_target}.wasm")))
}

trait InstallPhaseOperation {
    fn phase(&self) -> &'static str;
    fn attempted_action(&self) -> &'static str;
    fn evidence(&self) -> Vec<String>;
    fn execute(&self) -> Result<(), Box<dyn std::error::Error>>;
}

struct ResolveRootCanisterOperation<'a> {
    icp_root: &'a Path,
    network: &'a str,
    root_canister: &'a str,
    config_path: &'a Path,
}

impl<'a> ResolveRootCanisterOperation<'a> {
    const fn new(
        icp_root: &'a Path,
        network: &'a str,
        root_canister: &'a str,
        config_path: &'a Path,
    ) -> Self {
        Self {
            icp_root,
            network,
            root_canister,
            config_path,
        }
    }

    fn evidence(&self, root_canister_id: &str) -> Vec<String> {
        vec![
            format!("root_target:{}", self.root_canister),
            format!("root_canister:{root_canister_id}"),
        ]
    }

    fn execute(&self) -> Result<String, Box<dyn std::error::Error>> {
        ensure_root_canister_id(
            self.icp_root,
            self.network,
            self.root_canister,
            self.config_path,
        )
    }
}

struct BuildInstallTargetsOperation<'a> {
    network: &'a str,
    build_targets: Vec<String>,
    build_profile: Option<CanisterBuildProfile>,
    config_path: &'a Path,
    icp_root: &'a Path,
}

impl<'a> BuildInstallTargetsOperation<'a> {
    const fn new(
        network: &'a str,
        build_targets: Vec<String>,
        build_profile: Option<CanisterBuildProfile>,
        config_path: &'a Path,
        icp_root: &'a Path,
    ) -> Self {
        Self {
            network,
            build_targets,
            build_profile,
            config_path,
            icp_root,
        }
    }

    fn evidence(&self) -> Vec<String> {
        self.build_targets
            .iter()
            .map(|target| format!("build_target:{target}"))
            .collect()
    }

    fn role_names(&self) -> Vec<String> {
        self.build_targets.clone()
    }

    fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        run_canic_build_targets(
            self.network,
            &self.build_targets,
            self.build_profile,
            self.config_path,
            self.icp_root,
        )
    }
}

struct EmitRootManifestOperation<'a> {
    workspace_root: &'a Path,
    icp_root: &'a Path,
    network: &'a str,
    config_path: &'a Path,
}

impl<'a> EmitRootManifestOperation<'a> {
    const fn new(
        workspace_root: &'a Path,
        icp_root: &'a Path,
        network: &'a str,
        config_path: &'a Path,
    ) -> Self {
        Self {
            workspace_root,
            icp_root,
            network,
            config_path,
        }
    }

    fn evidence(manifest_path: &Path) -> Vec<String> {
        vec![format!("manifest_path:{}", manifest_path.display())]
    }

    fn execute(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        emit_root_release_set_manifest_with_config(
            self.workspace_root,
            self.icp_root,
            self.network,
            self.config_path,
        )
    }
}

struct InstallRootWasmOperation<'a> {
    icp_root: &'a Path,
    network: &'a str,
    root_canister_id: &'a str,
    root_wasm: PathBuf,
}

impl<'a> InstallRootWasmOperation<'a> {
    const fn new(
        icp_root: &'a Path,
        network: &'a str,
        root_canister_id: &'a str,
        root_wasm: PathBuf,
    ) -> Self {
        Self {
            icp_root,
            network,
            root_canister_id,
            root_wasm,
        }
    }
}

impl InstallPhaseOperation for InstallRootWasmOperation<'_> {
    fn phase(&self) -> &'static str {
        "install_root"
    }

    fn attempted_action(&self) -> &'static str {
        "install root wasm"
    }

    fn evidence(&self) -> Vec<String> {
        vec![
            format!("root_canister:{}", self.root_canister_id),
            format!("root_wasm:{}", self.root_wasm.display()),
        ]
    }

    fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        reinstall_root_wasm(
            self.icp_root,
            self.network,
            self.root_canister_id,
            &self.root_wasm,
        )
    }
}

struct EnsureRootCyclesOperation<'a> {
    icp_root: &'a Path,
    network: &'a str,
    root_canister_id: &'a str,
    phase: &'static str,
    attempted_action: &'static str,
    phase_label: &'a str,
}

impl<'a> EnsureRootCyclesOperation<'a> {
    const fn new(
        icp_root: &'a Path,
        network: &'a str,
        root_canister_id: &'a str,
        phase: &'static str,
        attempted_action: &'static str,
        phase_label: &'a str,
    ) -> Self {
        Self {
            icp_root,
            network,
            root_canister_id,
            phase,
            attempted_action,
            phase_label,
        }
    }
}

impl InstallPhaseOperation for EnsureRootCyclesOperation<'_> {
    fn phase(&self) -> &'static str {
        self.phase
    }

    fn attempted_action(&self) -> &'static str {
        self.attempted_action
    }

    fn evidence(&self) -> Vec<String> {
        vec![
            format!("root_canister:{}", self.root_canister_id),
            format!("minimum_cycles:{LOCAL_ROOT_MIN_READY_CYCLES}"),
            format!("funding_phase:{}", self.phase_label),
        ]
    }

    fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        ensure_local_root_min_cycles(
            self.icp_root,
            self.network,
            self.root_canister_id,
            self.phase_label,
        )
    }
}

struct ResumeBootstrapOperation<'a> {
    network: &'a str,
    root_canister_id: &'a str,
}

impl<'a> ResumeBootstrapOperation<'a> {
    const fn new(network: &'a str, root_canister_id: &'a str) -> Self {
        Self {
            network,
            root_canister_id,
        }
    }
}

impl InstallPhaseOperation for ResumeBootstrapOperation<'_> {
    fn phase(&self) -> &'static str {
        "resume_bootstrap"
    }

    fn attempted_action(&self) -> &'static str {
        "resume root bootstrap"
    }

    fn evidence(&self) -> Vec<String> {
        vec![format!("root_canister:{}", self.root_canister_id)]
    }

    fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        resume_root_bootstrap(self.network, self.root_canister_id)
    }
}

struct WaitRootReadyOperation<'a> {
    network: &'a str,
    root_canister_id: &'a str,
    timeout_seconds: u64,
}

impl<'a> WaitRootReadyOperation<'a> {
    const fn new(network: &'a str, root_canister_id: &'a str, timeout_seconds: u64) -> Self {
        Self {
            network,
            root_canister_id,
            timeout_seconds,
        }
    }
}

impl InstallPhaseOperation for WaitRootReadyOperation<'_> {
    fn phase(&self) -> &'static str {
        "wait_ready"
    }

    fn attempted_action(&self) -> &'static str {
        "wait for root bootstrap readiness"
    }

    fn evidence(&self) -> Vec<String> {
        vec![
            format!("root_canister:{}", self.root_canister_id),
            format!("timeout_seconds:{}", self.timeout_seconds),
        ]
    }

    fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        wait_for_root_ready(self.network, self.root_canister_id, self.timeout_seconds)
    }
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

fn write_artifact_promotion_execution_receipt_for_install(
    options: &InstallRootOptions,
    icp_root: &Path,
    network: &str,
    deployment_name: &str,
    check: &DeploymentCheckV1,
    execution_context: &DeploymentExecutionContextV1,
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    let Some(promotion_plan) = &options.artifact_promotion_plan_override else {
        return Ok(None);
    };
    let deployment_receipt =
        promotion_install_deployment_receipt(check, execution_context, promotion_plan)?;
    let provenance_report =
        artifact_promotion_provenance_report(ArtifactPromotionProvenanceReportRequest {
            report_id: format!("{}:execution-provenance", promotion_plan.plan_id),
            artifact_promotion_plan: promotion_plan.clone(),
            wasm_store_identity_report: None,
            wasm_store_catalog_verification: None,
            materialization_identity_report: None,
        })?;
    let receipt = artifact_promotion_execution_receipt(ArtifactPromotionExecutionReceiptRequest {
        receipt_id: format!("{}:execution-receipt", promotion_plan.plan_id),
        provenance_report,
        deployment_receipt,
    })?;
    let path =
        write_artifact_promotion_execution_receipt(icp_root, network, deployment_name, &receipt)?;
    println!(
        "Artifact promotion execution receipt JSON: {}",
        path.display()
    );
    Ok(Some(path))
}

fn promotion_install_deployment_receipt(
    check: &DeploymentCheckV1,
    execution_context: &DeploymentExecutionContextV1,
    promotion_plan: &ArtifactPromotionPlanV1,
) -> Result<DeploymentReceiptV1, Box<dyn std::error::Error>> {
    let started_at = current_unix_timestamp_label()?;
    let finished_at = current_unix_timestamp_label()?;
    let phase = phase_receipt(
        "promoted_plan_install",
        started_at.clone(),
        Some(finished_at.clone()),
        "execute promoted deployment plan through current install runner",
        ObservationStatusV1::Observed,
        vec![
            format!("artifact_promotion_plan:{}", promotion_plan.plan_id),
            format!(
                "artifact_promotion_plan_digest:{}",
                promotion_plan.artifact_promotion_plan_digest
            ),
            format!(
                "promotion_plan_lineage_digest:{}",
                promotion_plan.promotion_plan_lineage_digest
            ),
        ],
    );
    let role_phase_receipts = promotion_plan
        .transform
        .roles
        .iter()
        .map(|role| {
            completed_phase_role_receipt(
                check,
                "promoted_plan_install",
                &role.role,
                crate::deployment_truth::RolePhaseResultV1::Applied,
                None,
            )
            .ok_or_else(|| {
                format!(
                    "promoted role {} is missing from deployment plan artifacts",
                    role.role
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(receipt_with_execution_context(
        deployment_receipt_from_check_with_status(
            check,
            format!("{}:promoted_plan_install", check.check_id),
            DeploymentExecutionStatusV1::Complete,
            started_at,
            Some(finished_at),
            vec![phase],
            role_phase_receipts,
            DeploymentCommandResultV1::Succeeded,
        ),
        execution_context,
    ))
}

/// Build the same read-only deployment truth check that can be used as a
/// preflight for the current install inputs without mutating deployment state.
pub fn check_install_deployment_truth(
    options: &InstallRootOptions,
    observed_at: impl Into<String>,
) -> Result<DeploymentCheckV1, Box<dyn std::error::Error>> {
    let inputs = resolve_current_install_truth_inputs(options)?;
    current_install_deployment_truth_check_at(
        options,
        &inputs.workspace_root,
        &inputs.icp_root,
        &inputs.config_path,
        &inputs.deployment_name,
        observed_at.into(),
    )
}

/// Build a read-only execution preflight for the current install inputs.
///
/// This validates the current plan, safety report, authority reconciliation,
/// and executor capabilities without opening the mutating install path or
/// writing local receipt state.
pub fn check_install_execution_preflight(
    options: &InstallRootOptions,
    observed_at: impl Into<String>,
) -> Result<DeploymentExecutionPreflightV1, Box<dyn std::error::Error>> {
    let inputs = resolve_current_install_truth_inputs(options)?;
    let check = current_install_deployment_truth_check_at(
        options,
        &inputs.workspace_root,
        &inputs.icp_root,
        &inputs.config_path,
        &inputs.deployment_name,
        observed_at.into(),
    )?;
    let execution_context = current_install_execution_context(
        &inputs.workspace_root,
        &inputs.icp_root,
        &options.network,
    );
    let executor = CurrentCliDeploymentExecutor::new(
        execution_context.workspace_root,
        execution_context.icp_root,
        execution_context.artifact_roots,
    );
    let preflight = deployment_execution_preflight_from_check(
        &check,
        &executor,
        CURRENT_INSTALL_REQUIRED_CAPABILITIES,
    );
    validate_deployment_execution_preflight_for_check(&check, &preflight)?;
    Ok(preflight)
}

fn resolve_current_install_truth_inputs(
    options: &InstallRootOptions,
) -> Result<CurrentInstallTruthInputs, Box<dyn std::error::Error>> {
    let workspace_root = workspace_root()?;
    let icp_root = match &options.icp_root {
        Some(path) => path.canonicalize()?,
        None => icp_root()?,
    };
    let state = match options.deployment_name.as_deref() {
        Some(deployment) => {
            read_named_deployment_install_state_from_root(&icp_root, &options.network, deployment)?
        }
        None => None,
    };
    let config_path = match (options.config_path.as_deref(), state.as_ref()) {
        (Some(path), _) => resolve_install_config_path(
            &icp_root,
            Some(path),
            options.interactive_config_selection,
        )?,
        (None, Some(state)) => resolve_install_config_path(
            &icp_root,
            Some(&state.config_path),
            options.interactive_config_selection,
        )?,
        (None, None) => {
            let default_config = options
                .deployment_name
                .as_ref()
                .map(|deployment| default_config_path_for_deployment(deployment));
            resolve_install_config_path(
                &icp_root,
                default_config.as_deref(),
                options.interactive_config_selection,
            )?
        }
    };
    let fleet_template = configured_fleet_name(&config_path)?;
    let expected_fleet = options
        .expected_fleet
        .as_deref()
        .or_else(|| state.as_ref().map(|state| state.fleet_template.as_str()));
    validate_expected_fleet_name(expected_fleet, &fleet_template, &config_path)?;
    validate_state_name(&fleet_template)?;
    let deployment_name = options
        .deployment_name
        .clone()
        .unwrap_or_else(|| fleet_template.clone());
    validate_state_name(&deployment_name)?;
    Ok(CurrentInstallTruthInputs {
        workspace_root,
        icp_root,
        config_path,
        deployment_name,
    })
}

fn default_config_path_for_deployment(deployment: &str) -> String {
    format!("fleets/{deployment}/canic.toml")
}

fn current_install_deployment_truth_check_at(
    options: &InstallRootOptions,
    workspace_root: &Path,
    icp_root: &Path,
    config_path: &Path,
    deployment_name: &str,
    observed_at: String,
) -> Result<DeploymentCheckV1, Box<dyn std::error::Error>> {
    if let Some(plan) = &options.deployment_plan_override {
        validate_current_install_plan_override(plan, &options.network, deployment_name)?;
        return current_install_deployment_truth_check_for_plan(
            plan,
            workspace_root,
            icp_root,
            config_path,
            deployment_name,
            observed_at,
            &options.network,
        );
    }

    let build_profile = options
        .build_profile
        .unwrap_or_else(CanisterBuildProfile::current)
        .target_dir_name()
        .to_string();

    check_local_deployment(&LocalDeploymentCheckRequest {
        deployment_name: deployment_name.to_string(),
        network: options.network.clone(),
        workspace_root: workspace_root.to_path_buf(),
        icp_root: icp_root.to_path_buf(),
        config_path: Some(config_path.to_path_buf()),
        observed_at,
        runtime_variant: options.network.clone(),
        build_profile,
    })
    .map_err(Into::into)
}

fn current_install_deployment_truth_check_for_plan(
    plan: &DeploymentPlanV1,
    workspace_root: &Path,
    icp_root: &Path,
    config_path: &Path,
    deployment_name: &str,
    observed_at: String,
    network: &str,
) -> Result<DeploymentCheckV1, Box<dyn std::error::Error>> {
    let inventory = collect_local_deployment_inventory(&LocalInventoryRequest {
        deployment_name: deployment_name.to_string(),
        network: network.to_string(),
        workspace_root: workspace_root.to_path_buf(),
        icp_root: icp_root.to_path_buf(),
        config_path: Some(config_path.to_path_buf()),
        observed_at,
    })?;
    let diff = compare_plan_to_inventory(plan, &inventory);
    let report = safety_report_from_diff(
        format!("local:{network}:{deployment_name}:report"),
        Some(format!("local:{network}:{deployment_name}:diff")),
        &diff,
    );

    Ok(DeploymentCheckV1 {
        schema_version: crate::deployment_truth::DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        check_id: format!("local:{network}:{deployment_name}:check"),
        plan: plan.clone(),
        inventory,
        diff,
        report,
    })
}

fn validate_current_install_plan_override(
    plan: &DeploymentPlanV1,
    network: &str,
    deployment_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if plan.schema_version != crate::deployment_truth::DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(format!(
            "deployment plan schema mismatch: expected {}, found {}",
            crate::deployment_truth::DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            plan.schema_version
        )
        .into());
    }
    if plan.deployment_identity.network != network {
        return Err(format!(
            "deployment plan network mismatch: install network {network}, plan network {}",
            plan.deployment_identity.network
        )
        .into());
    }
    if plan.deployment_identity.deployment_name != deployment_name {
        return Err(format!(
            "deployment plan target mismatch: install deployment {deployment_name}, plan deployment {}",
            plan.deployment_identity.deployment_name
        )
        .into());
    }
    Ok(())
}

fn current_install_execution_context(
    workspace_root: &Path,
    icp_root: &Path,
    network: &str,
) -> DeploymentExecutionContextV1 {
    CurrentCliDeploymentExecutor::new(
        Some(workspace_root.display().to_string()),
        Some(icp_root.display().to_string()),
        current_install_artifact_roots(icp_root, network),
    )
    .execution_context()
}

fn ensure_current_install_executor_capabilities(
    execution_context: &DeploymentExecutionContextV1,
) -> Result<(), Box<dyn std::error::Error>> {
    let missing = current_install_executor_missing_capabilities(execution_context);
    if missing.is_empty() {
        return Ok(());
    }

    Err(format!(
        "current install executor backend {:?} is missing required capabilities: {missing:?}",
        execution_context.backend
    )
    .into())
}

fn current_install_executor_missing_capabilities(
    execution_context: &DeploymentExecutionContextV1,
) -> Vec<DeploymentExecutorCapabilityV1> {
    missing_executor_capabilities(
        &execution_context.backend_capabilities,
        CURRENT_INSTALL_REQUIRED_CAPABILITIES,
    )
}

fn current_install_artifact_roots(icp_root: &Path, network: &str) -> Vec<String> {
    let planned_root = planned_build_artifact_root(icp_root);
    let mut roots = vec![planned_root.display().to_string()];
    if let Ok(resolved_root) = resolve_artifact_root(icp_root, network)
        && resolved_root != planned_root
    {
        roots.push(resolved_root.display().to_string());
    }
    roots
}

fn run_install_deployment_truth_safety_gate(
    options: &InstallRootOptions,
    workspace_root: &Path,
    icp_root: &Path,
    config_path: &Path,
    deployment_name: &str,
    execution_context: &DeploymentExecutionContextV1,
) -> Result<DeploymentCheckV1, Box<dyn std::error::Error>> {
    let truth_gate_started_at = current_unix_timestamp_label()?;
    let deployment_truth_check = current_install_deployment_truth_check_at(
        options,
        workspace_root,
        icp_root,
        config_path,
        deployment_name,
        truth_gate_started_at.clone(),
    )?;
    let artifact_gate_receipt = artifact_gate_phase_receipt(
        &deployment_truth_check,
        truth_gate_started_at.clone(),
        Some(current_unix_timestamp_label()?),
    );
    let role_receipts = artifact_gate_role_phase_receipts(&deployment_truth_check);
    let deployment_receipt = receipt_with_execution_context(
        install_deployment_truth_gate_receipt(
            &deployment_truth_check,
            truth_gate_started_at,
            vec![artifact_gate_receipt],
            role_receipts,
        ),
        execution_context,
    );
    let receipt_write = write_install_deployment_truth_receipt(
        icp_root,
        &options.network,
        deployment_name,
        &deployment_receipt,
    );
    match &receipt_write {
        Ok(path) => println!("Deployment truth receipt JSON: {}", path.display()),
        Err(err) => eprintln!("Deployment truth receipt JSON write failed: {err}"),
    }
    print_install_deployment_truth_gate(&deployment_truth_check, &deployment_receipt);
    enforce_install_deployment_truth_gate(&deployment_truth_check)?;
    receipt_write?;
    write_current_install_execution_preflight_receipt(
        icp_root,
        &options.network,
        deployment_name,
        &deployment_truth_check,
        execution_context,
    )?;
    Ok(deployment_truth_check)
}

fn validate_expected_fleet_name(
    expected: Option<&str>,
    actual: &str,
    config_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(expected) = expected else {
        return Ok(());
    };
    if expected == actual {
        return Ok(());
    }
    Err(format!(
        "install requested fleet {expected}, but {} declares [fleet].name = {actual:?}",
        config_path.display()
    )
    .into())
}

fn ensure_root_canister_id(
    icp_root: &Path,
    network: &str,
    root_canister: &str,
    config_path: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    if Principal::from_text(root_canister).is_ok() {
        return Ok(root_canister.to_string());
    }

    match resolve_root_canister_id(icp_root, network, root_canister) {
        Ok(canister_id) => return Ok(canister_id),
        Err(err) if !is_missing_canister_id_error(&err.to_string()) => return Err(err),
        Err(_) => {}
    }

    let mut create = icp_canister_command_in_network(icp_root);
    add_create_root_target(&mut create, root_canister);
    add_local_root_create_cycles_arg(&mut create, config_path, network)?;
    add_icp_environment_target(&mut create, network);
    let output = run_command_stdout(&mut create)?;
    if let Some(canister_id) = parse_created_canister_id(&output) {
        return Ok(canister_id);
    }

    resolve_root_canister_id(icp_root, network, root_canister).map_err(|_| {
        format!(
            "created root canister target '{root_canister}', but ICP CLI still has no canister ID for environment '{network}' under ICP root {}\nExpected project-local state under {}/.icp/{network}. If another foreground replica is reachable, stop it and restart with `canic replica start --background` from this Canic project.",
            icp_root.display(),
            icp_root.display(),
        )
        .into()
    })
}
fn reinstall_root_wasm(
    icp_root: &Path,
    network: &str,
    root_canister: &str,
    root_wasm: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut install = icp_canister_command_in_network(icp_root);
    install.args(["install", root_canister, "--mode=reinstall", "-y", "--wasm"]);
    install.arg(root_wasm);
    install.args(["--args", &root_init_args(root_wasm)?]);
    add_icp_environment_target(&mut install, network);
    run_command(&mut install)
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

// Resolve the installed root id, accepting principal targets without a icp lookup.
fn resolve_root_canister_id(
    icp_root: &Path,
    network: &str,
    root_canister: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    if Principal::from_text(root_canister).is_ok() {
        return Ok(root_canister.to_string());
    }

    let mut command = icp_canister_command_in_network(icp_root);
    command.args(["status", root_canister, "--json"]);
    add_icp_environment_target(&mut command, network);
    let output = run_command_stdout(&mut command)?;
    parse_created_canister_id(&output).ok_or_else(|| {
        format!("could not parse root canister id from ICP status JSON output: {output}").into()
    })
}

// Read the current host clock as a unix timestamp for install state.
fn current_unix_secs() -> Result<u64, Box<dyn std::error::Error>> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

fn current_unix_timestamp_label() -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!("unix:{}", current_unix_secs()?))
}
