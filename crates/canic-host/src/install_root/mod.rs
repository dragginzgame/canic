use crate::canister_build::{
    CanisterBuildProfile, build_current_workspace_canister_artifact,
    current_workspace_build_context_once,
};
use crate::deployment_truth::{
    ArtifactPromotionExecutionReceiptRequest, ArtifactPromotionExecutionReceiptV1,
    ArtifactPromotionPlanV1, ArtifactPromotionProvenanceReportRequest, ArtifactTransportV1,
    CurrentCliDeploymentExecutor, DeploymentCheckV1, DeploymentCommandResultV1,
    DeploymentExecutionContextV1, DeploymentExecutionPreflightV1, DeploymentExecutionStatusV1,
    DeploymentExecutor, DeploymentExecutorCapabilityV1, DeploymentPlanV1, DeploymentReceiptV1,
    DeploymentRootVerificationEvidenceStatusV1, DeploymentRootVerificationReceiptV1,
    DeploymentRootVerificationReportV1, DeploymentRootVerificationRequestV1,
    DeploymentRootVerificationSourceV1, DeploymentRootVerificationStateTransitionV1,
    DeploymentRootVerificationStateV1, LocalDeploymentCheckRequest, LocalInventoryRequest,
    ObservationStatusV1, SafetyFindingV1, StagingReceiptV1, artifact_gate_phase_receipt,
    artifact_gate_role_phase_receipts, artifact_promotion_execution_receipt,
    artifact_promotion_provenance_report, check_local_deployment,
    collect_local_deployment_inventory, compare_plan_to_inventory,
    deployment_execution_preflight_from_check, deployment_receipt_from_check_with_status,
    deployment_root_verification_receipt_digest, deployment_root_verification_report_from_check,
    missing_executor_capabilities, phase_receipt, safety_report_from_diff,
    staging_receipt_evidence, validate_deployment_execution_preflight_for_check,
    validate_deployment_root_verification_receipt, validate_deployment_root_verification_report,
};
use crate::format::wasm_size_label;
use crate::icp::{self, CANIC_ICP_LOCAL_NETWORK_URL_ENV, CANIC_ICP_LOCAL_ROOT_KEY_ENV};
use crate::release_set::{
    LOCAL_ROOT_MIN_READY_CYCLES, RootReleaseSetManifest, configured_fleet_name,
    configured_install_targets, configured_local_root_create_cycles,
    emit_root_release_set_manifest_with_config, icp_query_on_network, icp_root,
    load_root_release_set_manifest, resolve_artifact_root, resume_root_bootstrap,
    stage_root_release_set, workspace_root,
};
use crate::replica_query;
use crate::response_parse::parse_cycle_balance_response;
use crate::table::{ColumnAlign, render_separator, render_table, render_table_row, table_widths};
use canic_core::cdk::utils::hash::wasm_hash_hex;
use canic_core::{
    CANIC_WASM_CHUNK_BYTES,
    cdk::{types::Principal, utils::hash::wasm_hash},
    protocol,
};
use config_selection::resolve_install_config_path;
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};
use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

mod config_selection;
mod readiness;
mod state;

pub use config_selection::{
    current_canic_project_root, discover_canic_config_choices, discover_canic_project_root_from,
    discover_project_canic_config_choices, project_fleet_roots,
};
use readiness::wait_for_root_ready;
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
use config_selection::config_selection_error;
#[cfg(test)]
use readiness::{parse_bootstrap_status_value, parse_root_ready_value};
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

struct RootVerificationReceiptInput {
    deployment_name: String,
    network: String,
    fleet_template: String,
    root_principal: String,
    previous_root_verification: DeploymentRootVerificationStateV1,
    state_transition: DeploymentRootVerificationStateTransitionV1,
    report: DeploymentRootVerificationReportV1,
    verified_at_unix_secs: u64,
    local_state_path: String,
    local_state_digest_before: String,
    local_state_digest_after: String,
}

fn root_verification_receipt_from_report(
    input: RootVerificationReceiptInput,
) -> Result<DeploymentRootVerificationReceiptV1, Box<dyn std::error::Error>> {
    let source_root_observation_source = input.report.observed_root_observation_source.ok_or(
        "deployment root verification report did not preserve observed root source evidence",
    )?;
    let source_observed_root_canister_id =
        input.report.observed_root_canister_id.clone().ok_or(
            "deployment root verification report did not preserve observed root canister id",
        )?;

    let mut receipt = DeploymentRootVerificationReceiptV1 {
        schema_version: crate::deployment_truth::DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        receipt_id: format!(
            "local:{}:{}:root-verification-receipt",
            input.network, input.deployment_name
        ),
        receipt_digest: String::new(),
        deployment_name: input.deployment_name,
        network: input.network,
        fleet_template: input.fleet_template,
        root_principal: input.root_principal,
        previous_root_verification: input.previous_root_verification,
        new_root_verification: DeploymentRootVerificationStateV1::Verified,
        state_transition: input.state_transition,
        source_report_id: input.report.report_id,
        source_report_digest: input.report.report_digest,
        source_report_evidence_status: input.report.evidence_status,
        source_report_current_root_verification: input.report.current_root_verification,
        source_report_state_transition: input.report.state_transition,
        source_root_observation_source,
        source_observed_root_canister_id,
        source_check_id: input.report.source_check_id,
        source_check_digest: input.report.source_check_digest,
        source_deployment_plan_id: input.report.source_deployment_plan_id,
        source_deployment_plan_digest: input.report.source_deployment_plan_digest,
        source_inventory_id: input.report.source_inventory_id,
        source_inventory_digest: input.report.source_inventory_digest,
        verified_at_unix_secs: input.verified_at_unix_secs,
        local_state_path: input.local_state_path,
        local_state_digest_before: input.local_state_digest_before,
        local_state_digest_after: input.local_state_digest_after,
        warnings: input.report.warnings,
    };
    receipt.receipt_digest = deployment_root_verification_receipt_digest(&receipt);
    validate_deployment_root_verification_receipt(&receipt)?;
    Ok(receipt)
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

#[derive(Clone, Copy)]
struct InstallReceiptScope<'a> {
    icp_root: &'a Path,
    network: &'a str,
    deployment_name: &'a str,
    check: &'a DeploymentCheckV1,
    execution_context: Option<&'a DeploymentExecutionContextV1>,
}

