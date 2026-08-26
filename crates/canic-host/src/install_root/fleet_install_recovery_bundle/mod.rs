//! Module: install_root::fleet_install_recovery_bundle
//!
//! Responsibility: retain and verify one path-confined, content-addressed copy of the complete
//! local authority needed to resume an incomplete fresh-Fleet installation.
//! Does not own: installation decisions, journal transitions, remote effects, or compatibility.
//! Boundary: bundle checkpoints copy only regular files below exact Canic-owned roots; import may
//! create missing identical local files but never overwrites conflicting state.

#[cfg(test)]
mod tests;

use super::fleet_install_session::{FleetInstallSession, session_path};
use super::{
    fleet_subnet_root_install_journal::{
        FleetSubnetRootInstallJournal, FleetSubnetRootInstallPhase,
        fleet_subnet_root_install_journal_path,
        validate_retained_fleet_subnet_root_install_journal_bytes,
    },
    fleet_subnet_root_repair::{
        repair_authority_path, repair_receipt_path, retained_artifact_path,
        retained_root_repair_operation_path, validate_recovery_bundle_repair_authority_bytes,
        validate_recovery_bundle_repair_operation_bytes,
        validate_recovery_bundle_repair_receipt_bytes,
    },
};
use crate::{
    durable_io::{
        BoundedRegularFileReadError, CanonicalJsonEncodeError, CanonicalJsonStyle,
        RegularFileLockError, RegularFileReadError, create_new_bytes_with_parents,
        encode_canonical_json, lock_regular_file_with_parents, read_optional_bounded_regular_bytes,
        write_bytes,
    },
    fleet_install_plan::{FleetInstallPlan, PersistedFleetInstallPlan},
    release_build::{release_build_plan_path, validate_retained_release_build_plan_bytes},
};
use canic_core::ids::ReleaseBuildId;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    env, fs, io,
    path::{Component, Path, PathBuf},
};
use thiserror::Error as ThisError;

