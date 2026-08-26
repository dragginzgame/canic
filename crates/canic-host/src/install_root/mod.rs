use crate::{
    deployment_truth::{DeploymentReceiptV1, FreshFleetInstallDecisionReceiptV1},
    fleet_install_input::{
        ResolvedFleetInstallInput, load_and_resolve_fleet_install_input,
        load_and_resolve_fleet_install_input_for_preflight,
    },
    fleet_install_plan::{
        FleetInstallPlanRequest, FreshFleetDecisionAuthorityRequest,
        FreshFleetDeploymentPlanRequest, FreshFleetDeploymentPlanV1, FreshFleetPreflightEffectsV1,
        FreshFleetPreflightRequest, FreshFleetPreflightV1, PersistedFleetInstallPlan,
        PlannedCanisterCreationFunding, compile_and_persist_fleet_install_plan,
        compile_fresh_fleet_deployment_plan, compile_fresh_fleet_preflight,
        fresh_fleet_maximum_operator_debit, load_fresh_fleet_decision_authority,
        observe_fresh_fleet_operator_funding,
    },
    network::resolve_canonical_network_id_from_root,
    release_set::{
        AppConfigSnapshot, CanicInfrastructureRole, icp_root,
        load_persisted_canic_infrastructure_artifact_manifest, workspace_root,
    },
};
use config_selection::resolve_install_config_path;
use sha2::{Digest, Sha256};
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
mod fleet_install_recovery;
mod fleet_install_recovery_bundle;
mod fleet_install_session;
mod fleet_registry_activation;
mod fleet_registry_activation_journal;
mod fleet_registry_recovery;
mod fleet_subnet_root_component_registry_preparation;
mod fleet_subnet_root_install;
mod fleet_subnet_root_install_journal;
mod fleet_subnet_root_registry_join;
mod fleet_subnet_root_registry_mirror_activation;
mod fleet_subnet_root_registry_sync;
mod fleet_subnet_root_repair;
mod fleet_subnet_root_store_bootstrap;
mod icp_context;
mod identity;
mod operations;
mod options;
mod output;
mod phase_receipts;
mod plan_artifacts;
mod preparation;
mod receipt_io;
mod reused_build;
mod timing;
mod truth_check;

use crate::release_build::{
    PlannedReleaseBuild, ReleaseBuildPlanError, load_finalized_release_build,
    plan_release_build_for_profile,
};
use build_network::resolve_install_build_context;
use build_snapshot::{InstallSnapshotSource, resolve_install_snapshot};
pub use config_selection::{
    ConfigDiscoveryError, current_canic_workspace_root, discover_canic_config_choices,
    discover_canic_workspace_root_from, discover_workspace_canic_config_choices,
    select_discovered_app_config_path, workspace_app_roots,
};
use coordinator_install::install_and_verify_fleet_coordinator;
use coordinator_install_journal::{
    FleetCoordinatorInstallPhase, PlanFleetCoordinatorInstallRequest,
    inspect_fleet_coordinator_install,
};
use fleet_component_provisioning_install::{
    InstallFleetComponentsRequest, install_fleet_components_and_publish_catalog,
};
pub use fleet_install_recovery::{
    FreshFleetInstallRecoveryClassificationV1, FreshFleetInstallRecoveryError,
    FreshFleetInstallRecoveryPlanV1, InspectFreshFleetInstallRecoveryRequest,
    RetainedInstallPlanContractV1, inspect_fresh_fleet_install_recovery,
};
use fleet_install_recovery_bundle::FleetInstallRecoveryBundleCheckpoint;
pub use fleet_install_recovery_bundle::{
    FleetInstallRecoveryBundleReportV1, import_fleet_install_recovery_bundle,
    verify_fleet_install_recovery_bundle,
};
pub use fleet_install_session::{
    RetainedFleetInstallSessionSummaryV1, inspect_incomplete_fleet_install_session,
};
use fleet_registry_activation::{ActivateFleetRegistryRequest, activate_and_verify_fleet_registry};
use fleet_registry_activation_journal::load_verified_installed_registry;
use fleet_subnet_root_component_registry_preparation::{
    PrepareFleetSubnetRootComponentRegistriesRequest,
    prepare_and_verify_fleet_subnet_root_component_registries,
};
use fleet_subnet_root_install::{
    InstallFleetSubnetRootsRequest, PreflightFleetSubnetRootsRequest,
    finalize_retained_root_repairs, install_and_verify_fleet_subnet_roots,
    preflight_fleet_subnet_roots,
};
use fleet_subnet_root_registry_join::register_and_verify_fleet_subnet_roots_joining;
use fleet_subnet_root_registry_mirror_activation::{
    ActivateFleetSubnetRootRegistryMirrorsRequest,
    activate_and_verify_fleet_subnet_root_registry_mirrors,
};
use fleet_subnet_root_registry_sync::{
    SynchronizeFleetSubnetRootsRequest, synchronize_and_verify_fleet_subnet_roots,
};
use fleet_subnet_root_store_bootstrap::bootstrap_and_verify_fleet_subnet_root_stores;
use icp_context::InstallIcpContext;
use identity::resolve_install_identity;
pub use options::{InstallRootOptions, RetainedRootRepairAdoption};
use output::{TerminalStyle, print_install_timing_summary};
use phase_receipts::{
    CompletedInstallPhase, InstallReceiptScope, write_completed_install_phase_receipt,
};
use plan_artifacts::emit_manifest_with_phase;
use preparation::prepare_install_deployment_truth;
pub use receipt_io::latest_deployment_truth_receipt_path_from_root;
use timing::InstallTimingSummary;
pub use truth_check::{check_install_deployment_truth, check_install_execution_preflight};

pub(crate) fn load_verified_installed_fleet_registry(
    fleet_install_plan: &PersistedFleetInstallPlan,
) -> Result<canic_core::dto::fleet_registry::FleetRegistry, String> {
    load_verified_installed_registry(fleet_install_plan).map_err(|error| error.to_string())
}

