//! Module: install_root::fleet_subnet_root_repair
//!
//! Responsibility: own one provisional successor-module authority and one terminal receipt for a
//! state-preserving Root repair while a retained fresh-install session is still incomplete.
//! Does not own: the original install journal, general managed upgrades, or product-version
//! compatibility.
//! Boundary: compatibility is admitted only by bounded typed schemas, exact retained authority,
//! exact predecessor/successor artifact hashes, Candid equality, canonical phase replay, and one
//! terminal repair receipt.

mod procedure;
#[cfg(test)]
mod tests;

pub(super) use procedure::{
    execute_retained_root_repair, reconcile_published_retained_root_repair,
    retained_root_repair_operation_path, validate_recovery_bundle_repair_operation_bytes,
};

use super::{
    fleet_install_session::FleetInstallSession,
    fleet_subnet_root_install_journal::{
        FleetSubnetRootInstallJournal, FleetSubnetRootInstallPhase, ResolvedFleetSubnetRootInstall,
    },
    options::RetainedRootRepairAdoption,
};
use crate::{
    canister_build::extract_candid_bytes,
    durable_io::{
        BoundedRegularFileReadError, CanonicalJsonEncodeError, CanonicalJsonStyle,
        RegularFileLockError, RegularFileReadError, create_new_bytes_with_parents,
        encode_canonical_json, lock_regular_file_with_parents, read_optional_bounded_regular_bytes,
    },
};
use candid::Principal;
use candid_parser::utils::CandidSource;
use canic_core::ids::{FleetBinding, FleetName, FleetRegistryAuthority, ReleaseBuildId, SubnetId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

const REPAIR_AUTHORITY_FILE: &str = "root-repair-authority.json";
const REPAIR_AUTHORITY_LOCK_FILE: &str = "root-repair-authority.lock";
const REPAIR_CANDIDATE_FILE: &str = "root-repair-candidate.json";
const REPAIR_CANDIDATE_LOCK_FILE: &str = "root-repair-candidate.lock";
const REPAIR_AUTHORITY_SCHEMA_VERSION: u32 = 1;
const REPAIR_RECEIPT_FILE: &str = "root-repair-terminal-receipt.json";
const REPAIR_RECEIPT_LOCK_FILE: &str = "root-repair-terminal-receipt.lock";
const REPAIR_RECEIPT_SCHEMA_VERSION: u32 = 1;
const REPAIR_ARTIFACT_DIRECTORY: &str = "root-repair-artifacts";
const SUPPORTED_SESSION_SCHEMA_VERSIONS: &[u32] = &[1];
const SUPPORTED_ROOT_JOURNAL_SCHEMA_VERSIONS: &[u32] = &[1];
const MAX_REPAIR_RECEIPT_BYTES: usize = 16_384;
const MAX_REPAIR_WASM_BYTES: usize = 64 * 1024 * 1024;
const MAX_REPAIR_CANDID_BYTES: usize = 1024 * 1024;
const MAX_CANDID_DIAGNOSTIC_CHARS: usize = 768;
const RETAINED_ROOT_REPAIR_TOP_UP_FEE_CYCLES: u128 = 100_000_000;
const RETAINED_ROOT_REPAIR_TOP_UP_MARGIN_CYCLES: u128 = 100_000_000;

/// The only repair semantics this exceptional fresh-install authority can attest.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum RetainedRootRepairModeV1 {
    StatePreservingUpgrade,
}

/// Immutable provisional evidence authorizing one exact repaired Root module during canonical
/// retained-install replay.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RetainedRootRepairAuthorityV1 {
    pub schema_version: u32,
    pub repair_operation_id: [u8; 32],
    pub repair_mode: RetainedRootRepairModeV1,
    pub session_schema_version: u32,
    pub root_journal_schema_version: u32,
    pub fleet_name: FleetName,
    pub fleet: FleetBinding,
    pub release_build_id: ReleaseBuildId,
    pub fresh_fleet_plan_digest: String,
    pub fleet_install_plan_digest: [u8; 32],
    pub infrastructure_manifest_digest: [u8; 32],
    pub component_topology_sha256: [u8; 32],
    pub root_plan_sha256: [u8; 32],
    pub install_operation_id: [u8; 32],
    pub authority: FleetRegistryAuthority,
    pub placement_subnet: SubnetId,
    pub fleet_subnet_root: Principal,
    pub wasm_store: Principal,
    pub pool_canister: Principal,
    pub installation_controller: Principal,
    pub authority_journal_phase: FleetSubnetRootInstallPhase,
    pub authority_journal_sequence: u64,
    pub retained_journal_module_sha256: [u8; 32],
    pub retained_journal_wasm_size_bytes: u64,
    pub upgrade_predecessor_module_sha256: [u8; 32],
    pub upgrade_predecessor_wasm_size_bytes: u64,
    pub successor_module_sha256: [u8; 32],
    pub successor_wasm_size_bytes: u64,
    pub retained_journal_candid_sha256: [u8; 32],
    pub upgrade_predecessor_candid_sha256: [u8; 32],
    pub successor_candid_sha256: [u8; 32],
    pub required_pool_cycles: u128,
    pub pool_policy_sha256: [u8; 32],
    pub top_up_fee_cycles: u128,
    pub top_up_margin_cycles: u128,
}

/// Terminal evidence that canonical replay reached and verified Component Registry preparation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RetainedRootRepairTerminalReceiptV1 {
    pub schema_version: u32,
    pub repair_operation_id: [u8; 32],
    pub authority_sha256: [u8; 32],
    pub terminal_journal_phase: FleetSubnetRootInstallPhase,
    pub terminal_journal_sequence: u64,
    pub terminal_journal_sha256: [u8; 32],
    pub component_registry_request_sha256: [u8; 32],
    pub component_registry_response_sha256: [u8; 32],
}

#[cfg(test)]
type RetainedRootRepairReceiptV1 = RetainedRootRepairAuthorityV1;

#[derive(Serialize)]
struct RetainedRootRepairOperationAuthority<'a> {
    session_schema_version: u32,
    root_journal_schema_version: u32,
    fleet_name: &'a FleetName,
    fleet: &'a FleetBinding,
    release_build_id: ReleaseBuildId,
    fresh_fleet_plan_digest: &'a str,
    fleet_install_plan_digest: [u8; 32],
    infrastructure_manifest_digest: [u8; 32],
    component_topology_sha256: [u8; 32],
    root_plan_sha256: [u8; 32],
    install_operation_id: [u8; 32],
    authority: &'a FleetRegistryAuthority,
    placement_subnet: SubnetId,
    fleet_subnet_root: Principal,
    wasm_store: Principal,
    pool_canister: Principal,
    installation_controller: Principal,
    authority_journal_phase: FleetSubnetRootInstallPhase,
    authority_journal_sequence: u64,
    retained_journal_module_sha256: [u8; 32],
    retained_journal_wasm_size_bytes: u64,
    upgrade_predecessor_module_sha256: [u8; 32],
    upgrade_predecessor_wasm_size_bytes: u64,
    successor_module_sha256: [u8; 32],
    successor_wasm_size_bytes: u64,
    retained_journal_candid_sha256: [u8; 32],
    upgrade_predecessor_candid_sha256: [u8; 32],
    successor_candid_sha256: [u8; 32],
    required_pool_cycles: u128,
    pool_policy_sha256: [u8; 32],
    top_up_fee_cycles: u128,
    top_up_margin_cycles: u128,
}

struct InspectedRepairWasm {
    bytes: Vec<u8>,
    candid: Vec<u8>,
    candid_sha256: [u8; 32],
}

#[derive(Clone, Copy)]
enum RepairCandidResolution {
    FinalizedSidecar,
    SuccessorSidecarOrBuildExport,
}

