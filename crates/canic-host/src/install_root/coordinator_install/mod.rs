//! Module: install_root::coordinator_install
//!
//! Responsibility: create, install, and independently verify the initial Fleet Coordinator.
//! Does not own: immutable Fleet planning, Registry mutation after genesis, or root effects.
//! Boundary: exact plan/artifact authority drives one journalled effect at a time; an existing
//! in-flight phase is observed but never blindly replayed.

use super::{
    commands::{
        add_icp_environment_target, icp_canister_command, icp_e8s_text, parse_created_canister_id,
        run_command, write_candid_args,
    },
    coordinator_install_journal::{
        FleetCoordinatorInstallJournal, FleetCoordinatorInstallPhase,
        PlanFleetCoordinatorInstallRequest, ResolvedFleetCoordinatorInstall,
        begin_coordinator_creation, begin_coordinator_install, coordinator_create_result_path,
        plan_fleet_coordinator_install, record_coordinator_created, record_coordinator_installed,
        record_coordinator_verified,
    },
    operations::{module_hash_text, parse_module_hash, query_live_registry},
};
use crate::{
    durable_io::{
        RegularFileReadError, create_new_bytes_with_parents, read_optional_regular_bytes,
    },
    fleet_install_plan::{PersistedFleetInstallPlan, PlannedCanisterCreationFunding},
    icp::{LocalReplicaTarget, run_output_to_file},
    release_set::{
        AppConfigSnapshot, CanicInfrastructureRole,
        load_persisted_canic_infrastructure_artifact_manifest, resolve_release_artifact_path,
    },
};
use candid::Principal;
use canic_control_plane::dto::fleet_coordinator::FleetCoordinatorInitArgs;
use canic_core::{
    control_plane_support::ops::fleet_registry::FleetRegistryOps,
    dto::fleet_registry::{FleetRegistry, FleetRegistryManifest, FleetRegistryVersion},
    ids::{FleetCoordinatorBinding, FleetRegistryAuthority},
};
use sha2::{Digest, Sha256};
use std::{
    fs, io,
    path::{Path, PathBuf},
};
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
#[error(
    "Coordinator install outcome for {coordinator} is unknown; no second install was attempted. Retry only after the original ICP command has settled: {detail}"
)]
struct CoordinatorInstallOutcomeUnknownError {
    coordinator: Principal,
    detail: String,
}

#[derive(Debug, ThisError)]
enum CoordinatorInstallStateError {
    #[error("Coordinator artifact {path} is missing")]
    ArtifactMissing { path: PathBuf },

    #[error("Coordinator artifact is not a regular no-follow file: {path}")]
    ArtifactUnsafe { path: PathBuf },

    #[error("Coordinator artifact {path} has size {actual}, expected {expected}")]
    ArtifactSize {
        path: PathBuf,
        expected: u64,
        actual: usize,
    },

    #[error("Coordinator artifact {path} has SHA-256 {actual}, expected {expected}")]
    ArtifactHash {
        path: PathBuf,
        expected: String,
        actual: String,
    },

    #[error("Coordinator creation result is not a regular no-follow file: {path}")]
    CreationResultUnsafe { path: PathBuf },

    #[error("Coordinator creation result is invalid: {path}")]
    InvalidCreationResult { path: PathBuf },

    #[error("created Coordinator {observed} does not match status principal {expected}")]
    StatusPrincipal {
        expected: Principal,
        observed: String,
    },

    #[error("Coordinator {coordinator} already has unexpected module hash {observed}")]
    UnexpectedModule {
        coordinator: Principal,
        observed: String,
    },

    #[error("Coordinator {coordinator} has no installed module")]
    MissingModule { coordinator: Principal },

    #[error("Coordinator Registry query differs from exact genesis authority")]
    RegistryMismatch,

    #[error("Coordinator Registry manifest query differs from exact genesis authority")]
    RegistryManifestMismatch,

    #[error("Coordinator Registry version query differs from exact genesis authority")]
    RegistryVersionMismatch,

    #[error("Coordinator installation exceeded its bounded phase transitions")]
    TransitionBoundExceeded,
}

