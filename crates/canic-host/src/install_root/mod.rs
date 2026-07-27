use crate::{
    canister_build::cache::DefaultCanisterBuildCacheCleanup,
    deployment_truth::DeploymentReceiptV1,
    fleet_install_input::{ResolvedFleetInstallInput, load_and_resolve_fleet_install_input},
    fleet_install_plan::{
        FleetInstallPlanRequest, PersistedFleetInstallPlan, compile_and_persist_fleet_install_plan,
    },
    network::resolve_canonical_network_id_from_root,
    release_set::{AppConfigSnapshot, icp_root, workspace_root},
};
use config_selection::resolve_install_config_path;
use std::{
    fmt,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use thiserror::Error as ThisError;

mod build_network;
mod build_snapshot;
mod build_targets;
mod capabilities;
mod clock;
mod commands;
mod config_selection;
mod coordinator_install;
mod coordinator_install_journal;
mod current_execution;
mod deployment_truth_gate;
mod execution_preflight;
mod fleet_install_session;
mod fleet_subnet_root_install;
mod fleet_subnet_root_install_journal;
mod identity;
mod operations;
mod options;
mod output;
mod phase_receipts;
mod plan_artifacts;
mod preparation;
mod receipt_io;
mod timing;
mod truth_check;

use crate::release_build::{ReleaseBuildPlanError, plan_release_build};
use build_network::resolve_install_build_context;
use build_snapshot::resolve_install_snapshot;
pub use config_selection::{
    ConfigDiscoveryError, current_canic_project_root, discover_canic_config_choices,
    discover_canic_project_root_from, discover_project_canic_config_choices, project_app_roots,
    select_discovered_app_config_path,
};
use coordinator_install::install_and_verify_fleet_coordinator;
use current_execution::current_install_execution_context;
use fleet_subnet_root_install::install_and_verify_fleet_subnet_roots;
use identity::resolve_install_identity;
pub use options::InstallRootOptions;
use output::print_install_timing_summary;
use phase_receipts::{
    CompletedInstallPhase, InstallReceiptScope, write_completed_install_phase_receipt,
};
use plan_artifacts::emit_manifest_with_phase;
use preparation::prepare_install_deployment_truth;
pub use receipt_io::latest_deployment_truth_receipt_path_from_root;
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
    Planning,
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
            Self::Planning => "Fleet installation planning",
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

#[derive(Debug, ThisError)]
#[error(
    "Fleet Coordinator {coordinator} and {verified_roots} planned Fleet Subnet Root(s) are installed and independently verified from the durable plan at {}; local Wasm Store bootstrap and Fleet Registry registration remain blocked until their journalled lifecycle is implemented",
    plan_path.display(),
)]
struct FleetRootBootstrapUnavailableError {
    plan_path: PathBuf,
    coordinator: canic_core::cdk::types::Principal,
    verified_roots: usize,
}

#[derive(Debug, ThisError)]
#[error("fresh Fleet installation requires --fleet-input <PATH>")]
struct MissingFleetInstallInputError;

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

// Execute fresh Fleet planning and the Coordinator-first installation workflow.
pub fn install_root(options: InstallRootOptions) -> Result<(), InstallRootError> {
    let (workspace_root, icp_root) = resolve_current_install_roots(&options)?;
    let _build_cache_cleanup = DefaultCanisterBuildCacheCleanup::for_install(&workspace_root);
    let config_path = current_install_config_path(&icp_root, &options)?;
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
    let resolved_fleet_install_input =
        resolve_current_fleet_install_input(&icp_root, environment, &options)
            .map_err(InstallRootError::in_phase(InstallRootPhase::Planning))?;

    print_install_identity(&app_id, &fleet_name);
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
    let emitted_manifest = emit_manifest_with_phase(
        &icp_root,
        &install_snapshot,
        &prepared.build_outputs,
        &prepared.infrastructure_build_outputs,
        prepared.plan_artifacts.as_ref(),
    )
    .map_err(InstallRootError::in_phase(InstallRootPhase::Manifest))?;
    timings.emit_manifest = emitted_manifest.duration;
    let finalized_release_build =
        require_finalized_release_build(emitted_manifest.finalized_release_build)?;
    let planned_install = plan_current_fleet_install(
        &icp_root,
        environment,
        &fleet_name,
        &app_id,
        &config_path,
        &finalized_release_build,
        resolved_fleet_install_input,
    )?;
    let receipt_scope = InstallReceiptScope {
        icp_root: &icp_root,
        fleet: planned_install.fleet(),
        check: &prepared.deployment_truth_check,
        execution_context: Some(&execution_context),
    };
    persist_current_pre_root_receipts(
        receipt_scope,
        &prepared.pre_activation_receipts,
        prepared.build_phase,
        emitted_manifest.phase,
    )?;
    let (coordinator, coordinator_duration) = install_current_fleet_coordinator(
        &icp_root,
        environment,
        build_context.local_replica.as_ref(),
        &config_path,
        &planned_install.plan,
    )?;
    timings.create_canisters = coordinator_duration;
    let (roots, roots_duration) = install_current_fleet_subnet_roots(
        &icp_root,
        environment,
        build_context.local_replica.as_ref(),
        &config_path,
        &planned_install,
        coordinator.coordinator,
    )?;
    timings.create_canisters += roots_duration;
    require_fleet_subnet_root_bootstrap(
        &planned_install.plan.path,
        coordinator.coordinator,
        roots.roots.len(),
    )
    .map_err(|source| InstallRootError::new(InstallRootPhase::Activation, source))?;

    print_install_timing_summary(&timings, total_started_at.elapsed());
    Ok(())
}