#[derive(Clone, Copy)]
struct RetainedRootRepairTransition {
    pool_canister: Principal,
    upgrade_predecessor_module_sha256: [u8; 32],
    upgrade_predecessor_wasm_size_bytes: u64,
    upgrade_predecessor_candid_sha256: [u8; 32],
    successor_module_sha256: [u8; 32],
    successor_wasm_size_bytes: u64,
    successor_candid_sha256: [u8; 32],
    required_pool_cycles: u128,
}

impl RetainedRootRepairTransition {
    const fn from_authority(authority: &RetainedRootRepairAuthorityV1) -> Self {
        Self {
            pool_canister: authority.pool_canister,
            upgrade_predecessor_module_sha256: authority.upgrade_predecessor_module_sha256,
            upgrade_predecessor_wasm_size_bytes: authority.upgrade_predecessor_wasm_size_bytes,
            upgrade_predecessor_candid_sha256: authority.upgrade_predecessor_candid_sha256,
            successor_module_sha256: authority.successor_module_sha256,
            successor_wasm_size_bytes: authority.successor_wasm_size_bytes,
            successor_candid_sha256: authority.successor_candid_sha256,
            required_pool_cycles: authority.required_pool_cycles,
        }
    }
}

impl RetainedRootRepairAuthorityV1 {
    #[must_use]
    pub(super) const fn successor_module_hash(&self) -> [u8; 32] {
        self.successor_module_sha256
    }
}

/// Optional repair evidence resolved for one exact retained Root journal.
pub(super) struct ResolvedRetainedRootRepair {
    pub authority: RetainedRootRepairAuthorityV1,
    pub terminal_receipt: Option<RetainedRootRepairTerminalReceiptV1>,
    pub needs_authority_publication: bool,
    path: PathBuf,
    pub successor_wasm_path: PathBuf,
}

#[derive(Debug, ThisError)]
pub(super) enum RetainedRootRepairError {
    #[error("retained Root repair artifact is missing: {path}")]
    ArtifactMissing { path: PathBuf },

    #[error("retained Root repair artifact is not a regular no-follow file: {path}")]
    ArtifactUnsafe { path: PathBuf },

    #[error("retained Root repair artifact exceeds the 64 MiB recovery bound: {path}")]
    ArtifactTooLarge { path: PathBuf },

    #[error("failed to read retained Root repair artifact {path}: {source}")]
    ArtifactIo {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("retained Root repair artifact changed while its Candid was inspected: {path}")]
    ArtifactChanged { path: PathBuf },

    #[error(
        "retained Root repair requires the exact finalized Candid sidecar at {path}; historical Wasm extraction is not a recovery compatibility path"
    )]
    FinalizedCandidSidecarMissing { path: PathBuf },

    #[error(
        "retained Root repair successor Candid sidecar is missing at {path}, and the Wasm has no get_candid_pointer build export"
    )]
    SuccessorCandidUnavailable { path: PathBuf },

    #[error("retained Root repair Candid sidecar is not a regular no-follow file: {path}")]
    CandidSidecarUnsafe { path: PathBuf },

    #[error("retained Root repair Candid sidecar exceeds the 1 MiB bound: {path}")]
    CandidSidecarTooLarge { path: PathBuf },

    #[error("failed to read retained Root repair Candid sidecar {path}: {source}")]
    CandidSidecarIo {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("retained Root repair Candid sidecar is invalid at {path}: {reason}")]
    InvalidCandidSidecar { path: PathBuf, reason: String },

    #[error("retained Root repair has incompatible Candid: {0}")]
    CandidMismatch(String),

    #[error("retained Root repair successor artifact does not advance the exact live Root")]
    NotARepair,

    #[error(
        "retained Root repair authority requires a post-infrastructure phase from store_bootstrapped through component_registry_preparation_verified"
    )]
    InvalidPhase,

    #[error(
        "retained Root repair terminal receipt requires component_registry_preparation_verified"
    )]
    PrematureTerminalReceipt,

    #[error("retained Root repair request names a Root outside the retained install plan")]
    RootNotFound,

    #[error("retained Root repair requires one nonzero initial Component pool-cycle target")]
    MissingPoolRequirement,

    #[error("retained Root repair receipt already has different immutable authority: {path}")]
    ConflictingAuthority { path: PathBuf },

    #[error("invalid retained Root repair receipt {path}: {reason}")]
    InvalidDocument { path: PathBuf, reason: String },

    #[error("retained Root repair receipt is not a regular no-follow file: {path}")]
    UnsafeFile { path: PathBuf },

    #[error("retained Root repair receipt lock is not a regular no-follow file: {path}")]
    UnsafeLock { path: PathBuf },

    #[error("failed to access retained Root repair receipt {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to inspect retained Root repair Candid: {0}")]
    CandidInspection(String),
}

/// Resolve an existing provisional authority or compile one candidate from explicit artifact
/// evidence. A terminal receipt is loaded independently and never substitutes for provisional
/// authority while canonical phase replay is incomplete.
pub(super) fn resolve_retained_root_repair(
    current: &ResolvedFleetSubnetRootInstall,
    session: &FleetInstallSession,
    adoption: Option<&RetainedRootRepairAdoption>,
    required_pool_cycles: Option<u128>,
) -> Result<Option<ResolvedRetainedRootRepair>, RetainedRootRepairError> {
    let path = repair_authority_path(&current.path);
    let candidate_path = repair_candidate_path(&current.path);
    let receipt_path = repair_receipt_path(&current.path);
    let retained = load_optional_authority(&path)?;
    let candidate = load_optional_authority(&candidate_path)?;
    if let Some(authority) = retained {
        if candidate
            .as_ref()
            .is_some_and(|candidate| candidate != &authority)
        {
            return Err(RetainedRootRepairError::ConflictingAuthority {
                path: candidate_path,
            });
        }
        validate_requested_authority(
            &path,
            &authority,
            session,
            &current.journal,
            adoption,
            required_pool_cycles,
        )?;
        let terminal_receipt = load_optional_receipt(&receipt_path)?;
        if let Some(receipt) = terminal_receipt.as_ref() {
            validate_terminal_receipt(
                &receipt_path,
                receipt,
                &authority,
                session,
                &current.journal,
            )?;
        } else {
            require_retained_repair_artifacts(&current.path, &authority)?;
        }
        return Ok(Some(resolved_repair(
            current,
            path,
            authority,
            terminal_receipt,
            false,
        )));
    }
    if let Some(authority) = candidate {
        validate_requested_authority(
            &candidate_path,
            &authority,
            session,
            &current.journal,
            adoption,
            required_pool_cycles,
        )?;
        require_retained_repair_artifacts(&current.path, &authority)?;
        if load_optional_receipt(&receipt_path)?.is_some() {
            return Err(RetainedRootRepairError::ConflictingAuthority { path: receipt_path });
        }
        return Ok(Some(resolved_repair(current, path, authority, None, true)));
    }
    let Some(adoption) = adoption else {
        return Ok(None);
    };
    let authority = compile_authority(
        session,
        &current.journal,
        adoption,
        required_pool_cycles.ok_or(RetainedRootRepairError::MissingPoolRequirement)?,
    )?;
    retain_repair_artifacts(&current.path, adoption, &authority)?;
    publish_repair_candidate(&current.path, &authority)?;
    Ok(Some(resolved_repair(current, path, authority, None, true)))
}

fn validate_requested_authority(
    path: &Path,
    authority: &RetainedRootRepairAuthorityV1,
    session: &FleetInstallSession,
    journal: &FleetSubnetRootInstallJournal,
    adoption: Option<&RetainedRootRepairAdoption>,
    required_pool_cycles: Option<u128>,
) -> Result<(), RetainedRootRepairError> {
    validate_authority(path, authority, session, journal, required_pool_cycles)?;
    let Some(adoption) = adoption else {
        return Ok(());
    };
    let mut authority_journal = journal.clone();
    authority_journal.phase = authority.authority_journal_phase;
    authority_journal.sequence = authority.authority_journal_sequence;
    let requested = compile_authority(
        session,
        &authority_journal,
        adoption,
        required_pool_cycles.ok_or(RetainedRootRepairError::MissingPoolRequirement)?,
    )?;
    if requested != *authority {
        return Err(RetainedRootRepairError::ConflictingAuthority {
            path: path.to_path_buf(),
        });
    }
    Ok(())
}