struct CompletedInstallPhase {
    phase: &'static str,
    attempted_action: &'static str,
    started_at: String,
    finished_at: Option<String>,
    evidence: Vec<String>,
    role_names: Vec<String>,
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

fn write_completed_install_phase_receipt(
    receipt_scope: InstallReceiptScope<'_>,
    completed: CompletedInstallPhase,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let role_phase_receipts = completed
        .role_names
        .iter()
        .filter_map(|role| {
            completed_phase_role_receipt(
                receipt_scope.check,
                completed.phase,
                role,
                crate::deployment_truth::RolePhaseResultV1::Applied,
                None,
            )
        })
        .collect();
    let receipt =
        receipt_scope.with_execution_context(install_deployment_truth_phase_receipt_with_result(
            receipt_scope.check,
            PhaseReceiptInput {
                phase: completed.phase,
                started_at: completed.started_at,
                finished_at: completed.finished_at,
                attempted_action: completed.attempted_action,
                status: crate::deployment_truth::ObservationStatusV1::Observed,
                evidence: completed.evidence,
                role_phase_receipts,
                operation_status: DeploymentExecutionStatusV1::Complete,
                command_result: DeploymentCommandResultV1::Succeeded,
            },
        ));
    receipt_scope.write_receipt(&receipt)
}

fn completed_phase_role_receipt(
    check: &DeploymentCheckV1,
    phase: &str,
    role: &str,
    result: crate::deployment_truth::RolePhaseResultV1,
    error: Option<String>,
) -> Option<crate::deployment_truth::RolePhaseReceiptV1> {
    let planned = check
        .plan
        .role_artifacts
        .iter()
        .find(|artifact| artifact.role == role)?;
    let observed = check
        .inventory
        .observed_artifacts
        .iter()
        .find(|artifact| artifact.role == role);
    let artifact_digest = observed
        .and_then(|artifact| artifact.file_sha256.clone())
        .or_else(|| observed.and_then(|artifact| artifact.payload_sha256.clone()))
        .or_else(|| planned.observed_wasm_gz_file_sha256.clone())
        .or_else(|| planned.wasm_gz_sha256.clone());

    Some(crate::deployment_truth::RolePhaseReceiptV1 {
        role: role.to_string(),
        phase: phase.to_string(),
        result,
        previous_module_hash: None,
        target_module_hash: planned.installed_module_hash.clone(),
        observed_module_hash_after: None,
        artifact_digest,
        canonical_embedded_config_sha256: planned.canonical_embedded_config_sha256.clone(),
        error,
    })
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

impl InstallReceiptScope<'_> {
    fn run_operation(
        self,
        operation: &impl InstallPhaseOperation,
    ) -> Result<Duration, Box<dyn std::error::Error>> {
        self.run_phase(
            operation.phase(),
            operation.attempted_action(),
            operation.evidence(),
            || operation.execute(),
        )
    }

    fn run_phase(
        self,
        phase: &str,
        attempted_action: &str,
        evidence: Vec<String>,
        run: impl FnOnce() -> Result<(), Box<dyn std::error::Error>>,
    ) -> Result<Duration, Box<dyn std::error::Error>> {
        let started_at = current_unix_timestamp_label()?;
        let started = Instant::now();
        match run() {
            Ok(()) => {
                let duration = started.elapsed();
                let receipt = self.with_execution_context(install_deployment_truth_phase_receipt(
                    self.check,
                    phase,
                    started_at,
                    Some(current_unix_timestamp_label()?),
                    attempted_action,
                    crate::deployment_truth::ObservationStatusV1::Observed,
                    evidence,
                ));
                self.write_receipt(&receipt)?;
                Ok(duration)
            }
            Err(err) => {
                self.try_write_failed_phase_receipt(
                    phase,
                    started_at,
                    attempted_action,
                    evidence,
                    err.as_ref(),
                );
                Err(err)
            }
        }
    }

    fn with_execution_context(self, receipt: DeploymentReceiptV1) -> DeploymentReceiptV1 {
        match self.execution_context {
            Some(context) => receipt_with_execution_context(receipt, context),
            None => receipt,
        }
    }

    fn write_receipt(
        self,
        receipt: &DeploymentReceiptV1,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let path = write_install_deployment_truth_receipt(
            self.icp_root,
            self.network,
            self.deployment_name,
            receipt,
        )?;
        println!("Deployment truth receipt JSON: {}", path.display());
        Ok(path)
    }

    fn try_write_failed_phase_receipt(
        self,
        phase: &str,
        started_at: String,
        attempted_action: &str,
        evidence: Vec<String>,
        err: &dyn std::error::Error,
    ) {
        let receipt = install_deployment_truth_phase_receipt_with_result(
            self.check,
            PhaseReceiptInput {
                phase,
                started_at,
                finished_at: Some(
                    current_unix_timestamp_label().unwrap_or_else(|_| "unknown".to_string()),
                ),
                attempted_action,
                status: crate::deployment_truth::ObservationStatusV1::Inconclusive,
                evidence,
                role_phase_receipts: Vec::new(),
                operation_status: DeploymentExecutionStatusV1::FailedAfterMutation,
                command_result: DeploymentCommandResultV1::Failed {
                    code: format!("{phase}_failed"),
                    message: err.to_string(),
                },
            },
        );
        let receipt = self.with_execution_context(receipt);
        if let Err(write_err) = self.write_receipt(&receipt) {
            eprintln!("Deployment truth receipt JSON write failed: {write_err}");
        }
    }
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

fn enforce_install_deployment_truth_gate(
    check: &DeploymentCheckV1,
) -> Result<(), Box<dyn std::error::Error>> {
    let blockers = install_deployment_truth_gate_blockers(check);
    if blockers.is_empty() {
        return Ok(());
    }

    let details = blockers
        .iter()
        .map(|finding| deployment_truth_finding_label(finding))
        .collect::<Vec<_>>()
        .join("; ");
    Err(format!("deployment truth safety gate blocked install: {details}").into())
}

fn install_deployment_truth_gate_blockers(check: &DeploymentCheckV1) -> Vec<&SafetyFindingV1> {
    check.report.hard_failures.iter().collect()
}

fn print_install_deployment_truth_gate(check: &DeploymentCheckV1, receipt: &DeploymentReceiptV1) {
    for line in install_deployment_truth_gate_lines(check, receipt) {
        println!("{line}");
    }
}

fn install_deployment_truth_gate_lines(
    check: &DeploymentCheckV1,
    receipt: &DeploymentReceiptV1,
) -> Vec<String> {
    let mut lines = vec![
        format!("Deployment truth: {}", check.report.summary),
        format!(
            "Deployment truth receipt: operation={} status={:?}",
            receipt.operation_id, receipt.operation_status
        ),
    ];
    for phase_receipt in &receipt.phase_receipts {
        lines.push(format!(
            "Deployment truth phase receipt: phase={} postcondition={:?}",
            phase_receipt.phase, phase_receipt.verified_postcondition.status
        ));
    }
    if !receipt.role_phase_receipts.is_empty() {
        lines.push(format!(
            "Deployment truth role receipts: {}",
            receipt.role_phase_receipts.len()
        ));
    }
    for role_receipt in &receipt.role_phase_receipts {
        lines.push(format!(
            "Deployment truth role receipt: phase={} role={} result={:?}",
            role_receipt.phase, role_receipt.role, role_receipt.result
        ));
    }

    if !check.report.hard_failures.is_empty() {
        lines.push(format!(
            "Deployment truth hard failures: {}",
            check.report.hard_failures.len()
        ));
    }
    for finding in install_deployment_truth_gate_blockers(check) {
        lines.push(format!(
            "Deployment truth blocker: {}",
            deployment_truth_finding_label(finding)
        ));
    }
    if !check.report.warnings.is_empty() {
        lines.push(format!(
            "Deployment truth warnings: {}",
            check.report.warnings.len()
        ));
    }
    for finding in &check.report.warnings {
        lines.push(format!(
            "Deployment truth warning: {}",
            deployment_truth_finding_label(finding)
        ));
    }
    lines
}

fn install_deployment_truth_gate_receipt(
    check: &DeploymentCheckV1,
    started_at: String,
    phase_receipts: Vec<crate::deployment_truth::PhaseReceiptV1>,
    role_phase_receipts: Vec<crate::deployment_truth::RolePhaseReceiptV1>,
) -> DeploymentReceiptV1 {
    let blockers = install_deployment_truth_gate_blockers(check);
    let (operation_status, command_result) = if blockers.is_empty() {
        (
            DeploymentExecutionStatusV1::Complete,
            DeploymentCommandResultV1::Succeeded,
        )
    } else {
        (
            DeploymentExecutionStatusV1::FailedBeforeMutation,
            DeploymentCommandResultV1::Failed {
                code: "deployment_truth_blocked".to_string(),
                message: check.report.summary.clone(),
            },
        )
    };
    deployment_receipt_from_check_with_status(
        check,
        format!("{}:materialize_artifacts", check.check_id),
        operation_status,
        started_at,
        Some(current_unix_timestamp_label().unwrap_or_else(|_| "unknown".to_string())),
        phase_receipts,
        role_phase_receipts,
        command_result,
    )
}

fn write_current_install_execution_preflight_receipt(
    icp_root: &Path,
    network: &str,
    deployment_name: &str,
    check: &DeploymentCheckV1,
    execution_context: &DeploymentExecutionContextV1,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let started_at = current_unix_timestamp_label()?;
    let executor = CurrentCliDeploymentExecutor::new(
        execution_context.workspace_root.clone(),
        execution_context.icp_root.clone(),
        execution_context.artifact_roots.clone(),
    );
    let preflight = deployment_execution_preflight_from_check(
        check,
        &executor,
        CURRENT_INSTALL_REQUIRED_CAPABILITIES,
    );
    validate_deployment_execution_preflight_for_check(check, &preflight)?;
    let blockers = preflight.blockers.clone();
    let (operation_status, command_result) = if blockers.is_empty() {
        (
            DeploymentExecutionStatusV1::Complete,
            DeploymentCommandResultV1::Succeeded,
        )
    } else {
        (
            DeploymentExecutionStatusV1::FailedBeforeMutation,
            DeploymentCommandResultV1::Failed {
                code: "execution_preflight_blocked".to_string(),
                message: "deployment execution preflight blocked current install".to_string(),
            },
        )
    };
    let finished_at = current_unix_timestamp_label()?;
    let receipt = receipt_with_execution_context(
        deployment_receipt_from_check_with_status(
            check,
            format!("{}:execution_preflight", check.check_id),
            operation_status,
            started_at.clone(),
            Some(finished_at.clone()),
            vec![phase_receipt(
                "execution_preflight",
                started_at,
                Some(finished_at),
                "validate deployment plan, authority, and executor capability readiness",
                crate::deployment_truth::ObservationStatusV1::Observed,
                current_install_execution_preflight_evidence(&preflight),
            )],
            Vec::new(),
            command_result,
        ),
        execution_context,
    );
    let path =
        write_install_deployment_truth_receipt(icp_root, network, deployment_name, &receipt)?;
    println!("Deployment truth receipt JSON: {}", path.display());
    if !blockers.is_empty() {
        let details = blockers
            .iter()
            .map(deployment_truth_finding_label)
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!("deployment execution preflight blocked install: {details}").into());
    }
    Ok(path)
}

struct StageReleaseSetOperation<'a> {
    icp_root: &'a Path,
    network: &'a str,
    root_canister_id: &'a str,
    manifest_path: &'a Path,
    manifest: RootReleaseSetManifest,
}

