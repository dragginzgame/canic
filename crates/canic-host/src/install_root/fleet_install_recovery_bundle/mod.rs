//! Module: install_root::fleet_install_recovery_bundle
//!
//! Responsibility: retain and verify one path-confined, content-addressed copy of the complete
//! local authority needed to resume an incomplete fresh-Fleet installation.
//! Does not own: installation decisions, journal transitions, remote effects, or compatibility.
//! Boundary: bundle checkpoints copy only regular files below exact Canic-owned roots; import may
//! create missing identical local files but never overwrites conflicting state.

#[cfg(test)]
mod tests;

use super::{
    PlannedCurrentFleetInstall,
    fleet_install_session::{FleetInstallSession, session_path},
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
    files: Vec<FleetInstallRecoveryBundleFileV1>,
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

/// Persist a stable checkpoint before the first remote effect and after every governed phase.
pub(super) fn checkpoint_current_fleet_install_recovery_bundle(
    icp_root: &Path,
    planned: &PlannedCurrentFleetInstall,
) -> Result<PathBuf, FleetInstallRecoveryBundleError> {
    checkpoint_bundle(icp_root, &planned.session, &planned.plan)
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
        files: files.into_values().collect(),
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
    require_session_binding(bundle_path, &manifest)?;
    require_plan_binding(bundle_path, &manifest)?;
    require_release_build_binding(bundle_path, &manifest)?;
    require_confined_authority_paths(bundle_path, &manifest)?;
    Ok(report(bundle_path, &manifest, total_bytes))
}

fn require_session_binding(
    bundle_path: &Path,
    manifest: &FleetInstallRecoveryBundleV1,
) -> Result<(), FleetInstallRecoveryBundleError> {
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
    Ok(())
}

fn require_plan_binding(
    bundle_path: &Path,
    manifest: &FleetInstallRecoveryBundleV1,
) -> Result<(), FleetInstallRecoveryBundleError> {
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
    Ok(())
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
