//! Module: install_root::coordinator_install
//!
//! Responsibility: create, install, and independently verify the initial Fleet Coordinator.
//! Does not own: immutable Fleet planning, Registry mutation after genesis, or root effects.
//! Boundary: exact plan/artifact authority drives one journalled effect at a time; an existing
//! in-flight phase is observed but never blindly replayed.

#[cfg(test)]
use super::commands::write_candid_args;
use super::{
    commands::prepare_creation_result,
    coordinator_install_journal::{
        FleetCoordinatorInstallJournal, FleetCoordinatorInstallPhase,
        PlanFleetCoordinatorInstallRequest, ResolvedFleetCoordinatorInstall,
        begin_coordinator_creation, begin_coordinator_install, coordinator_create_result_path,
        plan_fleet_coordinator_install, record_coordinator_created, record_coordinator_installed,
        record_coordinator_verified,
    },
    operations::{
        CreationEffectRequest, EffectAction, InstallArtifact, InstallEffectRequest,
        active_installation_controller, execute_or_observe_creation, execute_or_observe_install,
        query_live_registry, require_expected_controllers, require_expected_module_hash,
        resolve_install_artifact,
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
use canic_control_plane::dto::fleet_coordinator::FleetCoordinatorInitArgs;
use canic_core::{
    control_plane_support::ops::fleet_registry::FleetRegistryOps,
    dto::fleet_registry::{FleetRegistry, FleetRegistryManifest, FleetRegistryVersion},
    ids::{FleetCoordinatorBinding, FleetRegistryAuthority},
};
#[cfg(test)]
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error as ThisError;

const MAX_COORDINATOR_TRANSITIONS: usize = 8;
const COORDINATOR_INSTALL_ARGS_FILE: &str = "coordinator-install-args.bin";

///
/// VerifiedFleetCoordinator
///

pub(super) struct VerifiedFleetCoordinator {
    pub coordinator: Principal,
}

#[derive(Debug, ThisError)]
#[error(
    "Coordinator creation outcome is unknown; no second paid creation was attempted. Inspect durable result {result_path} and retry after the original ICP command has settled: {detail}"
)]
struct CoordinatorCreationOutcomeUnknownError {
    result_path: PathBuf,
    detail: String,
}

#[derive(Debug, ThisError)]
enum CoordinatorInstallStateError {
    #[error("Coordinator Registry query differs from exact genesis authority")]
    RegistryMismatch,

    #[error("Coordinator Registry manifest query differs from exact genesis authority")]
    RegistryManifestMismatch,

    #[error("Coordinator Registry version query differs from exact genesis authority")]
    RegistryVersionMismatch,

    #[error("Coordinator installation exceeded its bounded phase transitions")]
    TransitionBoundExceeded,
}

struct ExpectedCoordinatorGenesis {
    init_args: FleetCoordinatorInitArgs,
    registry: FleetRegistry,
    manifest: FleetRegistryManifest,
    version: FleetRegistryVersion,
}

/// Drive the exact Coordinator to independently verified Registry genesis.
pub(super) fn install_and_verify_fleet_coordinator(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    config_path: &Path,
    fleet_install_plan: &PersistedFleetInstallPlan,
) -> Result<VerifiedFleetCoordinator, Box<dyn std::error::Error>> {
    let config = AppConfigSnapshot::load(config_path)?;
    let component_topology = config.model().compile_component_topology()?;
    let infrastructure_manifest = load_persisted_canic_infrastructure_artifact_manifest(
        icp_root,
        fleet_install_plan.plan.release_build_id,
    )?;
    let artifact = resolve_install_artifact(
        icp_root,
        &infrastructure_manifest,
        CanicInfrastructureRole::FleetCoordinator,
        fleet_install_plan.plan.release_build_id,
    )?;
    let mut current = plan_fleet_coordinator_install(PlanFleetCoordinatorInstallRequest {
        fleet_install_plan,
        infrastructure_manifest: &infrastructure_manifest,
        component_topology,
    })?;

    for _ in 0..MAX_COORDINATOR_TRANSITIONS {
        current =
            match current.journal.phase {
                FleetCoordinatorInstallPhase::Planned => {
                    prepare_creation_result(
                        &coordinator_create_result_path(&fleet_install_plan.path),
                        "Coordinator",
                    )?;
                    let installation_controller = active_installation_controller(
                        &super::install_icp(icp_root, environment, local_replica),
                    )?;
                    begin_coordinator_creation(&current, installation_controller)?
                }
                FleetCoordinatorInstallPhase::CreationInFlight => recover_or_create_coordinator(
                    icp_root,
                    environment,
                    local_replica,
                    fleet_install_plan,
                    &current,
                )?,
                FleetCoordinatorInstallPhase::Created => begin_coordinator_install(&current)?,
                FleetCoordinatorInstallPhase::InstallInFlight => recover_or_install_coordinator(
                    icp_root,
                    environment,
                    local_replica,
                    &artifact,
                    &current,
                )?,
                FleetCoordinatorInstallPhase::Installed => {
                    verify_and_record_coordinator(icp_root, environment, local_replica, &current)?
                }
                FleetCoordinatorInstallPhase::Verified => {
                    let coordinator = current
                        .journal
                        .coordinator
                        .expect("validated Verified journal retains its Coordinator");
                    verify_live_coordinator_current(
                        icp_root,
                        environment,
                        local_replica,
                        &current.journal,
                    )?;
                    return Ok(VerifiedFleetCoordinator { coordinator });
                }
            };
    }

    Err(CoordinatorInstallStateError::TransitionBoundExceeded.into())
}