impl<'a> StageReleaseSetOperation<'a> {
    const fn new(
        icp_root: &'a Path,
        network: &'a str,
        root_canister_id: &'a str,
        manifest_path: &'a Path,
        manifest: RootReleaseSetManifest,
    ) -> Self {
        Self {
            icp_root,
            network,
            root_canister_id,
            manifest_path,
            manifest,
        }
    }
}

impl InstallPhaseOperation for StageReleaseSetOperation<'_> {
    fn phase(&self) -> &'static str {
        "stage_release_set"
    }

    fn attempted_action(&self) -> &'static str {
        "stage root release set"
    }

    fn evidence(&self) -> Vec<String> {
        current_install_staging_evidence(self.root_canister_id, self.manifest_path, &self.manifest)
    }

    fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        stage_root_release_set(
            self.icp_root,
            self.network,
            self.root_canister_id,
            &self.manifest,
        )
    }
}

fn current_install_execution_preflight_evidence(
    preflight: &crate::deployment_truth::DeploymentExecutionPreflightV1,
) -> Vec<String> {
    let mut evidence = vec![
        format!("execution_preflight_status:{:?}", preflight.status),
        format!("authority_plan:{}", preflight.authority_plan_id),
        format!("planned_phases:{}", preflight.planned_phases.len()),
        format!(
            "required_capabilities:{}",
            preflight.required_capabilities.len()
        ),
        format!(
            "missing_capabilities:{}",
            preflight.missing_capabilities.len()
        ),
        format!("blockers:{}", preflight.blockers.len()),
    ];
    evidence.extend(
        preflight
            .missing_capabilities
            .iter()
            .map(|capability| format!("missing_capability:{capability:?}")),
    );
    evidence.extend(
        preflight
            .blockers
            .iter()
            .map(|finding| format!("blocker:{}:{}", finding.code, finding.message)),
    );
    evidence
}