struct CoordinatorArtifact {
    wasm_path: PathBuf,
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
    let artifact =
        resolve_coordinator_artifact(icp_root, fleet_install_plan, &infrastructure_manifest)?;
    let mut current = plan_fleet_coordinator_install(PlanFleetCoordinatorInstallRequest {
        fleet_install_plan,
        infrastructure_manifest: &infrastructure_manifest,
        component_topology,
    })?;

    for _ in 0..MAX_COORDINATOR_TRANSITIONS {
        current = match current.journal.phase {
            FleetCoordinatorInstallPhase::Planned => {
                prepare_creation_result(&coordinator_create_result_path(&fleet_install_plan.path))?;
                begin_coordinator_creation(&current)?
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
    let mut command_error = None;
    if current.advanced {
        let result = open_creation_result_for_effect(&result_path)?;
        let mut command =
            coordinator_create_command(icp_root, environment, local_replica, &current.journal);
        if let Err(error) = run_output_to_file(&mut command, &result) {
            command_error = Some(error.to_string());
        }
    }

    let Some(coordinator) = read_created_coordinator(&result_path)? else {
        return Err(CoordinatorCreationOutcomeUnknownError {
            result_path,
            detail: command_error.unwrap_or_else(|| {
                "the journal is already creation_in_flight and contains no recoverable principal"
                    .to_string()
            }),
        }
        .into());
    };
    observe_created_canister(
        icp_root,
        environment,
        local_replica,
        coordinator,
        current.journal.expected_module_hash,
    )?;
    record_coordinator_created(current, coordinator).map_err(Into::into)
}

fn recover_or_install_coordinator(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    artifact: &CoordinatorArtifact,
    current: &ResolvedFleetCoordinatorInstall,
) -> Result<ResolvedFleetCoordinatorInstall, Box<dyn std::error::Error>> {
    let coordinator = current
        .journal
        .coordinator
        .expect("validated InstallInFlight journal retains its Coordinator");
    match observed_module_hash(icp_root, environment, local_replica, coordinator)? {
        Some(observed) if observed == current.journal.expected_module_hash => {
            return record_coordinator_installed(current, observed).map_err(Into::into);
        }
        Some(observed) => {
            return Err(CoordinatorInstallStateError::UnexpectedModule {
                coordinator,
                observed: module_hash_text(observed),
            }
            .into());
        }
        None if !current.advanced => {
            return Err(CoordinatorInstallOutcomeUnknownError {
                coordinator,
                detail: "the journal is already install_in_flight and the expected module is not yet observable"
                    .to_string(),
            }
            .into());
        }
        None => {}
    }

    let genesis = expected_genesis(&current.journal)?;
    let args_path = current.path.with_file_name(COORDINATOR_INSTALL_ARGS_FILE);
    write_candid_args(&args_path, &genesis.init_args)?;
    let mut install = coordinator_install_command(
        icp_root,
        environment,
        local_replica,
        coordinator,
        &artifact.wasm_path,
        &args_path,
    );
    let command_result = run_command(&mut install);
    match observed_module_hash(icp_root, environment, local_replica, coordinator) {
        Ok(Some(observed)) if observed == current.journal.expected_module_hash => {
            record_coordinator_installed(current, observed).map_err(Into::into)
        }
        Ok(Some(observed)) => Err(CoordinatorInstallStateError::UnexpectedModule {
            coordinator,
            observed: module_hash_text(observed),
        }
        .into()),
        Ok(None) => Err(CoordinatorInstallOutcomeUnknownError {
            coordinator,
            detail: command_result.err().map_or_else(
                || "install command completed but no module is observable".to_string(),
                |error| error.to_string(),
            ),
        }
        .into()),
        Err(observation) => Err(CoordinatorInstallOutcomeUnknownError {
            coordinator,
            detail: match command_result {
                Ok(()) => format!("post-install observation failed: {observation}"),
                Err(command) => {
                    format!(
                        "install command failed: {command}; reconciliation failed: {observation}"
                    )
                }
            },
        }
        .into()),
    }
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
    match observed_module_hash(icp_root, environment, local_replica, coordinator)? {
        Some(observed) if observed == journal.expected_module_hash => {}
        Some(observed) => {
            return Err(CoordinatorInstallStateError::UnexpectedModule {
                coordinator,
                observed: module_hash_text(observed),
            }
            .into());
        }
        None => return Err(CoordinatorInstallStateError::MissingModule { coordinator }.into()),
    }

    let expected = expected_genesis(journal)?;
    let icp = super::install_icp(icp_root, environment, local_replica);
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
    match observed_module_hash(icp_root, environment, local_replica, coordinator)? {
        Some(observed) if observed == journal.expected_module_hash => {}
        Some(observed) => {
            return Err(CoordinatorInstallStateError::UnexpectedModule {
                coordinator,
                observed: module_hash_text(observed),
            }
            .into());
        }
        None => return Err(CoordinatorInstallStateError::MissingModule { coordinator }.into()),
    }

    let expected = expected_genesis(journal)?;
    let icp = super::install_icp(icp_root, environment, local_replica);
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

fn resolve_coordinator_artifact(
    icp_root: &Path,
    fleet_install_plan: &PersistedFleetInstallPlan,
    infrastructure_manifest: &crate::release_set::PersistedCanicInfrastructureArtifactManifest,
) -> Result<CoordinatorArtifact, Box<dyn std::error::Error>> {
    let entry = infrastructure_manifest
        .manifest
        .entries
        .iter()
        .find(|entry| entry.role == CanicInfrastructureRole::FleetCoordinator)
        .expect("validated infrastructure manifest has one Coordinator entry");
    let wasm_path = resolve_release_artifact_path(icp_root, &entry.wasm_relative_path)?;
    let wasm = match read_optional_regular_bytes(&wasm_path) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => {
            return Err(CoordinatorInstallStateError::ArtifactMissing { path: wasm_path }.into());
        }
        Err(RegularFileReadError::NotRegular) => {
            return Err(CoordinatorInstallStateError::ArtifactUnsafe { path: wasm_path }.into());
        }
        Err(RegularFileReadError::Io(source)) => return Err(source.into()),
        #[cfg(not(unix))]
        Err(RegularFileReadError::UnsupportedPlatform) => {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Coordinator artifact reads are unsupported",
            )
            .into());
        }
    };
    if wasm.len() as u64 != entry.wasm_size_bytes {
        return Err(CoordinatorInstallStateError::ArtifactSize {
            path: wasm_path,
            expected: entry.wasm_size_bytes,
            actual: wasm.len(),
        }
        .into());
    }
    let actual_hash = module_hash_text(Sha256::digest(&wasm).into());
    if actual_hash != entry.wasm_sha256_hex {
        return Err(CoordinatorInstallStateError::ArtifactHash {
            path: wasm_path,
            expected: entry.wasm_sha256_hex.clone(),
            actual: actual_hash,
        }
        .into());
    }
    if entry.release_build_id != fleet_install_plan.plan.release_build_id {
        return Err("Coordinator artifact release build differs from Fleet install plan".into());
    }
    Ok(CoordinatorArtifact { wasm_path })
}

fn coordinator_create_command(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    journal: &FleetCoordinatorInstallJournal,
) -> std::process::Command {
    let mut command = icp_canister_command(icp_root);
    command.args(["create", "--detached", "--json", "--subnet"]);
    command.arg(journal.coordinator_subnet.to_string());
    match journal.creation_funding {
        PlannedCanisterCreationFunding::Cycles { cycles } => {
            command.args(["--cycles", &cycles.to_string()]);
        }
        PlannedCanisterCreationFunding::Icp { e8s } => {
            command.args(["--with-icp", &icp_e8s_text(e8s)]);
        }
    }
    add_icp_environment_target(&mut command, environment, local_replica);
    command
}

fn coordinator_install_command(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    coordinator: Principal,
    wasm_path: &Path,
    args_path: &Path,
) -> std::process::Command {
    let mut command = icp_canister_command(icp_root);
    command.args([
        "install",
        &coordinator.to_text(),
        "--mode=install",
        "-y",
        "--wasm",
    ]);
    command.arg(wasm_path);
    command.arg("--args-file");
    command.arg(args_path);
    command.args(["--args-format", "bin"]);
    add_icp_environment_target(&mut command, environment, local_replica);
    command
}

fn observed_module_hash(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    coordinator: Principal,
) -> Result<Option<[u8; 32]>, Box<dyn std::error::Error>> {
    let report = super::install_icp(icp_root, environment, local_replica)
        .canister_status_report(&coordinator.to_text())?;
    if report.id != coordinator.to_text() {
        return Err(CoordinatorInstallStateError::StatusPrincipal {
            expected: coordinator,
            observed: report.id,
        }
        .into());
    }
    report
        .module_hash
        .as_deref()
        .map(|value| {
            parse_module_hash(value).ok_or_else(|| {
                CoordinatorInstallStateError::UnexpectedModule {
                    coordinator,
                    observed: value.to_string(),
                }
                .into()
            })
        })
        .transpose()
}

fn observe_created_canister(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    coordinator: Principal,
    expected_module_hash: [u8; 32],
) -> Result<(), Box<dyn std::error::Error>> {
    match observed_module_hash(icp_root, environment, local_replica, coordinator)? {
        None => Ok(()),
        Some(observed) if observed == expected_module_hash => {
            Err("Coordinator module exists before its journalled install intent".into())
        }
        Some(observed) => Err(CoordinatorInstallStateError::UnexpectedModule {
            coordinator,
            observed: module_hash_text(observed),
        }
        .into()),
    }
}

fn prepare_creation_result(path: &Path) -> io::Result<()> {
    match create_new_bytes_with_parents(path, &[]) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {
            match read_optional_regular_bytes(path) {
                Ok(Some(bytes)) if bytes.is_empty() => Ok(()),
                Ok(Some(_)) => Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Coordinator creation result exists before creation intent",
                )),
                Ok(None) => Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "Coordinator creation result disappeared",
                )),
                Err(RegularFileReadError::NotRegular) => Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Coordinator creation result is not a regular file",
                )),
                Err(RegularFileReadError::Io(source)) => Err(source),
                #[cfg(not(unix))]
                Err(RegularFileReadError::UnsupportedPlatform) => Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "Coordinator creation result reads are unsupported",
                )),
            }
        }
        Err(source) => Err(source),
    }
}

