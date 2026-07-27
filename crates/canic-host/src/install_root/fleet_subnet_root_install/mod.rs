//! Module: install_root::fleet_subnet_root_install
//!
//! Responsibility: create, install, and independently verify every planned Fleet Subnet Root.
//! Does not own: local Wasm Store bootstrap, Fleet Registry registration, or runtime activation.
//! Boundary: roots are installed serially from canonical plan order; uncertain paid effects remain
//! in explicit durable in-flight phases and are observed rather than blindly replayed.

use super::{
    commands::{
        add_icp_environment_target, icp_canister_command, parse_created_canister_id,
        root_init_args, run_command,
    },
    fleet_subnet_root_install_journal::{
        FleetSubnetRootInstallJournal, FleetSubnetRootInstallPhase,
        PlanFleetSubnetRootInstallRequest, ResolvedFleetSubnetRootInstall, begin_root_creation,
        begin_root_install, create_result_path, expected_root_authority,
        plan_fleet_subnet_root_install, record_root_created, record_root_installed,
        record_root_verified,
    },
    operations::{module_hash_text, parse_module_hash},
};
use crate::{
    durable_io::{
        RegularFileReadError, create_new_bytes_with_parents, read_optional_regular_bytes,
    },
    fleet_install_plan::{PersistedFleetInstallPlan, PlannedCanisterCreationFunding},
    icp::{IcpCli, LocalReplicaTarget, decode_json_result_response, run_output_to_file},
    release_set::{
        AppConfigSnapshot, CanicInfrastructureRole,
        load_persisted_canic_infrastructure_artifact_manifest, resolve_release_artifact_path,
    },
};
use candid::Principal;
use canic_core::{
    dto::{
        fleet_activation::{FleetActivationPhase, FleetActivationStatusResponse},
        fleet_subnet_root::{FleetSubnetRootAuthority, FleetSubnetRootInitArgs},
    },
    protocol,
};
use sha2::{Digest, Sha256};
use std::{
    fs, io,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

const ICP_JSON_OUTPUT: &str = "json";
const MAX_ROOT_TRANSITIONS: usize = 8;

pub(super) struct VerifiedFleetSubnetRoots {
    pub roots: Vec<Principal>,
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
    "Fleet Subnet Root install outcome for {fleet_subnet_root} is unknown; no second install was attempted. Retry only after the original ICP command has settled: {detail}"
)]
struct RootInstallOutcomeUnknownError {
    fleet_subnet_root: Principal,
    detail: String,
}

#[derive(Debug, ThisError)]
enum RootInstallStateError {
    #[error("Fleet Subnet Root artifact {path} is missing")]
    ArtifactMissing { path: PathBuf },

    #[error("Fleet Subnet Root artifact is not a regular no-follow file: {path}")]
    ArtifactUnsafe { path: PathBuf },

    #[error("Fleet Subnet Root artifact {path} has size {actual}, expected {expected}")]
    ArtifactSize {
        path: PathBuf,
        expected: u64,
        actual: usize,
    },

    #[error("Fleet Subnet Root artifact {path} has SHA-256 {actual}, expected {expected}")]
    ArtifactHash {
        path: PathBuf,
        expected: String,
        actual: String,
    },

    #[error("Fleet Subnet Root creation result is not a regular no-follow file: {path}")]
    CreationResultUnsafe { path: PathBuf },

    #[error("Fleet Subnet Root creation result is invalid: {path}")]
    InvalidCreationResult { path: PathBuf },

