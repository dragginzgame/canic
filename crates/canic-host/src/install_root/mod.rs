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
mod fleet_catalog_closeout;
mod fleet_catalog_publication;
mod fleet_component_provisioning_install;
mod fleet_component_provisioning_journal;
mod fleet_component_provisioning_plan;
mod fleet_install_session;
mod fleet_registry_activation;
mod fleet_registry_activation_journal;
mod fleet_subnet_root_component_registry_preparation;
mod fleet_subnet_root_install;
mod fleet_subnet_root_install_journal;
mod fleet_subnet_root_registry_join;
mod fleet_subnet_root_registry_mirror_activation;
mod fleet_subnet_root_registry_sync;
mod fleet_subnet_root_store_bootstrap;
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

use crate::release_build::{PlannedReleaseBuild, ReleaseBuildPlanError, plan_release_build};
use build_network::resolve_install_build_context;
use build_snapshot::resolve_install_snapshot;
pub use config_selection::{
    ConfigDiscoveryError, current_canic_workspace_root, discover_canic_config_choices,
    discover_canic_workspace_root_from, discover_workspace_canic_config_choices,
    select_discovered_app_config_path, workspace_app_roots,
};
use coordinator_install::install_and_verify_fleet_coordinator;
use fleet_component_provisioning_install::{
    InstallFleetComponentsRequest, install_fleet_components_and_publish_catalog,
};
use fleet_registry_activation::{ActivateFleetRegistryRequest, activate_and_verify_fleet_registry};
use fleet_subnet_root_component_registry_preparation::{
    PrepareFleetSubnetRootComponentRegistriesRequest,
    prepare_and_verify_fleet_subnet_root_component_registries,
};
use fleet_subnet_root_install::install_and_verify_fleet_subnet_roots;
use fleet_subnet_root_registry_join::register_and_verify_fleet_subnet_roots_joining;
use fleet_subnet_root_registry_mirror_activation::{
    ActivateFleetSubnetRootRegistryMirrorsRequest,
    activate_and_verify_fleet_subnet_root_registry_mirrors,
};
use fleet_subnet_root_registry_sync::{
    SynchronizeFleetSubnetRootsRequest, synchronize_and_verify_fleet_subnet_roots,
};
use fleet_subnet_root_store_bootstrap::bootstrap_and_verify_fleet_subnet_root_stores;
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

fn install_icp(
    executable: &str,
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&crate::icp::LocalReplicaTarget>,
) -> crate::icp::IcpCli {
    crate::icp::IcpCli::new(executable, Some(environment.to_string()))
        .with_cwd(icp_root)
        .with_local_replica(local_replica.cloned())
}

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
    IcpProjectDiscovery,
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
            Self::IcpProjectDiscovery => "ICP project discovery",
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

    fn in_phase(phase: InstallRootPhase) -> impl FnOnce(Box<dyn std::error::Error>) -> Self {
        move |source| Self { phase, source }
    }

    #[must_use]
    pub const fn phase(&self) -> InstallRootPhase {
        self.phase
    }
}

#[derive(Debug, ThisError)]
#[error("fresh Fleet installation requires --fleet-input <PATH>")]
struct MissingFleetInstallInputError;

