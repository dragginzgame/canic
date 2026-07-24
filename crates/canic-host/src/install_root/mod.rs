use crate::{
    canister_build::cache::DefaultCanisterBuildCacheCleanup,
    network::resolve_canonical_network_id_from_root,
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
mod fleet_activation_journal;
mod identity;
mod operations;
mod options;
mod output;
mod phase_receipts;
mod plan_artifacts;
mod preparation;
mod receipt_io;
mod root_canister;
mod root_cycles;
mod root_verification;
mod state;
mod timing;
mod truth_check;

use crate::release_build::{ReleaseBuildPlanError, plan_release_build};
use activation::{PreparedRootInstall, install_root_prepared};
use build_network::resolve_install_build_context;
use build_snapshot::resolve_install_snapshot;
pub use config_selection::{
    ConfigDiscoveryError, current_canic_project_root, discover_canic_config_choices,
    discover_canic_project_root_from, discover_project_canic_config_choices, project_app_roots,
    select_discovered_app_config_path,
};
use current_execution::current_install_execution_context;
pub use deployment_registration::{
    RegisterDeploymentStateOptions, VerifyDeploymentRootOptions, register_deployment_state,
    verify_registered_deployment_root,
};
use identity::resolve_install_identity;
pub use operations::{
    InstallRootActivationStatusError, InstallRootExecutionReconciliationError,
    InstallRootModuleVerificationError,
};
pub use options::InstallRootOptions;
use output::print_install_timing_summary;
pub use phase_receipts::InstallPhaseFailureError;
use phase_receipts::InstallReceiptScope;
use plan_artifacts::emit_manifest_with_deployment_truth_receipt;
use preparation::{prepare_install_deployment_truth, resolve_root_canister_after_manifest};
pub use receipt_io::latest_deployment_truth_receipt_path_from_root;
pub(crate) use state::validate_environment_name;
pub use state::{
    InstallState, InstallStateError, RootVerificationStatus,
    read_named_deployment_install_state_from_root,
};
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

/// Typed terminal outcome while the next activation phase is not yet admitted.
#[derive(Debug, ThisError)]
#[error(
    "root {root_canister_id} is durably Prepared at activation journal {} sequence {sequence}; no operational Fleet state was published",
    journal_path.display()
)]
pub struct FleetActivationContinuationRequired {
    root_canister_id: String,
    journal_path: PathBuf,
    sequence: u64,
}

impl FleetActivationContinuationRequired {
    #[must_use]
    pub fn root_canister_id(&self) -> &str {
        &self.root_canister_id
    }

    #[must_use]
    pub fn journal_path(&self) -> &Path {
        &self.journal_path
    }

    #[must_use]
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }
}

fn continuation_required(
    root_canister_id: String,
    prepared_root: &PreparedRootInstall,
) -> InstallRootError {
    InstallRootError::new(
        InstallRootPhase::Activation,
        FleetActivationContinuationRequired {
            root_canister_id,
            journal_path: prepared_root.activation.path.clone(),
            sequence: prepared_root.activation.journal.sequence,
        },
    )
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
    let (app_id, fleet_name) =
        resolve_install_identity(&options, &config_path, &install_snapshot.app_id)
            .map_err(InstallRootError::in_phase(InstallRootPhase::Identity))?;
    let total_started_at = Instant::now();
    let mut timings = CurrentInstallTimingSummary::default();
    let environment = options.environment.as_str();
    let execution_context = current_install_execution_context(
        &workspace_root,
        &icp_root,
        options.artifact_environment(),
    );

    println!("Installing Fleet {fleet_name}");
    println!("Source App {app_id}");
    println!();
    let prepared = prepare_install_deployment_truth(
        &options,
        &icp_root,
        &config_path,
        &fleet_name,
        &execution_context,
        &build_context,
        &install_snapshot,
    )
    .map_err(InstallRootError::in_phase(InstallRootPhase::Preparation))?;
    timings.build_all = prepared.timings.build_all;
    let receipt_scope = InstallReceiptScope {
        icp_root: &icp_root,
        environment,
        deployment_name: &fleet_name,
        check: &prepared.deployment_truth_check,
        execution_context: Some(&execution_context),
    };

    let (_manifest_path, emit_manifest_duration, finalized_release_build) =
        emit_manifest_with_deployment_truth_receipt(
            receipt_scope,
            &options,
            &install_snapshot,
            &prepared.build_outputs,
            prepared.plan_artifacts.as_ref(),
        )
        .map_err(InstallRootError::in_phase(InstallRootPhase::Manifest))?;
    timings.emit_manifest = emit_manifest_duration;
    let finalized_release_build = finalized_release_build.ok_or_else(|| {
        InstallRootError::new(
            InstallRootPhase::Manifest,
            ReleaseBuildPlanError::MissingFinalizedAuthority,
        )
    })?;
    let canonical_network_id = resolve_canonical_network_id_from_root(&icp_root, environment)
        .map_err(|source| InstallRootError::new(InstallRootPhase::Activation, source))?;
    let activation = fleet_activation_journal::plan_fleet_install_activation(
        fleet_activation_journal::PlanFleetInstallActivationRequest {
            root: &icp_root,
            canonical_network_id,
            fleet_name: fleet_name
                .parse()
                .map_err(|source| InstallRootError::new(InstallRootPhase::Identity, source))?,
            app: app_id.into(),
            finalized_release_build: &finalized_release_build,
        },
    )
    .map_err(|source| InstallRootError::new(InstallRootPhase::Activation, source))?;
    let (root_canister_id, create_duration) =
        resolve_root_canister_after_manifest(receipt_scope, &options, &config_path, &build_context)
            .map_err(InstallRootError::in_phase(InstallRootPhase::Activation))?;
    timings.create_canisters = create_duration;
    let prepared_root = install_root_prepared(
        receipt_scope,
        &options,
        &root_canister_id,
        &build_context,
        prepared.plan_artifacts.as_ref(),
        &activation,
    )
    .map_err(InstallRootError::in_phase(InstallRootPhase::Activation))?;
    timings.record_activation(prepared_root.timings);

    print_install_timing_summary(&timings, total_started_at.elapsed());
    Err(continuation_required(root_canister_id, &prepared_root))
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
    let mut context = resolve_install_build_context(
        workspace_root,
        icp_root,
        config_path,
        &options.environment,
        &options.root_build_target,
        options.build_profile,
    )?;
    let mut snapshot = resolve_install_snapshot(
        &context,
        &options.root_build_target,
        options.deployment_plan_override.is_some(),
    )?;
    if snapshot.complete_build.is_some() {
        let release_build = plan_release_build(icp_root)?;
        context = context.with_release_build_id(release_build.record.release_build_id);
        snapshot.release_build = Some(release_build);
    }
    Ok((context, snapshot))
}
