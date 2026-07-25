//! Module: install_root::fleet_activation_journal
//!
//! Responsibility: own canonical host recovery evidence for one fresh Fleet activation.
//! Does not own: Canister activation, catalog publication, or release-build construction.
//! Boundary: a journal is admitted only from finalized release-build evidence and is durable
//! before any Canister mutation.

#[cfg(test)]
mod tests;

use crate::{
    deployment_truth::{
        DeploymentCommandResultV1, DeploymentExecutionStatusV1, DeploymentReceiptV1,
        ObservationStatusV1,
    },
    durable_io::{
        RegularFileLockError, RegularFileReadError, create_new_bytes_with_parents,
        lock_regular_file_with_parents, read_optional_regular_bytes, write_bytes,
    },
    entropy::{EntropyError, random_bytes_32},
    fleet_catalog::{CommittedFleetCatalog, FleetCatalogEntryV1},
    release_build::{
        FinalizedReleaseBuild, ReleaseBuildPlanError, ReleaseBuildPlanState,
        load_finalized_release_build,
    },
};
use canic_core::{
    api::fleet_activation::FleetActivationApi,
    cdk::types::Principal,
    dto::fleet_activation::{
        FleetActivationHostRecord, FleetActivationIdentity, FleetActivationPhase,
        FleetActivationResumeRequest, FleetActivationStatusResponse,
        FleetCascadeActivationEvidence, FleetCascadeManifestEntry, FleetCredentialGenerationRef,
        FleetCredentialManifest, FleetCredentialManifestEntry, FleetHostCanisterActivationEvidence,
        MAX_FLEET_ACTIVATION_CANISTERS, MAX_FLEET_ACTIVATION_HOST_RECORD_BYTES,
        MAX_FLEET_CREDENTIAL_MANIFEST_ENTRIES,
    },
    ids::{AppId, CanonicalNetworkId, FleetBinding, FleetId, FleetKey, FleetName, ReleaseBuildId},
};
use ciborium::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

const JOURNAL_HASH_DOMAIN: &[u8] = b"canic:fleet-install:activation-journal\0";
const MAX_FLEET_INSTALL_ACTIVATION_JOURNAL_BYTES: usize =
    MAX_FLEET_ACTIVATION_HOST_RECORD_BYTES + 1_024;
const RANDOM_ATTEMPTS: usize = 16;

///
/// FleetInstallActivationPhase
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum FleetInstallActivationPhase {
    Planned,
    RootInstalled,
    CanistersPrepared,
    CanistersActivated,
    HostAuthorityCommitted,
}

///
/// FleetInstallActivationJournal
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct FleetInstallActivationJournal {
    pub sequence: u64,
    pub phase: FleetInstallActivationPhase,
    pub fleet_name: FleetName,
    pub release_build_plan_hash: [u8; 32],
    pub release_set_manifest_digest: [u8; 32],
    pub root_install_receipt_hash: Option<[u8; 32]>,
    pub activation: FleetActivationHostRecord,
    pub committed_fleet_catalog_hash: Option<[u8; 32]>,
}

///
/// ResolvedFleetInstallActivation
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ResolvedFleetInstallActivation {
    pub journal: FleetInstallActivationJournal,
    pub journal_hash: [u8; 32],
    pub path: PathBuf,
    pub created: bool,
}

///
/// RootInstalledFleetInstallActivation
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RootInstalledFleetInstallActivation {
    pub journal: FleetInstallActivationJournal,
    pub journal_hash: [u8; 32],
    pub path: PathBuf,
    pub advanced: bool,
}

///
/// CanistersPreparedFleetInstallActivation
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CanistersPreparedFleetInstallActivation {
    pub journal: FleetInstallActivationJournal,
    pub journal_hash: [u8; 32],
    pub path: PathBuf,
    pub advanced: bool,
}

///
/// CanistersActivatedFleetInstallActivation
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CanistersActivatedFleetInstallActivation {
    pub journal: FleetInstallActivationJournal,
    pub journal_hash: [u8; 32],
    pub path: PathBuf,
    pub advanced: bool,
}

///
/// HostAuthorityCommittedFleetInstallActivation
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct HostAuthorityCommittedFleetInstallActivation {
    pub journal: FleetInstallActivationJournal,
    pub journal_hash: [u8; 32],
    pub path: PathBuf,
    pub advanced: bool,
}

///
/// PreparedFleetActivationEvidence
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PreparedFleetActivationEvidence {
    root_canister: Principal,
    activation: FleetActivationHostRecord,
}

///
/// ActivatedFleetActivationEvidence
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ActivatedFleetActivationEvidence {
    root_canister: Principal,
    activation: FleetActivationHostRecord,
}

///
/// RootInstallReceiptEvidence
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RootInstallReceiptEvidence {
    pub receipt_hash: [u8; 32],
    pub root_canister: Principal,
    pub module_hash: [u8; 32],
    pub activation_identity: FleetActivationIdentity,
}

///
/// PlanFleetInstallActivationRequest
///

pub(super) struct PlanFleetInstallActivationRequest<'a> {
    pub root: &'a Path,
    pub canonical_network_id: CanonicalNetworkId,
    pub fleet_name: FleetName,
    pub app: AppId,
    pub finalized_release_build: &'a FinalizedReleaseBuild,
}

///
/// FleetInstallActivationJournalError
///

#[derive(Debug, ThisError)]
pub(super) enum FleetInstallActivationJournalError {
    #[error("failed to access Fleet install activation journal {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("Fleet install activation journal is not a regular no-follow file: {path}")]
    UnsafeFile { path: PathBuf },

    #[error("root-install receipt is not a regular no-follow file: {path}")]
    UnsafeRootInstallReceipt { path: PathBuf },

    #[error("Fleet install activation journal is missing: {path}")]
    Missing { path: PathBuf },

    #[error("root-install receipt is missing: {path}")]
    MissingRootInstallReceipt { path: PathBuf },

    #[error(
        "root-install receipt identifies Canister {receipt_root}, not resolved root {resolved_root}"
    )]
    RootInstallReceiptCanisterMismatch {
        receipt_root: Principal,
        resolved_root: Principal,
    },

    #[error("invalid Fleet install activation journal {path}: {reason}")]
    InvalidDocument { path: PathBuf, reason: String },

    #[error("invalid root-install receipt {path}: {reason}")]
    InvalidRootInstallReceipt { path: PathBuf, reason: String },

    #[error("source App identity {app:?} is invalid: {reason}")]
    InvalidApp { app: String, reason: String },

    #[error("finalized release-build evidence changed before Fleet activation planning")]
    FinalizedReleaseBuildMismatch,

    #[error(
        "Fleet install activation journal changed before transition: expected {expected}, observed {observed}"
    )]
    JournalChanged { expected: String, observed: String },

    #[error("RootInstalled transition conflicts with the durable root-install receipt hash")]
    RootInstallReceiptMismatch,

    #[error("root-install receipt belongs to a different Fleet activation identity")]
    RootInstallReceiptIdentityMismatch,

    #[error("Fleet install activation journal cannot transition from {phase:?} to RootInstalled")]
    InvalidRootInstalledTransition { phase: FleetInstallActivationPhase },

    #[error("prepared Fleet activation evidence differs from the journalled identity")]
    PreparedActivationIdentityMismatch,

    #[error("invalid prepared Fleet activation evidence: {reason}")]
    InvalidPreparedActivationEvidence { reason: String },

    #[error("CanistersPrepared transition conflicts with durable prepared activation evidence")]
    PreparedActivationEvidenceMismatch,

    #[error(
        "Fleet install activation journal cannot transition from {phase:?} to CanistersPrepared"
    )]
    InvalidCanistersPreparedTransition { phase: FleetInstallActivationPhase },

    #[error("active Fleet activation evidence differs from the journalled Prepared authority")]
    ActivatedActivationEvidenceMismatch,

    #[error("invalid active Fleet activation evidence: {reason}")]
    InvalidActivatedActivationEvidence { reason: String },

    #[error(
        "Fleet install activation journal cannot transition from {phase:?} to CanistersActivated"
    )]
    InvalidCanistersActivatedTransition { phase: FleetInstallActivationPhase },

    #[error("committed Fleet catalog row differs from the activated journal authority")]
    CommittedFleetCatalogMismatch,

    #[error("committed Fleet catalog hash is absent")]
    MissingCommittedFleetCatalogHash,

    #[error(
        "Fleet install activation journal cannot transition from {phase:?} to HostAuthorityCommitted"
    )]
    InvalidHostAuthorityCommittedTransition { phase: FleetInstallActivationPhase },

    #[error(
        "existing Fleet activation {fleet_name} at {path} belongs to App {existing_app}, not requested App {requested_app}"
    )]
    ExistingAppMismatch {
        fleet_name: FleetName,
        existing_app: AppId,
        requested_app: AppId,
        path: PathBuf,
    },

    #[error(
        "existing Fleet activation {fleet_name} at {path} belongs to different finalized release-build evidence"
    )]
    ExistingReleaseBuildMismatch {
        fleet_name: FleetName,
        path: PathBuf,
    },

    #[error(
        "Fleet {fleet_name} has competing exact completed-install observations at {first} and {second}"
    )]
    CompetingCompletedObservations {
        fleet_name: FleetName,
        first: PathBuf,
        second: PathBuf,
    },

    #[error("Fleet {fleet_name} has competing active activation journals at {first} and {second}")]
    CompetingFleetNameAuthorities {
        fleet_name: FleetName,
        first: PathBuf,
        second: PathBuf,
    },

    #[error("Fleet ID {fleet_id} has competing active activation journals at {first} and {second}")]
    CompetingFleetIdAuthorities {
        fleet_id: FleetId,
        first: PathBuf,
        second: PathBuf,
    },

    #[error("unsafe Fleet install activation recovery directory entry: {path}")]
    UnsafeDirectoryEntry { path: PathBuf },

    #[error("invalid Fleet install activation recovery directory {path}: {reason}")]
    InvalidDirectory { path: PathBuf, reason: String },

    #[error("cryptographic random source returned only {actual} of 32 required bytes")]
    ShortRandomRead { actual: usize },

    #[error(
        "could not allocate a unique Fleet activation identity after {RANDOM_ATTEMPTS} attempts"
    )]
    IdentityAllocationExhausted,

    #[error(transparent)]
    ReleaseBuild(#[from] ReleaseBuildPlanError),
}