const BUNDLE_SCHEMA_VERSION: u32 = 1;
const MANIFEST_FILE: &str = "manifest.json";
const MANIFEST_LOCK_FILE: &str = "manifest.lock";
const OBJECT_DIRECTORY: &str = "objects";
const MAX_MANIFEST_BYTES: usize = 8 * 1_024 * 1_024;
const MAX_BUNDLE_FILES: usize = 20_000;
const MAX_BUNDLE_FILE_BYTES: usize = 128 * 1_024 * 1_024;
const MAX_BUNDLE_TOTAL_BYTES: u64 = 4 * 1_024 * 1_024 * 1_024;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct FleetInstallRecoveryBundleV1 {
    schema_version: u32,
    canonical_network_id: String,
    fleet_name: String,
    fleet_id: String,
    app_id: String,
    release_build_id: String,
    fresh_fleet_plan_digest: String,
    fleet_install_plan_digest: [u8; 32],
    install_operation_id: [u8; 32],
    root_checkpoints: Vec<FleetInstallRecoveryBundleRootCheckpointV1>,
    files: Vec<FleetInstallRecoveryBundleFileV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct FleetInstallRecoveryBundleRootCheckpointV1 {
    placement_subnet: String,
    journal: Option<FleetInstallRecoveryBundleRootJournalV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct FleetInstallRecoveryBundleRootJournalV1 {
    sequence: u64,
    phase: FleetSubnetRootInstallPhase,
    sha256: [u8; 32],
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct FleetInstallRecoveryBundleFileV1 {
    logical_path: String,
    sha256: [u8; 32],
    size_bytes: u64,
}

/// Read-only verification result for one complete recovery bundle.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FleetInstallRecoveryBundleReportV1 {
    pub schema_version: u32,
    pub canonical_network_id: String,
    pub fleet_name: String,
    pub fleet_id: String,
    pub app_id: String,
    pub release_build_id: String,
    pub fresh_fleet_plan_digest: String,
    pub fleet_install_plan_digest: String,
    pub install_operation_id: String,
    pub file_count: usize,
    pub total_bytes: u64,
    pub bundle_path: PathBuf,
}

#[derive(Debug, ThisError)]
pub enum FleetInstallRecoveryBundleError {
    #[error("Canic operator-state root is unavailable; set CANIC_OPERATOR_STATE_ROOT or HOME")]
    OperatorStateUnavailable,

    #[error("recovery bundle contains an unsafe or unconfined path: {path}")]
    UnsafePath { path: PathBuf },

    #[error("recovery bundle source is missing: {path}")]
    Missing { path: PathBuf },

    #[error("recovery bundle source is not a regular no-follow file: {path}")]
    NotRegular { path: PathBuf },

    #[error("recovery bundle file exceeds its 128 MiB bound: {path}")]
    FileTooLarge { path: PathBuf },

    #[error("recovery bundle exceeds {maximum} files")]
    TooManyFiles { maximum: usize },

    #[error("recovery bundle exceeds its {maximum_bytes}-byte aggregate bound")]
    TooLarge { maximum_bytes: u64 },

    #[error("recovery bundle is incomplete or tampered at {path}")]
    DigestMismatch { path: PathBuf },

    #[error("recovery bundle conflicts with existing operator state: {path}")]
    ImportConflict { path: PathBuf },

    #[error("invalid recovery bundle manifest {path}: {reason}")]
    InvalidManifest { path: PathBuf, reason: String },

    #[error("failed to access recovery bundle path {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

/// Stable operator-owned checkpoint boundary shared by the Fleet install phase owners.
///
/// The boundary deliberately retains only immutable references. Every caller must complete a
/// checkpoint after publishing local authority and before starting its associated remote effect.
pub(super) enum FleetInstallRecoveryBundleCheckpoint<'a> {
    Persistent {
        icp_root: &'a Path,
        session: &'a FleetInstallSession,
        plan: &'a PersistedFleetInstallPlan,
    },
    #[cfg(test)]
    PersistentAt {
        icp_root: &'a Path,
        session: &'a FleetInstallSession,
        plan: &'a PersistedFleetInstallPlan,
        bundle_path: PathBuf,
    },
}

impl<'a> FleetInstallRecoveryBundleCheckpoint<'a> {
    pub(super) const fn new(
        icp_root: &'a Path,
        session: &'a FleetInstallSession,
        plan: &'a PersistedFleetInstallPlan,
    ) -> Self {
        Self::Persistent {
            icp_root,
            session,
            plan,
        }
    }

    pub(super) fn checkpoint(&self) -> Result<PathBuf, FleetInstallRecoveryBundleError> {
        match self {
            Self::Persistent {
                icp_root,
                session,
                plan,
            } => checkpoint_bundle(icp_root, session, plan),
            #[cfg(test)]
            Self::PersistentAt {
                icp_root,
                session,
                plan,
                bundle_path,
            } => checkpoint_bundle_at(icp_root, session, plan, bundle_path),
        }
    }

    #[cfg(test)]
    pub(super) const fn persistent_at(
        icp_root: &'a Path,
        session: &'a FleetInstallSession,
        plan: &'a PersistedFleetInstallPlan,
        bundle_path: PathBuf,
    ) -> Self {
        Self::PersistentAt {
            icp_root,
            session,
            plan,
            bundle_path,
        }
    }
}

fn checkpoint_bundle(
    icp_root: &Path,
    session: &FleetInstallSession,
    plan: &PersistedFleetInstallPlan,
) -> Result<PathBuf, FleetInstallRecoveryBundleError> {
    let bundle_path = canonical_bundle_path(session)?;
    checkpoint_bundle_at(icp_root, session, plan, &bundle_path)
}

fn checkpoint_bundle_at(
    icp_root: &Path,
    session: &FleetInstallSession,
    plan: &PersistedFleetInstallPlan,
    bundle_path: &Path,
) -> Result<PathBuf, FleetInstallRecoveryBundleError> {
    let sources = bundle_source_roots(icp_root, session, plan);
    let mut files = BTreeMap::<String, FleetInstallRecoveryBundleFileV1>::new();
    let mut total_bytes = 0_u64;
    for source in sources {
        collect_source(icp_root, &source, bundle_path, &mut files, &mut total_bytes)?;
    }
    let files = files.into_values().collect::<Vec<_>>();
    let root_checkpoints = derive_root_checkpoints(icp_root, bundle_path, plan, &files)?;
    let manifest = FleetInstallRecoveryBundleV1 {
        schema_version: BUNDLE_SCHEMA_VERSION,
        canonical_network_id: session.fleet.fleet.canonical_network_id.to_string(),
        fleet_name: session.fleet_name.to_string(),
        fleet_id: session.fleet.fleet.fleet_id.to_string(),
        app_id: session.fleet.app.to_string(),
        release_build_id: session.release_build_id.to_string(),
        fresh_fleet_plan_digest: session.fresh_fleet_plan_digest.clone(),
        fleet_install_plan_digest: plan.digest,
        install_operation_id: session.operation_id,
        root_checkpoints,
        files,
    };
    validate_manifest(bundle_path, &manifest)?;
    let manifest_path = bundle_path.join(MANIFEST_FILE);
    let bytes = encode_manifest(&manifest_path, &manifest)?;
    let lock_path = bundle_path.join(MANIFEST_LOCK_FILE);
    let _lock = lock_regular_file_with_parents(&lock_path).map_err(|error| match error {
        RegularFileLockError::NotRegular => FleetInstallRecoveryBundleError::NotRegular {
            path: lock_path.clone(),
        },
        RegularFileLockError::Io(source) => FleetInstallRecoveryBundleError::Io {
            path: lock_path.clone(),
            source,
        },
        #[cfg(windows)]
        RegularFileLockError::UnsupportedPlatform => FleetInstallRecoveryBundleError::Io {
            path: lock_path.clone(),
            source: io::Error::new(io::ErrorKind::Unsupported, "bundle locks are unsupported"),
        },
    })?;
    write_bytes(&manifest_path, &bytes).map_err(|source| FleetInstallRecoveryBundleError::Io {
        path: manifest_path,
        source,
    })?;
    verify_fleet_install_recovery_bundle(bundle_path)?;
    Ok(bundle_path.to_path_buf())
}

/// Verify one bundle without writing local operator state or invoking ICP.
pub fn verify_fleet_install_recovery_bundle(
    bundle_path: &Path,
) -> Result<FleetInstallRecoveryBundleReportV1, FleetInstallRecoveryBundleError> {
    let manifest_path = bundle_path.join(MANIFEST_FILE);
    let bytes = read_bounded(&manifest_path, MAX_MANIFEST_BYTES)?;
    let manifest = serde_json::from_slice::<FleetInstallRecoveryBundleV1>(&bytes)
        .map_err(|error| invalid(&manifest_path, error.to_string()))?;
    if encode_manifest(&manifest_path, &manifest)? != bytes {
        return Err(invalid(&manifest_path, "manifest bytes are not canonical"));
    }
    validate_manifest(bundle_path, &manifest)?;
    let mut total_bytes = 0_u64;
    for entry in &manifest.files {
        let object_path = object_path(bundle_path, entry.sha256);
        let bytes = read_bounded(&object_path, MAX_BUNDLE_FILE_BYTES)?;
        let observed_sha256: [u8; 32] = Sha256::digest(&bytes).into();
        if observed_sha256 != entry.sha256
            || u64::try_from(bytes.len()).ok() != Some(entry.size_bytes)
        {
            return Err(FleetInstallRecoveryBundleError::DigestMismatch { path: object_path });
        }
        total_bytes = total_bytes.checked_add(entry.size_bytes).ok_or(
            FleetInstallRecoveryBundleError::TooLarge {
                maximum_bytes: MAX_BUNDLE_TOTAL_BYTES,
            },
        )?;
    }
    if total_bytes > MAX_BUNDLE_TOTAL_BYTES {
        return Err(FleetInstallRecoveryBundleError::TooLarge {
            maximum_bytes: MAX_BUNDLE_TOTAL_BYTES,
        });
    }
    let session = require_session_binding(bundle_path, &manifest)?;
    let plan = require_plan_binding(bundle_path, &manifest)?;
    require_release_build_binding(bundle_path, &manifest)?;
    require_root_checkpoint_binding(bundle_path, &manifest, &session, &plan)?;
    require_confined_authority_paths(bundle_path, &manifest)?;
    Ok(report(bundle_path, &manifest, total_bytes))
}

fn require_session_binding(
    bundle_path: &Path,
    manifest: &FleetInstallRecoveryBundleV1,
) -> Result<FleetInstallSession, FleetInstallRecoveryBundleError> {
    let sessions = manifest
        .files
        .iter()
        .filter(|entry| {
            entry
                .logical_path
                .contains("/.canic/recovery/fleet-install-sessions/")
                || (entry
                    .logical_path
                    .starts_with(".canic/recovery/fleet-install-sessions/")
                    && entry.logical_path.ends_with("/session.json"))
        })
        .filter(|entry| entry.logical_path.ends_with("/session.json"))
        .collect::<Vec<_>>();
    let [entry] = sessions.as_slice() else {
        return Err(invalid(
            &bundle_path.join(MANIFEST_FILE),
            "bundle must contain exactly one retained Fleet install session",
        ));
    };
    let path = object_path(bundle_path, entry.sha256);
    let bytes = read_bounded(&path, MAX_BUNDLE_FILE_BYTES)?;
    let session = serde_json::from_slice::<FleetInstallSession>(&bytes)
        .map_err(|error| invalid(&path, error.to_string()))?;
    let exact = session.schema_version == BUNDLE_SCHEMA_VERSION
        && session.fleet.fleet.canonical_network_id.to_string() == manifest.canonical_network_id
        && session.fleet_name.to_string() == manifest.fleet_name
        && session.fleet.fleet.fleet_id.to_string() == manifest.fleet_id
        && session.fleet.app.to_string() == manifest.app_id
        && session.release_build_id.to_string() == manifest.release_build_id
        && session.fresh_fleet_plan_digest == manifest.fresh_fleet_plan_digest
        && session.operation_id == manifest.install_operation_id;
    if !exact {
        return Err(invalid(
            &path,
            "bundle session differs from its network, Fleet, plan, release-build or operation binding",
        ));
    }
    Ok(session)
}

fn require_plan_binding(
    bundle_path: &Path,
    manifest: &FleetInstallRecoveryBundleV1,
) -> Result<FleetInstallPlan, FleetInstallRecoveryBundleError> {
    let prefix = format!(
        ".canic/recovery/fleet-install-plans/{}/{}/{}/",
        manifest.canonical_network_id, manifest.fleet_id, manifest.release_build_id
    );
    let expected_path = format!("{prefix}plan.json");
    let entry = exact_entry(bundle_path, manifest, &expected_path)?;
    if entry.sha256 != manifest.fleet_install_plan_digest {
        return Err(invalid(
            &bundle_path.join(MANIFEST_FILE),
            "bundle Fleet install plan digest differs from its manifest authority",
        ));
    }
    let path = object_path(bundle_path, entry.sha256);
    let bytes = read_bounded(&path, MAX_BUNDLE_FILE_BYTES)?;
    let plan = serde_json::from_slice::<FleetInstallPlan>(&bytes)
        .map_err(|error| invalid(&path, error.to_string()))?;
    let canonical =
        encode_canonical_json(&plan, CanonicalJsonStyle::Compact, MAX_BUNDLE_FILE_BYTES).map_err(
            |error| match error {
                CanonicalJsonEncodeError::Serialization(error) => invalid(&path, error.to_string()),
                CanonicalJsonEncodeError::TooLarge => {
                    invalid(&path, "Fleet install plan exceeds its bound")
                }
            },
        )?;
    let exact = canonical == bytes
        && plan.fleet.fleet.canonical_network_id.to_string() == manifest.canonical_network_id
        && plan.fleet.fleet.fleet_id.to_string() == manifest.fleet_id
        && plan.fleet.app.to_string() == manifest.app_id
        && plan.release_build_id.to_string() == manifest.release_build_id
        && plan.fresh_fleet_plan_digest == manifest.fresh_fleet_plan_digest;
    if !exact {
        return Err(invalid(
            &path,
            "bundle Fleet install plan differs from its network, Fleet, release-build or plan binding",
        ));
    }
    Ok(plan)
}

fn require_release_build_binding(
    bundle_path: &Path,
    manifest: &FleetInstallRecoveryBundleV1,
) -> Result<(), FleetInstallRecoveryBundleError> {
    let expected_path = format!(
        ".canic/release-builds/{}/plan.cbor",
        manifest.release_build_id
    );
    let entry = exact_entry(bundle_path, manifest, &expected_path)?;
    let path = object_path(bundle_path, entry.sha256);
    let bytes = read_bounded(&path, MAX_BUNDLE_FILE_BYTES)?;
    let release_build_id = manifest
        .release_build_id
        .parse::<ReleaseBuildId>()
        .map_err(|error| invalid(&path, format!("{error:?}")))?;
    validate_retained_release_build_plan_bytes(&path, &bytes, release_build_id)
        .map_err(|error| invalid(&path, error.to_string()))
}

fn require_confined_authority_paths(
    bundle_path: &Path,
    manifest: &FleetInstallRecoveryBundleV1,
) -> Result<(), FleetInstallRecoveryBundleError> {
    let prefixes = [
        format!(
            ".canic/recovery/fleet-install-sessions/{}/{}/",
            manifest.canonical_network_id, manifest.fleet_name
        ),
        format!(
            ".canic/recovery/fleet-install-plans/{}/{}/{}/",
            manifest.canonical_network_id, manifest.fleet_id, manifest.release_build_id
        ),
        format!(".canic/release-builds/{}/", manifest.release_build_id),
        format!(
            ".canic/networks/{}/fleets/{}/",
            manifest.canonical_network_id, manifest.fleet_id
        ),
    ];
    if manifest.files.iter().any(|entry| {
        !prefixes
            .iter()
            .any(|prefix| entry.logical_path.starts_with(prefix))
    }) {
        return Err(invalid(
            &bundle_path.join(MANIFEST_FILE),
            "bundle contains authority outside its exact session, plan, release-build or Fleet state roots",
        ));
    }
    Ok(())
}

fn derive_root_checkpoints(
    icp_root: &Path,
    bundle_path: &Path,
    plan: &PersistedFleetInstallPlan,
    files: &[FleetInstallRecoveryBundleFileV1],
) -> Result<Vec<FleetInstallRecoveryBundleRootCheckpointV1>, FleetInstallRecoveryBundleError> {
    plan.plan
        .fleet_subnet_roots
        .iter()
        .map(|root| {
            let journal_path =
                fleet_subnet_root_install_journal_path(&plan.path, root.placement_subnet);
            let logical_path = logical_path_under_root(icp_root, &journal_path)?;
            let journal = optional_entry(files, &logical_path)
                .map(|entry| {
                    let bytes = read_bounded(
                        &object_path(bundle_path, entry.sha256),
                        MAX_BUNDLE_FILE_BYTES,
                    )?;
                    let journal = validate_retained_fleet_subnet_root_install_journal_bytes(
                        &journal_path,
                        &bytes,
                    )
                    .map_err(|error| invalid(&journal_path, error.to_string()))?;
                    require_root_journal_authority(plan, root, &journal, &journal_path)?;
                    Ok(FleetInstallRecoveryBundleRootJournalV1 {
                        sequence: journal.sequence,
                        phase: journal.phase,
                        sha256: entry.sha256,
                    })
                })
                .transpose()?;
            Ok(FleetInstallRecoveryBundleRootCheckpointV1 {
                placement_subnet: root.placement_subnet.to_string(),
                journal,
            })
        })
        .collect()
}

fn require_root_checkpoint_binding(
    bundle_path: &Path,
    manifest: &FleetInstallRecoveryBundleV1,
    session: &FleetInstallSession,
    plan: &FleetInstallPlan,
) -> Result<(), FleetInstallRecoveryBundleError> {
    if manifest.root_checkpoints.len() != plan.fleet_subnet_roots.len() {
        return Err(invalid(
            &bundle_path.join(MANIFEST_FILE),
            "bundle Root checkpoint catalog differs from its retained Fleet plan",
        ));
    }
    let plan_path = PathBuf::from(format!(
        ".canic/recovery/fleet-install-plans/{}/{}/{}/plan.json",
        manifest.canonical_network_id, manifest.fleet_id, manifest.release_build_id
    ));
    let persisted = PersistedFleetInstallPlan {
        plan: plan.clone(),
        digest: manifest.fleet_install_plan_digest,
        path: plan_path,
        root_release_sets: Vec::new(),
    };
    let mut admitted_root_directories = Vec::with_capacity(plan.fleet_subnet_roots.len());

    for (root, checkpoint) in plan
        .fleet_subnet_roots
        .iter()
        .zip(&manifest.root_checkpoints)
    {
        if checkpoint.placement_subnet != root.placement_subnet.to_string() {
            return Err(invalid(
                &bundle_path.join(MANIFEST_FILE),
                "bundle Root checkpoints are not in exact Fleet-plan order",
            ));
        }
        let journal_path =
            fleet_subnet_root_install_journal_path(&persisted.path, root.placement_subnet);
        let journal_logical = logical_path_string(&journal_path)?;
        admitted_root_directories.push(
            journal_path
                .parent()
                .expect("Root journal has an identity directory")
                .to_path_buf(),
        );
        match (
            &checkpoint.journal,
            optional_entry(&manifest.files, &journal_logical),
        ) {
            (None, None) => {}
            (Some(checkpoint), Some(entry)) if checkpoint.sha256 == entry.sha256 => {
                let bytes = entry_bytes(bundle_path, entry)?;
                let journal = validate_retained_fleet_subnet_root_install_journal_bytes(
                    &journal_path,
                    &bytes,
                )
                .map_err(|error| invalid(&journal_path, error.to_string()))?;
                require_root_journal_authority(&persisted, root, &journal, &journal_path)?;
                if journal.install_operation_id != manifest.install_operation_id
                    || journal.sequence != checkpoint.sequence
                    || journal.phase != checkpoint.phase
                {
                    return Err(invalid(
                        &journal_path,
                        "bundle Root checkpoint differs from its exact durable journal",
                    ));
                }
                require_phase_files(bundle_path, manifest, session, &journal_path, &journal)?;
            }
            _ => {
                return Err(invalid(
                    &journal_path,
                    "bundle Root checkpoint and durable journal presence differ",
                ));
            }
        }
    }

    let root_prefix = persisted
        .path
        .parent()
        .expect("Fleet plan has an identity directory")
        .join("fleet-subnet-root-installs");
    if manifest.files.iter().any(|entry| {
        let path = Path::new(&entry.logical_path);
        path.starts_with(&root_prefix)
            && !admitted_root_directories
                .iter()
                .any(|directory| path.starts_with(directory))
    }) {
        return Err(invalid(
            &bundle_path.join(MANIFEST_FILE),
            "bundle contains Root evidence outside its exact Fleet-plan participant set",
        ));
    }
    Ok(())
}

fn require_root_journal_authority(
    plan: &PersistedFleetInstallPlan,
    root: &crate::fleet_install_plan::PlannedFleetSubnetRoot,
    journal: &FleetSubnetRootInstallJournal,
    path: &Path,
) -> Result<(), FleetInstallRecoveryBundleError> {
    let exact = journal.fleet_install_plan_digest == plan.digest
        && journal.release_build_id == plan.plan.release_build_id
        && journal.root_plan == *root
        && journal.authority.binding.fleet == plan.plan.fleet
        && journal.authority.binding.coordinator_subnet == plan.plan.coordinator.coordinator_subnet;
    if exact {
        Ok(())
    } else {
        Err(invalid(
            path,
            "bundle Root journal differs from its Fleet plan or placement authority",
        ))
    }
}

fn require_phase_files(
    bundle_path: &Path,
    manifest: &FleetInstallRecoveryBundleV1,
    session: &FleetInstallSession,
    journal_path: &Path,
    journal: &FleetSubnetRootInstallJournal,
) -> Result<(), FleetInstallRecoveryBundleError> {
    let directory = journal_path
        .parent()
        .expect("Root journal has an identity directory");
    for (required, file) in [
        (
            journal.phase.requires_root_creation_result(),
            "root-create-result.json",
        ),
        (
            journal.phase.requires_wasm_store_creation_result(),
            "wasm-store-create-result.json",
        ),
        (
            journal.phase.requires_wasm_store_install_args(),
            "wasm-store-install-args.bin",
        ),
        (
            journal.phase.requires_root_install_args(),
            "root-install-args.bin",
        ),
    ] {
        if required {
            exact_entry(
                bundle_path,
                manifest,
                &logical_path_string(&directory.join(file))?,
            )?;
        }
    }
    require_repair_files(bundle_path, manifest, session, journal_path, journal)
}

fn require_repair_files(
    bundle_path: &Path,
    manifest: &FleetInstallRecoveryBundleV1,
    session: &FleetInstallSession,
    journal_path: &Path,
    journal: &FleetSubnetRootInstallJournal,
) -> Result<(), FleetInstallRecoveryBundleError> {
    let authority_path = repair_authority_path(journal_path);
    let authority_logical = logical_path_string(&authority_path)?;
    let Some(authority_entry) = optional_entry(&manifest.files, &authority_logical) else {
        let directory = journal_path
            .parent()
            .expect("Root journal has an identity directory");
        if manifest.files.iter().any(|entry| {
            let path = Path::new(&entry.logical_path);
            is_repair_evidence_path(path, directory)
        }) {
            return Err(invalid(
                &authority_path,
                "bundle repair residue omitted its exact provisional authority",
            ));
        }
        return Ok(());
    };
    let authority_bytes = entry_bytes(bundle_path, authority_entry)?;
    let authority = validate_recovery_bundle_repair_authority_bytes(
        &authority_path,
        &authority_bytes,
        session,
        journal,
    )
    .map_err(|error| invalid(&authority_path, error.to_string()))?;
    for (path, expected_sha256, expected_size) in [
        (
            retained_artifact_path(
                journal_path,
                authority.upgrade_predecessor_module_sha256,
                "wasm",
            ),
            authority.upgrade_predecessor_module_sha256,
            Some(authority.upgrade_predecessor_wasm_size_bytes),
        ),
        (
            retained_artifact_path(journal_path, authority.successor_module_sha256, "wasm"),
            authority.successor_module_sha256,
            Some(authority.successor_wasm_size_bytes),
        ),
        (
            retained_artifact_path(
                journal_path,
                authority.upgrade_predecessor_candid_sha256,
                "did",
            ),
            authority.upgrade_predecessor_candid_sha256,
            None,
        ),
        (
            retained_artifact_path(journal_path, authority.successor_candid_sha256, "did"),
            authority.successor_candid_sha256,
            None,
        ),
    ] {
        let entry = exact_entry(bundle_path, manifest, &logical_path_string(&path)?)?;
        if entry.sha256 != expected_sha256
            || expected_size.is_some_and(|size| entry.size_bytes != size)
        {
            return Err(invalid(
                &path,
                "bundle retained repair artifact differs from its exact hash or size authority",
            ));
        }
    }

    let operation_path = retained_root_repair_operation_path(journal_path);
    let operation_logical = logical_path_string(&operation_path)?;
    if let Some(entry) = optional_entry(&manifest.files, &operation_logical) {
        validate_recovery_bundle_repair_operation_bytes(
            &operation_path,
            &entry_bytes(bundle_path, entry)?,
            &authority,
        )
        .map_err(|error| invalid(&operation_path, error.to_string()))?;
    }
    let receipt_path = repair_receipt_path(journal_path);
    let receipt_logical = logical_path_string(&receipt_path)?;
    if let Some(entry) = optional_entry(&manifest.files, &receipt_logical) {
        exact_entry(bundle_path, manifest, &operation_logical)?;
        validate_recovery_bundle_repair_receipt_bytes(
            &receipt_path,
            &entry_bytes(bundle_path, entry)?,
            &authority,
            session,
            journal,
        )
        .map_err(|error| invalid(&receipt_path, error.to_string()))?;
    }
    Ok(())
}

fn is_repair_evidence_path(path: &Path, root_directory: &Path) -> bool {
    let is_root_repair_file = path.parent() == Some(root_directory)
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("root-repair-"));
    let is_retained_artifact = path.starts_with(root_directory.join("root-repair-artifacts"));
    is_root_repair_file || is_retained_artifact
}

fn exact_entry<'a>(
    bundle_path: &Path,
    manifest: &'a FleetInstallRecoveryBundleV1,
    logical_path: &str,
) -> Result<&'a FleetInstallRecoveryBundleFileV1, FleetInstallRecoveryBundleError> {
    let matches = manifest
        .files
        .iter()
        .filter(|entry| entry.logical_path == logical_path)
        .collect::<Vec<_>>();
    let [entry] = matches.as_slice() else {
        return Err(invalid(
            &bundle_path.join(MANIFEST_FILE),
            format!("bundle must contain exact authority file {logical_path}"),
        ));
    };
    Ok(entry)
}

