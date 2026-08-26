//! Module: install_root::fleet_subnet_root_install
//!
//! Responsibility: create, install, and independently verify every planned Fleet Subnet Root.
//! Does not own: local Wasm Store bootstrap, Fleet Registry registration, or runtime activation.
//! Boundary: roots are installed serially from canonical plan order; uncertain paid effects remain
//! in explicit durable in-flight phases and are observed rather than blindly replayed.

use super::{
    commands::prepare_creation_result,
    fleet_install_recovery_bundle::FleetInstallRecoveryBundleCheckpoint,
    fleet_install_session::FleetInstallSession,
    fleet_subnet_root_component_registry_preparation::verify_retained_component_registry_preparation,
    fleet_subnet_root_install_journal::{
        FleetSubnetRootInstallJournal, FleetSubnetRootInstallPhase,
        PlanFleetSubnetRootInstallRequest, ResolvedFleetSubnetRootInstall, begin_root_creation,
        begin_root_install, begin_wasm_store_creation, begin_wasm_store_install,
        create_result_path, expected_root_authority, expected_wasm_store_authority,
        inspect_fleet_subnet_root_install, plan_fleet_subnet_root_install,
        record_infrastructure_verified, record_root_created, record_root_installed,
        record_wasm_store_created, record_wasm_store_installed, wasm_store_create_result_path,
    },
    fleet_subnet_root_repair::{
        ResolvedRetainedRootRepair, RetainedRootRepairAuthorityV1, execute_retained_root_repair,
        publish_retained_root_repair_authority, publish_retained_root_repair_receipt,
        reconcile_published_retained_root_repair, resolve_retained_root_repair,
    },
    icp_context::InstallIcpContext,
    operations::{
        CreationEffectRequest, EffectAction, InstallArtifact, InstallEffectRequest,
        active_installation_controller, execute_or_observe_creation, execute_or_observe_install,
        observe_module_hash, query_with_arg, require_expected_controllers,
        require_expected_module_hash, resolve_install_artifact,
    },
    options::RetainedRootRepairAdoption,
};
use crate::{
    fleet_install_plan::{PersistedFleetInstallPlan, required_initial_pool_asset_cycles},
    icp::IcpCli,
    protocol_binding::{ResolvedProtocolBinding, resolve_infrastructure_protocol_binding},
    release_set::{
        AppConfigSnapshot, CanicInfrastructureRole,
        load_persisted_canic_infrastructure_artifact_manifest,
    },
};
use std::path::{Path, PathBuf};

use candid::{CandidType, Principal};
use canic_control_plane::dto::{
    root::RootOperationStatusResponse,
    template::{StoreOperationStatusResponse, StoreStatusRequest, StoreStatusResponse},
};
use canic_core::{
    dto::{
        fleet_activation::{
            FleetActivationIdentity, FleetActivationPhase, FleetActivationStatusResponse,
        },
        fleet_subnet_root::{
            FleetSubnetRootAuthority, FleetSubnetRootInitArgs, FleetSubnetWasmStoreInitArgs,
        },
        role::OperationStatusRequest,
    },
    ids::FleetSubnetWasmStoreAuthority,
    protocol,
};
use serde::Deserialize;
use thiserror::Error as ThisError;

const MAX_ROOT_TRANSITIONS: usize = 12;
const ROOT_INSTALL_ARGS_FILE: &str = "root-install-args.bin";
const WASM_STORE_INSTALL_ARGS_FILE: &str = "wasm-store-install-args.bin";

#[derive(CandidType)]
enum RootStatusRequestFragment {
    FleetAuthority,
    Operation(OperationStatusRequest),
}

#[derive(CandidType, Deserialize)]
enum RootStatusResponseFragment {
    FleetAuthority(Box<FleetSubnetRootAuthority>),
    Operation(Box<RootOperationStatusResponse>),
}

#[derive(Debug, ThisError)]
#[error(
    "Fleet Subnet Root creation outcome on {placement_subnet} is unknown; no second paid creation was attempted. Inspect durable result {result_path} and retry after the original ICP command has settled: {detail}"
)]
struct RootCreationOutcomeUnknownError {
    placement_subnet: canic_core::ids::SubnetId,
    result_path: PathBuf,
    detail: String,
}

#[derive(Debug, ThisError)]
#[error(
    "Wasm Store creation outcome on {placement_subnet} is unknown; no second paid creation was attempted. Inspect durable result {result_path} and retry after the original ICP command has settled: {detail}"
)]
struct WasmStoreCreationOutcomeUnknownError {
    placement_subnet: canic_core::ids::SubnetId,
    result_path: PathBuf,
    detail: String,
}

