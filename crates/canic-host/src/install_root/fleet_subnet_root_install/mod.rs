//! Module: install_root::fleet_subnet_root_install
//!
//! Responsibility: create, install, and independently verify every planned Fleet Subnet Root.
//! Does not own: local Wasm Store bootstrap, Fleet Registry registration, or runtime activation.
//! Boundary: roots are installed serially from canonical plan order; uncertain paid effects remain
//! in explicit durable in-flight phases and are observed rather than blindly replayed.

use super::{
    commands::prepare_creation_result,
    fleet_subnet_root_install_journal::{
        FleetSubnetRootInstallJournal, FleetSubnetRootInstallPhase,
        PlanFleetSubnetRootInstallRequest, ResolvedFleetSubnetRootInstall, begin_root_creation,
        begin_root_install, begin_wasm_store_creation, begin_wasm_store_install,
        create_result_path, expected_root_authority, expected_wasm_store_authority,
        plan_fleet_subnet_root_install, record_infrastructure_verified, record_root_created,
        record_root_installed, record_wasm_store_created, record_wasm_store_installed,
        wasm_store_create_result_path,
    },
    operations::{
        CreationEffectRequest, EffectAction, InstallArtifact, InstallEffectRequest,
        active_installation_controller, execute_or_observe_creation, execute_or_observe_install,
        query_no_arg, require_expected_controllers, require_expected_module_hash,
        resolve_install_artifact,
    },
};
use crate::{
    fleet_install_plan::PersistedFleetInstallPlan,
    icp::{IcpCli, LocalReplicaTarget},
    release_set::{
        AppConfigSnapshot, CanicInfrastructureRole,
        load_persisted_canic_infrastructure_artifact_manifest,
    },
};
use std::path::{Path, PathBuf};

use candid::Principal;
use canic_core::{
    dto::{
        fleet_activation::{
            FleetActivationIdentity, FleetActivationPhase, FleetActivationStatusResponse,
        },
        fleet_subnet_root::{
            FleetSubnetRootAuthority, FleetSubnetRootInitArgs, FleetSubnetWasmStoreInitArgs,
        },
    },
    ids::FleetSubnetWasmStoreAuthority,
    protocol,
};
use thiserror::Error as ThisError;

const MAX_ROOT_TRANSITIONS: usize = 12;
const ROOT_INSTALL_ARGS_FILE: &str = "root-install-args.bin";
const WASM_STORE_INSTALL_ARGS_FILE: &str = "wasm-store-install-args.bin";

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
}

pub(super) fn install_and_verify_fleet_subnet_roots(
    icp_executable: &str,
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    config_path: &Path,
    fleet_install_plan: &PersistedFleetInstallPlan,
    coordinator: Principal,
    install_operation_id: [u8; 32],
) -> Result<(), Box<dyn std::error::Error>> {
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
    let mut roots = Vec::with_capacity(fleet_install_plan.plan.fleet_subnet_roots.len());

    for root_plan in &fleet_install_plan.plan.fleet_subnet_roots {
        let current = plan_fleet_subnet_root_install(PlanFleetSubnetRootInstallRequest {
            fleet_install_plan,
            infrastructure_manifest: &infrastructure_manifest,
            coordinator,
            install_operation_id,
            component_topology: component_topology.clone(),
            root_plan,
        })?;
        roots.push(drive_root_install(
            icp_executable,
            icp_root,
            environment,
            local_replica,
            &root_artifact,
            &wasm_store_artifact,
            current,
        )?);
    }

    let bindings = roots
        .iter()
        .map(|authority| authority.binding.clone())
        .collect::<Vec<_>>();
    component_topology.validate_fleet_subnet_root_bindings(&bindings)?;
    Ok(())
}