fn read_created_coordinator(path: &Path) -> Result<Option<Principal>, Box<dyn std::error::Error>> {
    let bytes = match read_optional_regular_bytes(path) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return Ok(None),
        Err(RegularFileReadError::NotRegular) => {
            return Err(CoordinatorInstallStateError::CreationResultUnsafe {
                path: path.to_path_buf(),
            }
            .into());
        }
        Err(RegularFileReadError::Io(source)) => return Err(source.into()),
        #[cfg(not(unix))]
        Err(RegularFileReadError::UnsupportedPlatform) => {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Coordinator creation result reads are unsupported",
            )
            .into());
        }
    };
    if bytes.is_empty() {
        return Ok(None);
    }
    let output = std::str::from_utf8(&bytes).map_err(|_| {
        CoordinatorInstallStateError::InvalidCreationResult {
            path: path.to_path_buf(),
        }
    })?;
    let principal = parse_created_canister_id(output)
        .and_then(|value| Principal::from_text(value).ok())
        .ok_or_else(|| CoordinatorInstallStateError::InvalidCreationResult {
            path: path.to_path_buf(),
        })?;
    Ok(Some(principal))
}

#[cfg(unix)]
fn open_creation_result_for_effect(path: &Path) -> io::Result<fs::File> {
    use rustix::{
        fd::OwnedFd,
        fs::{FileType, Mode, OFlags, fstat, open},
    };

    let bytes = match read_optional_regular_bytes(path) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Coordinator creation result is missing",
            ));
        }
        Err(RegularFileReadError::NotRegular) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Coordinator creation result is not a regular file",
            ));
        }
        Err(RegularFileReadError::Io(source)) => return Err(source),
    };
    if !bytes.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Coordinator creation result already contains evidence",
        ));
    }
    let fd: OwnedFd = open(
        path,
        OFlags::WRONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|source| io::Error::from_raw_os_error(source.raw_os_error()))?;
    let metadata =
        fstat(&fd).map_err(|source| io::Error::from_raw_os_error(source.raw_os_error()))?;
    if FileType::from_raw_mode(metadata.st_mode) != FileType::RegularFile {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Coordinator creation result is not a regular file",
        ));
    }
    if metadata.st_size != 0 {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Coordinator creation result already contains evidence",
        ));
    }
    Ok(fs::File::from(fd))
}