fn recover_or_create_coordinator(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    fleet_install_plan: &PersistedFleetInstallPlan,
    current: &ResolvedFleetCoordinatorInstall,
) -> Result<ResolvedFleetCoordinatorInstall, Box<dyn std::error::Error>> {
    let result_path = coordinator_create_result_path(&fleet_install_plan.path);
    let installation_controller = current
        .journal
        .installation_controller
        .expect("Coordinator creation intent retains its installation controller");
    let evidence = execute_or_observe_creation(CreationEffectRequest {
        icp_root,
        environment,
        local_replica,
        result_path: &result_path,
        subject: "Coordinator",
        placement_subnet: current.journal.coordinator_subnet,
        funding: &current.journal.creation_funding,
        controllers: std::slice::from_ref(&installation_controller),
        action: EffectAction::from_advanced(current.advanced),
        expected_module_hash: current.journal.expected_module_hash,
    })?;
    let Some(coordinator) = evidence.canister else {
        return Err(CoordinatorCreationOutcomeUnknownError {
            result_path,
            detail: evidence.command_error.unwrap_or_else(|| {
                "the journal is already creation_in_flight and contains no recoverable principal"
                    .to_string()
            }),
        }
        .into());
    };
    record_coordinator_created(current, coordinator).map_err(Into::into)
}

fn recover_or_install_coordinator(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    artifact: &InstallArtifact,
    current: &ResolvedFleetCoordinatorInstall,
) -> Result<ResolvedFleetCoordinatorInstall, Box<dyn std::error::Error>> {
    let coordinator = current
        .journal
        .coordinator
        .expect("validated InstallInFlight journal retains its Coordinator");
    let args_path = current.path.with_file_name(COORDINATOR_INSTALL_ARGS_FILE);
    let module_hash = execute_or_observe_install(
        InstallEffectRequest {
            icp_root,
            environment,
            local_replica,
            subject: "Coordinator",
            canister: coordinator,
            wasm_path: &artifact.wasm_path,
            args_path: &args_path,
            expected_module_hash: current.journal.expected_module_hash,
            action: EffectAction::from_advanced(current.advanced),
        },
        || Ok(expected_genesis(&current.journal)?.init_args),
    )?;
    record_coordinator_installed(current, module_hash).map_err(Into::into)
}

fn verify_and_record_coordinator(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    current: &ResolvedFleetCoordinatorInstall,
) -> Result<ResolvedFleetCoordinatorInstall, Box<dyn std::error::Error>> {
    let genesis =
        verify_live_coordinator_genesis(icp_root, environment, local_replica, &current.journal)?;
    record_coordinator_verified(current, genesis.manifest, genesis.version).map_err(Into::into)
}

fn verify_live_coordinator_genesis(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    journal: &FleetCoordinatorInstallJournal,
) -> Result<ExpectedCoordinatorGenesis, Box<dyn std::error::Error>> {
    let coordinator = journal
        .coordinator
        .expect("verified Coordinator phases retain a principal");
    let icp = super::install_icp(icp_root, environment, local_replica);
    require_expected_controllers(
        &icp,
        coordinator,
        std::slice::from_ref(
            &journal
                .installation_controller
                .expect("installed Coordinator retains its installation controller"),
        ),
        "Coordinator",
    )?;
    require_expected_module_hash(
        &icp,
        coordinator,
        journal.expected_module_hash,
        "Coordinator",
    )?;

    let expected = expected_genesis(journal)?;
    let live = query_live_registry(&icp, coordinator)?;
    if live.registry != expected.registry {
        return Err(CoordinatorInstallStateError::RegistryMismatch.into());
    }
    if live.manifest != expected.manifest {
        return Err(CoordinatorInstallStateError::RegistryManifestMismatch.into());
    }
    if live.version != expected.version {
        return Err(CoordinatorInstallStateError::RegistryVersionMismatch.into());
    }
    Ok(expected)
}