/// Discover installable Canic config choices under the current workspace.
pub fn discover_current_canic_config_choices() -> Result<Vec<PathBuf>, ConfigDiscoveryError> {
    let workspace_root = current_canic_workspace_root()?;
    let choices = config_selection::discover_workspace_canic_config_choices(&workspace_root)?;
    if !choices.is_empty() {
        return Ok(choices);
    }

    let icp_root = icp_root()?;
    if icp_root != workspace_root {
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
    let artifact_root = if options.deployment_plan_override.is_some() {
        crate::release_set::artifact_root_path(&icp_root, options.artifact_environment())
    } else {
        build_context.artifact_root()
    };
    let execution_context = current_execution::current_install_execution_context_at_root(
        &workspace_root,
        &icp_root,
        &artifact_root,
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
    install_current_fleet_infrastructure(
        &options.icp_executable,
        &icp_root,
        environment,
        build_context.local_replica.as_ref(),
        &config_path,
        &planned_install,
        &mut timings,
    )?;

    print_install_timing_summary(&timings, total_started_at.elapsed());
    Ok(())
}

fn install_current_fleet_infrastructure(
    icp_executable: &str,
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&crate::icp::LocalReplicaTarget>,
    config_path: &Path,
    planned: &PlannedCurrentFleetInstall,
    timings: &mut CurrentInstallTimingSummary,
) -> Result<(), InstallRootError> {
    let (coordinator, coordinator_duration) = install_current_fleet_coordinator(
        icp_executable,
        icp_root,
        environment,
        local_replica,
        config_path,
        &planned.plan,
    )?;
    timings.create_canisters = coordinator_duration;
    let roots_duration = install_current_fleet_subnet_roots(
        icp_executable,
        icp_root,
        environment,
        local_replica,
        config_path,
        planned,
        coordinator.coordinator,
    )?;
    timings.create_canisters += roots_duration;
    bootstrap_and_verify_fleet_subnet_root_stores(
        icp_executable,
        icp_root,
        environment,
        local_replica,
        config_path,
        &planned.plan,
        coordinator.coordinator,
        planned.session.operation_id,
    )
    .map_err(InstallRootError::in_phase(InstallRootPhase::Activation))?;
    let joining_version = register_and_verify_fleet_subnet_roots_joining(
        icp_executable,
        icp_root,
        environment,
        local_replica,
        config_path,
        &planned.plan,
        coordinator.coordinator,
        planned.session.operation_id,
    )
    .map_err(InstallRootError::in_phase(InstallRootPhase::Activation))?;
    synchronize_and_verify_fleet_subnet_roots(SynchronizeFleetSubnetRootsRequest {
        icp_executable,
        icp_root,
        environment,
        local_replica,
        config_path,
        fleet_install_plan: &planned.plan,
        coordinator: coordinator.coordinator,
        install_operation_id: planned.session.operation_id,
        joining_version: joining_version.clone(),
    })
    .map_err(InstallRootError::in_phase(InstallRootPhase::Activation))?;
    let active = activate_and_verify_fleet_registry(ActivateFleetRegistryRequest {
        icp_executable,
        icp_root,
        environment,
        local_replica,
        config_path,
        fleet_install_plan: &planned.plan,
        coordinator: coordinator.coordinator,
        install_operation_id: planned.session.operation_id,
        joining_version: joining_version.clone(),
    })
    .map_err(InstallRootError::in_phase(InstallRootPhase::Activation))?;
    activate_and_verify_fleet_subnet_root_registry_mirrors(
        ActivateFleetSubnetRootRegistryMirrorsRequest {
            icp_executable,
            icp_root,
            environment,
            local_replica,
            config_path,
            fleet_install_plan: &planned.plan,
            coordinator: coordinator.coordinator,
            install_operation_id: planned.session.operation_id,
            joining_version,
            active_registry: &active.registry,
            active_version: active.version.clone(),
        },
    )
    .map_err(InstallRootError::in_phase(InstallRootPhase::Activation))?;
    prepare_current_fleet_subnet_root_component_registries(
        icp_executable,
        icp_root,
        environment,
        local_replica,
        config_path,
        planned,
        coordinator.coordinator,
    )?;
    install_fleet_components_and_publish_catalog(InstallFleetComponentsRequest {
        icp_executable,
        icp_root,
        environment,
        local_replica,
        config_path,
        fleet_name: planned.session.fleet_name.clone(),
        fleet_install_plan: &planned.plan,
        coordinator: coordinator.coordinator,
        install_operation_id: planned.session.operation_id,
        initial_active_registry: &active.registry,
    })
    .map_err(InstallRootError::in_phase(InstallRootPhase::Activation))
}

fn prepare_current_fleet_subnet_root_component_registries(
    icp_executable: &str,
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&crate::icp::LocalReplicaTarget>,
    config_path: &Path,
    planned: &PlannedCurrentFleetInstall,
    coordinator: canic_core::cdk::types::Principal,
) -> Result<(), InstallRootError> {
    prepare_and_verify_fleet_subnet_root_component_registries(
        PrepareFleetSubnetRootComponentRegistriesRequest {
            icp_executable,
            icp_root,
            environment,
            local_replica,
            config_path,
            fleet_install_plan: &planned.plan,
            coordinator,
            install_operation_id: planned.session.operation_id,
        },
    )
    .map_err(InstallRootError::in_phase(InstallRootPhase::Activation))
}

fn resolve_current_install_roots(
    options: &InstallRootOptions,
) -> Result<(PathBuf, PathBuf), InstallRootError> {
    let workspace_root = workspace_root()
        .map_err(|source| InstallRootError::new(InstallRootPhase::WorkspaceDiscovery, source))?;
    let icp_root = match &options.icp_root {
        Some(path) => path.canonicalize().map_err(|source| {
            InstallRootError::new(InstallRootPhase::IcpProjectDiscovery, source)
        })?,
        None => icp_root().map_err(|source| {
            InstallRootError::new(InstallRootPhase::IcpProjectDiscovery, source)
        })?,
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
    icp_executable: &str,
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&crate::icp::LocalReplicaTarget>,
    config_path: &Path,
    plan: &PersistedFleetInstallPlan,
) -> Result<(coordinator_install::VerifiedFleetCoordinator, Duration), InstallRootError> {
    let started = Instant::now();
    let coordinator = install_and_verify_fleet_coordinator(
        icp_executable,
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
    icp_executable: &str,
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&crate::icp::LocalReplicaTarget>,
    config_path: &Path,
    planned: &PlannedCurrentFleetInstall,
    coordinator: canic_core::cdk::types::Principal,
) -> Result<Duration, InstallRootError> {
    let started = Instant::now();
    install_and_verify_fleet_subnet_roots(
        icp_executable,
        icp_root,
        environment,
        local_replica,
        config_path,
        &planned.plan,
        coordinator,
        planned.session.operation_id,
    )
    .map_err(InstallRootError::in_phase(InstallRootPhase::Activation))?;
    Ok(started.elapsed())
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
        &options.icp_executable,
        &options.environment,
        &options.root_build_target,
        options.build_profile,
    )?;
    if options.deployment_plan_override.is_some() {
        let snapshot = resolve_install_snapshot(&context, &options.root_build_target, true)?;
        return Ok((context, snapshot));
    }

    let config = AppConfigSnapshot::load(config_path)?;
    let release_build = current_install_release_build(
        icp_root,
        &options.environment,
        &options.fleet_name,
        config.app_id(),
    )?;
    context = context.with_release_build_id(release_build.record.release_build_id);
    let mut snapshot = resolve_install_snapshot(&context, &options.root_build_target, false)?;
    snapshot.release_build = Some(release_build);
    Ok((context, snapshot))
}

fn current_install_release_build(
    icp_root: &Path,
    environment: &str,
    fleet_name: &str,
    app_id: &str,
) -> Result<PlannedReleaseBuild, Box<dyn std::error::Error>> {
    let canonical_network_id = resolve_canonical_network_id_from_root(icp_root, environment)?;
    let fleet_name = fleet_name.parse()?;
    let app = app_id.into();
    if let Some(finalized) = fleet_install_session::recover_fleet_install_session_release_build(
        icp_root,
        canonical_network_id,
        &fleet_name,
        &app,
    )? {
        return Ok(PlannedReleaseBuild {
            record: finalized.record,
            path: finalized.path,
        });
    }

    plan_release_build(icp_root).map_err(Into::into)
}
