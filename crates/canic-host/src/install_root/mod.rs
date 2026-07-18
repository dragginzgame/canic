use crate::{
    canister_build::cache::DefaultCanisterBuildCacheCleanup,
    release_set::{icp_root, workspace_root},
};
use config_selection::resolve_install_config_path;
use std::{
    fmt,
    path::{Path, PathBuf},
    time::Instant,
};
use thiserror::Error as ThisError;

mod activation;
mod artifact_promotion;
mod build_network;
mod build_snapshot;
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
use build_network::resolve_install_build_context;
use build_snapshot::resolve_install_snapshot;
pub use config_selection::{
    ConfigDiscoveryError, current_canic_project_root, discover_canic_config_choices,
    discover_canic_project_root_from, discover_project_canic_config_choices, project_fleet_roots,
    select_discovered_fleet_config_path,
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
pub use phase_receipts::InstallPhaseFailureError;
use phase_receipts::InstallReceiptScope;
use plan_artifacts::emit_manifest_with_deployment_truth_receipt;
use preparation::prepare_install_deployment_truth;
pub use receipt_io::latest_deployment_truth_receipt_path_from_root;
pub use state::{
    InstallState, InstallStateError, RootVerificationStatus, read_named_deployment_install_state,
    read_named_deployment_install_state_from_root,
};
pub(crate) use state::{decode_install_state, validate_environment_name};
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

/// Stable phase in which a root install failed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstallRootPhase {
    WorkspaceDiscovery,
    ProjectDiscovery,
    Configuration,
    BuildInputs,
    Identity,
    Preparation,
    Manifest,
    Activation,
    StatePersistence,
    ArtifactPromotion,
}

impl fmt::Display for InstallRootPhase {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::WorkspaceDiscovery => "workspace discovery",
            Self::ProjectDiscovery => "ICP project discovery",
            Self::Configuration => "configuration selection",
            Self::BuildInputs => "build input validation",
            Self::Identity => "deployment identity resolution",
            Self::Preparation => "deployment preparation",
            Self::Manifest => "manifest emission",
            Self::Activation => "root activation",
            Self::StatePersistence => "install-state persistence",
            Self::ArtifactPromotion => "artifact-promotion receipt persistence",
        })
    }
}

/// Typed public failure for the root-install workflow.
#[derive(Debug, ThisError)]
#[error("root install failed during {phase}: {source}")]
pub struct InstallRootError {
    phase: InstallRootPhase,
    #[source]
    source: Box<dyn std::error::Error>,
}

struct InstallCompletion<'a> {
    fleet_name: &'a str,
    root_canister_id: &'a str,
    execution_context: &'a crate::deployment_truth::DeploymentExecutionContextV1,
}

impl InstallRootError {
    /// Preserve a concrete cause while assigning it to a stable install phase.
    pub fn new<E>(phase: InstallRootPhase, source: E) -> Self
    where
        E: std::error::Error + 'static,
    {
        Self {
            phase,
            source: Box::new(source),
        }
    }

    fn from_boxed(phase: InstallRootPhase, source: Box<dyn std::error::Error>) -> Self {
        Self { phase, source }
    }

    fn in_phase(phase: InstallRootPhase) -> impl FnOnce(Box<dyn std::error::Error>) -> Self {
        move |source| Self::from_boxed(phase, source)
    }

    #[must_use]
    pub const fn phase(&self) -> InstallRootPhase {
        self.phase
    }
}

/// Discover installable Canic config choices under the current workspace.
pub fn discover_current_canic_config_choices() -> Result<Vec<PathBuf>, ConfigDiscoveryError> {
    let project_root = current_canic_project_root()?;
    let choices = config_selection::discover_workspace_canic_config_choices(&project_root)?;
    if !choices.is_empty() {
        return Ok(choices);
    }

    let icp_root = icp_root()?;
    if icp_root != project_root {
        return config_selection::discover_workspace_canic_config_choices(&icp_root);
    }

    Ok(choices)
}