fn resolved_repair(
    current: &ResolvedFleetSubnetRootInstall,
    path: PathBuf,
    authority: RetainedRootRepairAuthorityV1,
    terminal_receipt: Option<RetainedRootRepairTerminalReceiptV1>,
    needs_authority_publication: bool,
) -> ResolvedRetainedRootRepair {
    let successor_wasm_path =
        retained_artifact_path(&current.path, authority.successor_module_sha256, "wasm");
    ResolvedRetainedRootRepair {
        authority,
        terminal_receipt,
        needs_authority_publication,
        path,
        successor_wasm_path,
    }
}

/// Publish provisional authority before any repair effect, reconciling a create-new race exactly.
pub(super) fn publish_retained_root_repair_authority(
    resolved: &ResolvedRetainedRootRepair,
    session: &FleetInstallSession,
    journal: &FleetSubnetRootInstallJournal,
) -> Result<(), RetainedRootRepairError> {
    if !resolved.needs_authority_publication {
        return Ok(());
    }
    let candidate_path = repair_candidate_path(&resolved.path);
    let candidate = load_optional_authority(&candidate_path)?.ok_or_else(|| {
        RetainedRootRepairError::ConflictingAuthority {
            path: candidate_path.clone(),
        }
    })?;
    if candidate != resolved.authority {
        return Err(RetainedRootRepairError::ConflictingAuthority {
            path: candidate_path,
        });
    }
    validate_authority(&resolved.path, &resolved.authority, session, journal, None)?;
    let bytes = encode_authority(&resolved.path, &resolved.authority)?;
    let _lock = lock_document(&resolved.path, REPAIR_AUTHORITY_LOCK_FILE)?;
    match create_new_bytes_with_parents(&resolved.path, &bytes) {
        Ok(()) => {}
        Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {
            let observed = load_optional_authority(&resolved.path)?.ok_or_else(|| {
                RetainedRootRepairError::ConflictingAuthority {
                    path: resolved.path.clone(),
                }
            })?;
            if observed != resolved.authority {
                return Err(RetainedRootRepairError::ConflictingAuthority {
                    path: resolved.path.clone(),
                });
            }
        }
        Err(source) => {
            return Err(RetainedRootRepairError::Io {
                path: resolved.path.clone(),
                source,
            });
        }
    }
    let durable = load_optional_authority(&resolved.path)?.ok_or_else(|| {
        RetainedRootRepairError::ConflictingAuthority {
            path: resolved.path.clone(),
        }
    })?;
    if durable != resolved.authority {
        return Err(RetainedRootRepairError::ConflictingAuthority {
            path: resolved.path.clone(),
        });
    }
    Ok(())
}

fn publish_repair_candidate(
    journal_path: &Path,
    authority: &RetainedRootRepairAuthorityV1,
) -> Result<(), RetainedRootRepairError> {
    let path = repair_candidate_path(journal_path);
    let bytes = encode_authority(&path, authority)?;
    let _lock = lock_document(&path, REPAIR_CANDIDATE_LOCK_FILE)?;
    match create_new_bytes_with_parents(&path, &bytes) {
        Ok(()) => {}
        Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {}
        Err(source) => {
            return Err(RetainedRootRepairError::Io { path, source });
        }
    }
    let retained = load_optional_authority(&path)?
        .ok_or_else(|| RetainedRootRepairError::ConflictingAuthority { path: path.clone() })?;
    if retained != *authority {
        return Err(RetainedRootRepairError::ConflictingAuthority { path });
    }
    Ok(())
}

pub(super) fn validate_recovery_bundle_repair_authority_bytes(
    path: &Path,
    bytes: &[u8],
    session: &FleetInstallSession,
    journal: &FleetSubnetRootInstallJournal,
) -> Result<RetainedRootRepairAuthorityV1, RetainedRootRepairError> {
    let authority = serde_json::from_slice::<RetainedRootRepairAuthorityV1>(bytes)
        .map_err(|error| invalid(path, error.to_string()))?;
    validate_authority(path, &authority, session, journal, None)?;
    if encode_authority(path, &authority)? != bytes {
        return Err(invalid(path, "repair authority bytes are not canonical"));
    }
    Ok(authority)
}

pub(super) fn validate_recovery_bundle_repair_receipt_bytes(
    path: &Path,
    bytes: &[u8],
    authority: &RetainedRootRepairAuthorityV1,
    session: &FleetInstallSession,
    journal: &FleetSubnetRootInstallJournal,
) -> Result<(), RetainedRootRepairError> {
    let receipt = serde_json::from_slice::<RetainedRootRepairTerminalReceiptV1>(bytes)
        .map_err(|error| invalid(path, error.to_string()))?;
    validate_terminal_receipt(path, &receipt, authority, session, journal)?;
    if encode_receipt(path, &receipt)? != bytes {
        return Err(invalid(path, "repair receipt bytes are not canonical"));
    }
    Ok(())
}

/// Compile and publish the terminal receipt only after normal replay has reached the protected
/// Component Registry proof boundary.
pub(super) fn publish_retained_root_repair_receipt(
    resolved: &ResolvedRetainedRootRepair,
    session: &FleetInstallSession,
    journal: &FleetSubnetRootInstallJournal,
) -> Result<RetainedRootRepairTerminalReceiptV1, RetainedRootRepairError> {
    let receipt_path = repair_receipt_path(&resolved.path);
    let expected = compile_terminal_receipt(&resolved.authority, session, journal)?;
    if let Some(observed) = load_optional_receipt(&receipt_path)? {
        if observed != expected {
            return Err(RetainedRootRepairError::ConflictingAuthority { path: receipt_path });
        }
        validate_terminal_receipt(
            &receipt_path,
            &observed,
            &resolved.authority,
            session,
            journal,
        )?;
        return Ok(observed);
    }
    let bytes = encode_receipt(&receipt_path, &expected)?;
    let _lock = lock_document(&receipt_path, REPAIR_RECEIPT_LOCK_FILE)?;
    match create_new_bytes_with_parents(&receipt_path, &bytes) {
        Ok(()) => {}
        Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {}
        Err(source) => {
            return Err(RetainedRootRepairError::Io {
                path: receipt_path,
                source,
            });
        }
    }
    let durable = load_optional_receipt(&receipt_path)?.ok_or_else(|| {
        RetainedRootRepairError::ConflictingAuthority {
            path: receipt_path.clone(),
        }
    })?;
    if durable != expected {
        return Err(RetainedRootRepairError::ConflictingAuthority { path: receipt_path });
    }
    Ok(durable)
}

fn require_durable_terminal_receipt(
    resolved: &ResolvedRetainedRootRepair,
) -> Result<(), RetainedRootRepairError> {
    let path = repair_receipt_path(&resolved.path);
    let receipt =
        load_optional_receipt(&path)?.ok_or(RetainedRootRepairError::PrematureTerminalReceipt)?;
    if receipt.repair_operation_id != resolved.authority.repair_operation_id
        || receipt.authority_sha256 != authority_sha256(&resolved.authority)?
        || receipt.terminal_journal_phase
            != FleetSubnetRootInstallPhase::ComponentRegistryPreparationVerified
    {
        return Err(invalid(
            &path,
            "terminal repair receipt does not bind the exact provisional authority",
        ));
    }
    Ok(())
}