#[derive(Debug, ThisError)]
enum RootInstallStateError {
    #[error("{subject} did not initialize with the exact Prepared Fleet runtime identity")]
    ActivationStatusMismatch { subject: &'static str },

    #[error("Fleet Subnet Root protected authority query differs from exact planned binding")]
    AuthorityMismatch,

    #[error("Wasm Store protected authority query differs from exact planned binding")]
    WasmStoreAuthorityMismatch,

    #[error("Fleet Subnet Root installation exceeded its bounded phase transitions")]
    TransitionBoundExceeded,

    #[error("retained Root repair request names a Root outside the retained install journals")]
    RepairRootNotFound,

    #[error("active installation identity differs from the retained Root repair controller")]
    RepairControllerMismatch,

    #[error("retained Root repair authority disappeared during exact policy revalidation")]
    RepairAuthorityMissing,

    #[error("retained install preflight requires an existing Root journal on {placement_subnet}")]
    PreflightJournalMissing {
        placement_subnet: canic_core::ids::SubnetId,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RetainedRootRepairLiveModule {
    Predecessor,
    Successor,
}

pub(super) struct InstallFleetSubnetRootsRequest<'a> {
    pub icp_context: &'a InstallIcpContext,
    pub config_path: &'a Path,
    pub fleet_install_plan: &'a PersistedFleetInstallPlan,
    pub fleet_install_session: &'a FleetInstallSession,
    pub coordinator: Principal,
    pub install_operation_id: [u8; 32],
    pub retained_root_repair_adoption: Option<&'a RetainedRootRepairAdoption>,
    pub recovery_bundle: &'a FleetInstallRecoveryBundleCheckpoint<'a>,
}

pub(super) struct PreflightFleetSubnetRootsRequest<'a> {
    pub icp_root: &'a Path,
    pub config_path: &'a Path,
    pub fleet_install_plan: &'a PersistedFleetInstallPlan,
    pub fleet_install_session: &'a FleetInstallSession,
    pub coordinator: Principal,
    pub install_operation_id: [u8; 32],
    pub retained_root_repair_adoption: Option<&'a RetainedRootRepairAdoption>,
    pub recovery_bundle: &'a FleetInstallRecoveryBundleCheckpoint<'a>,
}

#[derive(Clone, Copy)]
enum RootJournalAccess {
    InspectExisting,
    PlanOrRecover,
}

struct PreparedFleetSubnetRootInstall {
    config: AppConfigSnapshot,
    component_topology: canic_core::bootstrap::compiled::ComponentTopology,
    root_artifact: InstallArtifact,
    wasm_store_artifact: InstallArtifact,
    planned_roots: Vec<ResolvedFleetSubnetRootInstall>,
}

pub(super) fn install_and_verify_fleet_subnet_roots(
    request: InstallFleetSubnetRootsRequest<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let InstallFleetSubnetRootsRequest {
        icp_context,
        config_path,
        fleet_install_plan,
        fleet_install_session,
        coordinator,
        install_operation_id,
        retained_root_repair_adoption,
        recovery_bundle,
    } = request;
    let prepared = prepare_fleet_subnet_root_install(
        icp_context.root(),
        config_path,
        fleet_install_plan,
        coordinator,
        install_operation_id,
        RootJournalAccess::PlanOrRecover,
    )?;
    require_repair_root(&prepared.planned_roots, retained_root_repair_adoption)?;
    recovery_bundle.checkpoint()?;

    let mut roots = Vec::with_capacity(prepared.planned_roots.len());
    for current in prepared.planned_roots {
        let adoption = retained_root_repair_adoption.filter(|adoption| {
            current.journal.fleet_subnet_root == Some(adoption.fleet_subnet_root)
        });
        let required_pool_cycles = adoption
            .map(|_| {
                required_initial_pool_asset_cycles(
                    prepared.config.model(),
                    &current.journal.root_plan,
                )
            })
            .transpose()?
            .map(|cycles| cycles.to_u128());
        let repair = resolve_and_execute_retained_root_repair(
            icp_context,
            &current,
            fleet_install_session,
            adoption,
            required_pool_cycles,
            recovery_bundle,
        )?;
        roots.push(drive_root_install(
            icp_context,
            &prepared.root_artifact,
            &prepared.wasm_store_artifact,
            &fleet_install_plan.plan.fresh_fleet_plan_digest,
            current,
            repair.as_ref().map(|repair| &repair.authority),
            recovery_bundle,
        )?);
    }

    let bindings = roots
        .iter()
        .map(|authority| authority.binding.clone())
        .collect::<Vec<_>>();
    prepared
        .component_topology
        .validate_fleet_subnet_root_bindings(&bindings)?;
    Ok(())
}

pub(super) fn preflight_fleet_subnet_roots(
    request: PreflightFleetSubnetRootsRequest<'_>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let PreflightFleetSubnetRootsRequest {
        icp_root,
        config_path,
        fleet_install_plan,
        fleet_install_session,
        coordinator,
        install_operation_id,
        retained_root_repair_adoption,
        recovery_bundle,
    } = request;
    let prepared = prepare_fleet_subnet_root_install(
        icp_root,
        config_path,
        fleet_install_plan,
        coordinator,
        install_operation_id,
        RootJournalAccess::InspectExisting,
    )?;
    require_repair_root(&prepared.planned_roots, retained_root_repair_adoption)?;
    recovery_bundle.checkpoint()?;

