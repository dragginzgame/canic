//! Module: install_root::fleet_subnet_root_repair
//!
//! Responsibility: own one immutable authority for adopting an already-applied, state-preserving
//! Root repair while a retained fresh-install session is still incomplete.
//! Does not own: the original install journal, Root upgrade execution, general managed upgrades,
//! or product-version compatibility.
//! Boundary: compatibility is admitted only by bounded typed schemas, exact retained authority,
//! exact predecessor/successor artifact hashes, Candid equality, and one terminal repair receipt.

#[cfg(test)]
mod tests;

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
use canic_core::ids::{FleetBinding, FleetName, FleetRegistryAuthority, ReleaseBuildId, SubnetId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

const REPAIR_RECEIPT_FILE: &str = "root-repair-receipt.json";
const REPAIR_RECEIPT_LOCK_FILE: &str = "root-repair-receipt.lock";
const REPAIR_RECEIPT_SCHEMA_VERSION: u32 = 1;
const SUPPORTED_SESSION_SCHEMA_VERSIONS: &[u32] = &[1];
const SUPPORTED_ROOT_JOURNAL_SCHEMA_VERSIONS: &[u32] = &[1];
const MAX_REPAIR_RECEIPT_BYTES: usize = 16_384;
const MAX_REPAIR_WASM_BYTES: usize = 64 * 1024 * 1024;

/// The only repair semantics this exceptional fresh-install authority can attest.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum RetainedRootRepairModeV1 {
    StatePreservingUpgrade,
}

/// Immutable evidence authorizing one exact repaired Root module during retained-install replay.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RetainedRootRepairReceiptV1 {
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
    pub install_operation_id: [u8; 32],
    pub authority: FleetRegistryAuthority,
    pub placement_subnet: SubnetId,
    pub fleet_subnet_root: Principal,
    pub installation_controller: Principal,
    pub retained_journal_phase: FleetSubnetRootInstallPhase,
    pub retained_journal_sequence: u64,
    pub predecessor_module_sha256: [u8; 32],
    pub successor_module_sha256: [u8; 32],
    pub successor_wasm_size_bytes: u64,
    pub predecessor_candid_sha256: [u8; 32],
    pub successor_candid_sha256: [u8; 32],
}

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
    install_operation_id: [u8; 32],
    authority: &'a FleetRegistryAuthority,
    placement_subnet: SubnetId,
    fleet_subnet_root: Principal,
    installation_controller: Principal,
    retained_journal_phase: FleetSubnetRootInstallPhase,
    retained_journal_sequence: u64,
    predecessor_module_sha256: [u8; 32],
    successor_module_sha256: [u8; 32],
    successor_wasm_size_bytes: u64,
    predecessor_candid_sha256: [u8; 32],
    successor_candid_sha256: [u8; 32],
}

impl RetainedRootRepairReceiptV1 {
    #[must_use]
    pub(super) const fn successor_module_hash(&self) -> [u8; 32] {
        self.successor_module_sha256
    }
}

/// Optional repair evidence resolved for one exact retained Root journal.
pub(super) struct ResolvedRetainedRootRepair {
    pub receipt: RetainedRootRepairReceiptV1,
    pub needs_publication: bool,
    path: PathBuf,
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

    #[error("retained Root repair has incompatible Candid; exact predecessor Candid is required")]
    CandidMismatch,

    #[error("retained Root repair successor artifact equals the original journal artifact")]
    NotARepair,

    #[error("retained Root repair may be adopted only at component_registry_preparation_verified")]
    InvalidPhase,