fn compile_authority(
    session: &FleetInstallSession,
    journal: &FleetSubnetRootInstallJournal,
    adoption: &RetainedRootRepairAdoption,
    required_pool_cycles: u128,
) -> Result<RetainedRootRepairAuthorityV1, RetainedRootRepairError> {
    if !journal.phase.admits_retained_root_repair() {
        return Err(RetainedRootRepairError::InvalidPhase);
    }
    let fleet_subnet_root = journal
        .fleet_subnet_root
        .ok_or(RetainedRootRepairError::RootNotFound)?;
    if fleet_subnet_root != adoption.fleet_subnet_root {
        return Err(RetainedRootRepairError::RootNotFound);
    }
    if required_pool_cycles == 0 {
        return Err(RetainedRootRepairError::MissingPoolRequirement);
    }
    let installation_controller = journal
        .installation_controller
        .ok_or(RetainedRootRepairError::RootNotFound)?;
    let wasm_store = journal
        .wasm_store
        .ok_or(RetainedRootRepairError::RootNotFound)?;
    let transition = compile_repair_transition(journal, adoption, required_pool_cycles)?;
    let repair_operation_id = repair_operation_id(
        session,
        journal,
        &transition,
        journal.phase,
        journal.sequence,
    )?;
    let authority = RetainedRootRepairAuthorityV1 {
        schema_version: REPAIR_AUTHORITY_SCHEMA_VERSION,
        repair_operation_id,
        repair_mode: RetainedRootRepairModeV1::StatePreservingUpgrade,
        session_schema_version: session.schema_version,
        root_journal_schema_version: journal.schema_version,
        fleet_name: session.fleet_name.clone(),
        fleet: session.fleet.clone(),
        release_build_id: session.release_build_id,
        fresh_fleet_plan_digest: session.fresh_fleet_plan_digest.clone(),
        fleet_install_plan_digest: journal.fleet_install_plan_digest,
        infrastructure_manifest_digest: journal.infrastructure_manifest_digest,
        component_topology_sha256: domain_digest(
            b"canic.root-repair.component-topology.v1\0",
            &journal.component_topology,
        )?,
        root_plan_sha256: domain_digest(b"canic.root-repair.root-plan.v1\0", &journal.root_plan)?,
        install_operation_id: session.operation_id,
        authority: journal.authority.clone(),
        placement_subnet: journal.root_plan.placement_subnet,
        fleet_subnet_root,
        wasm_store,
        pool_canister: transition.pool_canister,
        installation_controller,
        authority_journal_phase: journal.phase,
        authority_journal_sequence: journal.sequence,
        retained_journal_module_sha256: journal.expected_root_module_hash,
        retained_journal_wasm_size_bytes: journal.root_artifact.wasm_size_bytes,
        upgrade_predecessor_module_sha256: transition.upgrade_predecessor_module_sha256,
        upgrade_predecessor_wasm_size_bytes: transition.upgrade_predecessor_wasm_size_bytes,
        successor_module_sha256: transition.successor_module_sha256,
        successor_wasm_size_bytes: transition.successor_wasm_size_bytes,
        retained_journal_candid_sha256: journal.root_artifact.candid_sha256,
        upgrade_predecessor_candid_sha256: transition.upgrade_predecessor_candid_sha256,
        successor_candid_sha256: transition.successor_candid_sha256,
        required_pool_cycles,
        pool_policy_sha256: repair_pool_policy_sha256(journal)?,
        top_up_fee_cycles: RETAINED_ROOT_REPAIR_TOP_UP_FEE_CYCLES,
        top_up_margin_cycles: RETAINED_ROOT_REPAIR_TOP_UP_MARGIN_CYCLES,
    };
    validate_authority(
        Path::new(REPAIR_AUTHORITY_FILE),
        &authority,
        session,
        journal,
        Some(required_pool_cycles),
    )?;
    Ok(authority)
}

fn compile_repair_transition(
    journal: &FleetSubnetRootInstallJournal,
    adoption: &RetainedRootRepairAdoption,
    required_pool_cycles: u128,
) -> Result<RetainedRootRepairTransition, RetainedRootRepairError> {
    let upgrade_predecessor = inspect_wasm(
        &adoption.live_predecessor_wasm,
        RepairCandidResolution::FinalizedSidecar,
    )?;
    let upgrade_predecessor_module_sha256: [u8; 32] =
        Sha256::digest(&upgrade_predecessor.bytes).into();
    let upgrade_predecessor_wasm_size_bytes = u64::try_from(upgrade_predecessor.bytes.len())
        .map_err(|_| RetainedRootRepairError::ArtifactTooLarge {
            path: adoption.live_predecessor_wasm.clone(),
        })?;
    if upgrade_predecessor.candid_sha256 != journal.root_artifact.candid_sha256 {
        return Err(RetainedRootRepairError::CandidMismatch(
            "the finalized predecessor sidecar does not match the infrastructure manifest and retained journal Candid"
                .to_string(),
        ));
    }
    let successor = inspect_wasm(
        &adoption.successor_wasm,
        RepairCandidResolution::SuccessorSidecarOrBuildExport,
    )?;
    if successor.candid_sha256 != journal.root_artifact.candid_sha256 {
        return Err(RetainedRootRepairError::CandidMismatch(
            "the successor does not preserve the retained Root Candid exactly".to_string(),
        ));
    }
    let successor_module_sha256: [u8; 32] = Sha256::digest(&successor.bytes).into();
    if successor_module_sha256 == upgrade_predecessor_module_sha256
        || successor_module_sha256 == journal.expected_root_module_hash
    {
        return Err(RetainedRootRepairError::NotARepair);
    }
    let successor_wasm_size_bytes = u64::try_from(successor.bytes.len()).map_err(|_| {
        RetainedRootRepairError::ArtifactTooLarge {
            path: adoption.successor_wasm.clone(),
        }
    })?;
    Ok(RetainedRootRepairTransition {
        pool_canister: adoption.pool_canister,
        upgrade_predecessor_module_sha256,
        upgrade_predecessor_wasm_size_bytes,
        upgrade_predecessor_candid_sha256: upgrade_predecessor.candid_sha256,
        successor_module_sha256,
        successor_wasm_size_bytes,
        successor_candid_sha256: successor.candid_sha256,
        required_pool_cycles,
    })
}

#[cfg(test)]
fn compile_adoption(
    session: &FleetInstallSession,
    journal: &FleetSubnetRootInstallJournal,
    adoption: &RetainedRootRepairAdoption,
    required_pool_cycles: u128,
) -> Result<RetainedRootRepairAuthorityV1, RetainedRootRepairError> {
    compile_authority(session, journal, adoption, required_pool_cycles)
}

#[cfg(test)]
fn validate_receipt(
    path: &Path,
    authority: &RetainedRootRepairAuthorityV1,
    session: &FleetInstallSession,
    journal: &FleetSubnetRootInstallJournal,
    expected_required_pool_cycles: Option<u128>,
) -> Result<(), RetainedRootRepairError> {
    validate_authority(
        path,
        authority,
        session,
        journal,
        expected_required_pool_cycles,
    )
}

#[cfg(test)]
fn publish_retained_root_repair(
    resolved: &ResolvedRetainedRootRepair,
    session: &FleetInstallSession,
    journal: &FleetSubnetRootInstallJournal,
) -> Result<(), RetainedRootRepairError> {
    publish_retained_root_repair_authority(resolved, session, journal)?;
    let _ = publish_retained_root_repair_receipt(resolved, session, journal)?;
    Ok(())
}

fn inspect_wasm(
    path: &Path,
    candid_resolution: RepairCandidResolution,
) -> Result<InspectedRepairWasm, RetainedRootRepairError> {
    let before = read_bounded_artifact(path)?;
    let candid = resolve_repair_candid(path, &before, candid_resolution)?;
    let after = read_bounded_artifact(path)?;
    if before != after {
        return Err(RetainedRootRepairError::ArtifactChanged {
            path: path.to_path_buf(),
        });
    }
    let candid_sha256: [u8; 32] = Sha256::digest(&candid).into();
    Ok(InspectedRepairWasm {
        bytes: after,
        candid,
        candid_sha256,
    })
}