    for current in prepared.planned_roots {
        let adoption = retained_root_repair_adoption.filter(|adoption| {
            current.journal.fleet_subnet_root == Some(adoption.fleet_subnet_root)
        });
        let required_pool_cycles = adoption
            .map(|_| {
                required_initial_pool_asset_cycles(
                    prepared.config.model(),
                    &current.journal.root_plan,
                )
            })
            .transpose()?
            .map(|cycles| cycles.to_u128());
        let _repair = resolve_and_checkpoint_retained_root_repair(
            &current,
            fleet_install_session,
            adoption,
            required_pool_cycles,
            recovery_bundle,
        )?;
    }

    recovery_bundle.checkpoint().map_err(Into::into)
}

fn prepare_fleet_subnet_root_install(
    icp_root: &Path,
    config_path: &Path,
    fleet_install_plan: &PersistedFleetInstallPlan,
    coordinator: Principal,
    install_operation_id: [u8; 32],
    journal_access: RootJournalAccess,
) -> Result<PreparedFleetSubnetRootInstall, Box<dyn std::error::Error>> {
    let config = AppConfigSnapshot::load(config_path)?;
    let component_topology = config.model().compile_component_topology()?;
    let infrastructure_manifest = load_persisted_canic_infrastructure_artifact_manifest(
        icp_root,
        fleet_install_plan.plan.release_build_id,
    )?;
    let root_artifact = resolve_install_artifact(
        icp_root,
        &infrastructure_manifest,
        CanicInfrastructureRole::FleetSubnetRoot,
        fleet_install_plan.plan.release_build_id,
    )?;
    let wasm_store_artifact = resolve_install_artifact(
        icp_root,
        &infrastructure_manifest,
        CanicInfrastructureRole::WasmStore,
        fleet_install_plan.plan.release_build_id,
    )?;
    let mut planned_roots = Vec::with_capacity(fleet_install_plan.plan.fleet_subnet_roots.len());
    for root_plan in &fleet_install_plan.plan.fleet_subnet_roots {
        let request = PlanFleetSubnetRootInstallRequest {
            fleet_install_plan,
            infrastructure_manifest: &infrastructure_manifest,
            coordinator,
            install_operation_id,
            component_topology: component_topology.clone(),
            root_plan,
        };
        let current = match journal_access {
            RootJournalAccess::InspectExisting => inspect_fleet_subnet_root_install(request)?
                .ok_or(RootInstallStateError::PreflightJournalMissing {
                    placement_subnet: root_plan.placement_subnet,
                })?,
            RootJournalAccess::PlanOrRecover => plan_fleet_subnet_root_install(request)?,
        };
        planned_roots.push(current);
    }
    Ok(PreparedFleetSubnetRootInstall {
        config,
        component_topology,
        root_artifact,
        wasm_store_artifact,
        planned_roots,
    })
}

fn require_repair_root(
    planned_roots: &[ResolvedFleetSubnetRootInstall],
    adoption: Option<&RetainedRootRepairAdoption>,
) -> Result<(), RootInstallStateError> {
    if adoption.is_some_and(|adoption| {
        !planned_roots
            .iter()
            .any(|current| current.journal.fleet_subnet_root == Some(adoption.fleet_subnet_root))
    }) {
        return Err(RootInstallStateError::RepairRootNotFound);
    }
    Ok(())
}