/// Create and durably publish one new `Planned` activation journal.
pub(super) fn plan_fleet_install_activation(
    request: PlanFleetInstallActivationRequest<'_>,
) -> Result<ResolvedFleetInstallActivation, FleetInstallActivationJournalError> {
    validate_app(&request.app)?;
    let _lock = lock_fleet_install_activation(
        request.root,
        request.canonical_network_id,
        &request.fleet_name,
    )?;
    let finalized = load_finalized_release_build(
        request.root,
        request.finalized_release_build.record.release_build_id,
    )?;
    if finalized.record != request.finalized_release_build.record
        || finalized.plan_hash != request.finalized_release_build.plan_hash
    {
        return Err(FleetInstallActivationJournalError::FinalizedReleaseBuildMismatch);
    }
    let ReleaseBuildPlanState::Finalized {
        release_set_manifest_digest,
    } = finalized.record.state
    else {
        unreachable!("load_finalized_release_build admits only finalized records");
    };

    let mut discovered = discover_fleet_install_activation(
        request.root,
        request.canonical_network_id,
        &request.fleet_name,
    )?;
    if let Some(active) = discovered.active {
        return resolve_discovered_activation(
            &request,
            &finalized,
            release_set_manifest_digest,
            active,
        );
    }
    let matching_completed = discovered
        .completed
        .iter()
        .enumerate()
        .filter(|completed| {
            discovered_activation_matches(
                completed.1,
                &request,
                &finalized,
                release_set_manifest_digest,
            )
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if let [first, second, ..] = matching_completed.as_slice() {
        return Err(
            FleetInstallActivationJournalError::CompetingCompletedObservations {
                fleet_name: request.fleet_name,
                first: discovered.completed[*first].path.clone(),
                second: discovered.completed[*second].path.clone(),
            },
        );
    }
    if let Some(index) = matching_completed.first() {
        return Ok(resolved_discovered_activation(
            discovered.completed.swap_remove(*index),
        ));
    }
    if let Some(completed) = discovered.completed.pop() {
        return resolve_discovered_activation(
            &request,
            &finalized,
            release_set_manifest_digest,
            completed,
        );
    }

    for _ in 0..RANDOM_ATTEMPTS {
        let fleet_id = FleetId::from_generated_bytes(random_identity_bytes()?);
        let operation_id = random_identity_bytes()?;
        match plan_fleet_install_activation_with_ids(
            &request,
            &finalized,
            release_set_manifest_digest,
            fleet_id,
            operation_id,
        ) {
            Ok(planned) => return Ok(planned),
            Err(FleetInstallActivationJournalError::Io { source, .. })
                if source.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }

    Err(FleetInstallActivationJournalError::IdentityAllocationExhausted)
}

/// Load one exact journal and verify every path identity component.
pub(super) fn load_fleet_install_activation_journal(
    root: &Path,
    canonical_network_id: CanonicalNetworkId,
    fleet_id: FleetId,
    operation_id: [u8; 32],
) -> Result<FleetInstallActivationJournal, FleetInstallActivationJournalError> {
    let path =
        fleet_install_activation_journal_path(root, canonical_network_id, fleet_id, operation_id);
    let bytes = read_journal_bytes(&path)?;
    let journal = decode_journal(&path, &bytes)?;
    if journal.activation.identity.fleet.fleet.canonical_network_id != canonical_network_id {
        return Err(invalid(
            &path,
            "path canonical network does not match activation identity",
        ));
    }
    if journal.activation.identity.fleet.fleet.fleet_id != fleet_id {
        return Err(invalid(
            &path,
            "path Fleet ID does not match activation identity",
        ));
    }
    if journal.activation.identity.operation_id != operation_id {
        return Err(invalid(
            &path,
            "path operation ID does not match activation identity",
        ));
    }
    Ok(journal)
}

/// Return the canonical path for one fresh-install activation journal.
#[must_use]
pub(super) fn fleet_install_activation_journal_path(
    root: &Path,
    canonical_network_id: CanonicalNetworkId,
    fleet_id: FleetId,
    operation_id: [u8; 32],
) -> PathBuf {
    root.join(".canic")
        .join("recovery")
        .join("fleet-install-activations")
        .join(canonical_network_id.to_string())
        .join(fleet_id.to_string())
        .join(hex_digest(operation_id))
        .join("journal.cbor")
}

/// Hash one exact journal under the frozen activation-journal separator.
#[must_use]
pub(super) fn fleet_install_activation_journal_hash(
    journal: &FleetInstallActivationJournal,
) -> [u8; 32] {
    let bytes = encode_journal(journal)
        .expect("an admitted activation journal must retain canonical encodable fields");
    domain_hash(JOURNAL_HASH_DOMAIN, &bytes)
}

/// Admit one exact durable root-install receipt with verified module evidence.
pub(super) fn admit_root_install_receipt(
    path: &Path,
) -> Result<RootInstallReceiptEvidence, FleetInstallActivationJournalError> {
    let bytes = read_root_install_receipt_bytes(path)?;
    let receipt = serde_json::from_slice::<DeploymentReceiptV1>(&bytes)
        .map_err(|error| invalid_root_install_receipt(path, error.to_string()))?;
    let mut canonical = serde_json::to_vec_pretty(&receipt)
        .map_err(|error| invalid_root_install_receipt(path, error.to_string()))?;
    canonical.push(b'\n');
    if canonical != bytes {
        return Err(invalid_root_install_receipt(
            path,
            "JSON bytes are not the canonical durable receipt encoding",
        ));
    }
    if receipt.operation_status != DeploymentExecutionStatusV1::Complete
        || receipt.command_result != DeploymentCommandResultV1::Succeeded
        || !receipt.operation_id.ends_with(":install_root")
        || receipt.finished_at.is_none()
        || receipt.phase_receipts.len() != 1
    {
        return Err(invalid_root_install_receipt(
            path,
            "receipt is not one completed successful install_root operation",
        ));
    }
    let phase = &receipt.phase_receipts[0];
    if phase.phase != "install_root"
        || phase.finished_at.is_none()
        || phase.verified_postcondition.status != ObservationStatusV1::Observed
    {
        return Err(invalid_root_install_receipt(
            path,
            "install_root phase lacks a completed observed postcondition",
        ));
    }
    let root_canister_text = exact_receipt_evidence(
        path,
        &phase.verified_postcondition.evidence,
        "root_canister:",
    )?;
    let root_canister = root_canister_text.parse().map_err(|error| {
        invalid_root_install_receipt(path, format!("root_canister is invalid: {error}"))
    })?;
    if receipt.root_principal.as_deref() != Some(root_canister_text) {
        return Err(invalid_root_install_receipt(
            path,
            "root_principal must exactly match root_canister evidence",
        ));
    }
    let root_wasm =
        exact_receipt_evidence(path, &phase.verified_postcondition.evidence, "root_wasm:")?;
    if root_wasm.is_empty() {
        return Err(invalid_root_install_receipt(
            path,
            "root_wasm evidence must not be empty",
        ));
    }
    let expected_module_hash = parse_receipt_digest(
        path,
        exact_receipt_evidence(
            path,
            &phase.verified_postcondition.evidence,
            "expected_module_hash:",
        )?,
        "expected_module_hash",
    )?;
    let observed_module_hash = parse_receipt_digest(
        path,
        exact_receipt_evidence(
            path,
            &phase.verified_postcondition.evidence,
            "observed_module_hash:",
        )?,
        "observed_module_hash",
    )?;
    if observed_module_hash != expected_module_hash {
        return Err(invalid_root_install_receipt(
            path,
            "observed module hash does not match the installed Wasm",
        ));
    }
    let activation_identity =
        root_install_activation_identity(path, &phase.verified_postcondition.evidence)?;

    Ok(RootInstallReceiptEvidence {
        receipt_hash: Sha256::digest(&bytes).into(),
        root_canister,
        module_hash: observed_module_hash,
        activation_identity,
    })
}

/// Recover the exact root-install receipt named by its journalled raw-byte hash.
pub(super) fn recover_root_install_receipt(
    receipt_directory: &Path,
    expected_hash: [u8; 32],
) -> Result<RootInstallReceiptEvidence, FleetInstallActivationJournalError> {
    let metadata = fs::symlink_metadata(receipt_directory).map_err(|source| {
        if source.kind() == io::ErrorKind::NotFound {
            FleetInstallActivationJournalError::MissingRootInstallReceipt {
                path: receipt_directory.to_path_buf(),
            }
        } else {
            FleetInstallActivationJournalError::Io {
                path: receipt_directory.to_path_buf(),
                source,
            }
        }
    })?;
    if !metadata.file_type().is_dir() {
        return Err(FleetInstallActivationJournalError::UnsafeDirectoryEntry {
            path: receipt_directory.to_path_buf(),
        });
    }
    let mut entries = fs::read_dir(receipt_directory)
        .map_err(|source| FleetInstallActivationJournalError::Io {
            path: receipt_directory.to_path_buf(),
            source,
        })?
        .map(|entry| {
            entry.map_err(|source| FleetInstallActivationJournalError::Io {
                path: receipt_directory.to_path_buf(),
                source,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let file_type =
            entry
                .file_type()
                .map_err(|source| FleetInstallActivationJournalError::Io {
                    path: path.clone(),
                    source,
                })?;
        if !file_type.is_file() {
            return Err(FleetInstallActivationJournalError::UnsafeRootInstallReceipt { path });
        }
        let bytes = read_root_install_receipt_bytes(&path)?;
        if <[u8; 32]>::from(Sha256::digest(&bytes)) == expected_hash {
            return admit_root_install_receipt(&path);
        }
    }
    Err(
        FleetInstallActivationJournalError::MissingRootInstallReceipt {
            path: receipt_directory.to_path_buf(),
        },
    )
}

/// Recover the already installed root before any root-resolution side effect.
pub(super) fn recover_activation_root_canister(
    resolved: &ResolvedFleetInstallActivation,
    receipt_directory: &Path,
) -> Result<Option<Principal>, FleetInstallActivationJournalError> {
    match resolved.journal.phase {
        FleetInstallActivationPhase::Planned => Ok(None),
        FleetInstallActivationPhase::RootInstalled => {
            let receipt = recover_root_install_receipt(
                receipt_directory,
                resolved
                    .journal
                    .root_install_receipt_hash
                    .expect("validated RootInstalled journal retains its receipt hash"),
            )?;
            if receipt.activation_identity != resolved.journal.activation.identity {
                return Err(FleetInstallActivationJournalError::RootInstallReceiptIdentityMismatch);
            }
            Ok(Some(receipt.root_canister))
        }
        FleetInstallActivationPhase::CanistersPrepared => {
            validate_prepared_activation_record(&resolved.journal.activation)
                .map(Some)
                .map_err(invalid_prepared)
        }
        FleetInstallActivationPhase::CanistersActivated
        | FleetInstallActivationPhase::HostAuthorityCommitted => {
            validate_activated_activation_record(&resolved.journal.activation)
                .map(Some)
                .map_err(invalid_active)
        }
    }
}

/// Advance one exact `Planned` journal from durable root-install evidence.
pub(super) fn record_root_installed(
    root: &Path,
    resolved: &ResolvedFleetInstallActivation,
    receipt: &RootInstallReceiptEvidence,
) -> Result<RootInstalledFleetInstallActivation, FleetInstallActivationJournalError> {
    let identity = &resolved.journal.activation.identity;
    let expected_path = fleet_install_activation_journal_path(
        root,
        identity.fleet.fleet.canonical_network_id,
        identity.fleet.fleet.fleet_id,
        identity.operation_id,
    );
    if resolved.path != expected_path {
        return Err(invalid(
            &resolved.path,
            "resolved journal path is not canonical for its activation identity",
        ));
    }
    if receipt.activation_identity != *identity {
        return Err(FleetInstallActivationJournalError::RootInstallReceiptIdentityMismatch);
    }
    let _lock = lock_fleet_install_activation(
        root,
        identity.fleet.fleet.canonical_network_id,
        &resolved.journal.fleet_name,
    )?;
    let observed = load_fleet_install_activation_journal(
        root,
        identity.fleet.fleet.canonical_network_id,
        identity.fleet.fleet.fleet_id,
        identity.operation_id,
    )?;

    if observed.phase == FleetInstallActivationPhase::RootInstalled {
        if observed.root_install_receipt_hash == Some(receipt.receipt_hash) {
            return Ok(root_installed_result(
                observed,
                resolved.path.clone(),
                false,
            ));
        }
        return Err(FleetInstallActivationJournalError::RootInstallReceiptMismatch);
    }
    if observed.phase != FleetInstallActivationPhase::Planned {
        return Err(
            FleetInstallActivationJournalError::InvalidRootInstalledTransition {
                phase: observed.phase,
            },
        );
    }
    let observed_hash = fleet_install_activation_journal_hash(&observed);
    if observed_hash != resolved.journal_hash || observed != resolved.journal {
        return Err(FleetInstallActivationJournalError::JournalChanged {
            expected: hex_digest(resolved.journal_hash),
            observed: hex_digest(observed_hash),
        });
    }

    let mut next = observed;
    next.sequence = next
        .sequence
        .checked_add(1)
        .expect("validated Planned sequence zero advances to one");
    next.phase = FleetInstallActivationPhase::RootInstalled;
    next.root_install_receipt_hash = Some(receipt.receipt_hash);
    let bytes = encode_journal(&next)?;
    if let Err(source) = write_bytes(&resolved.path, &bytes) {
        match load_fleet_install_activation_journal(
            root,
            identity.fleet.fleet.canonical_network_id,
            identity.fleet.fleet.fleet_id,
            identity.operation_id,
        ) {
            Ok(observed) if observed == next => {
                return Ok(root_installed_result(next, resolved.path.clone(), true));
            }
            _ => {
                return Err(FleetInstallActivationJournalError::Io {
                    path: resolved.path.clone(),
                    source,
                });
            }
        }
    }
    let durable = load_fleet_install_activation_journal(
        root,
        identity.fleet.fleet.canonical_network_id,
        identity.fleet.fleet.fleet_id,
        identity.operation_id,
    )?;
    if durable != next {
        return Err(invalid(
            &resolved.path,
            "published RootInstalled journal differs from the transition record",
        ));
    }
    Ok(root_installed_result(next, resolved.path.clone(), true))
}

/// Admit one successfully cascaded root `Prepared` manifest into canonical host evidence.
pub(super) fn admit_canisters_prepared(
    root_canister: Principal,
    expected_identity: &FleetActivationIdentity,
    root_status: &FleetActivationStatusResponse,
) -> Result<PreparedFleetActivationEvidence, FleetInstallActivationJournalError> {
    let admitted_root = admit_prepared_root_status(expected_identity, root_status)?;
    let mut canisters = prepared_child_inventory(root_canister, admitted_root.cascade_manifest)?;
    canisters.push(FleetHostCanisterActivationEvidence {
        principal: root_canister,
        activation_evidence_hash: Some(admitted_root.activation_evidence_hash),
    });
    canisters.sort_by(|left, right| left.principal.as_slice().cmp(right.principal.as_slice()));

    let activation = FleetActivationHostRecord {
        identity: expected_identity.clone(),
        cascade_manifest: Some(admitted_root.cascade_manifest.to_vec()),
        credential: Some(admitted_root.credential),
        credential_manifest: Some(admitted_root.credential_manifest.clone()),
        canisters,
    };
    let observed_root =
        validate_prepared_activation_record(&activation).map_err(invalid_prepared)?;
    if observed_root != root_canister {
        return Err(invalid_prepared(
            "prepared activation record does not identify the installed root",
        ));
    }
    Ok(PreparedFleetActivationEvidence {
        root_canister,
        activation,
    })
}

struct AdmittedPreparedRoot<'a> {
    cascade_manifest: &'a [FleetCascadeManifestEntry],
    credential: FleetCredentialGenerationRef,
    credential_manifest: &'a FleetCredentialManifest,
    activation_evidence_hash: [u8; 32],
}

fn admit_prepared_root_status<'a>(
    expected_identity: &FleetActivationIdentity,
    root_status: &'a FleetActivationStatusResponse,
) -> Result<AdmittedPreparedRoot<'a>, FleetInstallActivationJournalError> {
    if root_status.phase != FleetActivationPhase::Prepared
        || root_status.identity != *expected_identity
        || root_status.activated_at_ns.is_some()
    {
        return Err(invalid_prepared(
            "root status is not the exact expected Prepared activation",
        ));
    }
    let cascade_manifest = root_status
        .cascade_manifest
        .as_ref()
        .ok_or_else(|| invalid_prepared("root status is missing its cascade manifest"))?;
    let credential = root_status
        .credential
        .ok_or_else(|| invalid_prepared("root status is missing its credential generation"))?;
    let credential_manifest = root_status
        .credential_manifest
        .as_ref()
        .ok_or_else(|| invalid_prepared("root status is missing its credential manifest"))?;
    let FleetCascadeActivationEvidence::Source {
        cascade_manifest_hash,
    } = root_status
        .cascade
        .as_ref()
        .ok_or_else(|| invalid_prepared("root status is missing source cascade evidence"))?
    else {
        return Err(invalid_prepared(
            "root status must contain source cascade evidence",
        ));
    };
    if cascade_manifest
        .len()
        .checked_add(1)
        .is_none_or(|count| count > MAX_FLEET_ACTIVATION_CANISTERS)
    {
        return Err(invalid_prepared(
            "prepared activation exceeds the Canister inventory bound",
        ));
    }
    if credential_manifest.entries.len() > MAX_FLEET_CREDENTIAL_MANIFEST_ENTRIES {
        return Err(invalid_prepared(
            "prepared credential manifest exceeds its entry bound",
        ));
    }

    let observed_cascade_hash = FleetActivationApi::cascade_manifest_hash(cascade_manifest)
        .map_err(|error| invalid_prepared(format!("invalid cascade manifest: {error}")))?;
    if observed_cascade_hash != *cascade_manifest_hash {
        return Err(invalid_prepared(
            "root source cascade hash does not match its manifest",
        ));
    }
    if credential_manifest.fleet != expected_identity.fleet.fleet
        || credential_manifest.activation_id != expected_identity.operation_id
        || credential_manifest.generation != credential.generation
    {
        return Err(invalid_prepared(
            "credential manifest does not match the activation identity and generation",
        ));
    }
    let observed_credential_hash =
        FleetActivationApi::credential_manifest_hash(credential_manifest)
            .map_err(|error| invalid_prepared(format!("invalid credential manifest: {error}")))?;
    if observed_credential_hash != credential.manifest_hash {
        return Err(invalid_prepared(
            "credential generation hash does not match its manifest",
        ));
    }

    let activation_evidence_hash = FleetActivationApi::activation_evidence_hash(
        expected_identity,
        root_status
            .cascade
            .as_ref()
            .expect("root cascade evidence was admitted"),
        credential,
    )
    .map_err(|error| invalid_prepared(format!("invalid root activation evidence: {error}")))?;
    Ok(AdmittedPreparedRoot {
        cascade_manifest,
        credential,
        credential_manifest,
        activation_evidence_hash,
    })
}

fn prepared_child_inventory(
    root_canister: Principal,
    cascade_manifest: &[FleetCascadeManifestEntry],
) -> Result<Vec<FleetHostCanisterActivationEvidence>, FleetInstallActivationJournalError> {
    cascade_manifest
        .iter()
        .map(|entry| {
            if entry.principal == root_canister {
                return Err(invalid_prepared(
                    "root Canister must not appear in its child cascade manifest",
                ));
            }
            Ok(FleetHostCanisterActivationEvidence {
                principal: entry.principal,
                activation_evidence_hash: None,
            })
        })
        .collect()
}

/// Advance one exact `RootInstalled` journal from complete Prepared status evidence.
pub(super) fn record_canisters_prepared(
    root: &Path,
    resolved: &RootInstalledFleetInstallActivation,
    evidence: &PreparedFleetActivationEvidence,
) -> Result<CanistersPreparedFleetInstallActivation, FleetInstallActivationJournalError> {
    let identity = &resolved.journal.activation.identity;
    let expected_path = fleet_install_activation_journal_path(
        root,
        identity.fleet.fleet.canonical_network_id,
        identity.fleet.fleet.fleet_id,
        identity.operation_id,
    );
    if resolved.path != expected_path {
        return Err(invalid(
            &resolved.path,
            "resolved journal path is not canonical for its activation identity",
        ));
    }
    if evidence.activation.identity != *identity {
        return Err(FleetInstallActivationJournalError::PreparedActivationIdentityMismatch);
    }
    let observed_root =
        validate_prepared_activation_record(&evidence.activation).map_err(invalid_prepared)?;
    if observed_root != evidence.root_canister {
        return Err(invalid_prepared(
            "prepared activation root differs from the installed root",
        ));
    }

    let _lock = lock_fleet_install_activation(
        root,
        identity.fleet.fleet.canonical_network_id,
        &resolved.journal.fleet_name,
    )?;
    let observed = load_fleet_install_activation_journal(
        root,
        identity.fleet.fleet.canonical_network_id,
        identity.fleet.fleet.fleet_id,
        identity.operation_id,
    )?;
    if observed.phase == FleetInstallActivationPhase::CanistersPrepared {
        if observed.activation == evidence.activation {
            return Ok(canisters_prepared_result(
                observed,
                resolved.path.clone(),
                false,
            ));
        }
        return Err(FleetInstallActivationJournalError::PreparedActivationEvidenceMismatch);
    }
    if observed.phase != FleetInstallActivationPhase::RootInstalled {
        return Err(
            FleetInstallActivationJournalError::InvalidCanistersPreparedTransition {
                phase: observed.phase,
            },
        );
    }
    let observed_hash = fleet_install_activation_journal_hash(&observed);
    if observed_hash != resolved.journal_hash || observed != resolved.journal {
        return Err(FleetInstallActivationJournalError::JournalChanged {
            expected: hex_digest(resolved.journal_hash),
            observed: hex_digest(observed_hash),
        });
    }

    let mut next = observed;
    next.sequence = next
        .sequence
        .checked_add(1)
        .expect("validated RootInstalled sequence one advances to two");
    next.phase = FleetInstallActivationPhase::CanistersPrepared;
    next.activation = evidence.activation.clone();
    let bytes = encode_journal(&next)?;
    if let Err(source) = write_bytes(&resolved.path, &bytes) {
        match load_fleet_install_activation_journal(
            root,
            identity.fleet.fleet.canonical_network_id,
            identity.fleet.fleet.fleet_id,
            identity.operation_id,
        ) {
            Ok(observed) if observed == next => {
                return Ok(canisters_prepared_result(next, resolved.path.clone(), true));
            }
            _ => {
                return Err(FleetInstallActivationJournalError::Io {
                    path: resolved.path.clone(),
                    source,
                });
            }
        }
    }
    let durable = load_fleet_install_activation_journal(
        root,
        identity.fleet.fleet.canonical_network_id,
        identity.fleet.fleet.fleet_id,
        identity.operation_id,
    )?;
    if durable != next {
        return Err(invalid(
            &resolved.path,
            "published CanistersPrepared journal differs from the transition record",
        ));
    }
    Ok(canisters_prepared_result(next, resolved.path.clone(), true))
}

/// Resolve an already durable `CanistersPrepared` journal without another effect.
pub(super) fn resume_canisters_prepared(
    resolved: &ResolvedFleetInstallActivation,
) -> Result<CanistersPreparedFleetInstallActivation, FleetInstallActivationJournalError> {
    if resolved.journal.phase != FleetInstallActivationPhase::CanistersPrepared {
        return Err(
            FleetInstallActivationJournalError::InvalidCanistersPreparedTransition {
                phase: resolved.journal.phase,
            },
        );
    }
    Ok(canisters_prepared_result(
        resolved.journal.clone(),
        resolved.path.clone(),
        false,
    ))
}

/// Build the exact idempotent request for the journalled Prepared generation.
pub(super) const fn canisters_prepared_resume_request(
    prepared: &CanistersPreparedFleetInstallActivation,
) -> FleetActivationResumeRequest {
    FleetActivationResumeRequest {
        operation_id: prepared.journal.activation.identity.operation_id,
        credential: prepared
            .journal
            .activation
            .credential
            .expect("validated CanistersPrepared journal retains its credential generation"),
    }
}

/// Admit one root `Active` observation after the root-owned resume completed.
pub(super) fn admit_canisters_activated(
    root_canister: Principal,
    prepared: &CanistersPreparedFleetInstallActivation,
    root_status: &FleetActivationStatusResponse,
) -> Result<ActivatedFleetActivationEvidence, FleetInstallActivationJournalError> {
    let prepared_root = validate_prepared_activation_record(&prepared.journal.activation)
        .map_err(invalid_prepared)?;
    if prepared_root != root_canister {
        return Err(invalid_active(
            "active root differs from the journalled Prepared root",
        ));
    }
    admit_active_root_status(&prepared.journal.activation, root_status)?;

    let mut activation = prepared.journal.activation.clone();
    let credential = activation
        .credential
        .expect("validated CanistersPrepared journal retains its credential generation");
    let cascade_manifest = activation
        .cascade_manifest
        .as_ref()
        .expect("validated CanistersPrepared journal retains its cascade manifest")
        .iter()
        .map(|entry| (entry.principal, entry))
        .collect::<BTreeMap<_, _>>();
    for canister in &mut activation.canisters {
        if canister.principal == root_canister {
            continue;
        }
        let entry = cascade_manifest
            .get(&canister.principal)
            .expect("validated CanistersPrepared inventory exactly covers its cascade manifest");
        let cascade = FleetCascadeActivationEvidence::Applied {
            state_snapshot_hash: entry.state_snapshot_hash,
            topology_snapshot_hash: entry.topology_snapshot_hash,
        };
        canister.activation_evidence_hash = Some(
            FleetActivationApi::activation_evidence_hash(
                &activation.identity,
                &cascade,
                credential,
            )
            .map_err(|error| {
                invalid_active(format!(
                    "invalid activation evidence for child {}: {error}",
                    canister.principal
                ))
            })?,
        );
    }
    let observed_root =
        validate_activated_activation_record(&activation).map_err(invalid_active)?;
    if observed_root != root_canister {
        return Err(invalid_active(
            "active activation record does not identify the journalled root",
        ));
    }
    Ok(ActivatedFleetActivationEvidence {
        root_canister,
        activation,
    })
}

fn admit_active_root_status(
    prepared: &FleetActivationHostRecord,
    root_status: &FleetActivationStatusResponse,
) -> Result<(), FleetInstallActivationJournalError> {
    let cascade_manifest = prepared
        .cascade_manifest
        .as_ref()
        .expect("validated CanistersPrepared journal retains its cascade manifest");
    let cascade_manifest_hash = FleetActivationApi::cascade_manifest_hash(cascade_manifest)
        .map_err(|error| invalid_active(format!("invalid cascade manifest: {error}")))?;
    let expected_cascade = FleetCascadeActivationEvidence::Source {
        cascade_manifest_hash,
    };
    if root_status.phase != FleetActivationPhase::Active
        || root_status.identity != prepared.identity
        || root_status.cascade.as_ref() != Some(&expected_cascade)
        || root_status.cascade_manifest.as_ref() != Some(cascade_manifest)
        || root_status.credential != prepared.credential
        || root_status.credential_manifest != prepared.credential_manifest
        || root_status.activated_at_ns.is_none()
    {
        return Err(invalid_active(
            "root status does not prove the exact journalled activation became Active",
        ));
    }
    Ok(())
}

/// Advance one exact `CanistersPrepared` journal from complete active evidence.
pub(super) fn record_canisters_activated(
    root: &Path,
    prepared: &CanistersPreparedFleetInstallActivation,
    evidence: &ActivatedFleetActivationEvidence,
) -> Result<CanistersActivatedFleetInstallActivation, FleetInstallActivationJournalError> {
    let identity = &prepared.journal.activation.identity;
    let expected_path = fleet_install_activation_journal_path(
        root,
        identity.fleet.fleet.canonical_network_id,
        identity.fleet.fleet.fleet_id,
        identity.operation_id,
    );
    if prepared.path != expected_path {
        return Err(invalid(
            &prepared.path,
            "prepared journal path is not canonical for its activation identity",
        ));
    }
    let observed_root =
        validate_activated_activation_record(&evidence.activation).map_err(invalid_active)?;
    if !activated_preserves_prepared_authority(&prepared.journal.activation, &evidence.activation)
        || evidence.root_canister != observed_root
        || evidence.root_canister
            != validate_prepared_activation_record(&prepared.journal.activation)
                .map_err(invalid_prepared)?
    {
        return Err(FleetInstallActivationJournalError::ActivatedActivationEvidenceMismatch);
    }

    let _lock = lock_fleet_install_activation(
        root,
        identity.fleet.fleet.canonical_network_id,
        &prepared.journal.fleet_name,
    )?;
    let observed = load_fleet_install_activation_journal(
        root,
        identity.fleet.fleet.canonical_network_id,
        identity.fleet.fleet.fleet_id,
        identity.operation_id,
    )?;
    if observed.phase == FleetInstallActivationPhase::CanistersActivated {
        if observed.activation == evidence.activation {
            return Ok(canisters_activated_result(
                observed,
                prepared.path.clone(),
                false,
            ));
        }
        return Err(FleetInstallActivationJournalError::ActivatedActivationEvidenceMismatch);
    }
    if observed.phase != FleetInstallActivationPhase::CanistersPrepared {
        return Err(
            FleetInstallActivationJournalError::InvalidCanistersActivatedTransition {
                phase: observed.phase,
            },
        );
    }
    let observed_hash = fleet_install_activation_journal_hash(&observed);
    if observed_hash != prepared.journal_hash || observed != prepared.journal {
        return Err(FleetInstallActivationJournalError::JournalChanged {
            expected: hex_digest(prepared.journal_hash),
            observed: hex_digest(observed_hash),
        });
    }

    let next = next_canisters_activated_journal(observed, &evidence.activation);
    let bytes = encode_journal(&next)?;
    if let Err(source) = write_bytes(&prepared.path, &bytes) {
        match load_fleet_install_activation_journal(
            root,
            identity.fleet.fleet.canonical_network_id,
            identity.fleet.fleet.fleet_id,
            identity.operation_id,
        ) {
            Ok(observed) if observed == next => {
                return Ok(canisters_activated_result(
                    next,
                    prepared.path.clone(),
                    true,
                ));
            }
            _ => {
                return Err(FleetInstallActivationJournalError::Io {
                    path: prepared.path.clone(),
                    source,
                });
            }
        }
    }
    let durable = load_fleet_install_activation_journal(
        root,
        identity.fleet.fleet.canonical_network_id,
        identity.fleet.fleet.fleet_id,
        identity.operation_id,
    )?;
    if durable != next {
        return Err(invalid(
            &prepared.path,
            "published CanistersActivated journal differs from the transition record",
        ));
    }
    Ok(canisters_activated_result(
        next,
        prepared.path.clone(),
        true,
    ))
}

fn activated_preserves_prepared_authority(
    prepared: &FleetActivationHostRecord,
    activated: &FleetActivationHostRecord,
) -> bool {
    prepared.identity == activated.identity
        && prepared.cascade_manifest == activated.cascade_manifest
        && prepared.credential == activated.credential
        && prepared.credential_manifest == activated.credential_manifest
        && prepared.canisters.len() == activated.canisters.len()
        && prepared
            .canisters
            .iter()
            .zip(&activated.canisters)
            .all(|(prepared, activated)| {
                prepared.principal == activated.principal
                    && prepared
                        .activation_evidence_hash
                        .is_none_or(|hash| activated.activation_evidence_hash == Some(hash))
            })
}

fn next_canisters_activated_journal(
    mut prepared: FleetInstallActivationJournal,
    activation: &FleetActivationHostRecord,
) -> FleetInstallActivationJournal {
    prepared.sequence = prepared
        .sequence
        .checked_add(1)
        .expect("validated CanistersPrepared sequence two advances to three");
    prepared.phase = FleetInstallActivationPhase::CanistersActivated;
    prepared.activation = activation.clone();
    prepared
}

/// Resolve an already durable `CanistersActivated` journal without another effect.
pub(super) fn resume_canisters_activated(
    resolved: &ResolvedFleetInstallActivation,
) -> Result<CanistersActivatedFleetInstallActivation, FleetInstallActivationJournalError> {
    if resolved.journal.phase != FleetInstallActivationPhase::CanistersActivated {
        return Err(
            FleetInstallActivationJournalError::InvalidCanistersActivatedTransition {
                phase: resolved.journal.phase,
            },
        );
    }
    Ok(canisters_activated_result(
        resolved.journal.clone(),
        resolved.path.clone(),
        false,
    ))
}

/// Advance one exact active journal after the canonical Fleet catalog is durable.
pub(super) fn record_host_authority_committed(
    root: &Path,
    activated: &CanistersActivatedFleetInstallActivation,
    receipt_directory: &Path,
    committed_catalog: &CommittedFleetCatalog,
) -> Result<HostAuthorityCommittedFleetInstallActivation, FleetInstallActivationJournalError> {
    if !committed_catalog.belongs_to(root) {
        return Err(FleetInstallActivationJournalError::CommittedFleetCatalogMismatch);
    }
    validate_committed_catalog(&activated.journal, committed_catalog.entry())?;
    require_root_install_receipt(
        &activated.journal,
        receipt_directory,
        committed_catalog.entry(),
    )?;
    let identity = &activated.journal.activation.identity;
    let expected_path = fleet_install_activation_journal_path(
        root,
        identity.fleet.fleet.canonical_network_id,
        identity.fleet.fleet.fleet_id,
        identity.operation_id,
    );
    if activated.path != expected_path {
        return Err(invalid(
            &activated.path,
            "active journal path is not canonical for its activation identity",
        ));
    }

    let _lock = lock_fleet_install_activation(
        root,
        identity.fleet.fleet.canonical_network_id,
        &activated.journal.fleet_name,
    )?;
    let observed = load_fleet_install_activation_journal(
        root,
        identity.fleet.fleet.canonical_network_id,
        identity.fleet.fleet.fleet_id,
        identity.operation_id,
    )?;
    if observed.phase == FleetInstallActivationPhase::HostAuthorityCommitted {
        if observed.committed_fleet_catalog_hash == Some(committed_catalog.catalog_hash()) {
            return Ok(host_authority_committed_result(
                observed,
                activated.path.clone(),
                false,
            ));
        }
        return Err(FleetInstallActivationJournalError::CommittedFleetCatalogMismatch);
    }
    if observed.phase != FleetInstallActivationPhase::CanistersActivated {
        return Err(
            FleetInstallActivationJournalError::InvalidHostAuthorityCommittedTransition {
                phase: observed.phase,
            },
        );
    }
    let observed_hash = fleet_install_activation_journal_hash(&observed);
    if observed_hash != activated.journal_hash || observed != activated.journal {
        return Err(FleetInstallActivationJournalError::JournalChanged {
            expected: hex_digest(activated.journal_hash),
            observed: hex_digest(observed_hash),
        });
    }

    let next = next_host_authority_committed_journal(observed, committed_catalog.catalog_hash());
    let bytes = encode_journal(&next)?;
    if let Err(source) = write_bytes(&activated.path, &bytes) {
        match load_fleet_install_activation_journal(
            root,
            identity.fleet.fleet.canonical_network_id,
            identity.fleet.fleet.fleet_id,
            identity.operation_id,
        ) {
            Ok(observed) if observed == next => {
                return Ok(host_authority_committed_result(
                    next,
                    activated.path.clone(),
                    true,
                ));
            }
            _ => {
                return Err(FleetInstallActivationJournalError::Io {
                    path: activated.path.clone(),
                    source,
                });
            }
        }
    }
    let durable = load_fleet_install_activation_journal(
        root,
        identity.fleet.fleet.canonical_network_id,
        identity.fleet.fleet.fleet_id,
        identity.operation_id,
    )?;
    if durable != next {
        return Err(invalid(
            &activated.path,
            "published HostAuthorityCommitted journal differs from the transition record",
        ));
    }
    Ok(host_authority_committed_result(
        next,
        activated.path.clone(),
        true,
    ))
}

/// Observe one terminal journal only after its catalog row and receipt remain valid.
pub(super) fn observe_host_authority_committed(
    resolved: &ResolvedFleetInstallActivation,
    receipt_directory: &Path,
    catalog_entry: &FleetCatalogEntryV1,
) -> Result<HostAuthorityCommittedFleetInstallActivation, FleetInstallActivationJournalError> {
    if resolved.journal.phase != FleetInstallActivationPhase::HostAuthorityCommitted {
        return Err(
            FleetInstallActivationJournalError::InvalidHostAuthorityCommittedTransition {
                phase: resolved.journal.phase,
            },
        );
    }
    resolved
        .journal
        .committed_fleet_catalog_hash
        .ok_or(FleetInstallActivationJournalError::MissingCommittedFleetCatalogHash)?;
    validate_committed_catalog(&resolved.journal, catalog_entry)?;
    require_root_install_receipt(&resolved.journal, receipt_directory, catalog_entry)?;
    Ok(host_authority_committed_result(
        resolved.journal.clone(),
        resolved.path.clone(),
        false,
    ))
}

fn validate_committed_catalog(
    journal: &FleetInstallActivationJournal,
    entry: &FleetCatalogEntryV1,
) -> Result<(), FleetInstallActivationJournalError> {
    let root_canister =
        validate_activated_activation_record(&journal.activation).map_err(invalid_active)?;
    let identity = &journal.activation.identity;
    if entry.canonical_network_id != identity.fleet.fleet.canonical_network_id
        || entry.fleet_id != identity.fleet.fleet.fleet_id
        || entry.fleet_name != journal.fleet_name
        || entry.app != identity.fleet.app
        || entry.root_principal != root_canister.to_text()
    {
        return Err(FleetInstallActivationJournalError::CommittedFleetCatalogMismatch);
    }
    Ok(())
}

fn require_root_install_receipt(
    journal: &FleetInstallActivationJournal,
    receipt_directory: &Path,
    catalog_entry: &FleetCatalogEntryV1,
) -> Result<(), FleetInstallActivationJournalError> {
    let receipt = recover_root_install_receipt(
        receipt_directory,
        journal
            .root_install_receipt_hash
            .expect("validated active journal retains its root-install receipt hash"),
    )?;
    if receipt.activation_identity != journal.activation.identity
        || receipt.root_canister.to_text() != catalog_entry.root_principal
    {
        return Err(FleetInstallActivationJournalError::RootInstallReceiptIdentityMismatch);
    }
    Ok(())
}

const fn next_host_authority_committed_journal(
    mut activated: FleetInstallActivationJournal,
    catalog_hash: [u8; 32],
) -> FleetInstallActivationJournal {
    activated.sequence = activated
        .sequence
        .checked_add(1)
        .expect("validated CanistersActivated sequence three advances to four");
    activated.phase = FleetInstallActivationPhase::HostAuthorityCommitted;
    activated.committed_fleet_catalog_hash = Some(catalog_hash);
    activated
}

fn plan_fleet_install_activation_with_ids(
    request: &PlanFleetInstallActivationRequest<'_>,
    finalized_release_build: &FinalizedReleaseBuild,
    release_set_manifest_digest: [u8; 32],
    fleet_id: FleetId,
    operation_id: [u8; 32],
) -> Result<ResolvedFleetInstallActivation, FleetInstallActivationJournalError> {
    let canonical_network_id = request.canonical_network_id;
    let journal = FleetInstallActivationJournal {
        sequence: 0,
        phase: FleetInstallActivationPhase::Planned,
        fleet_name: request.fleet_name.clone(),
        release_build_plan_hash: finalized_release_build.plan_hash,
        release_set_manifest_digest,
        root_install_receipt_hash: None,
        activation: FleetActivationHostRecord {
            identity: FleetActivationIdentity {
                fleet: FleetBinding {
                    fleet: FleetKey {
                        canonical_network_id,
                        fleet_id,
                    },
                    app: request.app.clone(),
                },
                operation_id,
                release_build_id: finalized_release_build.record.release_build_id,
            },
            cascade_manifest: None,
            credential: None,
            credential_manifest: None,
            canisters: Vec::new(),
        },
        committed_fleet_catalog_hash: None,
    };
    let path = fleet_install_activation_journal_path(
        request.root,
        canonical_network_id,
        fleet_id,
        operation_id,
    );
    let bytes = encode_journal(&journal)?;
    create_new_bytes_with_parents(&path, &bytes).map_err(|source| {
        FleetInstallActivationJournalError::Io {
            path: path.clone(),
            source,
        }
    })?;
    let observed = load_fleet_install_activation_journal(
        request.root,
        canonical_network_id,
        fleet_id,
        operation_id,
    )?;
    if observed != journal {
        return Err(invalid(
            &path,
            "published journal differs from the planned record",
        ));
    }
    let journal_hash = fleet_install_activation_journal_hash(&journal);
    Ok(ResolvedFleetInstallActivation {
        journal,
        journal_hash,
        path,
        created: true,
    })
}

struct DiscoveredFleetInstallActivation {
    journal: FleetInstallActivationJournal,
    path: PathBuf,
}

#[derive(Default)]
struct FleetInstallActivationDiscovery {
    active: Option<DiscoveredFleetInstallActivation>,
    completed: Vec<DiscoveredFleetInstallActivation>,
}

fn discover_fleet_install_activation(
    root: &Path,
    canonical_network_id: CanonicalNetworkId,
    fleet_name: &FleetName,
) -> Result<FleetInstallActivationDiscovery, FleetInstallActivationJournalError> {
    let network_directory = fleet_install_activation_network_directory(root, canonical_network_id);
    let mut fleet_ids = BTreeMap::new();
    let mut matching_active = Vec::new();
    let mut completed = Vec::new();

    for fleet_entry in canonical_directory_entries(&network_directory, true)? {
        let fleet_path = fleet_entry.path();
        let fleet_file_name = fleet_entry.file_name();
        let fleet_text = canonical_entry_text(&fleet_path, &fleet_file_name)?;
        let fleet_id = fleet_text.parse().map_err(|error| {
            invalid_directory(
                &fleet_path,
                format!("Fleet ID directory name is invalid: {error}"),
            )
        })?;

        for operation_entry in canonical_directory_entries(&fleet_path, false)? {
            let operation_path = operation_entry.path();
            let operation_file_name = operation_entry.file_name();
            let operation_text = canonical_entry_text(&operation_path, &operation_file_name)?;
            let operation_id = parse_operation_id(operation_text).ok_or_else(|| {
                invalid_directory(
                    &operation_path,
                    "operation ID directory name must be exactly 64 lowercase hexadecimal characters",
                )
            })?;
            let journal = match load_fleet_install_activation_journal(
                root,
                canonical_network_id,
                fleet_id,
                operation_id,
            ) {
                Ok(journal) => journal,
                Err(FleetInstallActivationJournalError::Missing { .. }) => continue,
                Err(error) => return Err(error),
            };
            let journal_path = fleet_install_activation_journal_path(
                root,
                canonical_network_id,
                fleet_id,
                operation_id,
            );
            if journal.phase == FleetInstallActivationPhase::HostAuthorityCommitted {
                if journal.fleet_name == *fleet_name {
                    completed.push(DiscoveredFleetInstallActivation {
                        journal,
                        path: journal_path,
                    });
                }
                continue;
            }
            if let Some(first) = fleet_ids.insert(fleet_id, journal_path.clone()) {
                return Err(
                    FleetInstallActivationJournalError::CompetingFleetIdAuthorities {
                        fleet_id,
                        first,
                        second: journal_path,
                    },
                );
            }
            if journal.fleet_name == *fleet_name {
                matching_active.push(DiscoveredFleetInstallActivation {
                    journal,
                    path: journal_path,
                });
            }
        }
    }

    let active = match matching_active.as_slice() {
        [] => None,
        [_] => matching_active.pop(),
        [first, second, ..] => Err(
            FleetInstallActivationJournalError::CompetingFleetNameAuthorities {
                fleet_name: fleet_name.clone(),
                first: first.path.clone(),
                second: second.path.clone(),
            },
        )?,
    };
    Ok(FleetInstallActivationDiscovery { active, completed })
}

fn resolve_discovered_activation(
    request: &PlanFleetInstallActivationRequest<'_>,
    finalized: &FinalizedReleaseBuild,
    release_set_manifest_digest: [u8; 32],
    existing: DiscoveredFleetInstallActivation,
) -> Result<ResolvedFleetInstallActivation, FleetInstallActivationJournalError> {
    let identity = &existing.journal.activation.identity;
    if identity.fleet.app != request.app {
        return Err(FleetInstallActivationJournalError::ExistingAppMismatch {
            fleet_name: request.fleet_name.clone(),
            existing_app: identity.fleet.app.clone(),
            requested_app: request.app.clone(),
            path: existing.path,
        });
    }
    if !discovered_activation_matches(&existing, request, finalized, release_set_manifest_digest) {
        return Err(
            FleetInstallActivationJournalError::ExistingReleaseBuildMismatch {
                fleet_name: request.fleet_name.clone(),
                path: existing.path,
            },
        );
    }
    Ok(resolved_discovered_activation(existing))
}

fn discovered_activation_matches(
    existing: &DiscoveredFleetInstallActivation,
    request: &PlanFleetInstallActivationRequest<'_>,
    finalized: &FinalizedReleaseBuild,
    release_set_manifest_digest: [u8; 32],
) -> bool {
    let identity = &existing.journal.activation.identity;
    identity.fleet.app == request.app
        && identity.release_build_id == finalized.record.release_build_id
        && existing.journal.release_build_plan_hash == finalized.plan_hash
        && existing.journal.release_set_manifest_digest == release_set_manifest_digest
}

fn resolved_discovered_activation(
    existing: DiscoveredFleetInstallActivation,
) -> ResolvedFleetInstallActivation {
    ResolvedFleetInstallActivation {
        journal_hash: fleet_install_activation_journal_hash(&existing.journal),
        journal: existing.journal,
        path: existing.path,
        created: false,
    }
}

fn fleet_install_activation_network_directory(
    root: &Path,
    canonical_network_id: CanonicalNetworkId,
) -> PathBuf {
    root.join(".canic")
        .join("recovery")
        .join("fleet-install-activations")
        .join(canonical_network_id.to_string())
}

fn fleet_install_activation_lock_path(
    root: &Path,
    canonical_network_id: CanonicalNetworkId,
    fleet_name: &FleetName,
) -> PathBuf {
    root.join(".canic")
        .join("recovery")
        .join("fleet-install-activation-locks")
        .join(canonical_network_id.to_string())
        .join(format!("{fleet_name}.lock"))
}

fn lock_fleet_install_activation(
    root: &Path,
    canonical_network_id: CanonicalNetworkId,
    fleet_name: &FleetName,
) -> Result<fs::File, FleetInstallActivationJournalError> {
    let path = fleet_install_activation_lock_path(root, canonical_network_id, fleet_name);
    lock_regular_file_with_parents(&path).map_err(|error| match error {
        RegularFileLockError::NotRegular => {
            FleetInstallActivationJournalError::UnsafeFile { path: path.clone() }
        }
        RegularFileLockError::Io(source) => FleetInstallActivationJournalError::Io {
            path: path.clone(),
            source,
        },
        #[cfg(windows)]
        RegularFileLockError::UnsupportedPlatform => FleetInstallActivationJournalError::Io {
            path,
            source: io::Error::new(
                io::ErrorKind::Unsupported,
                "Fleet install activation locking is unsupported on Windows",
            ),
        },
    })
}

fn canonical_directory_entries(
    path: &Path,
    missing_is_empty: bool,
) -> Result<Vec<fs::DirEntry>, FleetInstallActivationJournalError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(source) if missing_is_empty && source.kind() == io::ErrorKind::NotFound => {
            return Ok(Vec::new());
        }
        Err(source) => {
            return Err(FleetInstallActivationJournalError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    if !metadata.file_type().is_dir() {
        return Err(FleetInstallActivationJournalError::UnsafeDirectoryEntry {
            path: path.to_path_buf(),
        });
    }

    let entries = fs::read_dir(path).map_err(|source| FleetInstallActivationJournalError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut entries = entries
        .map(|entry| {
            entry.map_err(|source| FleetInstallActivationJournalError::Io {
                path: path.to_path_buf(),
                source,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in &entries {
        let entry_path = entry.path();
        let file_type =
            entry
                .file_type()
                .map_err(|source| FleetInstallActivationJournalError::Io {
                    path: entry_path.clone(),
                    source,
                })?;
        if !file_type.is_dir() {
            return Err(FleetInstallActivationJournalError::UnsafeDirectoryEntry {
                path: entry_path,
            });
        }
    }
    Ok(entries)
}

fn canonical_entry_text<'a>(
    path: &Path,
    name: &'a std::ffi::OsStr,
) -> Result<&'a str, FleetInstallActivationJournalError> {
    name.to_str()
        .ok_or_else(|| invalid_directory(path, "directory name is not valid UTF-8"))
}

fn parse_operation_id(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return None;
    }
    let mut bytes = [0; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        bytes[index] = (decode_hex_nibble(pair[0]) << 4) | decode_hex_nibble(pair[1]);
    }
    Some(bytes)
}

fn decode_hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => unreachable!("hexadecimal operation identity was validated before decoding"),
    }
}

fn encode_journal(
    journal: &FleetInstallActivationJournal,
) -> Result<Vec<u8>, FleetInstallActivationJournalError> {
    let path = Path::new("<candidate Fleet install activation journal>");
    validate_journal(path, journal)?;
    let bytes = encode_value(&Value::Array(vec![
        integer(journal.sequence),
        integer(phase_discriminant(journal.phase)),
        Value::Text(journal.fleet_name.to_string()),
        digest(journal.release_build_plan_hash),
        digest(journal.release_set_manifest_digest),
        optional_digest(journal.root_install_receipt_hash),
        encode_activation_host_record(&journal.activation),
        optional_digest(journal.committed_fleet_catalog_hash),
    ]));
    if bytes.len() > MAX_FLEET_INSTALL_ACTIVATION_JOURNAL_BYTES {
        return Err(invalid(
            path,
            "canonical Fleet install activation journal exceeds its byte bound",
        ));
    }
    Ok(bytes)
}

fn decode_journal(
    path: &Path,
    bytes: &[u8],
) -> Result<FleetInstallActivationJournal, FleetInstallActivationJournalError> {
    if bytes.len() > MAX_FLEET_INSTALL_ACTIVATION_JOURNAL_BYTES {
        return Err(invalid(
            path,
            "Fleet install activation journal exceeds its byte bound",
        ));
    }
    let value: Value =
        ciborium::de::from_reader(bytes).map_err(|error| invalid(path, error.to_string()))?;
    let fields = exact_array(path, value, 8, "journal")?;
    let journal = FleetInstallActivationJournal {
        sequence: exact_u64(path, &fields[0], "sequence")?,
        phase: decode_phase(path, exact_u64(path, &fields[1], "phase")?)?,
        fleet_name: exact_text(path, &fields[2], "fleet_name")?
            .parse()
            .map_err(|error| invalid(path, format!("invalid fleet_name: {error}")))?,
        release_build_plan_hash: exact_digest(path, &fields[3], "release_build_plan_hash")?,
        release_set_manifest_digest: exact_digest(path, &fields[4], "release_set_manifest_digest")?,
        root_install_receipt_hash: exact_optional_digest(
            path,
            &fields[5],
            "root_install_receipt_hash",
        )?,
        activation: decode_activation_host_record(path, &fields[6])?,
        committed_fleet_catalog_hash: exact_optional_digest(
            path,
            &fields[7],
            "committed_fleet_catalog_hash",
        )?,
    };
    validate_journal(path, &journal)?;
    if encode_journal(&journal)? != bytes {
        return Err(invalid(path, "CBOR bytes are not canonical"));
    }
    Ok(journal)
}

fn validate_journal(
    path: &Path,
    journal: &FleetInstallActivationJournal,
) -> Result<(), FleetInstallActivationJournalError> {
    validate_app(&journal.activation.identity.fleet.app)?;
    match journal.phase {
        FleetInstallActivationPhase::Planned
            if journal.sequence == 0
                && journal.root_install_receipt_hash.is_none()
                && journal.committed_fleet_catalog_hash.is_none()
                && is_initial_activation(&journal.activation) =>
        {
            Ok(())
        }
        FleetInstallActivationPhase::RootInstalled
            if journal.sequence == 1
                && journal.root_install_receipt_hash.is_some()
                && journal.committed_fleet_catalog_hash.is_none()
                && is_initial_activation(&journal.activation) =>
        {
            Ok(())
        }
        FleetInstallActivationPhase::CanistersPrepared
            if journal.sequence == 2
                && journal.root_install_receipt_hash.is_some()
                && journal.committed_fleet_catalog_hash.is_none() =>
        {
            validate_prepared_activation_record(&journal.activation)
                .map(|_| ())
                .map_err(|reason| invalid(path, reason))
        }
        FleetInstallActivationPhase::CanistersActivated
            if journal.sequence == 3
                && journal.root_install_receipt_hash.is_some()
                && journal.committed_fleet_catalog_hash.is_none() =>
        {
            validate_activated_activation_record(&journal.activation)
                .map(|_| ())
                .map_err(|reason| invalid(path, reason))
        }
        FleetInstallActivationPhase::HostAuthorityCommitted
            if journal.sequence == 4
                && journal.root_install_receipt_hash.is_some()
                && journal.committed_fleet_catalog_hash.is_some() =>
        {
            validate_activated_activation_record(&journal.activation)
                .map(|_| ())
                .map_err(|reason| invalid(path, reason))
        }
        FleetInstallActivationPhase::Planned => Err(invalid(
            path,
            "Planned requires sequence 0 and no later-phase evidence",
        )),
        FleetInstallActivationPhase::RootInstalled => Err(invalid(
            path,
            "RootInstalled requires sequence 1, one root-install receipt and no later-phase evidence",
        )),
        FleetInstallActivationPhase::CanistersPrepared => Err(invalid(
            path,
            "CanistersPrepared requires sequence 2, one root-install receipt, complete Prepared evidence and no catalog hash",
        )),
        FleetInstallActivationPhase::CanistersActivated => Err(invalid(
            path,
            "CanistersActivated requires sequence 3, one root-install receipt, complete Active evidence and no catalog hash",
        )),
        FleetInstallActivationPhase::HostAuthorityCommitted => Err(invalid(
            path,
            "HostAuthorityCommitted requires sequence 4, one root-install receipt, complete Active evidence and a catalog hash",
        )),
    }
}

fn validate_prepared_activation_record(
    record: &FleetActivationHostRecord,
) -> Result<Principal, String> {
    validate_app(&record.identity.fleet.app).map_err(|error| error.to_string())?;
    let authority = validate_prepared_manifest_authority(record)?;
    let inventory = validate_prepared_canister_inventory(
        record,
        authority.cascade_manifest,
        authority.credential_manifest,
    )?;
    let root_cascade = FleetCascadeActivationEvidence::Source {
        cascade_manifest_hash: authority.cascade_manifest_hash,
    };
    let expected_root_hash = FleetActivationApi::activation_evidence_hash(
        &record.identity,
        &root_cascade,
        authority.credential,
    )
    .map_err(|error| format!("invalid Prepared root activation evidence: {error}"))?;
    if inventory
        .canisters
        .get(&inventory.root_canister)
        .copied()
        .flatten()
        != Some(expected_root_hash)
    {
        return Err(
            "Prepared root activation evidence hash does not match the canonical record"
                .to_string(),
        );
    }
    for entry in authority.cascade_manifest {
        if inventory
            .canisters
            .get(&entry.principal)
            .copied()
            .flatten()
            .is_some()
        {
            return Err(format!(
                "Prepared child {} must not claim activation evidence before resume",
                entry.principal
            ));
        }
    }
    if encode_value(&encode_activation_host_record(record)).len()
        > MAX_FLEET_ACTIVATION_HOST_RECORD_BYTES
    {
        return Err("Prepared activation host record exceeds its byte bound".to_string());
    }
    Ok(inventory.root_canister)
}

fn validate_activated_activation_record(
    record: &FleetActivationHostRecord,
) -> Result<Principal, String> {
    validate_app(&record.identity.fleet.app).map_err(|error| error.to_string())?;
    let authority = validate_prepared_manifest_authority(record)?;
    let inventory = validate_prepared_canister_inventory(
        record,
        authority.cascade_manifest,
        authority.credential_manifest,
    )?;
    let root_cascade = FleetCascadeActivationEvidence::Source {
        cascade_manifest_hash: authority.cascade_manifest_hash,
    };
    let expected_root_hash = FleetActivationApi::activation_evidence_hash(
        &record.identity,
        &root_cascade,
        authority.credential,
    )
    .map_err(|error| format!("invalid active root activation evidence: {error}"))?;
    if inventory
        .canisters
        .get(&inventory.root_canister)
        .copied()
        .flatten()
        != Some(expected_root_hash)
    {
        return Err(
            "Active root activation evidence hash does not match the canonical record".to_string(),
        );
    }
    for entry in authority.cascade_manifest {
        let cascade = FleetCascadeActivationEvidence::Applied {
            state_snapshot_hash: entry.state_snapshot_hash,
            topology_snapshot_hash: entry.topology_snapshot_hash,
        };
        let expected_hash = FleetActivationApi::activation_evidence_hash(
            &record.identity,
            &cascade,
            authority.credential,
        )
        .map_err(|error| {
            format!(
                "invalid active child activation evidence for {}: {error}",
                entry.principal
            )
        })?;
        if inventory.canisters.get(&entry.principal).copied().flatten() != Some(expected_hash) {
            return Err(format!(
                "Active child {} activation evidence hash does not match the canonical record",
                entry.principal
            ));
        }
    }
    if encode_value(&encode_activation_host_record(record)).len()
        > MAX_FLEET_ACTIVATION_HOST_RECORD_BYTES
    {
        return Err("Active activation host record exceeds its byte bound".to_string());
    }
    Ok(inventory.root_canister)
}

struct ValidatedPreparedManifestAuthority<'a> {
    cascade_manifest: &'a [FleetCascadeManifestEntry],
    credential: FleetCredentialGenerationRef,
    credential_manifest: &'a FleetCredentialManifest,
    cascade_manifest_hash: [u8; 32],
}

struct ValidatedPreparedCanisterInventory {
    root_canister: Principal,
    canisters: BTreeMap<Principal, Option<[u8; 32]>>,
}

fn validate_prepared_manifest_authority(
    record: &FleetActivationHostRecord,
) -> Result<ValidatedPreparedManifestAuthority<'_>, String> {
    let cascade_manifest = record
        .cascade_manifest
        .as_ref()
        .ok_or_else(|| "Prepared activation is missing its cascade manifest".to_string())?;
    let credential = record
        .credential
        .ok_or_else(|| "Prepared activation is missing its credential generation".to_string())?;
    let credential_manifest = record
        .credential_manifest
        .as_ref()
        .ok_or_else(|| "Prepared activation is missing its credential manifest".to_string())?;
    if record.canisters.is_empty()
        || record.canisters.len() > MAX_FLEET_ACTIVATION_CANISTERS
        || cascade_manifest
            .len()
            .checked_add(1)
            .is_none_or(|count| count != record.canisters.len())
    {
        return Err(
            "Prepared activation must contain exactly one root plus every cascade child"
                .to_string(),
        );
    }
    if credential_manifest.entries.len() > MAX_FLEET_CREDENTIAL_MANIFEST_ENTRIES {
        return Err("Prepared credential manifest exceeds its entry bound".to_string());
    }
    if credential_manifest.fleet != record.identity.fleet.fleet
        || credential_manifest.activation_id != record.identity.operation_id
        || credential_manifest.generation != credential.generation
    {
        return Err(
            "Prepared credential manifest does not match its activation identity and generation"
                .to_string(),
        );
    }
    let cascade_manifest_hash = FleetActivationApi::cascade_manifest_hash(cascade_manifest)
        .map_err(|error| format!("invalid Prepared cascade manifest: {error}"))?;
    let credential_manifest_hash =
        FleetActivationApi::credential_manifest_hash(credential_manifest)
            .map_err(|error| format!("invalid Prepared credential manifest: {error}"))?;
    if credential_manifest_hash != credential.manifest_hash {
        return Err("Prepared credential generation hash does not match its manifest".to_string());
    }
    Ok(ValidatedPreparedManifestAuthority {
        cascade_manifest,
        credential,
        credential_manifest,
        cascade_manifest_hash,
    })
}

fn validate_prepared_canister_inventory(
    record: &FleetActivationHostRecord,
    cascade_manifest: &[FleetCascadeManifestEntry],
    credential_manifest: &FleetCredentialManifest,
) -> Result<ValidatedPreparedCanisterInventory, String> {
    if record
        .canisters
        .windows(2)
        .any(|entries| entries[0].principal.as_slice() >= entries[1].principal.as_slice())
    {
        return Err("Prepared Canister inventory must use strict raw-principal order".to_string());
    }

    let cascade_canisters = cascade_manifest
        .iter()
        .map(|entry| entry.principal)
        .collect::<BTreeSet<_>>();
    let host_canisters = record
        .canisters
        .iter()
        .map(|entry| (entry.principal, entry.activation_evidence_hash))
        .collect::<BTreeMap<_, _>>();
    if host_canisters.len() != record.canisters.len()
        || !cascade_canisters
            .iter()
            .all(|principal| host_canisters.contains_key(principal))
    {
        return Err(
            "Prepared Canister inventory does not exactly cover the cascade manifest".to_string(),
        );
    }
    let roots = host_canisters
        .keys()
        .copied()
        .filter(|principal| !cascade_canisters.contains(principal))
        .collect::<Vec<_>>();
    let [root_canister] = roots.as_slice() else {
        return Err(
            "Prepared Canister inventory must identify exactly one root outside the cascade manifest"
                .to_string(),
        );
    };
    if credential_manifest
        .entries
        .iter()
        .any(|entry| !host_canisters.contains_key(&entry.subject_canister))
    {
        return Err(
            "Prepared credential manifest contains a subject outside the Canister inventory"
                .to_string(),
        );
    }
    Ok(ValidatedPreparedCanisterInventory {
        root_canister: *root_canister,
        canisters: host_canisters,
    })
}

const fn is_initial_activation(record: &FleetActivationHostRecord) -> bool {
    record.cascade_manifest.is_none()
        && record.credential.is_none()
        && record.credential_manifest.is_none()
        && record.canisters.is_empty()
}

fn encode_activation_host_record(record: &FleetActivationHostRecord) -> Value {
    Value::Array(vec![
        encode_activation_identity(&record.identity),
        record
            .cascade_manifest
            .as_ref()
            .map_or(Value::Null, |value| {
                Value::Array(value.iter().map(encode_cascade_manifest_entry).collect())
            }),
        record
            .credential
            .map_or(Value::Null, encode_credential_generation),
        record
            .credential_manifest
            .as_ref()
            .map_or(Value::Null, encode_credential_manifest),
        Value::Array(
            record
                .canisters
                .iter()
                .map(encode_host_canister_evidence)
                .collect(),
        ),
    ])
}

fn decode_activation_host_record(
    path: &Path,
    value: &Value,
) -> Result<FleetActivationHostRecord, FleetInstallActivationJournalError> {
    let fields = exact_array_ref(path, value, 5, "activation")?;
    Ok(FleetActivationHostRecord {
        identity: decode_activation_identity(path, &fields[0])?,
        cascade_manifest: decode_optional_cascade_manifest(path, &fields[1])?,
        credential: decode_optional_credential_generation(path, &fields[2])?,
        credential_manifest: decode_optional_credential_manifest(path, &fields[3])?,
        canisters: decode_host_canister_evidence(path, &fields[4])?,
    })
}

fn encode_cascade_manifest_entry(entry: &FleetCascadeManifestEntry) -> Value {
    Value::Array(vec![
        principal(entry.principal),
        digest(entry.state_snapshot_hash),
        digest(entry.topology_snapshot_hash),
    ])
}

fn decode_optional_cascade_manifest(
    path: &Path,
    value: &Value,
) -> Result<Option<Vec<FleetCascadeManifestEntry>>, FleetInstallActivationJournalError> {
    if matches!(value, Value::Null) {
        return Ok(None);
    }
    let Value::Array(entries) = value else {
        return Err(invalid(path, "cascade_manifest must be null or an array"));
    };
    if entries.len() >= MAX_FLEET_ACTIVATION_CANISTERS {
        return Err(invalid(
            path,
            "cascade_manifest exceeds the Canister inventory bound",
        ));
    }
    entries
        .iter()
        .map(|entry| {
            let fields = exact_array_ref(path, entry, 3, "cascade_manifest entry")?;
            Ok(FleetCascadeManifestEntry {
                principal: exact_principal(path, &fields[0], "cascade principal")?,
                state_snapshot_hash: exact_digest(path, &fields[1], "state_snapshot_hash")?,
                topology_snapshot_hash: exact_digest(path, &fields[2], "topology_snapshot_hash")?,
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

fn encode_credential_generation(value: FleetCredentialGenerationRef) -> Value {
    Value::Array(vec![integer(value.generation), digest(value.manifest_hash)])
}

fn decode_optional_credential_generation(
    path: &Path,
    value: &Value,
) -> Result<Option<FleetCredentialGenerationRef>, FleetInstallActivationJournalError> {
    if matches!(value, Value::Null) {
        return Ok(None);
    }
    let fields = exact_array_ref(path, value, 2, "credential")?;
    Ok(Some(FleetCredentialGenerationRef {
        generation: exact_u64(path, &fields[0], "credential generation")?,
        manifest_hash: exact_digest(path, &fields[1], "credential manifest_hash")?,
    }))
}

fn encode_credential_manifest(value: &FleetCredentialManifest) -> Value {
    Value::Array(vec![
        encode_fleet_key(value.fleet),
        digest(value.activation_id),
        integer(value.generation),
        digest(value.root_policy_set_hash),
        digest(value.renewal_template_set_hash),
        Value::Array(
            value
                .entries
                .iter()
                .map(encode_credential_manifest_entry)
                .collect(),
        ),
    ])
}

fn decode_optional_credential_manifest(
    path: &Path,
    value: &Value,
) -> Result<Option<FleetCredentialManifest>, FleetInstallActivationJournalError> {
    if matches!(value, Value::Null) {
        return Ok(None);
    }
    let fields = exact_array_ref(path, value, 6, "credential_manifest")?;
    let Value::Array(entries) = &fields[5] else {
        return Err(invalid(
            path,
            "credential_manifest entries must be an array",
        ));
    };
    if entries.len() > MAX_FLEET_CREDENTIAL_MANIFEST_ENTRIES {
        return Err(invalid(path, "credential_manifest exceeds its entry bound"));
    }
    let entries = entries
        .iter()
        .map(|entry| {
            let fields = exact_array_ref(path, entry, 8, "credential_manifest entry")?;
            Ok::<_, FleetInstallActivationJournalError>(FleetCredentialManifestEntry {
                root_issuer: exact_principal(path, &fields[0], "root_issuer")?,
                subject_canister: exact_principal(path, &fields[1], "subject_canister")?,
                not_before_ns: exact_u64(path, &fields[2], "not_before_ns")?,
                expires_at_ns: exact_u64(path, &fields[3], "expires_at_ns")?,
                key_identity_hash: exact_digest(path, &fields[4], "key_identity_hash")?,
                cert_hash: exact_digest(path, &fields[5], "cert_hash")?,
                proof_hash: exact_digest(path, &fields[6], "proof_hash")?,
                bundle_hash: exact_digest(path, &fields[7], "bundle_hash")?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(FleetCredentialManifest {
        fleet: decode_fleet_key(path, &fields[0])?,
        activation_id: exact_digest(path, &fields[1], "credential activation_id")?,
        generation: exact_u64(path, &fields[2], "credential generation")?,
        root_policy_set_hash: exact_digest(path, &fields[3], "root_policy_set_hash")?,
        renewal_template_set_hash: exact_digest(path, &fields[4], "renewal_template_set_hash")?,
        entries,
    }))
}

fn encode_credential_manifest_entry(value: &FleetCredentialManifestEntry) -> Value {
    Value::Array(vec![
        principal(value.root_issuer),
        principal(value.subject_canister),
        integer(value.not_before_ns),
        integer(value.expires_at_ns),
        digest(value.key_identity_hash),
        digest(value.cert_hash),
        digest(value.proof_hash),
        digest(value.bundle_hash),
    ])
}

fn encode_host_canister_evidence(value: &FleetHostCanisterActivationEvidence) -> Value {
    Value::Array(vec![
        principal(value.principal),
        optional_digest(value.activation_evidence_hash),
    ])
}

fn decode_host_canister_evidence(
    path: &Path,
    value: &Value,
) -> Result<Vec<FleetHostCanisterActivationEvidence>, FleetInstallActivationJournalError> {
    let Value::Array(entries) = value else {
        return Err(invalid(path, "canisters must be an array"));
    };
    if entries.len() > MAX_FLEET_ACTIVATION_CANISTERS {
        return Err(invalid(
            path,
            "canisters exceeds the activation inventory bound",
        ));
    }
    entries
        .iter()
        .map(|entry| {
            let fields = exact_array_ref(path, entry, 2, "canister evidence")?;
            Ok(FleetHostCanisterActivationEvidence {
                principal: exact_principal(path, &fields[0], "canister principal")?,
                activation_evidence_hash: exact_optional_digest(
                    path,
                    &fields[1],
                    "activation_evidence_hash",
                )?,
            })
        })
        .collect()
}

fn encode_activation_identity(identity: &FleetActivationIdentity) -> Value {
    Value::Array(vec![
        Value::Array(vec![
            encode_fleet_key(identity.fleet.fleet),
            Value::Bytes(identity.fleet.app.as_str().as_bytes().to_vec()),
        ]),
        digest(identity.operation_id),
        digest(*identity.release_build_id.as_bytes()),
    ])
}

fn decode_activation_identity(
    path: &Path,
    value: &Value,
) -> Result<FleetActivationIdentity, FleetInstallActivationJournalError> {
    let fields = exact_array_ref(path, value, 3, "activation identity")?;
    let binding = exact_array_ref(path, &fields[0], 2, "Fleet binding")?;
    let key = exact_array_ref(path, &binding[0], 2, "Fleet key")?;
    let app = AppId::owned(exact_utf8_bytes(path, &binding[1], "app")?.to_string());
    validate_app(&app)?;
    Ok(FleetActivationIdentity {
        fleet: FleetBinding {
            fleet: decode_fleet_key_fields(path, key)?,
            app,
        },
        operation_id: exact_digest(path, &fields[1], "operation_id")?,
        release_build_id: id_from_digest(
            exact_digest(path, &fields[2], "release_build_id")?,
            "release_build_id",
            path,
        )?,
    })
}

fn encode_fleet_key(value: FleetKey) -> Value {
    Value::Array(vec![
        digest(*value.canonical_network_id.as_bytes()),
        digest(*value.fleet_id.as_bytes()),
    ])
}

fn decode_fleet_key(
    path: &Path,
    value: &Value,
) -> Result<FleetKey, FleetInstallActivationJournalError> {
    let fields = exact_array_ref(path, value, 2, "Fleet key")?;
    decode_fleet_key_fields(path, fields)
}

fn decode_fleet_key_fields(
    path: &Path,
    fields: &[Value],
) -> Result<FleetKey, FleetInstallActivationJournalError> {
    Ok(FleetKey {
        canonical_network_id: id_from_digest(
            exact_digest(path, &fields[0], "canonical_network_id")?,
            "canonical_network_id",
            path,
        )?,
        fleet_id: FleetId::from_generated_bytes(exact_digest(path, &fields[1], "fleet_id")?),
    })
}

fn decode_phase(
    path: &Path,
    discriminant: u64,
) -> Result<FleetInstallActivationPhase, FleetInstallActivationJournalError> {
    match discriminant {
        0 => Ok(FleetInstallActivationPhase::Planned),
        1 => Ok(FleetInstallActivationPhase::RootInstalled),
        2 => Ok(FleetInstallActivationPhase::CanistersPrepared),
        3 => Ok(FleetInstallActivationPhase::CanistersActivated),
        4 => Ok(FleetInstallActivationPhase::HostAuthorityCommitted),
        _ => Err(invalid(path, "phase has an unknown discriminant")),
    }
}

const fn phase_discriminant(phase: FleetInstallActivationPhase) -> u64 {
    match phase {
        FleetInstallActivationPhase::Planned => 0,
        FleetInstallActivationPhase::RootInstalled => 1,
        FleetInstallActivationPhase::CanistersPrepared => 2,
        FleetInstallActivationPhase::CanistersActivated => 3,
        FleetInstallActivationPhase::HostAuthorityCommitted => 4,
    }
}

fn read_journal_bytes(path: &Path) -> Result<Vec<u8>, FleetInstallActivationJournalError> {
    match read_optional_regular_bytes(path) {
        Ok(Some(bytes)) => Ok(bytes),
        Ok(None) => Err(FleetInstallActivationJournalError::Missing {
            path: path.to_path_buf(),
        }),
        Err(RegularFileReadError::NotRegular) => {
            Err(FleetInstallActivationJournalError::UnsafeFile {
                path: path.to_path_buf(),
            })
        }
        Err(RegularFileReadError::Io(source)) => Err(FleetInstallActivationJournalError::Io {
            path: path.to_path_buf(),
            source,
        }),
        #[cfg(not(unix))]
        Err(RegularFileReadError::UnsupportedPlatform) => {
            Err(FleetInstallActivationJournalError::Io {
                path: path.to_path_buf(),
                source: io::Error::new(
                    io::ErrorKind::Unsupported,
                    "no-follow activation-journal reads are unsupported on this platform",
                ),
            })
        }
    }
}

fn read_root_install_receipt_bytes(
    path: &Path,
) -> Result<Vec<u8>, FleetInstallActivationJournalError> {
    match read_optional_regular_bytes(path) {
        Ok(Some(bytes)) => Ok(bytes),
        Ok(None) => Err(
            FleetInstallActivationJournalError::MissingRootInstallReceipt {
                path: path.to_path_buf(),
            },
        ),
        Err(RegularFileReadError::NotRegular) => Err(
            FleetInstallActivationJournalError::UnsafeRootInstallReceipt {
                path: path.to_path_buf(),
            },
        ),
        Err(RegularFileReadError::Io(source)) => Err(FleetInstallActivationJournalError::Io {
            path: path.to_path_buf(),
            source,
        }),
        #[cfg(not(unix))]
        Err(RegularFileReadError::UnsupportedPlatform) => {
            Err(FleetInstallActivationJournalError::Io {
                path: path.to_path_buf(),
                source: io::Error::new(
                    io::ErrorKind::Unsupported,
                    "no-follow root-install receipt reads are unsupported on this platform",
                ),
            })
        }
    }
}

fn exact_receipt_evidence<'a>(
    path: &Path,
    evidence: &'a [String],
    prefix: &str,
) -> Result<&'a str, FleetInstallActivationJournalError> {
    let mut matches = evidence
        .iter()
        .filter_map(|entry| entry.strip_prefix(prefix));
    let Some(value) = matches.next() else {
        return Err(invalid_root_install_receipt(
            path,
            format!("missing {prefix} evidence"),
        ));
    };
    if matches.next().is_some() {
        return Err(invalid_root_install_receipt(
            path,
            format!("duplicate {prefix} evidence"),
        ));
    }
    Ok(value)
}

fn root_install_activation_identity(
    path: &Path,
    evidence: &[String],
) -> Result<FleetActivationIdentity, FleetInstallActivationJournalError> {
    let canonical_network_id = exact_receipt_evidence(path, evidence, "canonical_network_id:")?
        .parse::<CanonicalNetworkId>()
        .map_err(|error| {
            invalid_root_install_receipt(path, format!("canonical_network_id is invalid: {error}"))
        })?;
    let app = AppId::from(exact_receipt_evidence(path, evidence, "app:")?);
    validate_app(&app)
        .map_err(|error| invalid_root_install_receipt(path, format!("app is invalid: {error}")))?;
    let fleet_id = exact_receipt_evidence(path, evidence, "fleet_id:")?
        .parse::<FleetId>()
        .map_err(|error| {
            invalid_root_install_receipt(path, format!("fleet_id is invalid: {error}"))
        })?;
    let operation_id = parse_receipt_digest(
        path,
        exact_receipt_evidence(path, evidence, "activation_operation_id:")?,
        "activation_operation_id",
    )?;
    let release_build_id = exact_receipt_evidence(path, evidence, "release_build_id:")?
        .parse::<ReleaseBuildId>()
        .map_err(|error| {
            invalid_root_install_receipt(path, format!("release_build_id is invalid: {error}"))
        })?;
    if exact_receipt_evidence(path, evidence, "fleet_activation_phase:")? != "prepared" {
        return Err(invalid_root_install_receipt(
            path,
            "fleet_activation_phase must be prepared",
        ));
    }
    Ok(FleetActivationIdentity {
        fleet: FleetBinding {
            fleet: FleetKey {
                canonical_network_id,
                fleet_id,
            },
            app,
        },
        operation_id,
        release_build_id,
    })
}

fn parse_receipt_digest(
    path: &Path,
    value: &str,
    field: &str,
) -> Result<[u8; 32], FleetInstallActivationJournalError> {
    parse_operation_id(value).ok_or_else(|| {
        invalid_root_install_receipt(
            path,
            format!("{field} must be exactly 64 lowercase hexadecimal characters"),
        )
    })
}

fn root_installed_result(
    journal: FleetInstallActivationJournal,
    path: PathBuf,
    advanced: bool,
) -> RootInstalledFleetInstallActivation {
    RootInstalledFleetInstallActivation {
        journal_hash: fleet_install_activation_journal_hash(&journal),
        journal,
        path,
        advanced,
    }
}

fn canisters_prepared_result(
    journal: FleetInstallActivationJournal,
    path: PathBuf,
    advanced: bool,
) -> CanistersPreparedFleetInstallActivation {
    CanistersPreparedFleetInstallActivation {
        journal_hash: fleet_install_activation_journal_hash(&journal),
        journal,
        path,
        advanced,
    }
}

fn canisters_activated_result(
    journal: FleetInstallActivationJournal,
    path: PathBuf,
    advanced: bool,
) -> CanistersActivatedFleetInstallActivation {
    CanistersActivatedFleetInstallActivation {
        journal_hash: fleet_install_activation_journal_hash(&journal),
        journal,
        path,
        advanced,
    }
}

fn host_authority_committed_result(
    journal: FleetInstallActivationJournal,
    path: PathBuf,
    advanced: bool,
) -> HostAuthorityCommittedFleetInstallActivation {
    HostAuthorityCommittedFleetInstallActivation {
        journal_hash: fleet_install_activation_journal_hash(&journal),
        journal,
        path,
        advanced,
    }
}

fn validate_app(app: &AppId) -> Result<(), FleetInstallActivationJournalError> {
    let value = app.as_str();
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(FleetInstallActivationJournalError::InvalidApp {
            app: app.to_string(),
            reason: "must use only ASCII letters, numbers, '-' or '_'".to_string(),
        });
    }
    Ok(())
}

fn random_identity_bytes() -> Result<[u8; 32], FleetInstallActivationJournalError> {
    random_bytes_32().map_err(|error| match error {
        EntropyError::Io(source) => FleetInstallActivationJournalError::Io {
            path: PathBuf::from("<operating-system random source>"),
            source,
        },
        EntropyError::ShortRead { actual } => {
            FleetInstallActivationJournalError::ShortRandomRead { actual }
        }
    })
}

fn exact_array(
    path: &Path,
    value: Value,
    len: usize,
    field: &str,
) -> Result<Vec<Value>, FleetInstallActivationJournalError> {
    let Value::Array(values) = value else {
        return Err(invalid(path, format!("{field} must be an array")));
    };
    if values.len() != len {
        return Err(invalid(
            path,
            format!("{field} must contain exactly {len} values"),
        ));
    }
    Ok(values)
}

fn exact_array_ref<'a>(
    path: &Path,
    value: &'a Value,
    len: usize,
    field: &str,
) -> Result<&'a [Value], FleetInstallActivationJournalError> {
    let Value::Array(values) = value else {
        return Err(invalid(path, format!("{field} must be an array")));
    };
    if values.len() != len {
        return Err(invalid(
            path,
            format!("{field} must contain exactly {len} values"),
        ));
    }
    Ok(values)
}

fn exact_u64(
    path: &Path,
    value: &Value,
    field: &str,
) -> Result<u64, FleetInstallActivationJournalError> {
    value
        .as_integer()
        .and_then(|value| u64::try_from(value).ok())
        .ok_or_else(|| invalid(path, format!("{field} must be an unsigned integer")))
}

fn exact_text<'a>(
    path: &Path,
    value: &'a Value,
    field: &str,
) -> Result<&'a str, FleetInstallActivationJournalError> {
    value
        .as_text()
        .ok_or_else(|| invalid(path, format!("{field} must be text")))
}

fn exact_utf8_bytes<'a>(
    path: &Path,
    value: &'a Value,
    field: &str,
) -> Result<&'a str, FleetInstallActivationJournalError> {
    let Value::Bytes(bytes) = value else {
        return Err(invalid(path, format!("{field} must be a byte string")));
    };
    std::str::from_utf8(bytes)
        .map_err(|error| invalid(path, format!("{field} must contain UTF-8 bytes: {error}")))
}

fn exact_digest(
    path: &Path,
    value: &Value,
    field: &str,
) -> Result<[u8; 32], FleetInstallActivationJournalError> {
    let Value::Bytes(bytes) = value else {
        return Err(invalid(path, format!("{field} must be a byte string")));
    };
    bytes
        .as_slice()
        .try_into()
        .map_err(|_| invalid(path, format!("{field} must contain exactly 32 bytes")))
}

fn exact_principal(
    path: &Path,
    value: &Value,
    field: &str,
) -> Result<Principal, FleetInstallActivationJournalError> {
    let Value::Bytes(bytes) = value else {
        return Err(invalid(path, format!("{field} must be a byte string")));
    };
    Principal::try_from(bytes.as_slice())
        .map_err(|error| invalid(path, format!("{field} is invalid: {error}")))
}

fn exact_optional_digest(
    path: &Path,
    value: &Value,
    field: &str,
) -> Result<Option<[u8; 32]>, FleetInstallActivationJournalError> {
    if matches!(value, Value::Null) {
        Ok(None)
    } else {
        exact_digest(path, value, field).map(Some)
    }
}

fn id_from_digest<T>(
    bytes: [u8; 32],
    field: &str,
    path: &Path,
) -> Result<T, FleetInstallActivationJournalError>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    hex_digest(bytes)
        .parse()
        .map_err(|error| invalid(path, format!("{field} is invalid: {error}")))
}