fn optional_entry<'a>(
    files: &'a [FleetInstallRecoveryBundleFileV1],
    logical_path: &str,
) -> Option<&'a FleetInstallRecoveryBundleFileV1> {
    files
        .binary_search_by(|entry| entry.logical_path.as_str().cmp(logical_path))
        .ok()
        .map(|index| &files[index])
}

fn entry_bytes(
    bundle_path: &Path,
    entry: &FleetInstallRecoveryBundleFileV1,
) -> Result<Vec<u8>, FleetInstallRecoveryBundleError> {
    read_bounded(
        &object_path(bundle_path, entry.sha256),
        MAX_BUNDLE_FILE_BYTES,
    )
}

fn logical_path_under_root(
    icp_root: &Path,
    path: &Path,
) -> Result<String, FleetInstallRecoveryBundleError> {
    let relative =
        path.strip_prefix(icp_root)
            .map_err(|_| FleetInstallRecoveryBundleError::UnsafePath {
                path: path.to_path_buf(),
            })?;
    logical_path_string(relative)
}

fn logical_path_string(path: &Path) -> Result<String, FleetInstallRecoveryBundleError> {
    Ok(checked_logical_path(&path.to_string_lossy())?
        .to_string_lossy()
        .replace('\\', "/"))
}

/// Import missing exact files into an operator root after complete no-effect bundle verification.
/// Existing bytes are never overwritten.
pub fn import_fleet_install_recovery_bundle(
    bundle_path: &Path,
    icp_root: &Path,
) -> Result<FleetInstallRecoveryBundleReportV1, FleetInstallRecoveryBundleError> {
    let report = verify_fleet_install_recovery_bundle(bundle_path)?;
    let manifest_path = bundle_path.join(MANIFEST_FILE);
    let bytes = read_bounded(&manifest_path, MAX_MANIFEST_BYTES)?;
    let manifest = serde_json::from_slice::<FleetInstallRecoveryBundleV1>(&bytes)
        .map_err(|error| invalid(&manifest_path, error.to_string()))?;
    let mut missing = Vec::new();
    for entry in &manifest.files {
        let relative = checked_logical_path(&entry.logical_path)?;
        let destination = icp_root.join(relative);
        let object = read_bounded(
            &object_path(bundle_path, entry.sha256),
            MAX_BUNDLE_FILE_BYTES,
        )?;
        match read_optional_bounded_regular_bytes(&destination, MAX_BUNDLE_FILE_BYTES) {
            Ok(Some(existing)) if existing == object => {}
            Ok(Some(_)) => {
                return Err(FleetInstallRecoveryBundleError::ImportConflict { path: destination });
            }
            Ok(None) => missing.push((entry, destination)),
            Err(BoundedRegularFileReadError::TooLarge) => {
                return Err(FleetInstallRecoveryBundleError::ImportConflict { path: destination });
            }
            Err(BoundedRegularFileReadError::Read(RegularFileReadError::NotRegular)) => {
                return Err(FleetInstallRecoveryBundleError::NotRegular { path: destination });
            }
            Err(BoundedRegularFileReadError::Read(RegularFileReadError::Io(source))) => {
                return Err(FleetInstallRecoveryBundleError::Io {
                    path: destination,
                    source,
                });
            }
            #[cfg(not(unix))]
            Err(BoundedRegularFileReadError::Read(RegularFileReadError::UnsupportedPlatform)) => {
                return Err(FleetInstallRecoveryBundleError::Io {
                    path: destination,
                    source: io::Error::new(io::ErrorKind::Unsupported, "bundle import unsupported"),
                });
            }
        }
    }
    // Check the complete destination before the first local mutation. A concurrent publisher may
    // supply only the same bytes; a conflicting winner still fails closed.
    for (entry, destination) in missing {
        let object = read_bounded(
            &object_path(bundle_path, entry.sha256),
            MAX_BUNDLE_FILE_BYTES,
        )?;
        match create_new_bytes_with_parents(&destination, &object) {
            Ok(()) => {}
            Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {
                let existing = read_bounded(&destination, MAX_BUNDLE_FILE_BYTES)?;
                if existing != object {
                    return Err(FleetInstallRecoveryBundleError::ImportConflict {
                        path: destination,
                    });
                }
            }
            Err(source) => {
                return Err(FleetInstallRecoveryBundleError::Io {
                    path: destination,
                    source,
                });
            }
        }
    }
    Ok(report)
}