fn resolve_and_execute_retained_root_repair(
    icp_context: &InstallIcpContext,
    current: &ResolvedFleetSubnetRootInstall,
    session: &FleetInstallSession,
    adoption: Option<&RetainedRootRepairAdoption>,
    required_pool_cycles: Option<u128>,
    recovery_bundle: &FleetInstallRecoveryBundleCheckpoint<'_>,
) -> Result<Option<ResolvedRetainedRootRepair>, Box<dyn std::error::Error>> {
    let repair = resolve_and_checkpoint_retained_root_repair(
        current,
        session,
        adoption,
        required_pool_cycles,
        recovery_bundle,
    )?;
    let Some(resolved) = repair.as_ref() else {
        return Ok(None);
    };
    let active_controller = active_installation_controller(icp_context.cli())?;
    if current.journal.installation_controller != Some(active_controller) {
        return Err(RootInstallStateError::RepairControllerMismatch.into());
    }
    let root_binding = resolve_infrastructure_protocol_binding(
        icp_context.root(),
        icp_context.environment(),
        &current.journal.root_artifact,
    )?;
    let live_module = verify_pre_repair_root_authority(
        icp_context,
        &root_binding,
        &current.journal,
        &resolved.authority,
    )?;
    report_retained_root_repair_position(live_module, current.journal.phase);
    publish_retained_root_repair_authority(resolved, session, &current.journal)?;
    recovery_bundle.checkpoint()?;
    if resolved.terminal_receipt.is_some() {
        reconcile_published_retained_root_repair(resolved)?;
        recovery_bundle.checkpoint()?;
    } else {
        let _operation = execute_retained_root_repair(
            icp_context,
            &root_binding,
            resolved,
            &resolved.successor_wasm_path,
            recovery_bundle,
        )?;
        verify_live_infrastructure(icp_context, &current.journal, Some(&resolved.authority))?;
    }
    Ok(repair)
}

fn resolve_and_checkpoint_retained_root_repair(
    current: &ResolvedFleetSubnetRootInstall,
    session: &FleetInstallSession,
    adoption: Option<&RetainedRootRepairAdoption>,
    required_pool_cycles: Option<u128>,
    recovery_bundle: &FleetInstallRecoveryBundleCheckpoint<'_>,
) -> Result<Option<ResolvedRetainedRootRepair>, Box<dyn std::error::Error>> {
    let repair = resolve_retained_root_repair(current, session, adoption, required_pool_cycles)?;
    if repair
        .as_ref()
        .is_some_and(|repair| repair.needs_authority_publication)
    {
        recovery_bundle.checkpoint()?;
    }
    Ok(repair)
}

fn report_retained_root_repair_position(
    live_module: RetainedRootRepairLiveModule,
    phase: FleetSubnetRootInstallPhase,
) {
    match live_module {
        RetainedRootRepairLiveModule::Predecessor => println!(
            "Retained Root recovery: the exact predecessor is live; the authorized repair will begin from durable phase {phase:?} before canonical replay continues."
        ),
        RetainedRootRepairLiveModule::Successor => println!(
            "Retained Root recovery: durable phase {phase:?} is behind the verified exact live successor; canonical phase owners will re-observe monotonic state without synthesizing journal evidence."
        ),
    }
}

/// Finalize retained Root repairs only after the ordinary phase owners have replayed every Root to
/// the protected Component Registry proof boundary.
pub(super) fn finalize_retained_root_repairs(
    icp_context: &InstallIcpContext,
    config_path: &Path,
    fleet_install_plan: &PersistedFleetInstallPlan,
    fleet_install_session: &FleetInstallSession,
    coordinator: Principal,
    recovery_bundle: &FleetInstallRecoveryBundleCheckpoint<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfigSnapshot::load(config_path)?;
    let component_topology = config.model().compile_component_topology()?;
    let infrastructure_manifest = load_persisted_canic_infrastructure_artifact_manifest(
        icp_context.root(),
        fleet_install_plan.plan.release_build_id,
    )?;
    for root_plan in &fleet_install_plan.plan.fleet_subnet_roots {
        let current = plan_fleet_subnet_root_install(PlanFleetSubnetRootInstallRequest {
            fleet_install_plan,
            infrastructure_manifest: &infrastructure_manifest,
            coordinator,
            install_operation_id: fleet_install_session.operation_id,
            component_topology: component_topology.clone(),
            root_plan,
        })?;
        let Some(_) = resolve_retained_root_repair(&current, fleet_install_session, None, None)?
        else {
            continue;
        };
        let required_pool_cycles =
            required_initial_pool_asset_cycles(config.model(), &current.journal.root_plan)?
                .to_u128();
        let repair = resolve_retained_root_repair(
            &current,
            fleet_install_session,
            None,
            Some(required_pool_cycles),
        )?
        .ok_or(RootInstallStateError::RepairAuthorityMissing)?;
        verify_live_infrastructure(icp_context, &current.journal, Some(&repair.authority))?;
        verify_retained_component_registry_preparation(icp_context, &current.journal)?;
        let _receipt =
            publish_retained_root_repair_receipt(&repair, fleet_install_session, &current.journal)?;
        recovery_bundle.checkpoint()?;
        reconcile_published_retained_root_repair(&repair)?;
        recovery_bundle.checkpoint()?;
    }
    Ok(())
}