fn current_install_staging_evidence(
    root_canister_id: &str,
    manifest_path: &Path,
    manifest: &RootReleaseSetManifest,
) -> Vec<String> {
    let mut evidence = vec![
        format!("root_canister:{root_canister_id}"),
        format!("manifest_path:{}", manifest_path.display()),
        format!("release_version:{}", manifest.release_version),
    ];
    let staging_receipts = current_install_staging_receipts(root_canister_id, manifest);
    evidence.extend(staging_receipt_evidence(&staging_receipts));
    evidence
}

fn current_install_staging_receipts(
    root_canister_id: &str,
    manifest: &RootReleaseSetManifest,
) -> Vec<StagingReceiptV1> {
    manifest
        .entries
        .iter()
        .map(|entry| StagingReceiptV1 {
            schema_version: crate::deployment_truth::DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            role: entry.role.clone(),
            artifact_identity: format!(
                "{}:{}:{}",
                entry.template_id, manifest.release_version, entry.payload_sha256_hex
            ),
            transport: ArtifactTransportV1::WasmStore,
            wasm_store_locator: Some(format!("root:{root_canister_id}:bootstrap")),
            prepared_chunk_hashes: entry.chunk_sha256_hex.clone(),
            published_chunk_count: entry.chunk_sha256_hex.len(),
            verified_postcondition: crate::deployment_truth::VerifiedPostconditionV1 {
                status: ObservationStatusV1::Observed,
                evidence: vec![
                    format!("payload_sha256:{}", entry.payload_sha256_hex),
                    format!("payload_size_bytes:{}", entry.payload_size_bytes),
                    format!("chunk_size_bytes:{}", entry.chunk_size_bytes),
                    format!("chunk_count:{}", entry.chunk_sha256_hex.len()),
                ],
            },
        })
        .collect()
}