fn resolve_current_install_roots(
    options: &InstallRootOptions,
) -> Result<(PathBuf, PathBuf), InstallRootError> {
    let workspace_root = workspace_root()
        .map_err(|source| InstallRootError::new(InstallRootPhase::WorkspaceDiscovery, source))?;
    let icp_root = match &options.icp_root {
        Some(path) => path
            .canonicalize()
            .map_err(|source| InstallRootError::new(InstallRootPhase::ProjectDiscovery, source))?,
        None => icp_root()
            .map_err(|source| InstallRootError::new(InstallRootPhase::ProjectDiscovery, source))?,
    };
    Ok((workspace_root, icp_root))
}

fn plan_current_fleet_install(
    icp_root: &Path,
    environment: &str,
    fleet_name: &str,
    app_id: &str,
    config_path: &Path,
    finalized_release_build: &crate::release_build::FinalizedReleaseBuild,
    input: ResolvedFleetInstallInput,
) -> Result<PlannedCurrentFleetInstall, InstallRootError> {
    let session = plan_current_fleet_install_session(
        icp_root,
        environment,
        fleet_name,
        app_id,
        finalized_release_build,
    )?;
    let plan = persist_current_fleet_install_plan(
        icp_root,
        config_path,
        session.fleet.clone(),
        finalized_release_build,
        input,
    )
    .map_err(InstallRootError::in_phase(InstallRootPhase::Planning))?;
    Ok(PlannedCurrentFleetInstall { session, plan })
}

struct PlannedCurrentFleetInstall {
    session: fleet_install_session::FleetInstallSession,
    plan: PersistedFleetInstallPlan,
}

impl PlannedCurrentFleetInstall {
    const fn fleet(&self) -> canic_core::ids::FleetKey {
        self.session.fleet.fleet
    }
}

fn require_finalized_release_build(
    finalized: Option<crate::release_build::FinalizedReleaseBuild>,
) -> Result<crate::release_build::FinalizedReleaseBuild, InstallRootError> {
    finalized.ok_or_else(|| {
        InstallRootError::new(
            InstallRootPhase::Manifest,
            ReleaseBuildPlanError::MissingFinalizedAuthority,
        )
    })
}

fn resolve_current_fleet_install_input(
    icp_root: &Path,
    environment: &str,
    options: &InstallRootOptions,
) -> Result<ResolvedFleetInstallInput, Box<dyn std::error::Error>> {
    let input_path = options
        .fleet_install_input_path
        .as_ref()
        .ok_or(MissingFleetInstallInputError)?;
    let input_path = if input_path.is_absolute() {
        input_path.clone()
    } else {
        icp_root.join(input_path)
    };
    load_and_resolve_fleet_install_input(icp_root, environment, &input_path).map_err(Into::into)
}

fn persist_current_fleet_install_plan(
    icp_root: &Path,
    config_path: &Path,
    fleet: canic_core::ids::FleetBinding,
    finalized_release_build: &crate::release_build::FinalizedReleaseBuild,
    input: ResolvedFleetInstallInput,
) -> Result<PersistedFleetInstallPlan, Box<dyn std::error::Error>> {
    let config = AppConfigSnapshot::load(config_path)?;
    compile_and_persist_fleet_install_plan(FleetInstallPlanRequest {
        root: icp_root,
        config: config.model(),
        fleet,
        release_build_id: finalized_release_build.record.release_build_id,
        coordinator: input.coordinator,
        fleet_subnet_roots: input.fleet_subnet_roots,
    })
    .map_err(Into::into)
}