fn drive_root_install(
    icp_context: &InstallIcpContext,
    root_artifact: &InstallArtifact,
    wasm_store_artifact: &InstallArtifact,
    fresh_fleet_plan_digest: &str,
    mut current: ResolvedFleetSubnetRootInstall,
    repair: Option<&RetainedRootRepairAuthorityV1>,
    recovery_bundle: &FleetInstallRecoveryBundleCheckpoint<'_>,
) -> Result<FleetSubnetRootAuthority, Box<dyn std::error::Error>> {
    for _ in 0..MAX_ROOT_TRANSITIONS {
        current = match current.journal.phase {
            FleetSubnetRootInstallPhase::Planned => {
                prepare_creation_result(&create_result_path(&current.path), "Fleet Subnet Root")?;
                let installation_controller = active_installation_controller(icp_context.cli())?;
                begin_root_creation(&current, installation_controller)?
            }
            FleetSubnetRootInstallPhase::RootCreationInFlight => {
                recover_or_create_root(icp_context, &current)?
            }
            FleetSubnetRootInstallPhase::RootCreated => {
                prepare_creation_result(
                    &wasm_store_create_result_path(&current.path),
                    "Wasm Store",
                )?;
                begin_wasm_store_creation(&current)?
            }
            FleetSubnetRootInstallPhase::WasmStoreCreationInFlight => {
                recover_or_create_wasm_store(icp_context, &current)?
            }
            FleetSubnetRootInstallPhase::WasmStoreCreated => begin_wasm_store_install(&current)?,
            FleetSubnetRootInstallPhase::WasmStoreInstallInFlight => recover_or_install_wasm_store(
                icp_context,
                wasm_store_artifact,
                fresh_fleet_plan_digest,
                &current,
            )?,
            FleetSubnetRootInstallPhase::WasmStoreInstalled => begin_root_install(&current)?,
            FleetSubnetRootInstallPhase::RootInstallInFlight => recover_or_install_root(
                icp_context,
                root_artifact,
                fresh_fleet_plan_digest,
                &current,
            )?,
            FleetSubnetRootInstallPhase::RootInstalled => {
                verify_and_record_infrastructure(icp_context, &current, repair)?
            }
            FleetSubnetRootInstallPhase::InfrastructureVerified
            | FleetSubnetRootInstallPhase::StoreAdoptionInFlight
            | FleetSubnetRootInstallPhase::StoreAdopted
            | FleetSubnetRootInstallPhase::StoreStaging
            | FleetSubnetRootInstallPhase::StoreStaged
            | FleetSubnetRootInstallPhase::StoreBootstrapInFlight
            | FleetSubnetRootInstallPhase::StoreBootstrapped
            | FleetSubnetRootInstallPhase::StoreVerified
            | FleetSubnetRootInstallPhase::RegistryJoinInFlight
            | FleetSubnetRootInstallPhase::RegistryJoined
            | FleetSubnetRootInstallPhase::RegistryJoinVerified
            | FleetSubnetRootInstallPhase::RegistrySyncInFlight
            | FleetSubnetRootInstallPhase::RegistrySynchronized
            | FleetSubnetRootInstallPhase::RegistrySyncVerified
            | FleetSubnetRootInstallPhase::RegistryMirrorActivationInFlight
            | FleetSubnetRootInstallPhase::RegistryMirrorActivated
            | FleetSubnetRootInstallPhase::RegistryMirrorActivationVerified
            | FleetSubnetRootInstallPhase::ComponentRegistryPreparationInFlight
            | FleetSubnetRootInstallPhase::ComponentRegistryPrepared
            | FleetSubnetRootInstallPhase::ComponentRegistryPreparationVerified => {
                let (authority, _) =
                    verify_live_infrastructure(icp_context, &current.journal, repair)?;
                return Ok(authority);
            }
        };
        recovery_bundle.checkpoint()?;
    }
    Err(RootInstallStateError::TransitionBoundExceeded.into())
}