fn install_deployment_truth_phase_receipt(
    check: &DeploymentCheckV1,
    phase: &str,
    started_at: String,
    finished_at: Option<String>,
    attempted_action: &str,
    status: crate::deployment_truth::ObservationStatusV1,
    evidence: Vec<String>,
) -> DeploymentReceiptV1 {
    install_deployment_truth_phase_receipt_with_result(
        check,
        PhaseReceiptInput {
            phase,
            started_at,
            finished_at,
            attempted_action,
            status,
            evidence,
            role_phase_receipts: Vec::new(),
            operation_status: DeploymentExecutionStatusV1::Complete,
            command_result: DeploymentCommandResultV1::Succeeded,
        },
    )
}

fn install_deployment_truth_phase_receipt_with_result(
    check: &DeploymentCheckV1,
    input: PhaseReceiptInput<'_>,
) -> DeploymentReceiptV1 {
    deployment_receipt_from_check_with_status(
        check,
        format!("{}:{}", check.check_id, input.phase),
        input.operation_status,
        input.started_at.clone(),
        input.finished_at.clone(),
        vec![phase_receipt(
            input.phase,
            input.started_at,
            input.finished_at,
            input.attempted_action,
            input.status,
            input.evidence,
        )],
        input.role_phase_receipts,
        input.command_result,
    )
}

fn receipt_with_execution_context(
    mut receipt: DeploymentReceiptV1,
    execution_context: &DeploymentExecutionContextV1,
) -> DeploymentReceiptV1 {
    receipt.execution_context = Some(execution_context.clone());
    receipt
}

struct PhaseReceiptInput<'a> {
    phase: &'a str,
    started_at: String,
    finished_at: Option<String>,
    attempted_action: &'a str,
    status: crate::deployment_truth::ObservationStatusV1,
    evidence: Vec<String>,
    role_phase_receipts: Vec<crate::deployment_truth::RolePhaseReceiptV1>,
    operation_status: DeploymentExecutionStatusV1,
    command_result: DeploymentCommandResultV1,
}

fn write_install_deployment_truth_receipt(
    icp_root: &Path,
    network: &str,
    deployment_name: &str,
    receipt: &DeploymentReceiptV1,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = install_deployment_truth_receipt_path(icp_root, network, deployment_name, receipt)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut bytes = serde_json::to_vec_pretty(receipt)?;
    bytes.push(b'\n');
    fs::write(&path, bytes)?;
    Ok(path)
}

fn write_artifact_promotion_execution_receipt(
    icp_root: &Path,
    network: &str,
    deployment_name: &str,
    receipt: &ArtifactPromotionExecutionReceiptV1,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path =
        artifact_promotion_execution_receipt_path(icp_root, network, deployment_name, receipt)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut bytes = serde_json::to_vec_pretty(receipt)?;
    bytes.push(b'\n');
    fs::write(&path, bytes)?;
    Ok(path)
}

fn artifact_promotion_execution_receipt_path(
    icp_root: &Path,
    network: &str,
    deployment_name: &str,
    receipt: &ArtifactPromotionExecutionReceiptV1,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    validate_network_name(network)?;
    validate_state_name(deployment_name)?;
    let file_stem = format!(
        "{}-{}",
        safe_deployment_truth_path_label(&receipt.started_at),
        safe_deployment_truth_path_label(&receipt.receipt_id)
    );
    Ok(
        artifact_promotion_execution_receipts_dir(icp_root, network, deployment_name)?
            .join(format!("{file_stem}.json")),
    )
}

fn artifact_promotion_execution_receipts_dir(
    icp_root: &Path,
    network: &str,
    deployment_name: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    validate_network_name(network)?;
    validate_state_name(deployment_name)?;
    Ok(icp_root
        .join(".canic")
        .join(network)
        .join("artifact-promotion-execution-receipts")
        .join(deployment_name))
}

fn install_deployment_truth_receipt_path(
    icp_root: &Path,
    network: &str,
    deployment_name: &str,
    receipt: &DeploymentReceiptV1,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    validate_network_name(network)?;
    validate_state_name(deployment_name)?;
    let file_stem = format!(
        "{}-{}",
        safe_deployment_truth_path_label(&receipt.started_at),
        safe_deployment_truth_path_label(&receipt.operation_id)
    );
    Ok(
        install_deployment_truth_receipts_dir(icp_root, network, deployment_name)?
            .join(format!("{file_stem}.json")),
    )
}

/// Find the latest persisted deployment-truth receipt for one local deployment target.
pub fn latest_deployment_truth_receipt_path_from_root(
    icp_root: &Path,
    network: &str,
    deployment_name: &str,
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    let dir = install_deployment_truth_receipts_dir(icp_root, network, deployment_name)?;
    if !dir.is_dir() {
        return Ok(None);
    }

    let mut latest = None;
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if !path.is_file()
            || path
                .extension()
                .is_none_or(|ext| !ext.eq_ignore_ascii_case("json"))
        {
            continue;
        }
        if latest.as_ref().is_none_or(|current| path > *current) {
            latest = Some(path);
        }
    }
    Ok(latest)
}

fn install_deployment_truth_receipts_dir(
    icp_root: &Path,
    network: &str,
    deployment_name: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    validate_network_name(network)?;
    validate_state_name(deployment_name)?;
    Ok(icp_root
        .join(".canic")
        .join(network)
        .join("deployment-receipts")
        .join(deployment_name))
}

fn safe_deployment_truth_path_label(value: &str) -> String {
    let label = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if label.is_empty() {
        "unknown".to_string()
    } else {
        label
    }
}