fn require_fleet_subnet_root_bootstrap(
    plan_path: &Path,
    coordinator: canic_core::cdk::types::Principal,
    verified_roots: usize,
) -> Result<(), FleetRootBootstrapUnavailableError> {
    Err(FleetRootBootstrapUnavailableError {
        plan_path: plan_path.to_path_buf(),
        coordinator,
        verified_roots,
    })
}

fn current_install_config_path(
    icp_root: &Path,
    options: &InstallRootOptions,
) -> Result<PathBuf, InstallRootError> {
    resolve_install_config_path(
        icp_root,
        options.config_path.as_deref(),
        options.interactive_config_selection,
    )
    .map_err(InstallRootError::in_phase(InstallRootPhase::Configuration))
}

fn plan_current_fleet_install_session(
    icp_root: &Path,
    environment: &str,
    fleet_name: &str,
    app_id: &str,
    finalized_release_build: &crate::release_build::FinalizedReleaseBuild,
) -> Result<fleet_install_session::FleetInstallSession, InstallRootError> {
    let canonical_network_id = resolve_canonical_network_id_from_root(icp_root, environment)
        .map_err(|source| InstallRootError::new(InstallRootPhase::Activation, source))?;
    let fleet_name = fleet_name
        .parse()
        .map_err(|source| InstallRootError::new(InstallRootPhase::Identity, source))?;
    fleet_install_session::plan_fleet_install_session(
        fleet_install_session::PlanFleetInstallSessionRequest {
            root: icp_root,
            canonical_network_id,
            fleet_name,
            app: app_id.into(),
            finalized_release_build,
        },
    )
    .map_err(|source| InstallRootError::new(InstallRootPhase::Activation, source))
}

fn print_install_identity(app: &str, fleet_name: &str) {
    println!("Installing Fleet {fleet_name}");
    println!("Source App {app}");
    println!();
}

fn persist_current_pre_root_receipts(
    receipt_scope: InstallReceiptScope<'_>,
    prepared_receipts: &[DeploymentReceiptV1],
    build_phase: CompletedInstallPhase,
    manifest_phase: CompletedInstallPhase,
) -> Result<(), InstallRootError> {
    persist_pre_root_receipts(
        receipt_scope,
        prepared_receipts,
        build_phase,
        manifest_phase,
    )
    .map_err(InstallRootError::in_phase(InstallRootPhase::Activation))
}

fn install_current_fleet_coordinator(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&crate::icp::LocalReplicaTarget>,
    config_path: &Path,
    plan: &PersistedFleetInstallPlan,
) -> Result<(coordinator_install::VerifiedFleetCoordinator, Duration), InstallRootError> {
    let started = Instant::now();
    let coordinator = install_and_verify_fleet_coordinator(
        icp_root,
        environment,
        local_replica,
        config_path,
        plan,
    )
    .map_err(InstallRootError::in_phase(InstallRootPhase::Activation))?;
    Ok((coordinator, started.elapsed()))
}

fn install_current_fleet_subnet_roots(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&crate::icp::LocalReplicaTarget>,
    config_path: &Path,
    planned: &PlannedCurrentFleetInstall,
    coordinator: canic_core::cdk::types::Principal,
) -> Result<
    (
        fleet_subnet_root_install::VerifiedFleetSubnetRoots,
        Duration,
    ),
    InstallRootError,
> {
    let started = Instant::now();
    let roots = install_and_verify_fleet_subnet_roots(
        icp_root,
        environment,
        local_replica,
        config_path,
        &planned.plan,
        coordinator,
        planned.session.operation_id,
    )
    .map_err(InstallRootError::in_phase(InstallRootPhase::Activation))?;
    Ok((roots, started.elapsed()))
}

fn persist_pre_root_receipts(
    receipt_scope: InstallReceiptScope<'_>,
    prepared_receipts: &[DeploymentReceiptV1],
    build_phase: CompletedInstallPhase,
    manifest_phase: CompletedInstallPhase,
) -> Result<(), Box<dyn std::error::Error>> {
    for receipt in prepared_receipts {
        receipt_scope.write_receipt(receipt)?;
    }
    write_completed_install_phase_receipt(receipt_scope, build_phase)?;
    write_completed_install_phase_receipt(receipt_scope, manifest_phase)?;
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