pub(super) fn root_component_provisioning_operation_id(install_operation_id: [u8; 32]) -> [u8; 32] {
    root_install_phase_operation_id(install_operation_id, b"component-provisioning")
}

pub(super) fn root_registry_synchronization_operation_id(
    install_operation_id: [u8; 32],
) -> [u8; 32] {
    root_install_phase_operation_id(install_operation_id, b"registry-synchronization")
}

pub(super) fn root_store_adoption_operation_id(install_operation_id: [u8; 32]) -> [u8; 32] {
    root_install_phase_operation_id(install_operation_id, b"store-adoption")
}

pub(super) fn root_store_bootstrap_operation_id(install_operation_id: [u8; 32]) -> [u8; 32] {
    root_install_phase_operation_id(install_operation_id, b"store-bootstrap")
}

fn root_install_phase_operation_id(install_operation_id: [u8; 32], phase: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"canic.fleet-install.root-operation.v1\0");
    hasher.update(phase);
    hasher.update([0]);
    hasher.update(install_operation_id);
    let mut operation_id: [u8; 32] = hasher.finalize().into();
    if operation_id == [0; 32] {
        operation_id[31] = 1;
    }
    operation_id
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

#[derive(Debug, ThisError)]
#[error("preflight App {preflight} differs from build snapshot App {build_snapshot}")]
struct InstallAppSnapshotChangedError {
    preflight: String,
    build_snapshot: String,
}

#[derive(Debug, ThisError)]
#[error("fresh-Fleet plan digest differs: expected {expected}, observed {observed}")]
struct FreshFleetPlanDigestMismatchError {
    expected: String,
    observed: String,
}

#[derive(Debug, ThisError)]
enum InstallPreflightError {
    #[error("install preflight requires an existing incomplete retained Fleet session")]
    MissingRetainedSession,

    #[error("install preflight requires a verified retained Fleet Coordinator journal")]
    CoordinatorNotVerified,
}

#[derive(Debug)]
struct PreparedFreshFleetDecision {
    app_id: String,
    fleet_name: String,
    input: ResolvedFleetInstallInput,
    plan: FreshFleetDeploymentPlanV1,
    recovery: Option<FreshFleetInstallRecoveryPlanV1>,
}

#[derive(Clone, Copy)]
enum FleetCatalogAcquisition {
    CacheOnly,
    RefreshMissingOrInvalid,
}

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

#[derive(Clone, Copy, Eq, PartialEq)]
enum InstallExecutionMode {
    Apply,
    Preflight,
}

struct ExecuteCurrentFleetInstallRequest<'a> {
    mode: InstallExecutionMode,
    icp_context: InstallIcpContext,
    config_path: &'a Path,
    planned_install: &'a PlannedCurrentFleetInstall,
    local_replica: Option<crate::icp::LocalReplicaTarget>,
    receipt_scope: InstallReceiptScope<'a>,
    pre_activation_receipts: Vec<DeploymentReceiptV1>,
    build_phase: CompletedInstallPhase,
    manifest_phase: CompletedInstallPhase,
    timings: InstallTimingSummary,
    total_started_at: Instant,
}

/// Execute fresh Fleet planning and the Coordinator-first installation workflow.
pub fn install_root(options: InstallRootOptions) -> Result<(), InstallRootError> {
    run_install_root(options, InstallExecutionMode::Apply)
}

/// Run retained-recovery installer preparation through the last verified no-effect checkpoint.
pub fn preflight_install_root(options: InstallRootOptions) -> Result<(), InstallRootError> {
    run_install_root(options, InstallExecutionMode::Preflight)
}

fn run_install_root(
    mut options: InstallRootOptions,
    mode: InstallExecutionMode,
) -> Result<(), InstallRootError> {
    let (workspace_root, icp_root) = resolve_current_install_roots(&options)?;
    let config_path = current_install_config_path(&icp_root, &options)?;
    let (fresh_fleet, icp_context) =
        prepare_and_admit_current_fresh_fleet(&workspace_root, &icp_root, &config_path, &options)?;
    let PreparedFreshFleetDecision {
        app_id,
        fleet_name,
        input: resolved_fleet_install_input,
        plan: fresh_fleet_plan,
        recovery: fresh_fleet_recovery,
    } = fresh_fleet;
    require_preflight_recovery(mode, fresh_fleet_recovery.as_ref())?;
    options.admitted_fresh_fleet_plan_digest = Some(fresh_fleet_plan.plan_digest.clone());

    let (build_context, install_snapshot) = current_install_build_inputs(
        &workspace_root,
        &icp_root,
        &config_path,
        &icp_context,
        &options,
    )
    .map_err(InstallRootError::in_phase(InstallRootPhase::BuildInputs))?;
    options.build_profile = Some(build_context.profile);
    require_install_app_unchanged(&app_id, &install_snapshot)?;
    let total_started_at = Instant::now();
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
    print_install_identity(&app_id, &fleet_name);
    let prepared = prepare_install_deployment_truth(
        &options,
        &icp_context,
        &config_path,
        &fleet_name,
        &execution_context,
        &build_context,
        &install_snapshot,
    )
    .map_err(InstallRootError::in_phase(InstallRootPhase::Preparation))?;
    let mut timings = prepared.timings;
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
    recheck_fresh_fleet_operator_funding(
        &icp_context,
        &fresh_fleet_plan,
        fresh_fleet_recovery.as_ref(),
    )?;
    let planned_install = plan_current_fleet_install(CurrentFleetInstallPlanRequest {
        icp_root: &icp_root,
        environment,
        fleet_name: &fleet_name,
        app_id: &app_id,
        config_path: &config_path,
        finalized_release_build: &finalized_release_build,
        input: resolved_fleet_install_input,
        fresh_fleet_plan: &fresh_fleet_plan,
        recovery: fresh_fleet_recovery.as_ref(),
        retained_root_repair_adoption: options.retained_root_repair_adoption.clone(),
    })?;
    let fresh_fleet_receipt_decision = FreshFleetInstallDecisionReceiptV1 {
        plan_digest: fresh_fleet_plan.plan_digest.clone(),
        catalog: fresh_fleet_plan.authority.catalog,
    };
    let receipt_scope = InstallReceiptScope {
        icp_root: &icp_root,
        fleet: planned_install.fleet(),
        check: &prepared.deployment_truth_check,
        execution_context: Some(&execution_context),
        fresh_fleet_decision: Some(&fresh_fleet_receipt_decision),
    };
    execute_planned_current_fleet_install(ExecuteCurrentFleetInstallRequest {
        mode,
        icp_context,
        config_path: &config_path,
        planned_install: &planned_install,
        local_replica: build_context.local_replica,
        receipt_scope,
        pre_activation_receipts: prepared.pre_activation_receipts,
        build_phase: prepared.build_phase,
        manifest_phase: emitted_manifest.phase,
        timings,
        total_started_at,
    })
}