    #[error("created Fleet Subnet Root {observed} differs from status principal {expected}")]
    StatusPrincipal {
        expected: Principal,
        observed: String,
    },

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

struct RootArtifact {
    wasm_path: PathBuf,
}

pub(super) fn install_and_verify_fleet_subnet_roots(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    config_path: &Path,
    fleet_install_plan: &PersistedFleetInstallPlan,
    coordinator: Principal,
    install_operation_id: [u8; 32],
) -> Result<VerifiedFleetSubnetRoots, Box<dyn std::error::Error>> {
    let config = AppConfigSnapshot::load(config_path)?;
    let component_topology = config.model().compile_component_topology()?;
    let infrastructure_manifest = load_persisted_canic_infrastructure_artifact_manifest(
        icp_root,
        fleet_install_plan.plan.release_build_id,
    )?;
    let artifact = resolve_root_artifact(icp_root, fleet_install_plan, &infrastructure_manifest)?;
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
    component_topology.validate_fleet_root_bindings(&bindings)?;
    Ok(VerifiedFleetSubnetRoots {
        roots: roots
            .into_iter()
            .map(|authority| authority.binding.fleet_subnet_root)
            .collect(),
    })
}

fn drive_root_install(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    artifact: &RootArtifact,
    mut current: ResolvedFleetSubnetRootInstall,
) -> Result<FleetSubnetRootAuthority, Box<dyn std::error::Error>> {
    for _ in 0..MAX_ROOT_TRANSITIONS {
        current = match current.journal.phase {
            FleetSubnetRootInstallPhase::Planned => {
                prepare_creation_result(&create_result_path(&current.path))?;
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
                let authority =
                    verify_live_root(icp_root, environment, local_replica, &current.journal)?;
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
            | FleetSubnetRootInstallPhase::RegistryMirrorActivationVerified => {
                return verify_live_root(icp_root, environment, local_replica, &current.journal);
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
    let mut command_error = None;
    if current.advanced {
        let result = open_creation_result_for_effect(&result_path)?;
        let mut command =
            root_create_command(icp_root, environment, local_replica, &current.journal);
        if let Err(error) = run_output_to_file(&mut command, &result) {
            command_error = Some(error.to_string());
        }
    }
    let Some(fleet_subnet_root) = read_created_root(&result_path)? else {
        return Err(RootCreationOutcomeUnknownError {
            placement_subnet: current.journal.root_plan.placement_subnet,
            result_path,
            detail: command_error.unwrap_or_else(|| {
                "the journal is already creation_in_flight and has no recoverable principal"
                    .to_string()
            }),
        }
        .into());
    };
    observe_created_root(
        icp_root,
        environment,
        local_replica,
        fleet_subnet_root,
        current.journal.expected_module_hash,
    )?;
    record_root_created(current, fleet_subnet_root).map_err(Into::into)
}

fn recover_or_install_root(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    artifact: &RootArtifact,
    current: &ResolvedFleetSubnetRootInstall,
) -> Result<ResolvedFleetSubnetRootInstall, Box<dyn std::error::Error>> {
    let fleet_subnet_root = current
        .journal
        .fleet_subnet_root
        .expect("validated InstallInFlight journal retains its root");
    match observed_module_hash(icp_root, environment, local_replica, fleet_subnet_root)? {
        Some(observed) if observed == current.journal.expected_module_hash => {
            return record_root_installed(current, observed).map_err(Into::into);
        }
        Some(observed) => {
            return Err(RootInstallStateError::UnexpectedModule {
                fleet_subnet_root,
                observed: module_hash_text(observed),
            }
            .into());
        }
        None if !current.advanced => {
            return Err(RootInstallOutcomeUnknownError {
                fleet_subnet_root,
                detail: "the journal is already install_in_flight and the expected module is not observable"
                    .to_string(),
            }
            .into());
        }
        None => {}
    }

    let init_args = root_install_args(&current.journal)?;
    let mut install = root_install_command(
        icp_root,
        environment,
        local_replica,
        fleet_subnet_root,
        &artifact.wasm_path,
        &init_args,
    )?;
    let command_result = run_command(&mut install);
    match observed_module_hash(icp_root, environment, local_replica, fleet_subnet_root) {
        Ok(Some(observed)) if observed == current.journal.expected_module_hash => {
            record_root_installed(current, observed).map_err(Into::into)
        }
        Ok(Some(observed)) => Err(RootInstallStateError::UnexpectedModule {
            fleet_subnet_root,
            observed: module_hash_text(observed),
        }
        .into()),
        Ok(None) => Err(RootInstallOutcomeUnknownError {
            fleet_subnet_root,
            detail: command_result.err().map_or_else(
                || "install command completed but no module is observable".to_string(),
                |error| error.to_string(),
            ),
        }
        .into()),
        Err(observation) => Err(RootInstallOutcomeUnknownError {
            fleet_subnet_root,
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

fn verify_and_record_root(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    current: &ResolvedFleetSubnetRootInstall,
) -> Result<ResolvedFleetSubnetRootInstall, Box<dyn std::error::Error>> {
    let authority = verify_live_root(icp_root, environment, local_replica, &current.journal)?;
    record_root_verified(current, authority).map_err(Into::into)
}

fn verify_live_root(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    journal: &FleetSubnetRootInstallJournal,
) -> Result<FleetSubnetRootAuthority, Box<dyn std::error::Error>> {
    let fleet_subnet_root = journal
        .fleet_subnet_root
        .expect("installed root journal retains its principal");
    match observed_module_hash(icp_root, environment, local_replica, fleet_subnet_root)? {
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
    let icp = root_icp(icp_root, environment, local_replica);
    let status = query_root::<FleetActivationStatusResponse>(
        &icp,
        fleet_subnet_root,
        protocol::CANIC_FLEET_ACTIVATION_STATUS,
    )?;
    if !matches_exact_prepared(journal, &status) {
        return Err(RootInstallStateError::ActivationStatusMismatch.into());
    }
    let observed = query_root::<FleetSubnetRootAuthority>(
        &icp,
        fleet_subnet_root,
        protocol::CANIC_FLEET_SUBNET_ROOT_AUTHORITY,
    )?;
    if observed != expected {
        return Err(RootInstallStateError::AuthorityMismatch.into());
    }
    Ok(expected)
}

fn matches_exact_prepared(
    journal: &FleetSubnetRootInstallJournal,
    status: &FleetActivationStatusResponse,
) -> bool {
    status.phase == FleetActivationPhase::Prepared
        && status.identity.fleet == journal.authority.binding.fleet
        && status.identity.operation_id == journal.install_operation_id
        && status.identity.release_build_id == journal.release_build_id
        && status.cascade.is_none()
        && status.cascade_manifest.is_none()
        && status.credential.is_none()
        && status.credential_manifest.is_none()
        && status.activated_at_ns.is_none()
}

fn query_root<T>(
    icp: &IcpCli,
    fleet_subnet_root: Principal,
    method: &str,
) -> Result<T, Box<dyn std::error::Error>>
where
    T: candid::CandidType + serde::de::DeserializeOwned,
{
    let output = icp.canister_query_output_with_candid(
        &fleet_subnet_root.to_text(),
        method,
        Some(ICP_JSON_OUTPUT),
        None,
    )?;
    decode_json_result_response(&output).map_err(Into::into)
}

fn root_install_args(
    journal: &FleetSubnetRootInstallJournal,
) -> Result<FleetSubnetRootInitArgs, Box<dyn std::error::Error>> {
    Ok(FleetSubnetRootInitArgs {
        authority: expected_root_authority(journal)?,
        install_id: journal.install_operation_id,
    })
}

fn resolve_root_artifact(
    icp_root: &Path,
    fleet_install_plan: &PersistedFleetInstallPlan,
    infrastructure_manifest: &crate::release_set::PersistedCanicInfrastructureArtifactManifest,
) -> Result<RootArtifact, Box<dyn std::error::Error>> {
    let entry = infrastructure_manifest
        .manifest
        .entries
        .iter()
        .find(|entry| entry.role == CanicInfrastructureRole::FleetSubnetRoot)
        .expect("validated infrastructure manifest has one root entry");
    let wasm_path = resolve_release_artifact_path(icp_root, &entry.wasm_relative_path)?;
    let wasm = match read_optional_regular_bytes(&wasm_path) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return Err(RootInstallStateError::ArtifactMissing { path: wasm_path }.into()),
        Err(RegularFileReadError::NotRegular) => {
            return Err(RootInstallStateError::ArtifactUnsafe { path: wasm_path }.into());
        }
        Err(RegularFileReadError::Io(source)) => return Err(source.into()),
        #[cfg(not(unix))]
        Err(RegularFileReadError::UnsupportedPlatform) => {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Fleet Subnet Root artifact reads are unsupported",
            )
            .into());
        }
    };
    if wasm.len() as u64 != entry.wasm_size_bytes {
        return Err(RootInstallStateError::ArtifactSize {
            path: wasm_path,
            expected: entry.wasm_size_bytes,
            actual: wasm.len(),
        }
        .into());
    }
    let actual_hash = hex_digest(Sha256::digest(&wasm).into());
    if actual_hash != entry.wasm_sha256_hex {
        return Err(RootInstallStateError::ArtifactHash {
            path: wasm_path,
            expected: entry.wasm_sha256_hex.clone(),
            actual: actual_hash,
        }
        .into());
    }
    if entry.release_build_id != fleet_install_plan.plan.release_build_id {
        return Err("root artifact release build differs from Fleet install plan".into());
    }
    Ok(RootArtifact { wasm_path })
}

fn root_create_command(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    journal: &FleetSubnetRootInstallJournal,
) -> std::process::Command {
    let mut command = icp_canister_command(icp_root);
    command.args(["create", "--detached", "--json", "--subnet"]);
    command.arg(journal.root_plan.placement_subnet.to_string());
    match journal.root_plan.creation_funding {
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

fn root_install_command(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    fleet_subnet_root: Principal,
    wasm_path: &Path,
    init_args: &FleetSubnetRootInitArgs,
) -> Result<std::process::Command, candid::Error> {
    let mut command = icp_canister_command(icp_root);
    command.args([
        "install",
        &fleet_subnet_root.to_text(),
        "--mode=install",
        "-y",
        "--wasm",
    ]);
    command.arg(wasm_path);
    command.args(["--args", &root_init_args(init_args)?]);
    add_icp_environment_target(&mut command, environment, local_replica);
    Ok(command)
}

fn root_icp(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
) -> IcpCli {
    IcpCli::new("icp", Some(environment.to_string()))
        .with_cwd(icp_root)
        .with_local_replica(local_replica.cloned())
}

fn observed_module_hash(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    fleet_subnet_root: Principal,
) -> Result<Option<[u8; 32]>, Box<dyn std::error::Error>> {
    let report = root_icp(icp_root, environment, local_replica)
        .canister_status_report(&fleet_subnet_root.to_text())?;
    if report.id != fleet_subnet_root.to_text() {
        return Err(RootInstallStateError::StatusPrincipal {
            expected: fleet_subnet_root,
            observed: report.id,
        }
        .into());
    }
    report
        .module_hash
        .as_deref()
        .map(|value| {
            parse_module_hash(value).ok_or_else(|| {
                RootInstallStateError::UnexpectedModule {
                    fleet_subnet_root,
                    observed: value.to_string(),
                }
                .into()
            })
        })
        .transpose()
}

fn observe_created_root(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    fleet_subnet_root: Principal,
    expected_module_hash: [u8; 32],
) -> Result<(), Box<dyn std::error::Error>> {
    match observed_module_hash(icp_root, environment, local_replica, fleet_subnet_root)? {
        None => Ok(()),
        Some(observed) if observed == expected_module_hash => {
            Err("root module exists before journalled install intent".into())
        }
        Some(observed) => Err(RootInstallStateError::UnexpectedModule {
            fleet_subnet_root,
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
                    "Fleet Subnet Root creation result exists before creation intent",
                )),
                Ok(None) => Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "Fleet Subnet Root creation result disappeared",
                )),
                Err(RegularFileReadError::NotRegular) => Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Fleet Subnet Root creation result is not a regular file",
                )),
                Err(RegularFileReadError::Io(source)) => Err(source),
                #[cfg(not(unix))]
                Err(RegularFileReadError::UnsupportedPlatform) => Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "Fleet Subnet Root creation result reads are unsupported",
                )),
            }
        }
        Err(source) => Err(source),
    }
}

fn read_created_root(path: &Path) -> Result<Option<Principal>, Box<dyn std::error::Error>> {
    let bytes = match read_optional_regular_bytes(path) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return Ok(None),
        Err(RegularFileReadError::NotRegular) => {
            return Err(RootInstallStateError::CreationResultUnsafe {
                path: path.to_path_buf(),
            }
            .into());
        }
        Err(RegularFileReadError::Io(source)) => return Err(source.into()),
        #[cfg(not(unix))]
        Err(RegularFileReadError::UnsupportedPlatform) => {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Fleet Subnet Root creation result reads are unsupported",
            )
            .into());
        }
    };
    if bytes.is_empty() {
        return Ok(None);
    }
    let output =
        std::str::from_utf8(&bytes).map_err(|_| RootInstallStateError::InvalidCreationResult {
            path: path.to_path_buf(),
        })?;
    let principal = parse_created_canister_id(output)
        .and_then(|value| Principal::from_text(value).ok())
        .ok_or_else(|| RootInstallStateError::InvalidCreationResult {
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
                "Fleet Subnet Root creation result is missing",
            ));
        }
        Err(RegularFileReadError::NotRegular) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Fleet Subnet Root creation result is not a regular file",
            ));
        }
        Err(RegularFileReadError::Io(source)) => return Err(source),
    };
    if !bytes.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Fleet Subnet Root creation result already contains evidence",
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
            "Fleet Subnet Root creation result is not a regular file",
        ));
    }
    if metadata.st_size != 0 {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Fleet Subnet Root creation result already contains evidence",
        ));
    }
    Ok(fs::File::from(fd))
}

#[cfg(not(unix))]
fn open_creation_result_for_effect(_path: &Path) -> io::Result<fs::File> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Fleet Subnet Root creation result capture is unsupported",
    ))
}

fn icp_e8s_text(e8s: u64) -> String {
    const E8S_PER_ICP: u64 = 100_000_000;
    let whole = e8s / E8S_PER_ICP;
    let remainder = e8s % E8S_PER_ICP;
    if remainder == 0 {
        return whole.to_string();
    }
    let fractional = format!("{remainder:08}");
    format!("{whole}.{}", fractional.trim_end_matches('0'))
}

fn hex_digest(bytes: [u8; 32]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(64), |mut text, byte| {
            use std::fmt::Write as _;
            let _ = write!(text, "{byte:02x}");
            text
        })
}