pub(super) fn verify_pre_repair_root_authority(
    icp_context: &InstallIcpContext,
    root_binding: &ResolvedProtocolBinding,
    journal: &FleetSubnetRootInstallJournal,
    repair: &RetainedRootRepairAuthorityV1,
) -> Result<RetainedRootRepairLiveModule, Box<dyn std::error::Error>> {
    let fleet_subnet_root = journal
        .fleet_subnet_root
        .expect("repair journal retains its Root");
    require_expected_controllers(
        icp_context.cli(),
        fleet_subnet_root,
        std::slice::from_ref(
            &journal
                .installation_controller
                .expect("repair journal retains its controller"),
        ),
        "Fleet Subnet Root repair predecessor",
    )?;
    let observed_module = observe_module_hash(icp_context.cli(), fleet_subnet_root)?;
    let live_module = match observed_module {
        Some(observed) if observed == repair.upgrade_predecessor_module_sha256 => {
            RetainedRootRepairLiveModule::Predecessor
        }
        Some(observed) if observed == repair.successor_module_sha256 => {
            RetainedRootRepairLiveModule::Successor
        }
        Some(_) | None => return Err(RootInstallStateError::AuthorityMismatch.into()),
    };
    let expected = expected_root_authority(journal)?;
    let observed = query_with_arg::<_, RootStatusResponseFragment>(
        icp_context.cli(),
        root_binding,
        fleet_subnet_root,
        protocol::CANIC_STATUS,
        &RootStatusRequestFragment::FleetAuthority,
    )?;
    let RootStatusResponseFragment::FleetAuthority(observed) = observed else {
        return Err(RootInstallStateError::AuthorityMismatch.into());
    };
    if *observed != expected {
        return Err(RootInstallStateError::AuthorityMismatch.into());
    }
    Ok(live_module)
}

fn recover_or_create_root(
    icp_context: &InstallIcpContext,
    current: &ResolvedFleetSubnetRootInstall,
) -> Result<ResolvedFleetSubnetRootInstall, Box<dyn std::error::Error>> {
    let result_path = create_result_path(&current.path);
    let installation_controller = current
        .journal
        .installation_controller
        .expect("root creation intent retains its installation controller");
    let evidence = execute_or_observe_creation(CreationEffectRequest {
        icp: icp_context,
        result_path: &result_path,
        subject: "Fleet Subnet Root",
        placement_subnet: current.journal.root_plan.placement_subnet,
        funding: &current.journal.root_plan.root_creation_funding,
        controllers: std::slice::from_ref(&installation_controller),
        action: EffectAction::from_advanced(current.advanced),
        expected_module_hash: current.journal.expected_root_module_hash,
    })?;
    let Some(fleet_subnet_root) = evidence.canister else {
        return Err(RootCreationOutcomeUnknownError {
            placement_subnet: current.journal.root_plan.placement_subnet,
            result_path,
            detail: evidence.command_error.unwrap_or_else(|| {
                "the journal is already creation_in_flight and has no recoverable principal"
                    .to_string()
            }),
        }
        .into());
    };
    record_root_created(current, fleet_subnet_root).map_err(Into::into)
}

fn recover_or_install_root(
    icp_context: &InstallIcpContext,
    artifact: &InstallArtifact,
    fresh_fleet_plan_digest: &str,
    current: &ResolvedFleetSubnetRootInstall,
) -> Result<ResolvedFleetSubnetRootInstall, Box<dyn std::error::Error>> {
    let fleet_subnet_root = current
        .journal
        .fleet_subnet_root
        .expect("validated InstallInFlight journal retains its root");
    let args_path = current.path.with_file_name(ROOT_INSTALL_ARGS_FILE);
    let module_hash = execute_or_observe_install(
        InstallEffectRequest {
            icp: icp_context,
            subject: "Fleet Subnet Root",
            canister: fleet_subnet_root,
            wasm_path: &artifact.wasm_path,
            args_path: &args_path,
            expected_module_hash: current.journal.expected_root_module_hash,
            fresh_fleet_plan_digest,
            action: EffectAction::from_advanced(current.advanced),
        },
        || root_install_args(&current.journal),
    )?;
    record_root_installed(current, module_hash).map_err(Into::into)
}

fn recover_or_create_wasm_store(
    icp_context: &InstallIcpContext,
    current: &ResolvedFleetSubnetRootInstall,
) -> Result<ResolvedFleetSubnetRootInstall, Box<dyn std::error::Error>> {
    let result_path = wasm_store_create_result_path(&current.path);
    let controllers = temporary_store_controllers(&current.journal);
    let evidence = execute_or_observe_creation(CreationEffectRequest {
        icp: icp_context,
        result_path: &result_path,
        subject: "Wasm Store",
        placement_subnet: current.journal.root_plan.placement_subnet,
        funding: &current.journal.root_plan.wasm_store_creation_funding,
        controllers: &controllers,
        action: EffectAction::from_advanced(current.advanced),
        expected_module_hash: current.journal.expected_wasm_store_module_hash,
    })?;
    let Some(wasm_store) = evidence.canister else {
        return Err(WasmStoreCreationOutcomeUnknownError {
            placement_subnet: current.journal.root_plan.placement_subnet,
            result_path,
            detail: evidence.command_error.unwrap_or_else(|| {
                "the journal is already wasm_store_creation_in_flight and has no recoverable principal"
                    .to_string()
            }),
        }
        .into());
    };
    record_wasm_store_created(current, wasm_store).map_err(Into::into)
}