fn drive_root_install(
    icp_executable: &str,
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    root_artifact: &InstallArtifact,
    wasm_store_artifact: &InstallArtifact,
    mut current: ResolvedFleetSubnetRootInstall,
) -> Result<FleetSubnetRootAuthority, Box<dyn std::error::Error>> {
    for _ in 0..MAX_ROOT_TRANSITIONS {
        current = match current.journal.phase {
            FleetSubnetRootInstallPhase::Planned => {
                prepare_creation_result(&create_result_path(&current.path), "Fleet Subnet Root")?;
                let installation_controller = active_installation_controller(&super::install_icp(
                    icp_executable,
                    icp_root,
                    environment,
                    local_replica,
                ))?;
                begin_root_creation(&current, installation_controller)?
            }
            FleetSubnetRootInstallPhase::RootCreationInFlight => recover_or_create_root(
                icp_executable,
                icp_root,
                environment,
                local_replica,
                &current,
            )?,
            FleetSubnetRootInstallPhase::RootCreated => {
                prepare_creation_result(
                    &wasm_store_create_result_path(&current.path),
                    "Wasm Store",
                )?;
                begin_wasm_store_creation(&current)?
            }
            FleetSubnetRootInstallPhase::WasmStoreCreationInFlight => recover_or_create_wasm_store(
                icp_executable,
                icp_root,
                environment,
                local_replica,
                &current,
            )?,
            FleetSubnetRootInstallPhase::WasmStoreCreated => begin_wasm_store_install(&current)?,
            FleetSubnetRootInstallPhase::WasmStoreInstallInFlight => recover_or_install_wasm_store(
                icp_executable,
                icp_root,
                environment,
                local_replica,
                wasm_store_artifact,
                &current,
            )?,
            FleetSubnetRootInstallPhase::WasmStoreInstalled => begin_root_install(&current)?,
            FleetSubnetRootInstallPhase::RootInstallInFlight => recover_or_install_root(
                icp_executable,
                icp_root,
                environment,
                local_replica,
                root_artifact,
                &current,
            )?,
            FleetSubnetRootInstallPhase::RootInstalled => verify_and_record_infrastructure(
                icp_executable,
                icp_root,
                environment,
                local_replica,
                &current,
            )?,
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
                let (authority, _) = verify_live_infrastructure(
                    icp_executable,
                    icp_root,
                    environment,
                    local_replica,
                    &current.journal,
                )?;
                return Ok(authority);
            }
        };
    }
    Err(RootInstallStateError::TransitionBoundExceeded.into())
}