fn resolve_repair_candid(
    wasm_path: &Path,
    wasm: &[u8],
    resolution: RepairCandidResolution,
) -> Result<Vec<u8>, RetainedRootRepairError> {
    let sidecar_path = repair_candid_sidecar_path(wasm_path)?;
    match read_optional_bounded_regular_bytes(&sidecar_path, MAX_REPAIR_CANDID_BYTES) {
        Ok(Some(candid)) => validate_repair_candid(&sidecar_path, candid),
        Ok(None) => match resolution {
            RepairCandidResolution::FinalizedSidecar => {
                Err(RetainedRootRepairError::FinalizedCandidSidecarMissing { path: sidecar_path })
            }
            RepairCandidResolution::SuccessorSidecarOrBuildExport => {
                if !wasm_exports_candid_pointer(wasm) {
                    return Err(RetainedRootRepairError::SuccessorCandidUnavailable {
                        path: sidecar_path,
                    });
                }
                let candid = extract_candid_bytes(wasm_path).map_err(|error| {
                    RetainedRootRepairError::CandidInspection(bounded_diagnostic(error))
                })?;
                if candid.len() > MAX_REPAIR_CANDID_BYTES {
                    return Err(RetainedRootRepairError::CandidSidecarTooLarge {
                        path: sidecar_path,
                    });
                }
                validate_repair_candid(&sidecar_path, candid)
            }
        },
        Err(BoundedRegularFileReadError::TooLarge) => {
            Err(RetainedRootRepairError::CandidSidecarTooLarge { path: sidecar_path })
        }
        Err(BoundedRegularFileReadError::Read(RegularFileReadError::NotRegular)) => {
            Err(RetainedRootRepairError::CandidSidecarUnsafe { path: sidecar_path })
        }
        Err(BoundedRegularFileReadError::Read(RegularFileReadError::Io(source))) => {
            Err(RetainedRootRepairError::CandidSidecarIo {
                path: sidecar_path,
                source,
            })
        }
        #[cfg(not(unix))]
        Err(BoundedRegularFileReadError::Read(RegularFileReadError::UnsupportedPlatform)) => {
            Err(RetainedRootRepairError::CandidSidecarIo {
                path: sidecar_path,
                source: io::Error::new(
                    io::ErrorKind::Unsupported,
                    "retained Root repair Candid sidecar reads are unsupported",
                ),
            })
        }
    }
}

fn repair_candid_sidecar_path(wasm_path: &Path) -> Result<PathBuf, RetainedRootRepairError> {
    if wasm_path
        .extension()
        .and_then(|extension| extension.to_str())
        != Some("wasm")
    {
        return Err(RetainedRootRepairError::InvalidCandidSidecar {
            path: wasm_path.to_path_buf(),
            reason: "raw Root artifact path must end in .wasm".to_string(),
        });
    }
    Ok(wasm_path.with_extension("did"))
}

fn validate_repair_candid(
    path: &Path,
    candid: Vec<u8>,
) -> Result<Vec<u8>, RetainedRootRepairError> {
    let text = std::str::from_utf8(&candid).map_err(|error| {
        RetainedRootRepairError::InvalidCandidSidecar {
            path: path.to_path_buf(),
            reason: error.to_string(),
        }
    })?;
    let (_, actor) = CandidSource::Text(text).load().map_err(|error| {
        RetainedRootRepairError::InvalidCandidSidecar {
            path: path.to_path_buf(),
            reason: bounded_diagnostic(error),
        }
    })?;
    if actor.is_none() {
        return Err(RetainedRootRepairError::InvalidCandidSidecar {
            path: path.to_path_buf(),
            reason: "Candid sidecar has no service interface".to_string(),
        });
    }
    Ok(candid)
}

fn wasm_exports_candid_pointer(wasm: &[u8]) -> bool {
    const WASM_HEADER_BYTES: usize = 8;
    if wasm.get(..WASM_HEADER_BYTES) != Some(b"\0asm\x01\0\0\0") {
        return false;
    }
    let mut offset = WASM_HEADER_BYTES;
    while offset < wasm.len() {
        let Some(section_id) = wasm.get(offset).copied() else {
            return false;
        };
        offset += 1;
        let Some(section_size) = read_wasm_u32(wasm, &mut offset) else {
            return false;
        };
        let Ok(section_size) = usize::try_from(section_size) else {
            return false;
        };
        let Some(section_end) = offset.checked_add(section_size) else {
            return false;
        };
        let Some(section) = wasm.get(offset..section_end) else {
            return false;
        };
        if section_id == 7 {
            return wasm_export_section_has_candid_pointer(section);
        }
        offset = section_end;
    }
    false
}

fn wasm_export_section_has_candid_pointer(section: &[u8]) -> bool {
    let mut offset = 0;
    let Some(count) = read_wasm_u32(section, &mut offset) else {
        return false;
    };
    for _ in 0..count {
        let Some(name_size) = read_wasm_u32(section, &mut offset) else {
            return false;
        };
        let Ok(name_size) = usize::try_from(name_size) else {
            return false;
        };
        let Some(name_end) = offset.checked_add(name_size) else {
            return false;
        };
        let Some(name) = section.get(offset..name_end) else {
            return false;
        };
        offset = name_end;
        if section.get(offset).is_none() {
            return false;
        }
        offset += 1;
        if read_wasm_u32(section, &mut offset).is_none() {
            return false;
        }
        if name == b"get_candid_pointer" {
            return true;
        }
    }
    false
}

fn read_wasm_u32(bytes: &[u8], offset: &mut usize) -> Option<u32> {
    let mut value = 0_u32;
    for shift in (0..35).step_by(7) {
        let byte = *bytes.get(*offset)?;
        *offset += 1;
        let lane = u32::from(byte & 0x7f);
        if shift == 28 && lane > 0x0f {
            return None;
        }
        value |= lane << shift;
        if byte & 0x80 == 0 {
            return Some(value);
        }
    }
    None
}

fn retain_repair_artifacts(
    journal_path: &Path,
    adoption: &RetainedRootRepairAdoption,
    authority: &RetainedRootRepairAuthorityV1,
) -> Result<(), RetainedRootRepairError> {
    let predecessor = inspect_wasm(
        &adoption.live_predecessor_wasm,
        RepairCandidResolution::FinalizedSidecar,
    )?;
    let successor = inspect_wasm(
        &adoption.successor_wasm,
        RepairCandidResolution::SuccessorSidecarOrBuildExport,
    )?;
    let artifacts = [
        (
            authority.upgrade_predecessor_module_sha256,
            authority.upgrade_predecessor_wasm_size_bytes,
            predecessor.bytes.as_slice(),
            "wasm",
        ),
        (
            authority.upgrade_predecessor_candid_sha256,
            u64::try_from(predecessor.candid.len()).map_err(|_| {
                RetainedRootRepairError::ArtifactTooLarge {
                    path: adoption.live_predecessor_wasm.clone(),
                }
            })?,
            predecessor.candid.as_slice(),
            "did",
        ),
        (
            authority.successor_module_sha256,
            authority.successor_wasm_size_bytes,
            successor.bytes.as_slice(),
            "wasm",
        ),
        (
            authority.successor_candid_sha256,
            u64::try_from(successor.candid.len()).map_err(|_| {
                RetainedRootRepairError::ArtifactTooLarge {
                    path: adoption.successor_wasm.clone(),
                }
            })?,
            successor.candid.as_slice(),
            "did",
        ),
    ];
    for (digest, size, bytes, extension) in artifacts {
        let path = retained_artifact_path(journal_path, digest, extension);
        retain_exact_artifact(&path, digest, size, bytes)?;
    }
    Ok(())
}

