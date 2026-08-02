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
        begin_root_install, create_result_path, expected_root_authority,
        plan_fleet_subnet_root_install, record_root_created, record_root_installed,
        record_root_verified, validate_live_root_activation_status,
    },
    operations::{
        CreationEffectRequest, EffectAction, InstallArtifact, InstallEffectRequest,
        execute_or_observe_creation, execute_or_observe_install, module_hash_text,
        observe_module_hash, query_no_arg, resolve_install_artifact,
    },
};
use crate::{
    fleet_install_plan::PersistedFleetInstallPlan,
    icp::LocalReplicaTarget,
    release_set::{
        AppConfigSnapshot, CanicInfrastructureRole,
        load_persisted_canic_infrastructure_artifact_manifest,
    },
};
use candid::Principal;
use canic_core::{
    dto::{
        fleet_activation::FleetActivationStatusResponse,
        fleet_subnet_root::{FleetSubnetRootAuthority, FleetSubnetRootInitArgs},
    },
    protocol,
};
use std::path::{Path, PathBuf};
use thiserror::Error as ThisError;

const MAX_ROOT_TRANSITIONS: usize = 8;
const ROOT_INSTALL_ARGS_FILE: &str = "root-install-args.bin";

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
enum RootInstallStateError {
    #[error("Fleet Subnet Root {fleet_subnet_root} already has unexpected module {observed}")]
    UnexpectedModule {
        fleet_subnet_root: Principal,
        observed: String,
    },

    #[error("Fleet Subnet Root {fleet_subnet_root} has no installed module")]
    MissingModule { fleet_subnet_root: Principal },

    #[error("Fleet Subnet Root activation status differs from exact Prepared install authority")]
    ActivationStatusMismatch,

    #[error("Fleet Subnet Root protected authority query differs from exact planned binding")]
    AuthorityMismatch,

    #[error("Fleet Subnet Root installation exceeded its bounded phase transitions")]
    TransitionBoundExceeded,
}

pub(super) fn install_and_verify_fleet_subnet_roots(
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
    let artifact = resolve_install_artifact(
        icp_root,
        &infrastructure_manifest,
        CanicInfrastructureRole::FleetSubnetRoot,
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
            icp_root,
            environment,
            local_replica,
            &artifact,
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
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    artifact: &InstallArtifact,
    mut current: ResolvedFleetSubnetRootInstall,
) -> Result<FleetSubnetRootAuthority, Box<dyn std::error::Error>> {
    for _ in 0..MAX_ROOT_TRANSITIONS {
        current = match current.journal.phase {
            FleetSubnetRootInstallPhase::Planned => {
                prepare_creation_result(&create_result_path(&current.path), "Fleet Subnet Root")?;
                begin_root_creation(&current)?
            }
            FleetSubnetRootInstallPhase::CreationInFlight => {
                recover_or_create_root(icp_root, environment, local_replica, &current)?
            }
            FleetSubnetRootInstallPhase::Created => begin_root_install(&current)?,
            FleetSubnetRootInstallPhase::InstallInFlight => {
                recover_or_install_root(icp_root, environment, local_replica, artifact, &current)?
            }
            FleetSubnetRootInstallPhase::Installed => {
                verify_and_record_root(icp_root, environment, local_replica, &current)?
            }
            FleetSubnetRootInstallPhase::Verified => {
                let authority = verify_live_root(
                    icp_root,
                    environment,
                    local_replica,
                    &current.path,
                    &current.journal,
                )?;
                return Ok(authority);
            }
            FleetSubnetRootInstallPhase::StoreStaging
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
            | FleetSubnetRootInstallPhase::ComponentRegistryPreparationVerified
            | FleetSubnetRootInstallPhase::RootActivationPreparationInFlight
            | FleetSubnetRootInstallPhase::RootActivationPrepared
            | FleetSubnetRootInstallPhase::RootActivationInFlight
            | FleetSubnetRootInstallPhase::RootActivated
            | FleetSubnetRootInstallPhase::RootActivationVerified => {
                return verify_live_root(
                    icp_root,
                    environment,
                    local_replica,
                    &current.path,
                    &current.journal,
                );
            }
        };
    }
    Err(RootInstallStateError::TransitionBoundExceeded.into())
}

fn recover_or_create_root(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    current: &ResolvedFleetSubnetRootInstall,
) -> Result<ResolvedFleetSubnetRootInstall, Box<dyn std::error::Error>> {
    let result_path = create_result_path(&current.path);
    let evidence = execute_or_observe_creation(CreationEffectRequest {
        icp_root,
        environment,
        local_replica,
        result_path: &result_path,
        subject: "Fleet Subnet Root",
        placement_subnet: current.journal.root_plan.placement_subnet,
        funding: &current.journal.root_plan.creation_funding,
        action: EffectAction::from_advanced(current.advanced),
        expected_module_hash: current.journal.expected_module_hash,
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
            icp_root,
            environment,
            local_replica,
            subject: "Fleet Subnet Root",
            canister: fleet_subnet_root,
            wasm_path: &artifact.wasm_path,
            args_path: &args_path,
            expected_module_hash: current.journal.expected_module_hash,
            action: EffectAction::from_advanced(current.advanced),
        },
        || root_install_args(&current.journal),
    )?;
    record_root_installed(current, module_hash).map_err(Into::into)
}

fn verify_and_record_root(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    current: &ResolvedFleetSubnetRootInstall,
) -> Result<ResolvedFleetSubnetRootInstall, Box<dyn std::error::Error>> {
    let authority = verify_live_root(
        icp_root,
        environment,
        local_replica,
        &current.path,
        &current.journal,
    )?;
    record_root_verified(current, authority).map_err(Into::into)
}

fn verify_live_root(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    journal_path: &Path,
    journal: &FleetSubnetRootInstallJournal,
) -> Result<FleetSubnetRootAuthority, Box<dyn std::error::Error>> {
    let fleet_subnet_root = journal
        .fleet_subnet_root
        .expect("installed root journal retains its principal");
    let icp = super::install_icp(icp_root, environment, local_replica);
    match observe_module_hash(&icp, fleet_subnet_root)? {
        Some(observed) if observed == journal.expected_module_hash => {}
        Some(observed) => {
            return Err(RootInstallStateError::UnexpectedModule {
                fleet_subnet_root,
                observed: module_hash_text(observed),
            }
            .into());
        }
        None => {
            return Err(RootInstallStateError::MissingModule { fleet_subnet_root }.into());
        }
    }

    let expected = expected_root_authority(journal)?;
    let status = query_no_arg::<FleetActivationStatusResponse>(
        &icp,
        fleet_subnet_root,
        protocol::CANIC_FLEET_ACTIVATION_STATUS,
    )?;
    validate_live_root_activation_status(journal_path, journal, &status)
        .map_err(|_| RootInstallStateError::ActivationStatusMismatch)?;
    let observed = query_no_arg::<FleetSubnetRootAuthority>(
        &icp,
        fleet_subnet_root,
        protocol::CANIC_FLEET_SUBNET_ROOT_AUTHORITY,
    )?;
    if observed != expected {
        return Err(RootInstallStateError::AuthorityMismatch.into());
    }
    Ok(expected)
}

fn root_install_args(
    journal: &FleetSubnetRootInstallJournal,
) -> Result<FleetSubnetRootInitArgs, Box<dyn std::error::Error>> {
    Ok(FleetSubnetRootInitArgs {
        authority: expected_root_authority(journal)?,
        install_id: journal.install_operation_id,
    })
}