fn deployment_truth_finding_label(finding: &SafetyFindingV1) -> String {
    let subject = finding
        .subject
        .as_ref()
        .map_or_else(|| "<none>".to_string(), Clone::clone);
    format!(
        "{}:{}:{}: {}",
        deployment_truth_finding_source(&finding.code),
        finding.code,
        subject,
        finding.message
    )
}

fn deployment_truth_finding_source(code: &str) -> &'static str {
    match code {
        "plan_assumption" => "plan",
        "observation_gap" => "inventory",
        _ => "diff",
    }
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

fn parse_created_canister_id(output: &str) -> Option<String> {
    if let Ok(value) = serde_json::from_str::<JsonValue>(output) {
        return parse_canister_id_json(&value);
    }

    output
        .lines()
        .map(str::trim)
        .find(|line| Principal::from_text(*line).is_ok())
        .map(ToString::to_string)
}

fn parse_canister_id_json(value: &JsonValue) -> Option<String> {
    match value {
        JsonValue::String(text) if Principal::from_text(text).is_ok() => Some(text.clone()),
        JsonValue::Array(values) => values.iter().find_map(parse_canister_id_json),
        JsonValue::Object(object) => ["canister_id", "id", "principal"]
            .iter()
            .filter_map(|key| object.get(*key))
            .find_map(parse_canister_id_json),
        _ => None,
    }
}

fn add_create_root_target(command: &mut Command, root_canister: &str) {
    if env::var_os(CANIC_ICP_LOCAL_NETWORK_URL_ENV).is_some() {
        command.args(["create", "--detached", "--json"]);
    } else {
        command.args(["create", root_canister, "--json"]);
    }
}

fn is_missing_canister_id_error(message: &str) -> bool {
    message.contains("failed to lookup canister ID")
        || message.contains("could not find ID for canister")
        || message.contains("Canister ID is missing")
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

fn root_init_args(root_wasm: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let wasm = std::fs::read(root_wasm)?;
    Ok(format!(
        "(variant {{ PrimeWithModuleHash = {} }})",
        idl_blob(&wasm_hash(&wasm))
    ))
}

fn idl_blob(bytes: &[u8]) -> String {
    let mut encoded = String::from("blob \"");
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(encoded, "\\{byte:02X}");
    }
    encoded.push('"');
    encoded
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

const fn deployment_root_verification_state(
    status: &RootVerificationStatus,
) -> DeploymentRootVerificationStateV1 {
    match status {
        RootVerificationStatus::Verified => DeploymentRootVerificationStateV1::Verified,
        RootVerificationStatus::NotVerified => DeploymentRootVerificationStateV1::NotVerified,
    }
}

const fn verified_root_state_transition(
    previous: DeploymentRootVerificationStateV1,
) -> DeploymentRootVerificationStateTransitionV1 {
    match previous {
        DeploymentRootVerificationStateV1::NotVerified => {
            DeploymentRootVerificationStateTransitionV1::PromotedNotVerifiedToVerified
        }
        DeploymentRootVerificationStateV1::Verified => {
            DeploymentRootVerificationStateTransitionV1::NoStateChange
        }
    }
}

fn write_verified_root_state_if_unchanged(
    icp_root: &Path,
    network: &str,
    state: &InstallState,
    expected_digest_before: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let path = deployment_install_state_path(icp_root, network, &state.deployment_name);
    let current_digest = file_sha256_hex(&path)?;
    if current_digest != expected_digest_before {
        return Err(format!(
            "deployment root verification state changed before write: expected {expected_digest_before}, found {current_digest}"
        )
        .into());
    }
    write_install_state(icp_root, network, state)?;
    file_sha256_hex(&path)
}

fn file_sha256_hex(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    Ok(bytes_sha256_hex(&fs::read(path)?))
}

fn bytes_sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut hex, "{byte:02x}");
    }
    hex
}

// Build each configured local install target through the host builder.
fn run_canic_build_targets(
    network: &str,
    targets: &[String],
    build_profile: Option<CanisterBuildProfile>,
    config_path: &Path,
    icp_root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let _env = BuildEnvGuard::apply(network, config_path, icp_root);
    let profile = build_profile.unwrap_or_else(CanisterBuildProfile::current);
    if let Some(context) = current_workspace_build_context_once(profile)? {
        for line in context.lines() {
            println!("{line}");
        }
        println!("config: {}", config_path.display());
        println!(
            "artifacts: {}",
            planned_build_artifact_root(icp_root).display()
        );
        println!();
    }

    fs::create_dir_all(planned_build_artifact_root(icp_root))?;
    println!("Building {} canisters", targets.len());
    println!();
    let headers = ["CANISTER", "PROGRESS", "WASM", "ELAPSED"];
    let planned_rows = targets
        .iter()
        .map(|target| {
            [
                target.clone(),
                progress_bar(targets.len(), targets.len(), 10),
                "000.00 MiB (gz 000.00 MiB)".to_string(),
                "0.00s".to_string(),
            ]
        })
        .collect::<Vec<_>>();
    let alignments = [
        ColumnAlign::Left,
        ColumnAlign::Left,
        ColumnAlign::Right,
        ColumnAlign::Right,
    ];
    let widths = table_widths(&headers, &planned_rows);
    println!("{}", render_table_row(&headers, &widths, &alignments));
    println!("{}", render_separator(&widths));

    for (index, target) in targets.iter().enumerate() {
        let started_at = Instant::now();
        let output = build_current_workspace_canister_artifact(target, profile)
            .map_err(|err| format!("artifact build failed for {target}: {err}"))?;
        let elapsed = started_at.elapsed();
        let artifact_size = wasm_artifact_size(&output.wasm_path, &output.wasm_gz_path)?;

        let row = [
            target.clone(),
            progress_bar(index + 1, targets.len(), 10),
            artifact_size,
            format!("{:.2}s", elapsed.as_secs_f64()),
        ];
        println!("{}", render_table_row(&row, &widths, &alignments));
    }

    println!();
    Ok(())
}