fn execute_planned_current_fleet_install(
    mut request: ExecuteCurrentFleetInstallRequest<'_>,
) -> Result<(), InstallRootError> {
    if request.mode == InstallExecutionMode::Preflight {
        let bundle_path = preflight_current_fleet_infrastructure(
            &request.icp_context,
            request.config_path,
            request.planned_install,
        )?;
        TerminalStyle::detected().print_section(
            "Install preflight complete",
            &format!(
                "verified exact installer inputs and recovery bundle {}; no operational authority was published and no IC update was issued",
                bundle_path.display()
            ),
        );
        println!();
        print_install_timing_summary(&request.timings, request.total_started_at.elapsed());
        return Ok(());
    }
    persist_current_pre_root_receipts(
        request.receipt_scope,
        &request.pre_activation_receipts,
        request.build_phase,
        request.manifest_phase,
    )?;
    let icp_context = request
        .icp_context
        .with_local_replica(request.local_replica);
    print_paid_effect_placement_warnings(&request.planned_install.plan.plan);
    let activation_started_at = Instant::now();
    install_current_fleet_infrastructure(
        &icp_context,
        request.config_path,
        request.planned_install,
    )?;
    request.timings.activate_fleet = activation_started_at.elapsed();

    print_install_timing_summary(&request.timings, request.total_started_at.elapsed());
    Ok(())
}

fn require_preflight_recovery(
    mode: InstallExecutionMode,
    recovery: Option<&FreshFleetInstallRecoveryPlanV1>,
) -> Result<(), InstallRootError> {
    if mode == InstallExecutionMode::Preflight && recovery.is_none() {
        return Err(InstallRootError::new(
            InstallRootPhase::Planning,
            InstallPreflightError::MissingRetainedSession,
        ));
    }
    Ok(())
}

fn prepare_and_admit_current_fresh_fleet(
    workspace_root: &Path,
    icp_root: &Path,
    config_path: &Path,
    options: &InstallRootOptions,
) -> Result<(PreparedFreshFleetDecision, InstallIcpContext), InstallRootError> {
    let icp_context =
        InstallIcpContext::new(&options.icp_executable, icp_root, &options.environment);
    let announced_fresh_fleet = prepare_current_fresh_fleet_preflight(
        workspace_root,
        icp_root,
        config_path,
        options,
        &icp_context,
        FleetCatalogAcquisition::RefreshMissingOrInvalid,
    )?;
    print_fresh_fleet_decision(&announced_fresh_fleet.plan);
    if let Some(recovery) = announced_fresh_fleet.recovery.as_ref() {
        print_fresh_fleet_recovery(recovery);
    }
    let fresh_fleet = prepare_current_fresh_fleet_preflight(
        workspace_root,
        icp_root,
        config_path,
        options,
        &icp_context,
        FleetCatalogAcquisition::CacheOnly,
    )?;
    require_recompiled_fresh_fleet_plan(&announced_fresh_fleet.plan, &fresh_fleet.plan)?;
    Ok((fresh_fleet, icp_context))
}

fn prepare_current_fresh_fleet_preflight(
    workspace_root: &Path,
    icp_root: &Path,
    config_path: &Path,
    options: &InstallRootOptions,
    icp_context: &InstallIcpContext,
    catalog_acquisition: FleetCatalogAcquisition,
) -> Result<PreparedFreshFleetDecision, InstallRootError> {
    let config = AppConfigSnapshot::load(config_path)
        .map_err(|source| InstallRootError::new(InstallRootPhase::Configuration, source))?;
    let (app_id, fleet_name) = resolve_install_identity(options, config_path, config.app_id())
        .map_err(InstallRootError::in_phase(InstallRootPhase::Identity))?;
    let canonical_network_id =
        resolve_canonical_network_id_from_root(icp_root, &options.environment)
            .map_err(|source| InstallRootError::new(InstallRootPhase::Identity, source))?;
    let input = resolve_current_fleet_install_input(
        icp_root,
        &options.environment,
        options,
        catalog_acquisition,
    )
    .map_err(InstallRootError::in_phase(InstallRootPhase::Planning))?;
    let release_source = current_install_preflight_release_source(
        icp_root,
        canonical_network_id,
        &fleet_name,
        &app_id,
        config.model(),
        options,
    )
    .map_err(InstallRootError::in_phase(InstallRootPhase::Planning))?;
    let preflight = compile_current_fresh_fleet_preflight(
        &config,
        &app_id,
        &fleet_name,
        &input,
        release_source.build_profile,
        release_source.release_build_id,
        release_source.recovery.as_ref(),
    )
    .map_err(InstallRootError::in_phase(InstallRootPhase::Planning))?;
    let maximum_operator_debit = fresh_fleet_maximum_operator_debit(&preflight)
        .map_err(|source| InstallRootError::new(InstallRootPhase::Planning, source))?;
    let required_operator_debit = release_source
        .recovery
        .as_ref()
        .map_or(&maximum_operator_debit, |recovery| {
            &recovery.remaining_operator_debit
        });
    let operator = observe_fresh_fleet_operator_funding(
        icp_context.cli(),
        &input.operator_principal,
        required_operator_debit,
    )
    .map_err(|source| InstallRootError::new(InstallRootPhase::Identity, source))?;
    let authority_request = FreshFleetDecisionAuthorityRequest {
        workspace_root,
        icp_root,
        config: &config,
        requested_environment: &options.environment,
        canonical_network_id,
        release_build_id: release_source.release_build_id,
        fleet_input: &input,
        operator: &operator,
    };
    let authority = match release_source.recovery.as_ref() {
        Some(recovery) => recovery
            .load_decision_authority(authority_request)
            .map_err(|source| InstallRootError::new(InstallRootPhase::Planning, source))?,
        None => load_fresh_fleet_decision_authority(authority_request)
            .map_err(|source| InstallRootError::new(InstallRootPhase::Planning, source))?,
    };
    let decision_request = FreshFleetDeploymentPlanRequest {
        preflight,
        authority,
    };
    let plan = match release_source.recovery.as_ref() {
        Some(recovery) => recovery
            .compile_decision(decision_request)
            .map_err(|source| InstallRootError::new(InstallRootPhase::Planning, source))?,
        None => compile_fresh_fleet_deployment_plan(decision_request)
            .map_err(|source| InstallRootError::new(InstallRootPhase::Planning, source))?,
    };
    require_fresh_fleet_plan_digest(
        options.expected_fresh_fleet_plan_digest.as_deref(),
        &plan.plan_digest,
    )
    .map_err(|source| InstallRootError::new(InstallRootPhase::Planning, source))?;
    require_fresh_fleet_plan_digest(
        release_source.recovered_plan_digest.as_deref(),
        &plan.plan_digest,
    )
    .map_err(|source| InstallRootError::new(InstallRootPhase::Planning, source))?;
    Ok(PreparedFreshFleetDecision {
        app_id,
        fleet_name,
        input,
        plan,
        recovery: release_source.recovery,
    })
}