fn temporary_store_controllers(journal: &FleetSubnetRootInstallJournal) -> Vec<Principal> {
    let mut controllers = vec![
        journal
            .installation_controller
            .expect("Wasm Store creation intent retains its installation controller"),
        journal
            .fleet_subnet_root
            .expect("Wasm Store creation intent retains its root"),
    ];
    controllers.sort();
    controllers.dedup();
    controllers
}

fn recover_or_install_wasm_store(
    icp_context: &InstallIcpContext,
    artifact: &InstallArtifact,
    fresh_fleet_plan_digest: &str,
    current: &ResolvedFleetSubnetRootInstall,
) -> Result<ResolvedFleetSubnetRootInstall, Box<dyn std::error::Error>> {
    let wasm_store = current
        .journal
        .wasm_store
        .expect("validated WasmStoreInstallInFlight journal retains its Store");
    let args_path = current.path.with_file_name(WASM_STORE_INSTALL_ARGS_FILE);
    let module_hash = execute_or_observe_install(
        InstallEffectRequest {
            icp: icp_context,
            subject: "Wasm Store",
            canister: wasm_store,
            wasm_path: &artifact.wasm_path,
            args_path: &args_path,
            expected_module_hash: current.journal.expected_wasm_store_module_hash,
            fresh_fleet_plan_digest,
            action: EffectAction::from_advanced(current.advanced),
        },
        || wasm_store_install_args(&current.journal),
    )?;
    record_wasm_store_installed(current, module_hash).map_err(Into::into)
}

fn verify_and_record_infrastructure(
    icp_context: &InstallIcpContext,
    current: &ResolvedFleetSubnetRootInstall,
    repair: Option<&RetainedRootRepairAuthorityV1>,
) -> Result<ResolvedFleetSubnetRootInstall, Box<dyn std::error::Error>> {
    let (root_authority, wasm_store_authority) =
        verify_live_infrastructure(icp_context, &current.journal, repair)?;
    record_infrastructure_verified(current, root_authority, wasm_store_authority)
        .map_err(Into::into)
}

fn verify_live_infrastructure(
    icp_context: &InstallIcpContext,
    journal: &FleetSubnetRootInstallJournal,
    repair: Option<&RetainedRootRepairAuthorityV1>,
) -> Result<(FleetSubnetRootAuthority, FleetSubnetWasmStoreAuthority), Box<dyn std::error::Error>> {
    let fleet_subnet_root = journal
        .fleet_subnet_root
        .expect("installed root journal retains its principal");
    let icp = icp_context.cli();
    let (root_binding, store_binding) = resolve_live_protocol_bindings(icp_context, journal)?;
    require_expected_controllers(
        icp,
        fleet_subnet_root,
        std::slice::from_ref(
            &journal
                .installation_controller
                .expect("installed root retains its installation controller"),
        ),
        "Fleet Subnet Root",
    )?;
    require_expected_module_hash(
        icp,
        fleet_subnet_root,
        repair.map_or(journal.expected_root_module_hash, |repair| {
            repair.successor_module_hash()
        }),
        "Fleet Subnet Root",
    )?;

    let expected = expected_root_authority(journal)?;
    let observed = query_with_arg::<_, RootStatusResponseFragment>(
        icp,
        &root_binding,
        fleet_subnet_root,
        protocol::CANIC_STATUS,
        &RootStatusRequestFragment::FleetAuthority,
    )?;
    let RootStatusResponseFragment::FleetAuthority(observed) = observed else {
        return Err(RootInstallStateError::AuthorityMismatch.into());
    };
    let observed = *observed;
    if observed != expected {
        return Err(RootInstallStateError::AuthorityMismatch.into());
    }
    if journal.phase == FleetSubnetRootInstallPhase::RootInstalled {
        require_initial_prepared_runtime(
            icp,
            &root_binding,
            fleet_subnet_root,
            journal,
            "Fleet Subnet Root",
            true,
        )?;
    }
    let direct_store_verification_required = matches!(
        journal.phase,
        FleetSubnetRootInstallPhase::RootInstalled
            | FleetSubnetRootInstallPhase::InfrastructureVerified
    );
    if !direct_store_verification_required {
        return Ok((expected.clone(), expected.wasm_store_authority));
    }
    let wasm_store = journal
        .wasm_store
        .expect("installed infrastructure journal retains its Store");
    require_expected_module_hash(
        icp,
        wasm_store,
        journal.expected_wasm_store_module_hash,
        "Wasm Store",
    )?;
    let expected_store = expected_wasm_store_authority(journal)?;
    require_expected_controllers(
        icp,
        wasm_store,
        &temporary_store_controllers(journal),
        "Wasm Store",
    )?;
    let observed_store = query_with_arg::<_, StoreStatusResponse>(
        icp,
        &store_binding,
        wasm_store,
        protocol::CANIC_STATUS,
        &StoreStatusRequest::Authority,
    )?;
    let StoreStatusResponse::Authority(observed_store) = observed_store else {
        return Err(RootInstallStateError::WasmStoreAuthorityMismatch.into());
    };
    if observed_store != expected_store {
        return Err(RootInstallStateError::WasmStoreAuthorityMismatch.into());
    }
    if journal.phase == FleetSubnetRootInstallPhase::RootInstalled {
        require_initial_prepared_runtime(
            icp,
            &store_binding,
            wasm_store,
            journal,
            "Wasm Store",
            false,
        )?;
    }
    Ok((expected, expected_store))
}