fn recover_or_create_root(
    icp_executable: &str,
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    current: &ResolvedFleetSubnetRootInstall,
) -> Result<ResolvedFleetSubnetRootInstall, Box<dyn std::error::Error>> {
    let result_path = create_result_path(&current.path);
    let installation_controller = current
        .journal
        .installation_controller
        .expect("root creation intent retains its installation controller");
    let evidence = execute_or_observe_creation(CreationEffectRequest {
        icp_executable,
        icp_root,
        environment,
        local_replica,
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
    icp_executable: &str,
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    artifact: &InstallArtifact,
    current: &ResolvedFleetSubnetRootInstall,
) -> Result<ResolvedFleetSubnetRootInstall, Box<dyn std::error::Error>> {
    let fleet_subnet_root = current
        .journal
        .fleet_subnet_root
        .expect("validated InstallInFlight journal retains its root");
    let args_path = current.path.with_file_name(ROOT_INSTALL_ARGS_FILE);
    let module_hash = execute_or_observe_install(
        InstallEffectRequest {
            icp_executable,
            icp_root,
            environment,
            local_replica,
            subject: "Fleet Subnet Root",
            canister: fleet_subnet_root,
            wasm_path: &artifact.wasm_path,
            args_path: &args_path,
            expected_module_hash: current.journal.expected_root_module_hash,
            action: EffectAction::from_advanced(current.advanced),
        },
        || root_install_args(&current.journal),
    )?;
    record_root_installed(current, module_hash).map_err(Into::into)
}

fn recover_or_create_wasm_store(
    icp_executable: &str,
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    current: &ResolvedFleetSubnetRootInstall,
) -> Result<ResolvedFleetSubnetRootInstall, Box<dyn std::error::Error>> {
    let result_path = wasm_store_create_result_path(&current.path);
    let controllers = temporary_store_controllers(&current.journal);
    let evidence = execute_or_observe_creation(CreationEffectRequest {
        icp_executable,
        icp_root,
        environment,
        local_replica,
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
    icp_executable: &str,
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    artifact: &InstallArtifact,
    current: &ResolvedFleetSubnetRootInstall,
) -> Result<ResolvedFleetSubnetRootInstall, Box<dyn std::error::Error>> {
    let wasm_store = current
        .journal
        .wasm_store
        .expect("validated WasmStoreInstallInFlight journal retains its Store");
    let args_path = current.path.with_file_name(WASM_STORE_INSTALL_ARGS_FILE);
    let module_hash = execute_or_observe_install(
        InstallEffectRequest {
            icp_executable,
            icp_root,
            environment,
            local_replica,
            subject: "Wasm Store",
            canister: wasm_store,
            wasm_path: &artifact.wasm_path,
            args_path: &args_path,
            expected_module_hash: current.journal.expected_wasm_store_module_hash,
            action: EffectAction::from_advanced(current.advanced),
        },
        || wasm_store_install_args(&current.journal),
    )?;
    record_wasm_store_installed(current, module_hash).map_err(Into::into)
}

fn verify_and_record_infrastructure(
    icp_executable: &str,
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    current: &ResolvedFleetSubnetRootInstall,
) -> Result<ResolvedFleetSubnetRootInstall, Box<dyn std::error::Error>> {
    let (root_authority, wasm_store_authority) = verify_live_infrastructure(
        icp_executable,
        icp_root,
        environment,
        local_replica,
        &current.journal,
    )?;
    record_infrastructure_verified(current, root_authority, wasm_store_authority)
        .map_err(Into::into)
}

fn verify_live_infrastructure(
    icp_executable: &str,
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    journal: &FleetSubnetRootInstallJournal,
) -> Result<(FleetSubnetRootAuthority, FleetSubnetWasmStoreAuthority), Box<dyn std::error::Error>> {
    let fleet_subnet_root = journal
        .fleet_subnet_root
        .expect("installed root journal retains its principal");
    let icp = super::install_icp(icp_executable, icp_root, environment, local_replica);
    require_expected_controllers(
        &icp,
        fleet_subnet_root,
        std::slice::from_ref(
            &journal
                .installation_controller
                .expect("installed root retains its installation controller"),
        ),
        "Fleet Subnet Root",
    )?;
    require_expected_module_hash(
        &icp,
        fleet_subnet_root,
        journal.expected_root_module_hash,
        "Fleet Subnet Root",
    )?;

    let expected = expected_root_authority(journal)?;
    let observed = query_no_arg::<FleetSubnetRootAuthority>(
        &icp,
        fleet_subnet_root,
        protocol::CANIC_FLEET_SUBNET_ROOT_AUTHORITY,
    )?;
    if observed != expected {
        return Err(RootInstallStateError::AuthorityMismatch.into());
    }
    if journal.phase == FleetSubnetRootInstallPhase::RootInstalled {
        require_initial_prepared_runtime(&icp, fleet_subnet_root, journal, "Fleet Subnet Root")?;
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
        &icp,
        wasm_store,
        journal.expected_wasm_store_module_hash,
        "Wasm Store",
    )?;
    let expected_store = expected_wasm_store_authority(journal)?;
    require_expected_controllers(
        &icp,
        wasm_store,
        &temporary_store_controllers(journal),
        "Wasm Store",
    )?;
    let observed_store = query_no_arg::<FleetSubnetWasmStoreAuthority>(
        &icp,
        wasm_store,
        protocol::CANIC_FLEET_SUBNET_WASM_STORE_AUTHORITY,
    )?;
    if observed_store != expected_store {
        return Err(RootInstallStateError::WasmStoreAuthorityMismatch.into());
    }
    if journal.phase == FleetSubnetRootInstallPhase::RootInstalled {
        require_initial_prepared_runtime(&icp, wasm_store, journal, "Wasm Store")?;
    }
    Ok((expected, expected_store))
}

fn require_initial_prepared_runtime(
    icp: &IcpCli,
    canister: Principal,
    journal: &FleetSubnetRootInstallJournal,
    subject: &'static str,
) -> Result<(), Box<dyn std::error::Error>> {
    let observed = query_no_arg::<FleetActivationStatusResponse>(
        icp,
        canister,
        protocol::CANIC_FLEET_ACTIVATION_STATUS,
    )?;
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