fn planned_build_artifact_root(icp_root: &Path) -> PathBuf {
    icp_root.join(".icp/local/canisters")
}

fn wasm_artifact_size(
    wasm_path: &Path,
    wasm_gz_path: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let wasm_bytes = Some(std::fs::metadata(wasm_path)?.len());
    let gzip_bytes = std::fs::metadata(wasm_gz_path)
        .ok()
        .map(|metadata| metadata.len());
    Ok(wasm_size_label(wasm_bytes, gzip_bytes))
}

struct BuildEnvGuard {
    previous_network: Option<OsString>,
    previous_config_path: Option<OsString>,
    previous_icp_root: Option<OsString>,
    previous_local_network_url: Option<OsString>,
    previous_local_root_key: Option<OsString>,
}

impl BuildEnvGuard {
    fn apply(network: &str, config_path: &Path, icp_root: &Path) -> Self {
        let guard = Self {
            previous_network: env::var_os("ICP_ENVIRONMENT"),
            previous_config_path: env::var_os("CANIC_CONFIG_PATH"),
            previous_icp_root: env::var_os("CANIC_ICP_ROOT"),
            previous_local_network_url: env::var_os(CANIC_ICP_LOCAL_NETWORK_URL_ENV),
            previous_local_root_key: env::var_os(CANIC_ICP_LOCAL_ROOT_KEY_ENV),
        };
        set_env("ICP_ENVIRONMENT", network);
        set_env("CANIC_CONFIG_PATH", config_path);
        set_env("CANIC_ICP_ROOT", icp_root);
        if let Some(target) = local_replica_icp_target(network, icp_root) {
            set_env(CANIC_ICP_LOCAL_NETWORK_URL_ENV, target.url);
            set_env(CANIC_ICP_LOCAL_ROOT_KEY_ENV, target.root_key);
        } else {
            remove_env(CANIC_ICP_LOCAL_NETWORK_URL_ENV);
            remove_env(CANIC_ICP_LOCAL_ROOT_KEY_ENV);
        }
        guard
    }
}

impl Drop for BuildEnvGuard {
    fn drop(&mut self) {
        restore_env("ICP_ENVIRONMENT", self.previous_network.take());
        restore_env("CANIC_CONFIG_PATH", self.previous_config_path.take());
        restore_env("CANIC_ICP_ROOT", self.previous_icp_root.take());
        restore_env(
            CANIC_ICP_LOCAL_NETWORK_URL_ENV,
            self.previous_local_network_url.take(),
        );
        restore_env(
            CANIC_ICP_LOCAL_ROOT_KEY_ENV,
            self.previous_local_root_key.take(),
        );
    }
}

struct LocalReplicaIcpTarget {
    url: String,
    root_key: String,
}

fn local_replica_icp_target(network: &str, icp_root: &Path) -> Option<LocalReplicaIcpTarget> {
    if !replica_query::should_use_local_replica_query(Some(network)) {
        return None;
    }
    if icp_ping(icp_root, network).unwrap_or(false) {
        return None;
    }
    let root_key = replica_query::local_replica_root_key_from_root(Some(network), icp_root)
        .ok()
        .flatten()?;
    Some(LocalReplicaIcpTarget {
        url: replica_query::local_replica_endpoint_from_root(Some(network), icp_root),
        root_key,
    })
}

fn set_env<K, V>(key: K, value: V)
where
    K: AsRef<std::ffi::OsStr>,
    V: AsRef<std::ffi::OsStr>,
{
    // Install builds are single-threaded host orchestration. The environment is
    // scoped by BuildEnvGuard so Cargo build scripts see the selected fleet.
    unsafe {
        env::set_var(key, value);
    }
}

fn remove_env<K>(key: K)
where
    K: AsRef<std::ffi::OsStr>,
{
    // Install builds are single-threaded host orchestration. The environment is
    // scoped by BuildEnvGuard so Cargo build scripts see the selected fleet.
    unsafe {
        env::remove_var(key);
    }
}

fn restore_env(key: &str, value: Option<OsString>) {
    // See set_env: this restores the single-threaded install build context.
    if let Some(value) = value {
        set_env(key, value);
    } else {
        remove_env(key);
    }
}

fn add_local_root_create_cycles_arg(
    command: &mut Command,
    config_path: &Path,
    network: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if network != "local" {
        return Ok(());
    }

    let cycles = configured_local_root_create_cycles(config_path)?;
    command.args(["--cycles", &cycles.to_string()]);
    Ok(())
}