fn resolve_live_protocol_bindings(
    icp_context: &InstallIcpContext,
    journal: &FleetSubnetRootInstallJournal,
) -> Result<(ResolvedProtocolBinding, ResolvedProtocolBinding), Box<dyn std::error::Error>> {
    let root = resolve_infrastructure_protocol_binding(
        icp_context.root(),
        icp_context.environment(),
        &journal.root_artifact,
    )?;
    let store = resolve_infrastructure_protocol_binding(
        icp_context.root(),
        icp_context.environment(),
        &journal.wasm_store_artifact,
    )?;
    Ok((root, store))
}

fn require_initial_prepared_runtime(
    icp: &IcpCli,
    binding: &ResolvedProtocolBinding,
    canister: Principal,
    journal: &FleetSubnetRootInstallJournal,
    subject: &'static str,
    root: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let observed = if root {
        let response = query_with_arg::<_, RootStatusResponseFragment>(
            icp,
            binding,
            canister,
            protocol::CANIC_STATUS,
            &RootStatusRequestFragment::Operation(OperationStatusRequest {
                operation_id: journal.install_operation_id,
            }),
        )?;
        match response {
            RootStatusResponseFragment::Operation(response) => match *response {
                RootOperationStatusResponse::FleetActivation(response) => response,
                _ => {
                    return Err(RootInstallStateError::ActivationStatusMismatch { subject }.into());
                }
            },
            RootStatusResponseFragment::FleetAuthority(_) => {
                return Err(RootInstallStateError::ActivationStatusMismatch { subject }.into());
            }
        }
    } else {
        let response = query_with_arg::<_, StoreStatusResponse>(
            icp,
            binding,
            canister,
            protocol::CANIC_STATUS,
            &StoreStatusRequest::Operation(OperationStatusRequest {
                operation_id: journal.install_operation_id,
            }),
        )?;
        match response {
            StoreStatusResponse::Operation(StoreOperationStatusResponse::FleetActivation(
                response,
            )) => response,
            _ => {
                return Err(RootInstallStateError::ActivationStatusMismatch { subject }.into());
            }
        }
    };
    let expected = FleetActivationStatusResponse {
        phase: FleetActivationPhase::Prepared,
        identity: FleetActivationIdentity {
            fleet: journal.authority.binding.fleet.clone(),
            operation_id: journal.install_operation_id,
            release_build_id: journal.release_build_id,
        },
        cascade: None,
        cascade_manifest: None,
        credential: None,
        credential_manifest: None,
        activated_at_ns: None,
    };
    if observed != expected {
        return Err(RootInstallStateError::ActivationStatusMismatch { subject }.into());
    }
    Ok(())
}

fn root_install_args(
    journal: &FleetSubnetRootInstallJournal,
) -> Result<FleetSubnetRootInitArgs, Box<dyn std::error::Error>> {
    Ok(FleetSubnetRootInitArgs {
        authority: expected_root_authority(journal)?,
        install_id: journal.install_operation_id,
        canister_pool_imports: journal.root_plan.canister_pool_imports.clone(),
    })
}

fn wasm_store_install_args(
    journal: &FleetSubnetRootInstallJournal,
) -> Result<FleetSubnetWasmStoreInitArgs, Box<dyn std::error::Error>> {
    Ok(FleetSubnetWasmStoreInitArgs {
        authority: expected_wasm_store_authority(journal)?,
        install_id: journal.install_operation_id,
    })
}