fn encode_value(value: &Value) -> Vec<u8> {
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(value, &mut bytes)
        .expect("serializing a validated activation-journal CBOR value cannot fail");
    bytes
}

fn integer(value: u64) -> Value {
    Value::Integer(value.into())
}

fn digest(value: [u8; 32]) -> Value {
    Value::Bytes(value.to_vec())
}

fn principal(value: Principal) -> Value {
    Value::Bytes(value.as_slice().to_vec())
}

fn optional_digest(value: Option<[u8; 32]>) -> Value {
    value.map_or(Value::Null, digest)
}

fn hex_digest(bytes: [u8; 32]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(64), |mut text, byte| {
            use std::fmt::Write as _;
            write!(text, "{byte:02x}").expect("writing to String cannot fail");
            text
        })
}

fn domain_hash(domain: &[u8], bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(
        u64::try_from(bytes.len())
            .expect("host evidence fits u64")
            .to_be_bytes(),
    );
    hasher.update(bytes);
    hasher.finalize().into()
}

fn invalid(path: &Path, reason: impl Into<String>) -> FleetInstallActivationJournalError {
    FleetInstallActivationJournalError::InvalidDocument {
        path: path.to_path_buf(),
        reason: reason.into(),
    }
}

fn invalid_prepared(reason: impl Into<String>) -> FleetInstallActivationJournalError {
    FleetInstallActivationJournalError::InvalidPreparedActivationEvidence {
        reason: reason.into(),
    }
}

fn invalid_active(reason: impl Into<String>) -> FleetInstallActivationJournalError {
    FleetInstallActivationJournalError::InvalidActivatedActivationEvidence {
        reason: reason.into(),
    }
}

fn invalid_directory(path: &Path, reason: impl Into<String>) -> FleetInstallActivationJournalError {
    FleetInstallActivationJournalError::InvalidDirectory {
        path: path.to_path_buf(),
        reason: reason.into(),
    }
}

fn invalid_root_install_receipt(
    path: &Path,
    reason: impl Into<String>,
) -> FleetInstallActivationJournalError {
    FleetInstallActivationJournalError::InvalidRootInstallReceipt {
        path: path.to_path_buf(),
        reason: reason.into(),
    }
}