// Execute the local thin-root install flow against an already running replica.
pub fn install_root(options: InstallRootOptions) -> Result<(), InstallRootError> {
    let workspace_root = workspace_root()
        .map_err(|source| InstallRootError::new(InstallRootPhase::WorkspaceDiscovery, source))?;
    let _build_cache_cleanup = DefaultCanisterBuildCacheCleanup::for_install(&workspace_root);
    let icp_root = match &options.icp_root {
        Some(path) => path
            .canonicalize()
            .map_err(|source| InstallRootError::new(InstallRootPhase::ProjectDiscovery, source))?,
        None => icp_root()
            .map_err(|source| InstallRootError::new(InstallRootPhase::ProjectDiscovery, source))?,
    };
    let config_path = resolve_install_config_path(
        &icp_root,
        options.config_path.as_deref(),
        options.interactive_config_selection,
    )
    .map_err(InstallRootError::in_phase(InstallRootPhase::Configuration))?;
    let (build_context, install_snapshot) =
        current_install_build_inputs(&workspace_root, &icp_root, &config_path, &options)
            .map_err(InstallRootError::in_phase(InstallRootPhase::BuildInputs))?;
    let (fleet_name, deployment_name) =
        resolve_install_identity(&options, &config_path, &install_snapshot.fleet_name)
            .map_err(InstallRootError::in_phase(InstallRootPhase::Identity))?;
    let total_started_at = Instant::now();
    let mut timings = CurrentInstallTimingSummary::default();
    let environment = options.environment.as_str();
    let execution_context = current_install_execution_context(
        &workspace_root,
        &icp_root,
        options.artifact_environment(),
    );

    println!("Installing deployment {deployment_name}");
    println!("Fleet template {fleet_name}");
    println!();
    let prepared = prepare_install_deployment_truth(
        &options,
        &icp_root,
        &config_path,
        &deployment_name,
        &execution_context,
        &build_context,
        &install_snapshot,
    )
    .map_err(InstallRootError::in_phase(InstallRootPhase::Preparation))?;
    timings.create_canisters = prepared.timings.create_canisters;
    timings.build_all = prepared.timings.build_all;

    let (manifest_path, emit_manifest_duration) = emit_manifest_with_deployment_truth_receipt(
        &icp_root,
        &options,
        &deployment_name,
        &prepared.deployment_truth_check,
        &execution_context,
        &install_snapshot,
        &prepared.build_outputs,
        prepared.plan_artifacts.as_ref(),
    )
    .map_err(InstallRootError::in_phase(InstallRootPhase::Manifest))?;
    timings.emit_manifest = emit_manifest_duration;
    let activation_timings = run_root_activation_phases(
        InstallReceiptScope {
            icp_root: &icp_root,
            environment,
            deployment_name: &deployment_name,
            check: &prepared.deployment_truth_check,
            execution_context: Some(&execution_context),
        },
        &options,
        &prepared.root_canister_id,
        &manifest_path,
        total_started_at,
        &build_context,
        prepared.plan_artifacts.as_ref(),
    )
    .map_err(InstallRootError::in_phase(InstallRootPhase::Activation))?;
    timings.install_root = activation_timings.install_root;
    timings.fund_root = activation_timings.fund_root;
    timings.stage_release_set = activation_timings.stage_release_set;
    timings.resume_bootstrap = activation_timings.resume_bootstrap;
    timings.wait_ready = activation_timings.wait_ready;
    timings.finalize_root_funding = activation_timings.finalize_root_funding;

    print_install_timing_summary(&timings, total_started_at.elapsed());
    persist_install_result(
        InstallReceiptScope {
            icp_root: &icp_root,
            environment,
            deployment_name: &deployment_name,
            check: &prepared.deployment_truth_check,
            execution_context: Some(&execution_context),
        },
        &options,
        &workspace_root,
        &config_path,
        &manifest_path,
        InstallCompletion {
            fleet_name: &fleet_name,
            root_canister_id: &prepared.root_canister_id,
            execution_context: &execution_context,
        },
    )
}

fn persist_install_result(
    receipt_scope: InstallReceiptScope<'_>,
    options: &InstallRootOptions,
    workspace_root: &Path,
    config_path: &Path,
    manifest_path: &Path,
    completion: InstallCompletion<'_>,
) -> Result<(), InstallRootError> {
    let state = build_install_state(
        options,
        workspace_root,
        receipt_scope.icp_root,
        config_path,
        manifest_path,
        (receipt_scope.deployment_name, completion.fleet_name),
        completion.root_canister_id,
    )
    .map_err(InstallRootError::in_phase(
        InstallRootPhase::StatePersistence,
    ))?;
    let state_path = write_install_state_with_deployment_truth_receipt(
        receipt_scope,
        &options.environment,
        &state,
    )
    .map_err(InstallRootError::in_phase(
        InstallRootPhase::StatePersistence,
    ))?;
    write_artifact_promotion_execution_receipt_for_install(
        options,
        receipt_scope.icp_root,
        receipt_scope.environment,
        receipt_scope.deployment_name,
        receipt_scope.check,
        completion.execution_context,
    )
    .map_err(InstallRootError::in_phase(
        InstallRootPhase::ArtifactPromotion,
    ))?;
    print_install_result_summary(
        receipt_scope.environment,
        &state.deployment_name,
        &state.fleet_template,
        &state_path,
    );
    Ok(())
}

fn current_install_build_inputs(
    workspace_root: &std::path::Path,
    icp_root: &std::path::Path,
    config_path: &std::path::Path,
    options: &InstallRootOptions,
) -> Result<
    (
        crate::canister_build::WorkspaceBuildContext,
        build_snapshot::ValidatedInstallSnapshot,
    ),
    Box<dyn std::error::Error>,
> {
    let context = resolve_install_build_context(
        workspace_root,
        icp_root,
        config_path,
        &options.environment,
        &options.root_build_target,
        options.build_profile,
    )?;
    let snapshot = resolve_install_snapshot(
        &context,
        &options.root_build_target,
        options.deployment_plan_override.is_some(),
    )?;
    Ok((context, snapshot))
}