fn bundle_source_roots(
    icp_root: &Path,
    session: &FleetInstallSession,
    plan: &PersistedFleetInstallPlan,
) -> Vec<PathBuf> {
    let session_dir = session_path(
        icp_root,
        session.fleet.fleet.canonical_network_id,
        &session.fleet_name,
    )
    .parent()
    .expect("session has a directory")
    .to_path_buf();
    let plan_dir = plan
        .path
        .parent()
        .expect("Fleet install plan has a directory")
        .to_path_buf();
    let release_build_dir = release_build_plan_path(icp_root, session.release_build_id)
        .parent()
        .expect("release build plan has a directory")
        .to_path_buf();
    let fleet_state_dir = icp_root
        .join(".canic/networks")
        .join(session.fleet.fleet.canonical_network_id.to_string())
        .join("fleets")
        .join(session.fleet.fleet.fleet_id.to_string());
    vec![session_dir, plan_dir, release_build_dir, fleet_state_dir]
}

fn collect_source(
    icp_root: &Path,
    source: &Path,
    bundle_path: &Path,
    files: &mut BTreeMap<String, FleetInstallRecoveryBundleFileV1>,
    total_bytes: &mut u64,
) -> Result<(), FleetInstallRecoveryBundleError> {
    if source == bundle_path {
        return Ok(());
    }
    let metadata = match fs::symlink_metadata(source) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(source_error) => {
            return Err(FleetInstallRecoveryBundleError::Io {
                path: source.to_path_buf(),
                source: source_error,
            });
        }
    };
    if metadata.file_type().is_symlink() {
        return Err(FleetInstallRecoveryBundleError::NotRegular {
            path: source.to_path_buf(),
        });
    }
    if metadata.is_file() {
        return collect_file(icp_root, source, bundle_path, files, total_bytes);
    }
    if !metadata.is_dir() {
        return Err(FleetInstallRecoveryBundleError::NotRegular {
            path: source.to_path_buf(),
        });
    }
    let mut children = fs::read_dir(source)
        .map_err(|source_error| FleetInstallRecoveryBundleError::Io {
            path: source.to_path_buf(),
            source: source_error,
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source_error| FleetInstallRecoveryBundleError::Io {
            path: source.to_path_buf(),
            source: source_error,
        })?;
    children.sort_by_key(fs::DirEntry::file_name);
    for child in children {
        collect_source(icp_root, &child.path(), bundle_path, files, total_bytes)?;
    }
    Ok(())
}