fn recheck_fresh_fleet_operator_funding(
    icp_context: &InstallIcpContext,
    plan: &FreshFleetDeploymentPlanV1,
    recovery: Option<&FreshFleetInstallRecoveryPlanV1>,
) -> Result<(), InstallRootError> {
    let required_operator_debit = recovery.map_or(&plan.maximum_operator_debit, |recovery| {
        &recovery.remaining_operator_debit
    });
    let operator = observe_fresh_fleet_operator_funding(
        icp_context.cli(),
        &plan.authority.operator.principal,
        required_operator_debit,
    )
    .map_err(|source| InstallRootError::new(InstallRootPhase::Identity, source))?;
    let mut authority = plan.authority.clone();
    authority.operator = operator;
    let decision_request = FreshFleetDeploymentPlanRequest {
        preflight: plan.preflight.clone(),
        authority,
    };
    let rechecked = match recovery {
        Some(recovery) => recovery
            .compile_decision(decision_request)
            .map_err(|source| InstallRootError::new(InstallRootPhase::Planning, source))?,
        None => compile_fresh_fleet_deployment_plan(decision_request)
            .map_err(|source| InstallRootError::new(InstallRootPhase::Planning, source))?,
    };
    require_recompiled_fresh_fleet_plan(plan, &rechecked)
}

fn preflight_current_fleet_infrastructure(
    icp_context: &InstallIcpContext,
    config_path: &Path,
    planned: &PlannedCurrentFleetInstall,
) -> Result<PathBuf, InstallRootError> {
    let coordinator =
        inspect_preflight_fleet_coordinator(icp_context.root(), config_path, &planned.plan)?;
    let recovery_bundle = FleetInstallRecoveryBundleCheckpoint::new(
        icp_context.root(),
        &planned.session,
        &planned.plan,
    );
    preflight_fleet_subnet_roots(PreflightFleetSubnetRootsRequest {
        icp_root: icp_context.root(),
        config_path,
        fleet_install_plan: &planned.plan,
        fleet_install_session: &planned.session,
        coordinator,
        install_operation_id: planned.session.operation_id,
        retained_root_repair_adoption: planned.retained_root_repair_adoption.as_ref(),
        recovery_bundle: &recovery_bundle,
    })
    .map_err(InstallRootError::in_phase(InstallRootPhase::Activation))
}

fn inspect_preflight_fleet_coordinator(
    icp_root: &Path,
    config_path: &Path,
    fleet_install_plan: &PersistedFleetInstallPlan,
) -> Result<canic_core::cdk::types::Principal, InstallRootError> {
    let config = AppConfigSnapshot::load(config_path)
        .map_err(|source| InstallRootError::new(InstallRootPhase::Configuration, source))?;
    let component_deployment_configuration = config
        .model()
        .compile_component_deployment_configuration()
        .map_err(|source| InstallRootError::new(InstallRootPhase::Planning, source))?;
    let infrastructure_manifest = load_persisted_canic_infrastructure_artifact_manifest(
        icp_root,
        fleet_install_plan.plan.release_build_id,
    )
    .map_err(|source| InstallRootError::new(InstallRootPhase::Planning, source))?;
    let _artifact = operations::resolve_install_artifact(
        icp_root,
        &infrastructure_manifest,
        CanicInfrastructureRole::FleetCoordinator,
        fleet_install_plan.plan.release_build_id,
    )
    .map_err(InstallRootError::in_phase(InstallRootPhase::Planning))?;
    let current = inspect_fleet_coordinator_install(PlanFleetCoordinatorInstallRequest {
        fleet_install_plan,
        infrastructure_manifest: &infrastructure_manifest,
        component_deployment_configuration,
    })
    .map_err(|source| InstallRootError::new(InstallRootPhase::Planning, source))?;
    let Some(current) = current else {
        return Err(InstallRootError::new(
            InstallRootPhase::Planning,
            InstallPreflightError::CoordinatorNotVerified,
        ));
    };
    if current.journal.phase != FleetCoordinatorInstallPhase::Verified {
        return Err(InstallRootError::new(
            InstallRootPhase::Planning,
            InstallPreflightError::CoordinatorNotVerified,
        ));
    }
    current.journal.coordinator.ok_or_else(|| {
        InstallRootError::new(
            InstallRootPhase::Planning,
            InstallPreflightError::CoordinatorNotVerified,
        )
    })
}