    #[error("retained Root repair request names a Root outside the retained install plan")]
    RootNotFound,

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

/// Resolve an existing receipt or compile one candidate from explicit artifact evidence.
pub(super) fn resolve_retained_root_repair(
    current: &ResolvedFleetSubnetRootInstall,
    session: &FleetInstallSession,
    adoption: Option<&RetainedRootRepairAdoption>,
) -> Result<Option<ResolvedRetainedRootRepair>, RetainedRootRepairError> {
    let path = repair_receipt_path(&current.path);
    let retained = load_optional_receipt(&path)?;
    if let Some(receipt) = retained {
        validate_receipt(&path, &receipt, session, &current.journal)?;
        if let Some(adoption) = adoption {
            let requested = compile_adoption(session, &current.journal, adoption)?;
            if requested != receipt {
                return Err(RetainedRootRepairError::ConflictingAuthority { path });
            }
        }
        return Ok(Some(ResolvedRetainedRootRepair {
            receipt,
            needs_publication: false,
            path,
        }));
    }
    let Some(adoption) = adoption else {
        return Ok(None);
    };
    let receipt = compile_adoption(session, &current.journal, adoption)?;
    Ok(Some(ResolvedRetainedRootRepair {
        receipt,
        needs_publication: true,
        path,
    }))
}

/// Publish a fully live-verified candidate, reconciling an uncertain create-new response exactly.
pub(super) fn publish_retained_root_repair(
    resolved: &ResolvedRetainedRootRepair,
    session: &FleetInstallSession,
    journal: &FleetSubnetRootInstallJournal,
) -> Result<(), RetainedRootRepairError> {
    if !resolved.needs_publication {
        return Ok(());
    }
    validate_receipt(&resolved.path, &resolved.receipt, session, journal)?;
    let bytes = encode_receipt(&resolved.path, &resolved.receipt)?;
    let _lock = lock_receipt(&resolved.path)?;
    match create_new_bytes_with_parents(&resolved.path, &bytes) {
        Ok(()) => {}
        Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {
            let observed = load_optional_receipt(&resolved.path)?.ok_or_else(|| {
                RetainedRootRepairError::ConflictingAuthority {
                    path: resolved.path.clone(),
                }
            })?;
            if observed != resolved.receipt {
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
    let durable = load_optional_receipt(&resolved.path)?.ok_or_else(|| {
        RetainedRootRepairError::ConflictingAuthority {
            path: resolved.path.clone(),
        }
    })?;
    if durable != resolved.receipt {
        return Err(RetainedRootRepairError::ConflictingAuthority {
            path: resolved.path.clone(),
        });
    }
    Ok(())
}

fn compile_adoption(
    session: &FleetInstallSession,
    journal: &FleetSubnetRootInstallJournal,
    adoption: &RetainedRootRepairAdoption,
) -> Result<RetainedRootRepairReceiptV1, RetainedRootRepairError> {
    if journal.phase != FleetSubnetRootInstallPhase::ComponentRegistryPreparationVerified {
        return Err(RetainedRootRepairError::InvalidPhase);
    }
    let fleet_subnet_root = journal
        .fleet_subnet_root
        .ok_or(RetainedRootRepairError::RootNotFound)?;
    if fleet_subnet_root != adoption.fleet_subnet_root {
        return Err(RetainedRootRepairError::RootNotFound);
    }
    let installation_controller = journal
        .installation_controller
        .ok_or(RetainedRootRepairError::RootNotFound)?;
    let (successor_bytes, successor_candid_sha256) = inspect_successor_wasm(
        &adoption.successor_wasm,
        journal.root_artifact.candid_sha256,
    )?;
    let successor_module_sha256: [u8; 32] = Sha256::digest(&successor_bytes).into();
    if successor_module_sha256 == journal.expected_root_module_hash {
        return Err(RetainedRootRepairError::NotARepair);
    }
    let successor_wasm_size_bytes = u64::try_from(successor_bytes.len()).map_err(|_| {
        RetainedRootRepairError::ArtifactTooLarge {
            path: adoption.successor_wasm.clone(),
        }
    })?;
    let repair_operation_id = repair_operation_id(
        session,
        journal,
        successor_module_sha256,
        successor_wasm_size_bytes,
        successor_candid_sha256,
    )?;
    let receipt = RetainedRootRepairReceiptV1 {
        schema_version: REPAIR_RECEIPT_SCHEMA_VERSION,
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
        install_operation_id: session.operation_id,
        authority: journal.authority.clone(),
        placement_subnet: journal.root_plan.placement_subnet,
        fleet_subnet_root,
        installation_controller,
        retained_journal_phase: journal.phase,
        retained_journal_sequence: journal.sequence,
        predecessor_module_sha256: journal.expected_root_module_hash,
        successor_module_sha256,
        successor_wasm_size_bytes,
        predecessor_candid_sha256: journal.root_artifact.candid_sha256,
        successor_candid_sha256,
    };
    validate_receipt(Path::new(REPAIR_RECEIPT_FILE), &receipt, session, journal)?;
    Ok(receipt)
}

fn inspect_successor_wasm(
    path: &Path,
    expected_candid_sha256: [u8; 32],
) -> Result<(Vec<u8>, [u8; 32]), RetainedRootRepairError> {
    let before = read_bounded_artifact(path)?;
    let candid = extract_candid_bytes(path)
        .map_err(|error| RetainedRootRepairError::CandidInspection(error.to_string()))?;
    let after = read_bounded_artifact(path)?;
    if before != after {
        return Err(RetainedRootRepairError::ArtifactChanged {
            path: path.to_path_buf(),
        });
    }
    let candid_sha256: [u8; 32] = Sha256::digest(candid).into();
    if candid_sha256 != expected_candid_sha256 {
        return Err(RetainedRootRepairError::CandidMismatch);
    }
    Ok((after, candid_sha256))
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

fn validate_receipt(
    path: &Path,
    receipt: &RetainedRootRepairReceiptV1,
    session: &FleetInstallSession,
    journal: &FleetSubnetRootInstallJournal,
) -> Result<(), RetainedRootRepairError> {
    if receipt.schema_version != REPAIR_RECEIPT_SCHEMA_VERSION {
        return Err(invalid(
            path,
            "unsupported repair-receipt schema; export with the matching Canic release before retrying",
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
    if journal.phase != FleetSubnetRootInstallPhase::ComponentRegistryPreparationVerified {
        return Err(RetainedRootRepairError::InvalidPhase);
    }
    let Some(fleet_subnet_root) = journal.fleet_subnet_root else {
        return Err(RetainedRootRepairError::RootNotFound);
    };
    let Some(installation_controller) = journal.installation_controller else {
        return Err(RetainedRootRepairError::RootNotFound);
    };
    let expected_operation_id = repair_operation_id(
        session,
        journal,
        receipt.successor_module_sha256,
        receipt.successor_wasm_size_bytes,
        receipt.successor_candid_sha256,
    )?;
    let exact_authority = [
        receipt_matches_session(receipt, session),
        receipt_matches_root_journal(
            receipt,
            session,
            journal,
            fleet_subnet_root,
            installation_controller,
        ),
        receipt_has_exact_artifact_transition(receipt, journal),
        receipt.repair_operation_id == expected_operation_id,
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

fn receipt_matches_session(
    receipt: &RetainedRootRepairReceiptV1,
    session: &FleetInstallSession,
) -> bool {
    [
        receipt.repair_mode == RetainedRootRepairModeV1::StatePreservingUpgrade,
        receipt.session_schema_version == session.schema_version,
        receipt.fleet_name == session.fleet_name,
        receipt.fleet == session.fleet,
        receipt.release_build_id == session.release_build_id,
        receipt.fresh_fleet_plan_digest == session.fresh_fleet_plan_digest,
        receipt.install_operation_id == session.operation_id,
    ]
    .into_iter()
    .all(std::convert::identity)
}

fn receipt_matches_root_journal(
    receipt: &RetainedRootRepairReceiptV1,
    session: &FleetInstallSession,
    journal: &FleetSubnetRootInstallJournal,
    fleet_subnet_root: Principal,
    installation_controller: Principal,
) -> bool {
    [
        receipt.root_journal_schema_version == journal.schema_version,
        receipt.fleet_install_plan_digest == journal.fleet_install_plan_digest,
        receipt.infrastructure_manifest_digest == journal.infrastructure_manifest_digest,
        receipt.install_operation_id == journal.install_operation_id,
        receipt.authority == journal.authority,
        receipt.authority.binding.fleet == session.fleet,
        receipt.placement_subnet == journal.root_plan.placement_subnet,
        receipt.fleet_subnet_root == fleet_subnet_root,
        receipt.installation_controller == installation_controller,
        receipt.retained_journal_phase == journal.phase,
        receipt.retained_journal_sequence == journal.sequence,
    ]
    .into_iter()
    .all(std::convert::identity)
}

fn receipt_has_exact_artifact_transition(
    receipt: &RetainedRootRepairReceiptV1,
    journal: &FleetSubnetRootInstallJournal,
) -> bool {
    [
        receipt.predecessor_module_sha256 == journal.expected_root_module_hash,
        receipt.predecessor_candid_sha256 == journal.root_artifact.candid_sha256,
        receipt.successor_candid_sha256 == journal.root_artifact.candid_sha256,
        receipt.successor_module_sha256 != journal.expected_root_module_hash,
        receipt.successor_wasm_size_bytes > 0,
        receipt.successor_wasm_size_bytes <= MAX_REPAIR_WASM_BYTES as u64,
    ]
    .into_iter()
    .all(std::convert::identity)
}

fn repair_operation_id(
    session: &FleetInstallSession,
    journal: &FleetSubnetRootInstallJournal,
    successor_module_sha256: [u8; 32],
    successor_wasm_size_bytes: u64,
    successor_candid_sha256: [u8; 32],
) -> Result<[u8; 32], RetainedRootRepairError> {
    let fleet_subnet_root = journal
        .fleet_subnet_root
        .ok_or(RetainedRootRepairError::RootNotFound)?;
    let installation_controller = journal
        .installation_controller
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
        install_operation_id: session.operation_id,
        authority: &journal.authority,
        placement_subnet: journal.root_plan.placement_subnet,
        fleet_subnet_root,
        installation_controller,
        retained_journal_phase: journal.phase,
        retained_journal_sequence: journal.sequence,
        predecessor_module_sha256: journal.expected_root_module_hash,
        successor_module_sha256,
        successor_wasm_size_bytes,
        predecessor_candid_sha256: journal.root_artifact.candid_sha256,
        successor_candid_sha256,
    };
    let mut hasher = Sha256::new();
    hasher.update(b"canic.retained-root-repair.v1\0");
    hasher.update(
        serde_json::to_vec(&authority)
            .map_err(|error| invalid(Path::new(REPAIR_RECEIPT_FILE), error.to_string()))?,
    );
    let mut digest: [u8; 32] = hasher.finalize().into();
    if digest == [0; 32] {
        digest[31] = 1;
    }
    Ok(digest)
}

fn load_optional_receipt(
    path: &Path,
) -> Result<Option<RetainedRootRepairReceiptV1>, RetainedRootRepairError> {
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
    let receipt = serde_json::from_slice::<RetainedRootRepairReceiptV1>(&bytes)
        .map_err(|error| invalid(path, error.to_string()))?;
    if encode_receipt(path, &receipt)? != bytes {
        return Err(invalid(path, "repair receipt bytes are not canonical"));
    }
    Ok(Some(receipt))
}

fn encode_receipt(
    path: &Path,
    receipt: &RetainedRootRepairReceiptV1,
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

fn repair_receipt_path(journal_path: &Path) -> PathBuf {
    journal_path.with_file_name(REPAIR_RECEIPT_FILE)
}

fn lock_receipt(path: &Path) -> Result<std::fs::File, RetainedRootRepairError> {
    let lock_path = path.with_file_name(REPAIR_RECEIPT_LOCK_FILE);
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