fn collect_file(
    icp_root: &Path,
    source: &Path,
    bundle_path: &Path,
    files: &mut BTreeMap<String, FleetInstallRecoveryBundleFileV1>,
    total_bytes: &mut u64,
) -> Result<(), FleetInstallRecoveryBundleError> {
    let relative =
        source
            .strip_prefix(icp_root)
            .map_err(|_| FleetInstallRecoveryBundleError::UnsafePath {
                path: source.to_path_buf(),
            })?;
    let logical_path = checked_logical_path(&relative.to_string_lossy())?
        .to_string_lossy()
        .replace('\\', "/");
    let bytes = read_bounded(source, MAX_BUNDLE_FILE_BYTES)?;
    let size_bytes =
        u64::try_from(bytes.len()).map_err(|_| FleetInstallRecoveryBundleError::FileTooLarge {
            path: source.to_path_buf(),
        })?;
    *total_bytes =
        total_bytes
            .checked_add(size_bytes)
            .ok_or(FleetInstallRecoveryBundleError::TooLarge {
                maximum_bytes: MAX_BUNDLE_TOTAL_BYTES,
            })?;
    if *total_bytes > MAX_BUNDLE_TOTAL_BYTES {
        return Err(FleetInstallRecoveryBundleError::TooLarge {
            maximum_bytes: MAX_BUNDLE_TOTAL_BYTES,
        });
    }
    if files.len() >= MAX_BUNDLE_FILES && !files.contains_key(&logical_path) {
        return Err(FleetInstallRecoveryBundleError::TooManyFiles {
            maximum: MAX_BUNDLE_FILES,
        });
    }
    let sha256: [u8; 32] = Sha256::digest(&bytes).into();
    let object = object_path(bundle_path, sha256);
    match create_new_bytes_with_parents(&object, &bytes) {
        Ok(()) => {}
        Err(source_error) if source_error.kind() == io::ErrorKind::AlreadyExists => {
            let observed = read_bounded(&object, MAX_BUNDLE_FILE_BYTES)?;
            if observed != bytes {
                return Err(FleetInstallRecoveryBundleError::DigestMismatch { path: object });
            }
        }
        Err(source_error) => {
            return Err(FleetInstallRecoveryBundleError::Io {
                path: object,
                source: source_error,
            });
        }
    }
    files.insert(
        logical_path.clone(),
        FleetInstallRecoveryBundleFileV1 {
            logical_path,
            sha256,
            size_bytes,
        },
    );
    Ok(())
}