fn require_retained_repair_artifacts(
    journal_path: &Path,
    authority: &RetainedRootRepairAuthorityV1,
) -> Result<(), RetainedRootRepairError> {
    require_retained_artifact(
        &retained_artifact_path(
            journal_path,
            authority.upgrade_predecessor_module_sha256,
            "wasm",
        ),
        authority.upgrade_predecessor_module_sha256,
        authority.upgrade_predecessor_wasm_size_bytes,
    )?;
    require_retained_artifact(
        &retained_artifact_path(journal_path, authority.successor_module_sha256, "wasm"),
        authority.successor_module_sha256,
        authority.successor_wasm_size_bytes,
    )?;
    require_retained_artifact_digest(
        &retained_artifact_path(
            journal_path,
            authority.upgrade_predecessor_candid_sha256,
            "did",
        ),
        authority.upgrade_predecessor_candid_sha256,
    )?;
    require_retained_artifact_digest(
        &retained_artifact_path(journal_path, authority.successor_candid_sha256, "did"),
        authority.successor_candid_sha256,
    )
}

fn retain_exact_artifact(
    path: &Path,
    expected_sha256: [u8; 32],
    expected_size: u64,
    bytes: &[u8],
) -> Result<(), RetainedRootRepairError> {
    let observed_sha256: [u8; 32] = Sha256::digest(bytes).into();
    if observed_sha256 != expected_sha256 || u64::try_from(bytes.len()).ok() != Some(expected_size)
    {
        return Err(RetainedRootRepairError::ArtifactChanged {
            path: path.to_path_buf(),
        });
    }
    match create_new_bytes_with_parents(path, bytes) {
        Ok(()) => {}
        Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {}
        Err(source) => {
            return Err(RetainedRootRepairError::ArtifactIo {
                path: path.to_path_buf(),
                source,
            });
        }
    }
    require_retained_artifact(path, expected_sha256, expected_size)
}

fn require_retained_artifact(
    path: &Path,
    expected_sha256: [u8; 32],
    expected_size: u64,
) -> Result<(), RetainedRootRepairError> {
    let bytes = read_bounded_artifact(path)?;
    let observed_sha256: [u8; 32] = Sha256::digest(&bytes).into();
    if u64::try_from(bytes.len()).ok() != Some(expected_size) || observed_sha256 != expected_sha256
    {
        return Err(RetainedRootRepairError::ArtifactChanged {
            path: path.to_path_buf(),
        });
    }
    Ok(())
}

fn require_retained_artifact_digest(
    path: &Path,
    expected_sha256: [u8; 32],
) -> Result<(), RetainedRootRepairError> {
    let bytes = read_bounded_artifact(path)?;
    let observed_sha256: [u8; 32] = Sha256::digest(&bytes).into();
    if observed_sha256 != expected_sha256 {
        return Err(RetainedRootRepairError::ArtifactChanged {
            path: path.to_path_buf(),
        });
    }
    Ok(())
}

pub(super) fn retained_artifact_path(
    journal_path: &Path,
    digest: [u8; 32],
    extension: &str,
) -> PathBuf {
    journal_path
        .parent()
        .expect("Root journal has one parent")
        .join(REPAIR_ARTIFACT_DIRECTORY)
        .join(format!("{}.{}", encode_hex(&digest), extension))
}

fn encode_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

fn read_bounded_artifact(path: &Path) -> Result<Vec<u8>, RetainedRootRepairError> {
    match read_optional_bounded_regular_bytes(path, MAX_REPAIR_WASM_BYTES) {
        Ok(Some(bytes)) => Ok(bytes),
        Ok(None) => Err(RetainedRootRepairError::ArtifactMissing {
            path: path.to_path_buf(),
        }),
        Err(BoundedRegularFileReadError::TooLarge) => {
            Err(RetainedRootRepairError::ArtifactTooLarge {
                path: path.to_path_buf(),
            })
        }
        Err(BoundedRegularFileReadError::Read(RegularFileReadError::NotRegular)) => {
            Err(RetainedRootRepairError::ArtifactUnsafe {
                path: path.to_path_buf(),
            })
        }
        Err(BoundedRegularFileReadError::Read(RegularFileReadError::Io(source))) => {
            Err(RetainedRootRepairError::ArtifactIo {
                path: path.to_path_buf(),
                source,
            })
        }
        #[cfg(not(unix))]
        Err(BoundedRegularFileReadError::Read(RegularFileReadError::UnsupportedPlatform)) => {
            Err(RetainedRootRepairError::ArtifactIo {
                path: path.to_path_buf(),
                source: io::Error::new(
                    io::ErrorKind::Unsupported,
                    "retained Root repair artifact reads are unsupported",
                ),
            })
        }
    }
}

fn validate_authority(
    path: &Path,
    authority: &RetainedRootRepairAuthorityV1,
    session: &FleetInstallSession,
    journal: &FleetSubnetRootInstallJournal,
    expected_required_pool_cycles: Option<u128>,
) -> Result<(), RetainedRootRepairError> {
    if authority.schema_version != REPAIR_AUTHORITY_SCHEMA_VERSION {
        return Err(invalid(
            path,
            "unsupported provisional repair-authority schema; export with the matching Canic release before retrying",
        ));
    }
    if !SUPPORTED_SESSION_SCHEMA_VERSIONS.contains(&session.schema_version)
        || !SUPPORTED_ROOT_JOURNAL_SCHEMA_VERSIONS.contains(&journal.schema_version)
    {
        return Err(invalid(
            path,
            "unsupported retained-install schema; export with the matching Canic release before retrying",
        ));
    }
    if !journal
        .phase
        .is_at_or_after_repair_phase(authority.authority_journal_phase)
    {
        return Err(RetainedRootRepairError::InvalidPhase);
    }
    let Some(fleet_subnet_root) = journal.fleet_subnet_root else {
        return Err(RetainedRootRepairError::RootNotFound);
    };
    let Some(installation_controller) = journal.installation_controller else {
        return Err(RetainedRootRepairError::RootNotFound);
    };
    let Some(wasm_store) = journal.wasm_store else {
        return Err(RetainedRootRepairError::RootNotFound);
    };
    let expected_operation_id = repair_operation_id(
        session,
        journal,
        &RetainedRootRepairTransition::from_authority(authority),
        authority.authority_journal_phase,
        authority.authority_journal_sequence,
    )?;
    let exact_authority = [
        authority_matches_session(authority, session),
        authority_matches_root_journal(
            authority,
            session,
            journal,
            fleet_subnet_root,
            wasm_store,
            installation_controller,
        ),
        authority_has_exact_artifact_transition(authority, journal, expected_required_pool_cycles),
        authority.repair_operation_id == expected_operation_id,
    ]
    .into_iter()
    .all(std::convert::identity);
    if !exact_authority {
        return Err(invalid(
            path,
            "repair receipt differs from the exact retained session, journal, artifact, or upgrade authority",
        ));
    }
    Ok(())
}

fn authority_matches_session(
    authority: &RetainedRootRepairAuthorityV1,
    session: &FleetInstallSession,
) -> bool {
    [
        authority.repair_mode == RetainedRootRepairModeV1::StatePreservingUpgrade,
        authority.session_schema_version == session.schema_version,
        authority.fleet_name == session.fleet_name,
        authority.fleet == session.fleet,
        authority.release_build_id == session.release_build_id,
        authority.fresh_fleet_plan_digest == session.fresh_fleet_plan_digest,
        authority.install_operation_id == session.operation_id,
    ]
    .into_iter()
    .all(std::convert::identity)
}