#[cfg(not(unix))]
fn open_creation_result_for_effect(_path: &Path) -> io::Result<fs::File> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Coordinator creation result capture is unsupported",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::ids::{
        AppId, CanonicalNetworkId, FleetBinding, FleetId, FleetKey, ReleaseBuildId,
        ReleaseBuildNonce, SubnetId,
    };

    #[test]
    fn coordinator_creation_command_binds_subnet_and_exact_cycles() {
        let subnet = SubnetId::from_principal(Principal::from_slice(&[41]));
        let journal = command_journal(
            subnet,
            PlannedCanisterCreationFunding::Cycles {
                cycles: 2_000_000_000_000,
            },
        );

        let command =
            coordinator_create_command(Path::new("/workspace"), "staging", None, &journal);

        assert_eq!(
            crate::icp::command_display(&command),
            format!(
                "icp --project-root-override /workspace canister create --detached --json --subnet {subnet} --cycles 2000000000000 -e staging"
            )
        );
    }

    #[test]
    fn coordinator_creation_command_preserves_exact_icp_e8s() {
        let subnet = SubnetId::from_principal(Principal::from_slice(&[42]));
        let journal = command_journal(subnet, PlannedCanisterCreationFunding::Icp { e8s: 1 });

        let command = coordinator_create_command(Path::new("/workspace"), "ic", None, &journal);

        assert_eq!(
            crate::icp::command_display(&command),
            format!(
                "icp --project-root-override /workspace canister create --detached --json --subnet {subnet} --with-icp 0.00000001 -e ic"
            )
        );
    }

    #[test]
    fn coordinator_install_command_uses_binary_candid_file() {
        let coordinator = Principal::from_slice(&[43]);
        let command = coordinator_install_command(
            Path::new("/workspace"),
            "staging",
            None,
            coordinator,
            Path::new("/artifacts/coordinator.wasm"),
            Path::new("/state/coordinator-install-args.bin"),
        );

        assert_eq!(
            crate::icp::command_display(&command),
            format!(
                "icp --project-root-override /workspace canister install {coordinator} --mode=install -y --wasm /artifacts/coordinator.wasm --args-file /state/coordinator-install-args.bin --args-format bin -e staging"
            )
        );
    }

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

    fn command_journal(
        coordinator_subnet: SubnetId,
        creation_funding: PlannedCanisterCreationFunding,
    ) -> FleetCoordinatorInstallJournal {
        FleetCoordinatorInstallJournal {
            schema_version: 1,
            sequence: 0,
            phase: FleetCoordinatorInstallPhase::Planned,
            fleet_install_plan_digest: [1; 32],
            infrastructure_manifest_digest: [2; 32],
            fleet: FleetBinding {
                fleet: FleetKey {
                    canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                    fleet_id: FleetId::from_generated_bytes([4; 32]),
                },
                app: AppId::from("test"),
            },
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [5; 32],
            )),
            coordinator_subnet,
            creation_funding,
            component_topology: canic_core::bootstrap::compiled::ComponentTopology {
                component_specs: Vec::new(),
                provisioning_grants: Vec::new(),
            },
            coordinator_artifact: crate::release_set::CanicInfrastructureArtifactEntry {
                role: CanicInfrastructureRole::FleetCoordinator,
                package: "canic-coordinator".to_string(),
                release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                    [5; 32],
                )),
                wasm_relative_path: "coordinator.wasm".to_string(),
                wasm_size_bytes: 1,
                wasm_sha256_hex: "00".repeat(32),
                wasm_gz_relative_path: "coordinator.wasm.gz".to_string(),
                wasm_gz_size_bytes: 1,
                wasm_gz_sha256_hex: "00".repeat(32),
            },
            expected_module_hash: [0; 32],
            coordinator: None,
            installed_module_hash: None,
            verified_registry_manifest: None,
            verified_registry_version: None,
        }
    }
}