fn validate_manifest(
    path: &Path,
    manifest: &FleetInstallRecoveryBundleV1,
) -> Result<(), FleetInstallRecoveryBundleError> {
    if manifest.schema_version != BUNDLE_SCHEMA_VERSION {
        return Err(invalid(
            path,
            "unsupported bundle schema; use the matching Canic release to export a supported bundle",
        ));
    }
    if manifest.root_checkpoints.len() > 4_096
        || manifest.root_checkpoints.iter().any(|checkpoint| {
            checkpoint.placement_subnet.is_empty()
                || checkpoint
                    .journal
                    .as_ref()
                    .is_some_and(|journal| journal.sha256 == [0; 32])
        })
    {
        return Err(invalid(path, "bundle Root checkpoint catalog is invalid"));
    }
    if manifest.canonical_network_id.is_empty()
        || manifest.fleet_name.is_empty()
        || manifest.fleet_id.is_empty()
        || manifest.app_id.is_empty()
        || manifest.release_build_id.is_empty()
        || manifest.fresh_fleet_plan_digest.is_empty()
        || manifest.fleet_install_plan_digest == [0; 32]
        || manifest.install_operation_id == [0; 32]
        || manifest.files.is_empty()
        || manifest.files.len() > MAX_BUNDLE_FILES
    {
        return Err(invalid(
            path,
            "bundle authority is empty or exceeds its bound",
        ));
    }
    let mut previous = None;
    for entry in &manifest.files {
        let logical = checked_logical_path(&entry.logical_path)?;
        if entry.sha256 == [0; 32]
            || entry.size_bytes > MAX_BUNDLE_FILE_BYTES as u64
            || previous
                .as_ref()
                .is_some_and(|value: &PathBuf| value >= &logical)
        {
            return Err(invalid(
                path,
                "bundle file entries are invalid or not canonical",
            ));
        }
        previous = Some(logical);
    }
    Ok(())
}