fn verify_live_coordinator_current(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    journal: &FleetCoordinatorInstallJournal,
) -> Result<(), Box<dyn std::error::Error>> {
    let coordinator = journal
        .coordinator
        .expect("verified Coordinator phases retain a principal");
    let icp = super::install_icp(icp_root, environment, local_replica);
    require_expected_controllers(
        &icp,
        coordinator,
        std::slice::from_ref(
            &journal
                .installation_controller
                .expect("verified Coordinator retains its installation controller"),
        ),
        "Coordinator",
    )?;
    require_expected_module_hash(
        &icp,
        coordinator,
        journal.expected_module_hash,
        "Coordinator",
    )?;

    let expected = expected_genesis(journal)?;
    let live = query_live_registry(&icp, coordinator)?;
    FleetRegistryOps::validate(
        &expected.init_args.authority,
        &journal.component_topology,
        &live.registry,
    )?;
    let expected_manifest = FleetRegistryOps::manifest(
        &expected.init_args.authority,
        &journal.component_topology,
        &live.registry,
    )?;
    if live.manifest != expected_manifest {
        return Err(CoordinatorInstallStateError::RegistryManifestMismatch.into());
    }
    let expected_version = FleetRegistryOps::version(
        &expected.init_args.authority,
        &journal.component_topology,
        &live.registry,
    )?;
    if live.version != expected_version {
        return Err(CoordinatorInstallStateError::RegistryVersionMismatch.into());
    }
    Ok(())
}

fn expected_genesis(
    journal: &FleetCoordinatorInstallJournal,
) -> Result<ExpectedCoordinatorGenesis, Box<dyn std::error::Error>> {
    let coordinator = journal
        .coordinator
        .expect("created Coordinator journal retains its principal");
    let authority = FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: journal.fleet.clone(),
            coordinator_subnet: journal.coordinator_subnet,
            coordinator,
        },
        epoch: 1,
    };
    let registry = FleetRegistryOps::compile_genesis(
        &journal.fleet.app,
        authority.clone(),
        &journal.component_topology,
    )?;
    let manifest = FleetRegistryOps::manifest(&authority, &journal.component_topology, &registry)?;
    let version = FleetRegistryOps::version(&authority, &journal.component_topology, &registry)?;
    Ok(ExpectedCoordinatorGenesis {
        init_args: FleetCoordinatorInitArgs {
            configured_app: journal.fleet.app.clone(),
            authority,
            component_topology: journal.component_topology.clone(),
        },
        registry,
        manifest,
        version,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::ids::{AppId, CanonicalNetworkId, FleetBinding, FleetId, FleetKey, SubnetId};

    #[test]
    fn coordinator_init_args_with_empty_topology_are_binary_candid() {
        let coordinator = Principal::from_slice(&[45]);
        let coordinator_subnet = SubnetId::from_principal(Principal::from_slice(&[46]));
        let configured_app = AppId::from("test");
        let fleet = FleetBinding {
            fleet: FleetKey {
                canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                fleet_id: FleetId::from_generated_bytes([47; 32]),
            },
            app: configured_app.clone(),
        };
        let init_args = FleetCoordinatorInitArgs {
            configured_app,
            authority: FleetRegistryAuthority {
                binding: FleetCoordinatorBinding {
                    fleet,
                    coordinator_subnet,
                    coordinator,
                },
                epoch: 1,
            },
            component_topology: canic_core::bootstrap::compiled::ComponentTopology {
                component_specs: Vec::new(),
                provisioning_grants: Vec::new(),
            },
        };
        let root = crate::test_support::temp_dir("canic-binary-coordinator-install-args");
        let path = root.join(COORDINATOR_INSTALL_ARGS_FILE);

        write_candid_args(&path, &init_args).expect("write Coordinator init args");
        let decoded: FleetCoordinatorInitArgs =
            candid::decode_one(&fs::read(&path).expect("read Coordinator init args"))
                .expect("decode Coordinator init args");

        assert_eq!(decoded, init_args);
        fs::remove_dir_all(root).expect("remove temp root");
    }
}