fn install_current_fleet_infrastructure(
    icp_context: &InstallIcpContext,
    config_path: &Path,
    planned: &PlannedCurrentFleetInstall,
) -> Result<(), InstallRootError> {
    let recovery_bundle = FleetInstallRecoveryBundleCheckpoint::new(
        icp_context.root(),
        &planned.session,
        &planned.plan,
    );
    checkpoint_current_recovery_bundle(&recovery_bundle)?;
    let (coordinator, _) =
        install_current_fleet_coordinator(icp_context, config_path, &planned.plan)?;
    checkpoint_current_recovery_bundle(&recovery_bundle)?;
    let _ = install_current_fleet_subnet_roots(
        icp_context,
        config_path,
        planned,
        coordinator.coordinator,
        &recovery_bundle,
    )?;
    checkpoint_current_recovery_bundle(&recovery_bundle)?;
    bootstrap_and_verify_fleet_subnet_root_stores(
        icp_context,
        config_path,
        &planned.plan,
        coordinator.coordinator,
        planned.session.operation_id,
    )
    .map_err(InstallRootError::in_phase(InstallRootPhase::Activation))?;
    checkpoint_current_recovery_bundle(&recovery_bundle)?;
    let joining_version = register_and_verify_fleet_subnet_roots_joining(
        icp_context,
        config_path,
        &planned.plan,
        coordinator.coordinator,
        planned.session.operation_id,
    )
    .map_err(InstallRootError::in_phase(InstallRootPhase::Activation))?;
    checkpoint_current_recovery_bundle(&recovery_bundle)?;
    synchronize_and_verify_fleet_subnet_roots(SynchronizeFleetSubnetRootsRequest {
        icp: icp_context,
        config_path,
        fleet_install_plan: &planned.plan,
        coordinator: coordinator.coordinator,
        install_operation_id: planned.session.operation_id,
        joining_version: joining_version.clone(),
    })
    .map_err(InstallRootError::in_phase(InstallRootPhase::Activation))?;
    checkpoint_current_recovery_bundle(&recovery_bundle)?;
    let active = activate_and_verify_fleet_registry(ActivateFleetRegistryRequest {
        icp: icp_context,
        config_path,
        fleet_install_plan: &planned.plan,
        coordinator: coordinator.coordinator,
        install_operation_id: planned.session.operation_id,
        joining_version: joining_version.clone(),
    })
    .map_err(InstallRootError::in_phase(InstallRootPhase::Activation))?;
    checkpoint_current_recovery_bundle(&recovery_bundle)?;
    activate_and_verify_fleet_subnet_root_registry_mirrors(
        ActivateFleetSubnetRootRegistryMirrorsRequest {
            icp: icp_context,
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
    checkpoint_current_recovery_bundle(&recovery_bundle)?;
    prepare_current_fleet_subnet_root_component_registries(
        icp_context,
        config_path,
        planned,
        coordinator.coordinator,
    )?;
    checkpoint_current_recovery_bundle(&recovery_bundle)?;
    finalize_retained_root_repairs(
        icp_context,
        config_path,
        &planned.plan,
        &planned.session,
        coordinator.coordinator,
        &recovery_bundle,
    )
    .map_err(InstallRootError::in_phase(InstallRootPhase::Activation))?;
    checkpoint_current_recovery_bundle(&recovery_bundle)?;
    install_fleet_components_and_publish_catalog(InstallFleetComponentsRequest {
        icp: icp_context,
        config_path,
        fleet_name: planned.session.fleet_name.clone(),
        fleet_install_session: &planned.session,
        fleet_install_plan: &planned.plan,
        coordinator: coordinator.coordinator,
        install_operation_id: planned.session.operation_id,
        initial_active_registry: &active.registry,
    })
    .map_err(InstallRootError::in_phase(InstallRootPhase::Activation))?;
    checkpoint_current_recovery_bundle(&recovery_bundle)
}

fn checkpoint_current_recovery_bundle(
    recovery_bundle: &FleetInstallRecoveryBundleCheckpoint<'_>,
) -> Result<(), InstallRootError> {
    let path = recovery_bundle
        .checkpoint()
        .map_err(|source| InstallRootError::new(InstallRootPhase::Activation, source))?;
    println!("Retained recovery bundle: {}", path.display());
    Ok(())
}

fn prepare_current_fleet_subnet_root_component_registries(
    icp_context: &InstallIcpContext,
    config_path: &Path,
    planned: &PlannedCurrentFleetInstall,
    coordinator: canic_core::cdk::types::Principal,
) -> Result<(), InstallRootError> {
    prepare_and_verify_fleet_subnet_root_component_registries(
        PrepareFleetSubnetRootComponentRegistriesRequest {
            icp: icp_context,
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

struct CurrentFleetInstallPlanRequest<'a> {
    icp_root: &'a Path,
    environment: &'a str,
    fleet_name: &'a str,
    app_id: &'a str,
    config_path: &'a Path,
    finalized_release_build: &'a crate::release_build::FinalizedReleaseBuild,
    input: ResolvedFleetInstallInput,
    fresh_fleet_plan: &'a FreshFleetDeploymentPlanV1,
    recovery: Option<&'a FreshFleetInstallRecoveryPlanV1>,
    retained_root_repair_adoption: Option<RetainedRootRepairAdoption>,
}

fn plan_current_fleet_install(
    request: CurrentFleetInstallPlanRequest<'_>,
) -> Result<PlannedCurrentFleetInstall, InstallRootError> {
    let session = plan_current_fleet_install_session(
        request.icp_root,
        request.environment,
        request.fleet_name,
        request.app_id,
        request.finalized_release_build,
        request.fresh_fleet_plan,
    )?;
    let plan = persist_current_fleet_install_plan(
        request.icp_root,
        request.config_path,
        request.fleet_name,
        session.fleet.clone(),
        request.finalized_release_build,
        request.input,
        request.fresh_fleet_plan,
        request.recovery,
    )
    .map_err(InstallRootError::in_phase(InstallRootPhase::Planning))?;
    Ok(PlannedCurrentFleetInstall {
        session,
        plan,
        retained_root_repair_adoption: request.retained_root_repair_adoption,
    })
}

struct PlannedCurrentFleetInstall {
    session: fleet_install_session::FleetInstallSession,
    plan: PersistedFleetInstallPlan,
    retained_root_repair_adoption: Option<RetainedRootRepairAdoption>,
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

fn require_recompiled_fresh_fleet_plan(
    announced: &FreshFleetDeploymentPlanV1,
    recompiled: &FreshFleetDeploymentPlanV1,
) -> Result<(), InstallRootError> {
    require_fresh_fleet_plan_digest(Some(&announced.plan_digest), &recompiled.plan_digest)
        .map_err(|source| InstallRootError::new(InstallRootPhase::Planning, source))
}

fn require_install_app_unchanged(
    preflight_app_id: &str,
    install_snapshot: &build_snapshot::ValidatedInstallSnapshot,
) -> Result<(), InstallRootError> {
    if install_snapshot.app_id == preflight_app_id {
        return Ok(());
    }
    Err(InstallRootError::new(
        InstallRootPhase::Identity,
        InstallAppSnapshotChangedError {
            preflight: preflight_app_id.to_string(),
            build_snapshot: install_snapshot.app_id.clone(),
        },
    ))
}

fn resolve_current_fleet_install_input(
    icp_root: &Path,
    environment: &str,
    options: &InstallRootOptions,
    catalog_acquisition: FleetCatalogAcquisition,
) -> Result<ResolvedFleetInstallInput, Box<dyn std::error::Error>> {
    let input_path = current_fleet_install_input_path(icp_root, options)?;
    match catalog_acquisition {
        FleetCatalogAcquisition::CacheOnly => {
            load_and_resolve_fleet_install_input_for_preflight(icp_root, environment, &input_path)
        }
        FleetCatalogAcquisition::RefreshMissingOrInvalid => {
            load_and_resolve_fleet_install_input(icp_root, environment, &input_path)
        }
    }
    .map_err(Into::into)
}

fn current_fleet_install_input_path(
    icp_root: &Path,
    options: &InstallRootOptions,
) -> Result<PathBuf, MissingFleetInstallInputError> {
    let input_path = options
        .fleet_install_input_path
        .as_ref()
        .ok_or(MissingFleetInstallInputError)?;
    Ok(if input_path.is_absolute() {
        input_path.clone()
    } else {
        icp_root.join(input_path)
    })
}

fn compile_current_fresh_fleet_preflight(
    config: &AppConfigSnapshot,
    app_id: &str,
    fleet_name: &str,
    input: &ResolvedFleetInstallInput,
    build_profile: crate::canister_build::CanisterBuildProfile,
    release_build_id: Option<canic_core::ids::ReleaseBuildId>,
    recovery: Option<&FreshFleetInstallRecoveryPlanV1>,
) -> Result<FreshFleetPreflightV1, Box<dyn std::error::Error>> {
    let fleet_name = fleet_name.parse()?;
    let request = FreshFleetPreflightRequest {
        config: config.model(),
        app: app_id,
        fleet_name: &fleet_name,
        coordinator: &input.coordinator,
        admission: &input.admission,
        fleet_subnet_roots: &input.fleet_subnet_roots,
        build_profile,
        release_build_id,
        effects: FreshFleetPreflightEffectsV1::none_started(),
    };
    match recovery {
        Some(recovery) => recovery.compile_preflight(request).map_err(Into::into),
        None => compile_fresh_fleet_preflight(request).map_err(Into::into),
    }
}

struct CurrentInstallPreflightReleaseSource {
    build_profile: crate::canister_build::CanisterBuildProfile,
    release_build_id: Option<canic_core::ids::ReleaseBuildId>,
    recovered_plan_digest: Option<String>,
    recovery: Option<FreshFleetInstallRecoveryPlanV1>,
}

fn current_install_preflight_release_source(
    icp_root: &Path,
    canonical_network_id: canic_core::ids::CanonicalNetworkId,
    fleet_name: &str,
    app_id: &str,
    config: &canic_core::bootstrap::compiled::ConfigModel,
    options: &InstallRootOptions,
) -> Result<CurrentInstallPreflightReleaseSource, Box<dyn std::error::Error>> {
    let fleet_name = fleet_name.parse()?;
    let app = app_id.into();
    if let Some(recovered) = fleet_install_session::recover_fleet_install_session_authority(
        icp_root,
        canonical_network_id,
        &fleet_name,
        &app,
    )? {
        let finalized = &recovered.finalized_release_build;
        if options
            .release_build_id
            .is_some_and(|requested| requested != finalized.record.release_build_id)
        {
            return Err(
                "requested release build differs from the interrupted Fleet install session".into(),
            );
        }
        require_requested_build_profile(options.build_profile, finalized.record.build_profile)?;
        let recovery = fleet_install_recovery::compile_recovery_plan(icp_root, config, &recovered)?;
        return Ok(CurrentInstallPreflightReleaseSource {
            build_profile: finalized.record.build_profile,
            release_build_id: recovered.decision_release_build_id,
            recovered_plan_digest: Some(recovered.fresh_fleet_plan_digest),
            recovery: Some(recovery),
        });
    }
    if let Some(release_build_id) = options.release_build_id {
        if options.retained_root_repair_adoption.is_some() {
            return Err(
                "--adopt-retained-root-repair requires an existing incomplete Fleet install session"
                    .into(),
            );
        }
        let finalized = load_finalized_release_build(icp_root, release_build_id)?;
        require_current_release_builder(&finalized.record.builder_version)?;
        require_requested_build_profile(options.build_profile, finalized.record.build_profile)?;
        return Ok(CurrentInstallPreflightReleaseSource {
            build_profile: finalized.record.build_profile,
            release_build_id: Some(release_build_id),
            recovered_plan_digest: None,
            recovery: None,
        });
    }
    if options.retained_root_repair_adoption.is_some() {
        return Err(
            "--adopt-retained-root-repair requires an existing incomplete Fleet install session"
                .into(),
        );
    }
    Ok(CurrentInstallPreflightReleaseSource {
        build_profile: options
            .build_profile
            .unwrap_or(crate::canister_build::CanisterBuildProfile::Release),
        release_build_id: None,
        recovered_plan_digest: None,
        recovery: None,
    })
}

#[expect(
    clippy::too_many_arguments,
    reason = "the retained-install boundary carries the exact immutable plan and recovery authorities independently"
)]
fn persist_current_fleet_install_plan(
    icp_root: &Path,
    config_path: &Path,
    fleet_name: &str,
    fleet: canic_core::ids::FleetBinding,
    finalized_release_build: &crate::release_build::FinalizedReleaseBuild,
    input: ResolvedFleetInstallInput,
    fresh_fleet_plan: &FreshFleetDeploymentPlanV1,
    recovery: Option<&FreshFleetInstallRecoveryPlanV1>,
) -> Result<PersistedFleetInstallPlan, Box<dyn std::error::Error>> {
    let config = AppConfigSnapshot::load(config_path)?;
    if let Some(recovery) = recovery {
        return recovery
            .load_install_plan(icp_root, config.model(), &fleet)
            .map_err(Into::into);
    }
    compile_and_persist_fleet_install_plan(FleetInstallPlanRequest {
        root: icp_root,
        config: config.model(),
        fleet,
        fleet_name: fleet_name.parse()?,
        fresh_fleet_plan_digest: fresh_fleet_plan.plan_digest.clone(),
        release_build_id: finalized_release_build.record.release_build_id,
        coordinator: input.coordinator,
        admission: input.admission,
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
    fresh_fleet_plan: &FreshFleetDeploymentPlanV1,
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
            decision_release_build_id: fresh_fleet_plan.preflight.release_build_id,
            fresh_fleet_plan_digest: &fresh_fleet_plan.plan_digest,
        },
    )
    .map_err(|source| InstallRootError::new(InstallRootPhase::Activation, source))
}

fn require_fresh_fleet_plan_digest(
    expected: Option<&str>,
    observed: &str,
) -> Result<(), FreshFleetPlanDigestMismatchError> {
    match expected {
        Some(expected) if expected != observed => Err(FreshFleetPlanDigestMismatchError {
            expected: expected.to_string(),
            observed: observed.to_string(),
        }),
        _ => Ok(()),
    }
}

fn print_fresh_fleet_decision(plan: &FreshFleetDeploymentPlanV1) {
    let maximum_debit = match &plan.maximum_operator_debit {
        PlannedCanisterCreationFunding::Cycles { cycles } => format!("{cycles} cycles"),
        PlannedCanisterCreationFunding::Icp { e8s } => format!("{e8s} ICP e8s"),
    };
    TerminalStyle::detected().print_section(
        "Fresh-Fleet decision",
        &format!(
            "plan {} with maximum operator debit {maximum_debit}",
            plan.plan_digest
        ),
    );
    print_fresh_fleet_placement_warnings(plan);
    println!();
}

fn print_fresh_fleet_recovery(recovery: &FreshFleetInstallRecoveryPlanV1) {
    let remaining = match &recovery.remaining_operator_debit {
        PlannedCanisterCreationFunding::Cycles { cycles } => format!("{cycles} cycles"),
        PlannedCanisterCreationFunding::Icp { e8s } => format!("{e8s} ICP e8s"),
    };
    TerminalStyle::detected().print_section(
        "Fresh-Fleet recovery",
        &format!(
            "session {} retains release build {} (Canic {}, contract {}), resumes at {}, and may issue at most {remaining} remaining operator debit",
            recovery.fleet_install_operation_id,
            recovery.release_build_id,
            recovery.retained_builder_version,
            recovery.retained_plan_contract.as_str(),
            recovery.next_replay_phase,
        ),
    );
    if recovery.has_uncertain_creation_outcome() {
        TerminalStyle::detected().print_section(
            "Fenced creation observation",
            &format!(
                "observe without reissuing: {}",
                recovery.uncertain_creation_outcomes.join(", ")
            ),
        );
    }
    println!();
}

fn print_fresh_fleet_placement_warnings(plan: &FreshFleetDeploymentPlanV1) {
    let style = TerminalStyle::detected();
    if let Some(warning) = plan.preflight.coordinator.placement_cost.warning.as_deref() {
        style.print_section("Fiduciary placement warning", warning);
    }
    for root in &plan.preflight.fleet_subnet_roots {
        if let Some(warning) = root.placement_cost.warning.as_deref() {
            style.print_section("Fiduciary placement warning", warning);
        }
    }
}

fn print_paid_effect_placement_warnings(plan: &crate::fleet_install_plan::FleetInstallPlan) {
    let style = TerminalStyle::detected();
    if let Some(warning) = plan.coordinator.placement_cost.warning.as_deref() {
        style.print_section("Fiduciary paid-effect warning", warning);
    }
    for root in &plan.fleet_subnet_roots {
        if let Some(warning) = root.placement_cost.warning.as_deref() {
            style.print_section("Fiduciary paid-effect warning", warning);
        }
    }
    if plan.coordinator.placement_cost.warning.is_some()
        || plan
            .fleet_subnet_roots
            .iter()
            .any(|root| root.placement_cost.warning.is_some())
    {
        println!();
    }
}

fn print_install_identity(app: &str, fleet_name: &str) {
    TerminalStyle::detected().print_section(
        &format!("Install Fleet {fleet_name}"),
        &format!("source App {app}"),
    );
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
    icp_context: &InstallIcpContext,
    config_path: &Path,
    plan: &PersistedFleetInstallPlan,
) -> Result<(coordinator_install::VerifiedFleetCoordinator, Duration), InstallRootError> {
    let started = Instant::now();
    let coordinator = install_and_verify_fleet_coordinator(icp_context, config_path, plan)
        .map_err(InstallRootError::in_phase(InstallRootPhase::Activation))?;
    Ok((coordinator, started.elapsed()))
}

fn install_current_fleet_subnet_roots(
    icp_context: &InstallIcpContext,
    config_path: &Path,
    planned: &PlannedCurrentFleetInstall,
    coordinator: canic_core::cdk::types::Principal,
    recovery_bundle: &FleetInstallRecoveryBundleCheckpoint<'_>,
) -> Result<Duration, InstallRootError> {
    let started = Instant::now();
    install_and_verify_fleet_subnet_roots(InstallFleetSubnetRootsRequest {
        icp_context,
        config_path,
        fleet_install_plan: &planned.plan,
        fleet_install_session: &planned.session,
        coordinator,
        install_operation_id: planned.session.operation_id,
        retained_root_repair_adoption: planned.retained_root_repair_adoption.as_ref(),
        recovery_bundle,
    })
    .map_err(InstallRootError::in_phase(InstallRootPhase::Activation))?;
    Ok(started.elapsed())
}

fn persist_pre_root_receipts(
    receipt_scope: InstallReceiptScope<'_>,
    prepared_receipts: &[DeploymentReceiptV1],
    build_phase: CompletedInstallPhase,
    manifest_phase: CompletedInstallPhase,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut paths = Vec::with_capacity(prepared_receipts.len() + 2);
    for receipt in prepared_receipts {
        paths.push(receipt_scope.write_receipt(receipt)?);
    }
    paths.push(write_completed_install_phase_receipt(
        receipt_scope,
        build_phase,
    )?);
    paths.push(write_completed_install_phase_receipt(
        receipt_scope,
        manifest_phase,
    )?);
    let receipt_root = paths.first().and_then(|path| path.parent()).map_or_else(
        || "deployment receipt directory".to_string(),
        |path| path.display().to_string(),
    );
    TerminalStyle::detected().print_section(
        "Receipts",
        &format!("{} written to {receipt_root}", paths.len()),
    );
    println!();
    Ok(())
}

fn current_install_build_inputs(
    workspace_root: &std::path::Path,
    icp_root: &std::path::Path,
    config_path: &std::path::Path,
    icp: &InstallIcpContext,
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
        config_path,
        icp,
        &options.root_build_target,
        options.build_profile,
    )?;
    if options.deployment_plan_override.is_some() {
        let snapshot = resolve_install_snapshot(
            &context,
            &options.root_build_target,
            InstallSnapshotSource::DeploymentPlan,
        )?;
        return Ok((context, snapshot));
    }

    let config = AppConfigSnapshot::load(config_path)?;
    let release_build = current_install_release_build(
        icp_root,
        &options.environment,
        &options.fleet_name,
        config.app_id(),
        options.release_build_id,
        options.build_profile,
    )?;
    context = context
        .with_profile(release_build.record.build_profile)
        .with_release_build_id(release_build.record.release_build_id);
    let source = if matches!(
        release_build.record.state,
        crate::release_build::ReleaseBuildPlanState::Finalized { .. }
    ) {
        InstallSnapshotSource::FinalizedRelease(&release_build)
    } else {
        InstallSnapshotSource::WorkspaceBuild
    };
    let mut snapshot = resolve_install_snapshot(&context, &options.root_build_target, source)?;
    snapshot.release_build = Some(release_build);
    Ok((context, snapshot))
}

fn current_install_release_build(
    icp_root: &Path,
    environment: &str,
    fleet_name: &str,
    app_id: &str,
    requested_release_build_id: Option<canic_core::ids::ReleaseBuildId>,
    requested_build_profile: Option<crate::canister_build::CanisterBuildProfile>,
) -> Result<PlannedReleaseBuild, Box<dyn std::error::Error>> {
    let canonical_network_id = resolve_canonical_network_id_from_root(icp_root, environment)?;
    let fleet_name = fleet_name.parse()?;
    let app = app_id.into();
    if let Some(recovered) = fleet_install_session::recover_fleet_install_session_authority(
        icp_root,
        canonical_network_id,
        &fleet_name,
        &app,
    )? {
        let finalized = recovered.finalized_release_build;
        if requested_release_build_id
            .is_some_and(|requested| requested != finalized.record.release_build_id)
        {
            return Err(
                "requested release build differs from the interrupted Fleet install session".into(),
            );
        }
        require_requested_build_profile(requested_build_profile, finalized.record.build_profile)?;
        return Ok(PlannedReleaseBuild {
            record: finalized.record,
            path: finalized.path,
        });
    }

    if let Some(release_build_id) = requested_release_build_id {
        let finalized = load_finalized_release_build(icp_root, release_build_id)?;
        require_current_release_builder(&finalized.record.builder_version)?;
        require_requested_build_profile(requested_build_profile, finalized.record.build_profile)?;
        return Ok(PlannedReleaseBuild {
            record: finalized.record,
            path: finalized.path,
        });
    }

    plan_release_build_for_profile(
        icp_root,
        requested_build_profile.unwrap_or(crate::canister_build::CanisterBuildProfile::Release),
    )
    .map_err(Into::into)
}

fn require_current_release_builder(recorded: &str) -> Result<(), Box<dyn std::error::Error>> {
    if recorded != env!("CARGO_PKG_VERSION") {
        return Err(format!(
            "finalized release build belongs to Canic {recorded}, not current Canic {}",
            env!("CARGO_PKG_VERSION")
        )
        .into());
    }
    Ok(())
}

fn require_requested_build_profile(
    requested: Option<crate::canister_build::CanisterBuildProfile>,
    recorded: crate::canister_build::CanisterBuildProfile,
) -> Result<(), Box<dyn std::error::Error>> {
    if requested.is_some_and(|requested| requested != recorded) {
        return Err(format!(
            "requested build profile differs from finalized release build profile {}",
            recorded.target_dir_name()
        )
        .into());
    }
    Ok(())
}