fn authority_matches_root_journal(
    authority: &RetainedRootRepairAuthorityV1,
    session: &FleetInstallSession,
    journal: &FleetSubnetRootInstallJournal,
    fleet_subnet_root: Principal,
    wasm_store: Principal,
    installation_controller: Principal,
) -> bool {
    let Ok(component_topology_sha256) = domain_digest(
        b"canic.root-repair.component-topology.v1\0",
        &journal.component_topology,
    ) else {
        return false;
    };
    let Ok(root_plan_sha256) =
        domain_digest(b"canic.root-repair.root-plan.v1\0", &journal.root_plan)
    else {
        return false;
    };
    [
        authority.root_journal_schema_version == journal.schema_version,
        authority.fleet_install_plan_digest == journal.fleet_install_plan_digest,
        authority.infrastructure_manifest_digest == journal.infrastructure_manifest_digest,
        authority.component_topology_sha256 == component_topology_sha256,
        authority.root_plan_sha256 == root_plan_sha256,
        authority.install_operation_id == journal.install_operation_id,
        authority.authority == journal.authority,
        authority.authority.binding.fleet == session.fleet,
        authority.placement_subnet == journal.root_plan.placement_subnet,
        authority.fleet_subnet_root == fleet_subnet_root,
        authority.wasm_store == wasm_store,
        authority.installation_controller == installation_controller,
        authority.pool_canister != Principal::anonymous(),
        authority.pool_canister != fleet_subnet_root,
        authority
            .authority_journal_phase
            .admits_retained_root_repair(),
        journal.sequence >= authority.authority_journal_sequence,
    ]
    .into_iter()
    .all(std::convert::identity)
}

fn authority_has_exact_artifact_transition(
    authority: &RetainedRootRepairAuthorityV1,
    journal: &FleetSubnetRootInstallJournal,
    expected_required_pool_cycles: Option<u128>,
) -> bool {
    let Ok(expected_pool_policy_sha256) = repair_pool_policy_sha256(journal) else {
        return false;
    };
    [
        authority.retained_journal_module_sha256 == journal.expected_root_module_hash,
        authority.retained_journal_wasm_size_bytes == journal.root_artifact.wasm_size_bytes,
        authority.retained_journal_candid_sha256 == journal.root_artifact.candid_sha256,
        authority.upgrade_predecessor_candid_sha256 == journal.root_artifact.candid_sha256,
        authority.successor_candid_sha256 == journal.root_artifact.candid_sha256,
        authority.upgrade_predecessor_wasm_size_bytes > 0,
        authority.upgrade_predecessor_wasm_size_bytes <= MAX_REPAIR_WASM_BYTES as u64,
        authority.successor_module_sha256 != authority.upgrade_predecessor_module_sha256,
        authority.successor_module_sha256 != journal.expected_root_module_hash,
        authority.successor_wasm_size_bytes > 0,
        authority.successor_wasm_size_bytes <= MAX_REPAIR_WASM_BYTES as u64,
        authority.required_pool_cycles > 0,
        authority.pool_policy_sha256 == expected_pool_policy_sha256,
        expected_required_pool_cycles
            .is_none_or(|expected| expected == authority.required_pool_cycles),
        authority.top_up_fee_cycles == RETAINED_ROOT_REPAIR_TOP_UP_FEE_CYCLES,
        authority.top_up_margin_cycles == RETAINED_ROOT_REPAIR_TOP_UP_MARGIN_CYCLES,
    ]
    .into_iter()
    .all(std::convert::identity)
}

fn repair_pool_policy_sha256(
    journal: &FleetSubnetRootInstallJournal,
) -> Result<[u8; 32], RetainedRootRepairError> {
    domain_digest(
        b"canic.root-repair.pool-policy.v1\0",
        &(
            &journal.root_plan.limits.canister_pool,
            &journal.root_plan.funding,
            &journal.root_plan.canister_pool_imports,
        ),
    )
}

fn compile_terminal_receipt(
    authority: &RetainedRootRepairAuthorityV1,
    session: &FleetInstallSession,
    journal: &FleetSubnetRootInstallJournal,
) -> Result<RetainedRootRepairTerminalReceiptV1, RetainedRootRepairError> {
    validate_authority(
        Path::new(REPAIR_AUTHORITY_FILE),
        authority,
        session,
        journal,
        None,
    )?;
    if journal.phase != FleetSubnetRootInstallPhase::ComponentRegistryPreparationVerified {
        return Err(RetainedRootRepairError::PrematureTerminalReceipt);
    }
    let request = journal
        .component_registry_preparation_request
        .as_ref()
        .ok_or(RetainedRootRepairError::PrematureTerminalReceipt)?;
    let response = journal
        .component_registry_preparation_response
        .as_ref()
        .ok_or(RetainedRootRepairError::PrematureTerminalReceipt)?;
    Ok(RetainedRootRepairTerminalReceiptV1 {
        schema_version: REPAIR_RECEIPT_SCHEMA_VERSION,
        repair_operation_id: authority.repair_operation_id,
        authority_sha256: authority_sha256(authority)?,
        terminal_journal_phase: journal.phase,
        terminal_journal_sequence: journal.sequence,
        terminal_journal_sha256: domain_digest(
            b"canic.root-repair.terminal-journal.v1\0",
            journal,
        )?,
        component_registry_request_sha256: domain_digest(
            b"canic.root-repair.component-registry-request.v1\0",
            request,
        )?,
        component_registry_response_sha256: domain_digest(
            b"canic.root-repair.component-registry-response.v1\0",
            response,
        )?,
    })
}

fn validate_terminal_receipt(
    path: &Path,
    receipt: &RetainedRootRepairTerminalReceiptV1,
    authority: &RetainedRootRepairAuthorityV1,
    session: &FleetInstallSession,
    journal: &FleetSubnetRootInstallJournal,
) -> Result<(), RetainedRootRepairError> {
    if receipt.schema_version != REPAIR_RECEIPT_SCHEMA_VERSION {
        return Err(invalid(
            path,
            "unsupported terminal repair-receipt schema; export with the matching Canic release before retrying",
        ));
    }
    let expected = compile_terminal_receipt(authority, session, journal)?;
    if receipt != &expected {
        return Err(invalid(
            path,
            "terminal repair receipt differs from the exact provisional authority or durable terminal journal",
        ));
    }
    Ok(())
}

fn authority_sha256(
    authority: &RetainedRootRepairAuthorityV1,
) -> Result<[u8; 32], RetainedRootRepairError> {
    domain_digest(b"canic.root-repair.authority.v1\0", authority)
}

fn domain_digest(
    domain: &[u8],
    value: &impl Serialize,
) -> Result<[u8; 32], RetainedRootRepairError> {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(
        serde_json::to_vec(value)
            .map_err(|error| invalid(Path::new(REPAIR_RECEIPT_FILE), error.to_string()))?,
    );
    let mut digest: [u8; 32] = hasher.finalize().into();
    if digest == [0; 32] {
        digest[31] = 1;
    }
    Ok(digest)
}

fn repair_operation_id(
    session: &FleetInstallSession,
    journal: &FleetSubnetRootInstallJournal,
    transition: &RetainedRootRepairTransition,
    authority_journal_phase: FleetSubnetRootInstallPhase,
    authority_journal_sequence: u64,
) -> Result<[u8; 32], RetainedRootRepairError> {
    let fleet_subnet_root = journal
        .fleet_subnet_root
        .ok_or(RetainedRootRepairError::RootNotFound)?;
    let installation_controller = journal
        .installation_controller
        .ok_or(RetainedRootRepairError::RootNotFound)?;
    let wasm_store = journal
        .wasm_store
        .ok_or(RetainedRootRepairError::RootNotFound)?;
    let authority = RetainedRootRepairOperationAuthority {
        session_schema_version: session.schema_version,
        root_journal_schema_version: journal.schema_version,
        fleet_name: &session.fleet_name,
        fleet: &session.fleet,
        release_build_id: session.release_build_id,
        fresh_fleet_plan_digest: &session.fresh_fleet_plan_digest,
        fleet_install_plan_digest: journal.fleet_install_plan_digest,
        infrastructure_manifest_digest: journal.infrastructure_manifest_digest,
        component_topology_sha256: domain_digest(
            b"canic.root-repair.component-topology.v1\0",
            &journal.component_topology,
        )?,
        root_plan_sha256: domain_digest(b"canic.root-repair.root-plan.v1\0", &journal.root_plan)?,
        install_operation_id: session.operation_id,
        authority: &journal.authority,
        placement_subnet: journal.root_plan.placement_subnet,
        fleet_subnet_root,
        wasm_store,
        pool_canister: transition.pool_canister,
        installation_controller,
        authority_journal_phase,
        authority_journal_sequence,
        retained_journal_module_sha256: journal.expected_root_module_hash,
        retained_journal_wasm_size_bytes: journal.root_artifact.wasm_size_bytes,
        upgrade_predecessor_module_sha256: transition.upgrade_predecessor_module_sha256,
        upgrade_predecessor_wasm_size_bytes: transition.upgrade_predecessor_wasm_size_bytes,
        successor_module_sha256: transition.successor_module_sha256,
        successor_wasm_size_bytes: transition.successor_wasm_size_bytes,
        retained_journal_candid_sha256: journal.root_artifact.candid_sha256,
        upgrade_predecessor_candid_sha256: transition.upgrade_predecessor_candid_sha256,
        successor_candid_sha256: transition.successor_candid_sha256,
        required_pool_cycles: transition.required_pool_cycles,
        pool_policy_sha256: repair_pool_policy_sha256(journal)?,
        top_up_fee_cycles: RETAINED_ROOT_REPAIR_TOP_UP_FEE_CYCLES,
        top_up_margin_cycles: RETAINED_ROOT_REPAIR_TOP_UP_MARGIN_CYCLES,
    };
    let mut hasher = Sha256::new();
    hasher.update(b"canic.retained-root-repair.v1\0");
    hasher.update(
        serde_json::to_vec(&authority)
            .map_err(|error| invalid(Path::new(REPAIR_AUTHORITY_FILE), error.to_string()))?,
    );
    let mut digest: [u8; 32] = hasher.finalize().into();
    if digest == [0; 32] {
        digest[31] = 1;
    }
    Ok(digest)
}