fn ensure_local_root_min_cycles(
    icp_root: &Path,
    network: &str,
    root_canister: &str,
    phase: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if network != "local" {
        return Ok(());
    }

    let current = query_root_cycle_balance(network, root_canister)?;
    if current >= LOCAL_ROOT_MIN_READY_CYCLES {
        return Ok(());
    }

    let amount = LOCAL_ROOT_MIN_READY_CYCLES.saturating_sub(current);
    let mut command = icp_canister_command_in_network(icp_root);
    command
        .args(["top-up", "--amount"])
        .arg(amount.to_string())
        .arg(root_canister);
    add_icp_environment_target(&mut command, network);
    run_command(&mut command)?;
    println!(
        "Local root cycles ({phase}): topped up {} ({} -> {} target)",
        crate::format::cycles_tc(amount),
        crate::format::cycles_tc(current),
        crate::format::cycles_tc(LOCAL_ROOT_MIN_READY_CYCLES)
    );
    Ok(())
}

fn query_root_cycle_balance(
    network: &str,
    root_canister: &str,
) -> Result<u128, Box<dyn std::error::Error>> {
    let output = icp_query_on_network(
        network,
        root_canister,
        protocol::CANIC_CYCLE_BALANCE,
        None,
        Some("json"),
    )?;
    parse_cycle_balance_response(&output).ok_or_else(|| {
        format!(
            "could not parse {root_canister} {} response: {output}",
            protocol::CANIC_CYCLE_BALANCE
        )
        .into()
    })
}

fn progress_bar(current: usize, total: usize, width: usize) -> String {
    if total == 0 || width == 0 {
        return "[] 0/0".to_string();
    }

    let filled = current.saturating_mul(width).div_ceil(total);
    let filled = filled.min(width);
    format!(
        "[{}{}] {current}/{total}",
        "#".repeat(filled),
        " ".repeat(width - filled)
    )
}

// Ensure the requested replica is reachable before the local install flow begins.
fn ensure_icp_environment_ready(
    icp_root: &Path,
    network: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if icp_ping(icp_root, network)? {
        return Ok(());
    }
    if replica_query::should_use_local_replica_query(Some(network))
        && replica_query::local_replica_status_reachable_from_root(Some(network), icp_root)
    {
        println!(
            "Replica reachable via HTTP status endpoint even though ICP CLI reports network '{network}' stopped; continuing from ICP root {}.",
            icp_root.display()
        );
        return Ok(());
    }

    Err(format!(
        "icp environment is not running for network '{network}'\nStart the target replica in another terminal with `canic replica start` and rerun."
    )
    .into())
}

// Check whether `icp network ping <network>` currently succeeds.
fn icp_ping(icp_root: &Path, network: &str) -> Result<bool, Box<dyn std::error::Error>> {
    Ok(icp::default_command_in(icp_root)
        .args(["network", "ping", network])
        .output()?
        .status
        .success())
}

fn print_install_timing_summary(timings: &InstallTimingSummary, total: Duration) {
    println!("Install timing summary:");
    println!("{}", render_install_timing_summary(timings, total));
}

fn render_install_timing_summary(timings: &InstallTimingSummary, total: Duration) -> String {
    let rows = [
        timing_row("create_canisters", timings.create_canisters),
        timing_row("build_all", timings.build_all),
        timing_row("emit_manifest", timings.emit_manifest),
        timing_row("install_root", timings.install_root),
        timing_row("fund_root", timings.fund_root),
        timing_row("stage_release_set", timings.stage_release_set),
        timing_row("resume_bootstrap", timings.resume_bootstrap),
        timing_row("wait_ready", timings.wait_ready),
        timing_row("finalize_root_funding", timings.finalize_root_funding),
        timing_row("total", total),
    ];
    render_table(
        &["PHASE", "ELAPSED"],
        &rows,
        &[ColumnAlign::Left, ColumnAlign::Right],
    )
}

fn timing_row(label: &str, duration: Duration) -> [String; 2] {
    [label.to_string(), format!("{:.2}s", duration.as_secs_f64())]
}

// Print the final install result as a compact whitespace table.
fn print_install_result_summary(
    network: &str,
    deployment: &str,
    fleet_template: &str,
    state_path: &Path,
) {
    println!("Install result:");
    println!("{:<14} success", "status");
    println!("{:<14} {}", "deployment", deployment);
    println!("{:<14} {}", "fleet_template", fleet_template);
    println!("{:<14} {}", "install_state", state_path.display());
    println!(
        "{:<14} canic list {} --network {}",
        "smoke_check", deployment, network
    );
}

// Run one command and require a zero exit status.
fn run_command(command: &mut Command) -> Result<(), Box<dyn std::error::Error>> {
    icp::run_status(command).map_err(Into::into)
}

// Run one command, require success, and return stdout.
fn run_command_stdout(command: &mut Command) -> Result<String, Box<dyn std::error::Error>> {
    icp::run_output(command).map_err(Into::into)
}

// Build an icp command with the selected install environment exported
// for Rust build scripts that inspect ICP_ENVIRONMENT at compile time.
fn icp_command_on_network(network: &str) -> Command {
    let mut command = icp::default_command();
    command.env("ICP_ENVIRONMENT", network);
    command
}

// Build an icp command in one project directory with ICP_ENVIRONMENT applied.
fn icp_command_in_network(icp_root: &Path, network: &str) -> Command {
    let mut command = icp::default_command_in(icp_root);
    command.env("ICP_ENVIRONMENT", network);
    command
}

// Build an icp canister command in one project directory.
fn icp_canister_command_in_network(icp_root: &Path) -> Command {
    let mut command = icp::default_command_in(icp_root);
    command.arg("canister");
    command
}

fn add_icp_environment_target(command: &mut Command, network: &str) {
    icp::add_target_args(command, Some(network), None);
}