fn checked_logical_path(value: &str) -> Result<PathBuf, FleetInstallRecoveryBundleError> {
    let path = PathBuf::from(value);
    if path.is_absolute()
        || !path.starts_with(".canic")
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(FleetInstallRecoveryBundleError::UnsafePath { path });
    }
    Ok(path)
}

fn canonical_bundle_path(
    session: &FleetInstallSession,
) -> Result<PathBuf, FleetInstallRecoveryBundleError> {
    let state_root = env::var_os("CANIC_OPERATOR_STATE_ROOT")
        .map(PathBuf::from)
        .or_else(|| env::var_os("XDG_STATE_HOME").map(|root| PathBuf::from(root).join("canic")))
        .or_else(|| env::var_os("HOME").map(|root| PathBuf::from(root).join(".local/state/canic")))
        .ok_or(FleetInstallRecoveryBundleError::OperatorStateUnavailable)?;
    Ok(state_root
        .join("recovery-bundles")
        .join(session.fleet.fleet.canonical_network_id.to_string())
        .join(session.fleet.fleet.fleet_id.to_string())
        .join(&session.fresh_fleet_plan_digest)
        .join(session.release_build_id.to_string()))
}

fn object_path(bundle_path: &Path, sha256: [u8; 32]) -> PathBuf {
    bundle_path.join(OBJECT_DIRECTORY).join(encode_hex(&sha256))
}