fn load_optional_receipt(
    path: &Path,
) -> Result<Option<RetainedRootRepairTerminalReceiptV1>, RetainedRootRepairError> {
    let bytes = match read_optional_bounded_regular_bytes(path, MAX_REPAIR_RECEIPT_BYTES) {
        Ok(bytes) => bytes,
        Err(BoundedRegularFileReadError::TooLarge) => {
            return Err(invalid(path, "repair receipt exceeds its byte bound"));
        }
        Err(BoundedRegularFileReadError::Read(RegularFileReadError::NotRegular)) => {
            return Err(RetainedRootRepairError::UnsafeFile {
                path: path.to_path_buf(),
            });
        }
        Err(BoundedRegularFileReadError::Read(RegularFileReadError::Io(source))) => {
            return Err(RetainedRootRepairError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
        #[cfg(not(unix))]
        Err(BoundedRegularFileReadError::Read(RegularFileReadError::UnsupportedPlatform)) => {
            return Err(RetainedRootRepairError::Io {
                path: path.to_path_buf(),
                source: io::Error::new(
                    io::ErrorKind::Unsupported,
                    "retained Root repair receipt reads are unsupported",
                ),
            });
        }
    };
    let Some(bytes) = bytes else {
        return Ok(None);
    };
    let receipt = serde_json::from_slice::<RetainedRootRepairTerminalReceiptV1>(&bytes)
        .map_err(|error| invalid(path, error.to_string()))?;
    if encode_receipt(path, &receipt)? != bytes {
        return Err(invalid(path, "repair receipt bytes are not canonical"));
    }
    Ok(Some(receipt))
}

fn load_optional_authority(
    path: &Path,
) -> Result<Option<RetainedRootRepairAuthorityV1>, RetainedRootRepairError> {
    let bytes = match read_optional_bounded_regular_bytes(path, MAX_REPAIR_RECEIPT_BYTES) {
        Ok(bytes) => bytes,
        Err(BoundedRegularFileReadError::TooLarge) => {
            return Err(invalid(path, "repair authority exceeds its byte bound"));
        }
        Err(BoundedRegularFileReadError::Read(RegularFileReadError::NotRegular)) => {
            return Err(RetainedRootRepairError::UnsafeFile {
                path: path.to_path_buf(),
            });
        }
        Err(BoundedRegularFileReadError::Read(RegularFileReadError::Io(source))) => {
            return Err(RetainedRootRepairError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
        #[cfg(not(unix))]
        Err(BoundedRegularFileReadError::Read(RegularFileReadError::UnsupportedPlatform)) => {
            return Err(RetainedRootRepairError::Io {
                path: path.to_path_buf(),
                source: io::Error::new(
                    io::ErrorKind::Unsupported,
                    "retained Root repair authority reads are unsupported",
                ),
            });
        }
    };
    let Some(bytes) = bytes else {
        return Ok(None);
    };
    let authority = serde_json::from_slice::<RetainedRootRepairAuthorityV1>(&bytes)
        .map_err(|error| invalid(path, error.to_string()))?;
    if encode_authority(path, &authority)? != bytes {
        return Err(invalid(path, "repair authority bytes are not canonical"));
    }
    Ok(Some(authority))
}

fn encode_receipt(
    path: &Path,
    receipt: &RetainedRootRepairTerminalReceiptV1,
) -> Result<Vec<u8>, RetainedRootRepairError> {
    encode_canonical_json(
        receipt,
        CanonicalJsonStyle::Compact,
        MAX_REPAIR_RECEIPT_BYTES,
    )
    .map_err(|error| match error {
        CanonicalJsonEncodeError::Serialization(error) => invalid(path, error.to_string()),
        CanonicalJsonEncodeError::TooLarge => {
            invalid(path, "repair receipt exceeds its byte bound")
        }
    })
}

fn encode_authority(
    path: &Path,
    authority: &RetainedRootRepairAuthorityV1,
) -> Result<Vec<u8>, RetainedRootRepairError> {
    encode_canonical_json(
        authority,
        CanonicalJsonStyle::Compact,
        MAX_REPAIR_RECEIPT_BYTES,
    )
    .map_err(|error| match error {
        CanonicalJsonEncodeError::Serialization(error) => invalid(path, error.to_string()),
        CanonicalJsonEncodeError::TooLarge => {
            invalid(path, "repair authority exceeds its byte bound")
        }
    })
}

pub(super) fn repair_authority_path(journal_path: &Path) -> PathBuf {
    journal_path.with_file_name(REPAIR_AUTHORITY_FILE)
}

pub(super) fn repair_candidate_path(journal_path: &Path) -> PathBuf {
    journal_path.with_file_name(REPAIR_CANDIDATE_FILE)
}

pub(super) fn repair_receipt_path(journal_path: &Path) -> PathBuf {
    journal_path.with_file_name(REPAIR_RECEIPT_FILE)
}

fn lock_document(path: &Path, lock_file: &str) -> Result<std::fs::File, RetainedRootRepairError> {
    let lock_path = path.with_file_name(lock_file);
    match lock_regular_file_with_parents(&lock_path) {
        Ok(lock) => Ok(lock),
        Err(RegularFileLockError::NotRegular) => {
            Err(RetainedRootRepairError::UnsafeLock { path: lock_path })
        }
        Err(RegularFileLockError::Io(source)) => Err(RetainedRootRepairError::Io {
            path: lock_path,
            source,
        }),
        #[cfg(windows)]
        Err(RegularFileLockError::UnsupportedPlatform) => Err(RetainedRootRepairError::Io {
            path: lock_path,
            source: io::Error::new(
                io::ErrorKind::Unsupported,
                "retained Root repair receipt locking is unsupported",
            ),
        }),
    }
}

fn invalid(path: &Path, reason: impl Into<String>) -> RetainedRootRepairError {
    RetainedRootRepairError::InvalidDocument {
        path: path.to_path_buf(),
        reason: reason.into(),
    }
}

fn bounded_diagnostic(error: impl std::fmt::Display) -> String {
    let message = error.to_string();
    if message.chars().count() <= MAX_CANDID_DIAGNOSTIC_CHARS {
        return message;
    }
    let mut bounded = message
        .chars()
        .take(MAX_CANDID_DIAGNOSTIC_CHARS.saturating_sub(3))
        .collect::<String>();
    bounded.push_str("...");
    bounded
}