fn read_bounded(path: &Path, maximum: usize) -> Result<Vec<u8>, FleetInstallRecoveryBundleError> {
    match read_optional_bounded_regular_bytes(path, maximum) {
        Ok(Some(bytes)) => Ok(bytes),
        Ok(None) => Err(FleetInstallRecoveryBundleError::Missing {
            path: path.to_path_buf(),
        }),
        Err(BoundedRegularFileReadError::TooLarge) => {
            Err(FleetInstallRecoveryBundleError::FileTooLarge {
                path: path.to_path_buf(),
            })
        }
        Err(BoundedRegularFileReadError::Read(RegularFileReadError::NotRegular)) => {
            Err(FleetInstallRecoveryBundleError::NotRegular {
                path: path.to_path_buf(),
            })
        }
        Err(BoundedRegularFileReadError::Read(RegularFileReadError::Io(source))) => {
            Err(FleetInstallRecoveryBundleError::Io {
                path: path.to_path_buf(),
                source,
            })
        }
        #[cfg(not(unix))]
        Err(BoundedRegularFileReadError::Read(RegularFileReadError::UnsupportedPlatform)) => {
            Err(FleetInstallRecoveryBundleError::Io {
                path: path.to_path_buf(),
                source: io::Error::new(io::ErrorKind::Unsupported, "bundle reads unsupported"),
            })
        }
    }
}

fn encode_manifest(
    path: &Path,
    manifest: &FleetInstallRecoveryBundleV1,
) -> Result<Vec<u8>, FleetInstallRecoveryBundleError> {
    encode_canonical_json(manifest, CanonicalJsonStyle::Compact, MAX_MANIFEST_BYTES).map_err(
        |error| match error {
            CanonicalJsonEncodeError::Serialization(error) => invalid(path, error.to_string()),
            CanonicalJsonEncodeError::TooLarge => {
                invalid(path, "bundle manifest exceeds its bound")
            }
        },
    )
}

fn report(
    bundle_path: &Path,
    manifest: &FleetInstallRecoveryBundleV1,
    total_bytes: u64,
) -> FleetInstallRecoveryBundleReportV1 {
    FleetInstallRecoveryBundleReportV1 {
        schema_version: manifest.schema_version,
        canonical_network_id: manifest.canonical_network_id.clone(),
        fleet_name: manifest.fleet_name.clone(),
        fleet_id: manifest.fleet_id.clone(),
        app_id: manifest.app_id.clone(),
        release_build_id: manifest.release_build_id.clone(),
        fresh_fleet_plan_digest: manifest.fresh_fleet_plan_digest.clone(),
        fleet_install_plan_digest: encode_hex(&manifest.fleet_install_plan_digest),
        install_operation_id: encode_hex(&manifest.install_operation_id),
        file_count: manifest.files.len(),
        total_bytes,
        bundle_path: bundle_path.to_path_buf(),
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

fn invalid(path: &Path, reason: impl Into<String>) -> FleetInstallRecoveryBundleError {
    FleetInstallRecoveryBundleError::InvalidManifest {
        path: path.to_path_buf(),
        reason: reason.into(),
    }
}
