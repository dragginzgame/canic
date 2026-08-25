//! Module: install_root::fleet_install_session
//!
//! Responsibility: own the durable Fleet identity and install operation for one fresh install.
//! Does not own: Coordinator, Fleet Subnet Root, Store, Registry, or activation effects.
//! Boundary: typed-schema retries recover one immutable session before terminal catalog closure.

#[cfg(test)]
mod tests;

use crate::{
    durable_io::{
        BoundedRegularFileReadError, CanonicalJsonEncodeError, CanonicalJsonStyle,
        RegularFileLockError, RegularFileReadError, create_new_bytes_with_parents,
        encode_canonical_json, lock_regular_file_with_parents, read_optional_bounded_regular_bytes,
        read_optional_regular_bytes,
    },
    entropy::{EntropyError, random_bytes_32},
    release_build::{
        FinalizedReleaseBuild, ReleaseBuildPlanError, ReleaseBuildPlanState,
        load_finalized_release_build,
    },
};
use crate::{
    fleet_catalog::FleetCatalogEntryV1,
    install_root::fleet_component_provisioning_journal::{
        FleetComponentProvisioningTerminalEvidence, JOURNAL_SCHEMA_VERSION,
    },
};
use std::{
    io,
    path::{Path, PathBuf},
};

use canic_core::ids::{
    AppId, CanonicalNetworkId, FleetBinding, FleetId, FleetKey, FleetName, ReleaseBuildId,
};
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

const SESSION_FILE: &str = "session.json";
const SESSION_LOCK_FILE: &str = "session.lock";
const SESSION_SCHEMA_VERSION: u32 = 1;
const MAX_SESSION_BYTES: usize = 16_384;
const COMPLETION_FILE: &str = "completion.json";
const COMPLETION_LOCK_FILE: &str = "completion.lock";
const COMPLETION_SCHEMA_VERSION: u32 = 1;
const MAX_COMPLETION_BYTES: usize = 65_536;

///
/// FleetInstallSession
///
/// Immutable pre-effect identity shared by every journal in one fresh Fleet install.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FleetInstallSession {
    pub schema_version: u32,
    pub fleet_name: FleetName,
    pub fleet: FleetBinding,
    pub release_build_id: ReleaseBuildId,
    pub release_build_plan_digest: [u8; 32],
    pub release_set_manifest_digest: [u8; 32],
    pub decision_release_build_id: Option<ReleaseBuildId>,
    pub fresh_fleet_plan_digest: String,
    pub operation_id: [u8; 32],
}

///
/// PlanFleetInstallSessionRequest
///
/// Exact finalized authority required before allocating or recovering a Fleet identity.
///

pub(super) struct PlanFleetInstallSessionRequest<'a> {
    pub root: &'a Path,
    pub canonical_network_id: CanonicalNetworkId,
    pub fleet_name: FleetName,
    pub app: AppId,
    pub finalized_release_build: &'a FinalizedReleaseBuild,
    pub decision_release_build_id: Option<ReleaseBuildId>,
    pub fresh_fleet_plan_digest: &'a str,
}

/// Exact plan/release authority retained for same-release install recovery.
pub(super) struct RecoveredFleetInstallAuthority {
    pub session: FleetInstallSession,
    pub finalized_release_build: FinalizedReleaseBuild,
    pub decision_release_build_id: Option<ReleaseBuildId>,
    pub fresh_fleet_plan_digest: String,
}

/// Exact terminal catalog evidence that permanently closes one fresh-install session.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct FleetInstallCompletionV1 {
    schema_version: u32,
    session_schema_version: u32,
    fleet_name: FleetName,
    fleet: FleetBinding,
    release_build_id: ReleaseBuildId,
    fresh_fleet_plan_digest: String,
    fleet_install_operation_id: [u8; 32],
    fleet_install_plan_digest: [u8; 32],
    component_journal_schema_version: u32,
    component_journal_sequence: u64,
    component_journal_digest: [u8; 32],
    component_provisioning_operation_id: [u8; 32],
    component_provisioning_plan_hash: [u8; 32],
    catalog_entry: FleetCatalogEntryV1,
    catalog_hash: [u8; 32],
}

pub(super) struct CloseFleetInstallSessionRequest<'a> {
    pub root: &'a Path,
    pub session: &'a FleetInstallSession,
    pub component_journal: &'a FleetComponentProvisioningTerminalEvidence,
}

///
/// FleetInstallSessionError
///
/// Typed failure while publishing or recovering immutable fresh-install identity.
///

#[derive(Debug, ThisError)]
pub(super) enum FleetInstallSessionError {
    #[error("Fleet install session already has different immutable authority: {path}")]
    ConflictingAuthority { path: PathBuf },

    #[error(
        "Fleet install session is complete and cannot re-enter fresh-install recovery: {path}; wait for and use a separately supported managed-upgrade workflow from the published Fleet baseline"
    )]
    Completed { path: PathBuf },

    #[error("failed to access Fleet install session {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("invalid Fleet install session {path}: {reason}")]
    InvalidDocument { path: PathBuf, reason: String },

    #[error("Fleet install session is not a regular no-follow file: {path}")]
    UnsafeFile { path: PathBuf },

    #[error("Fleet install session lock is not a regular no-follow file: {path}")]
    UnsafeLock { path: PathBuf },

    #[error("cryptographic random source returned only {actual} of 32 required bytes")]
    ShortRandomRead { actual: usize },

    #[error(transparent)]
    ReleaseBuild(#[from] ReleaseBuildPlanError),
}

/// Publish or recover the exact pre-effect identity for one fresh Fleet install.
pub(super) fn plan_fleet_install_session(
    request: PlanFleetInstallSessionRequest<'_>,
) -> Result<FleetInstallSession, FleetInstallSessionError> {
    let finalized = load_finalized_release_build(
        request.root,
        request.finalized_release_build.record.release_build_id,
    )?;
    if finalized.record != request.finalized_release_build.record
        || finalized.plan_hash != request.finalized_release_build.plan_hash
    {
        return Err(FleetInstallSessionError::ConflictingAuthority {
            path: request.finalized_release_build.path.clone(),
        });
    }
    let ReleaseBuildPlanState::Finalized {
        release_set_manifest_digest,
    } = finalized.record.state
    else {
        unreachable!("load_finalized_release_build admits only finalized records");
    };

    let path = session_path(
        request.root,
        request.canonical_network_id,
        &request.fleet_name,
    );
    let _lock = lock_session(&path)?;
    if let Some(session) = load_optional_session(&path)? {
        let completion_path = path.with_file_name(COMPLETION_FILE);
        if let Some(completion) = load_optional_completion(&completion_path)? {
            validate_completion(&completion_path, &completion, &session)?;
            return Err(FleetInstallSessionError::Completed {
                path: completion_path,
            });
        }
        if session_matches_request(
            &session,
            &request,
            finalized.plan_hash,
            release_set_manifest_digest,
        ) {
            return Ok(session);
        }
        return Err(FleetInstallSessionError::ConflictingAuthority { path });
    }

    let session = FleetInstallSession {
        schema_version: SESSION_SCHEMA_VERSION,
        fleet_name: request.fleet_name.clone(),
        fleet: FleetBinding {
            fleet: FleetKey {
                canonical_network_id: request.canonical_network_id,
                fleet_id: FleetId::from_generated_bytes(random_identity_bytes()?),
            },
            app: request.app.clone(),
        },
        release_build_id: finalized.record.release_build_id,
        release_build_plan_digest: finalized.plan_hash,
        release_set_manifest_digest,
        decision_release_build_id: request.decision_release_build_id,
        fresh_fleet_plan_digest: request.fresh_fleet_plan_digest.to_string(),
        operation_id: random_identity_bytes()?,
    };
    let bytes = encode_session(&path, &session)?;
    if let Err(source) = create_new_bytes_with_parents(&path, &bytes) {
        if source.kind() == io::ErrorKind::AlreadyExists {
            if let Some(observed) = load_optional_session(&path)?
                && session_matches_request(
                    &observed,
                    &request,
                    finalized.plan_hash,
                    release_set_manifest_digest,
                )
            {
                return Ok(observed);
            }
            return Err(FleetInstallSessionError::ConflictingAuthority { path });
        }
        return Err(FleetInstallSessionError::Io { path, source });
    }

    let observed = load_optional_session(&path)?.ok_or_else(|| {
        invalid(
            &path,
            "published Fleet install session could not be read back",
        )
    })?;
    if observed != session {
        return Err(invalid(
            &path,
            "published Fleet install session differs from planned authority",
        ));
    }
    Ok(observed)
}

/// Permanently close one fresh-install session after terminal Fleet catalog publication.
pub(super) fn close_fleet_install_session(
    request: CloseFleetInstallSessionRequest<'_>,
) -> Result<(), FleetInstallSessionError> {
    let journal = request.component_journal;
    let completion = FleetInstallCompletionV1 {
        schema_version: COMPLETION_SCHEMA_VERSION,
        session_schema_version: request.session.schema_version,
        fleet_name: request.session.fleet_name.clone(),
        fleet: request.session.fleet.clone(),
        release_build_id: request.session.release_build_id,
        fresh_fleet_plan_digest: request.session.fresh_fleet_plan_digest.clone(),
        fleet_install_operation_id: request.session.operation_id,
        fleet_install_plan_digest: journal.fleet_install_plan_digest,
        component_journal_schema_version: journal.schema_version,
        component_journal_sequence: journal.sequence,
        component_journal_digest: journal.journal_digest,
        component_provisioning_operation_id: journal.operation_id,
        component_provisioning_plan_hash: journal.plan_hash,
        catalog_entry: journal.catalog_entry.clone(),
        catalog_hash: journal.catalog_hash,
    };
    let path = completion_path(request.root, request.session);
    validate_completion(&path, &completion, request.session)?;
    let bytes = encode_completion(&path, &completion)?;
    let _lock = lock_completion(&path)?;
    match create_new_bytes_with_parents(&path, &bytes) {
        Ok(()) => {}
        Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {
            let observed = load_optional_completion(&path)?.ok_or_else(|| {
                FleetInstallSessionError::ConflictingAuthority { path: path.clone() }
            })?;
            if observed != completion {
                return Err(FleetInstallSessionError::ConflictingAuthority { path });
            }
        }
        Err(source) => return Err(FleetInstallSessionError::Io { path, source }),
    }
    let durable = load_optional_completion(&path)?
        .ok_or_else(|| FleetInstallSessionError::ConflictingAuthority { path: path.clone() })?;
    if durable != completion {
        return Err(FleetInstallSessionError::ConflictingAuthority { path });
    }
    Ok(())
}

/// Recover the original decision binding plus finalized artifacts for exact resume.
pub(super) fn recover_fleet_install_session_authority(
    root: &Path,
    canonical_network_id: CanonicalNetworkId,
    fleet_name: &FleetName,
    app: &AppId,
) -> Result<Option<RecoveredFleetInstallAuthority>, FleetInstallSessionError> {
    let path = session_path(root, canonical_network_id, fleet_name);
    let _lock = lock_session(&path)?;
    inspect_fleet_install_session_authority_at_path(
        root,
        &path,
        canonical_network_id,
        fleet_name,
        app,
    )
}

/// Inspect exact retained install authority without creating a recovery lock or other file.
pub(super) fn inspect_fleet_install_session_authority(
    root: &Path,
    canonical_network_id: CanonicalNetworkId,
    fleet_name: &FleetName,
    app: &AppId,
) -> Result<Option<RecoveredFleetInstallAuthority>, FleetInstallSessionError> {
    let path = session_path(root, canonical_network_id, fleet_name);
    inspect_fleet_install_session_authority_at_path(
        root,
        &path,
        canonical_network_id,
        fleet_name,
        app,
    )
}

fn inspect_fleet_install_session_authority_at_path(
    root: &Path,
    path: &Path,
    canonical_network_id: CanonicalNetworkId,
    fleet_name: &FleetName,
    app: &AppId,
) -> Result<Option<RecoveredFleetInstallAuthority>, FleetInstallSessionError> {
    let Some(session) = load_optional_session(path)? else {
        return Ok(None);
    };
    if session.fleet_name != *fleet_name
        || session.fleet.fleet.canonical_network_id != canonical_network_id
        || session.fleet.app != *app
    {
        return Err(FleetInstallSessionError::ConflictingAuthority {
            path: path.to_path_buf(),
        });
    }
    let completion_path = path.with_file_name(COMPLETION_FILE);
    if let Some(completion) = load_optional_completion(&completion_path)? {
        validate_completion(&completion_path, &completion, &session)?;
        return Err(FleetInstallSessionError::Completed {
            path: completion_path,
        });
    }

    let finalized = load_finalized_release_build(root, session.release_build_id)?;
    let ReleaseBuildPlanState::Finalized {
        release_set_manifest_digest,
    } = finalized.record.state
    else {
        unreachable!("load_finalized_release_build admits only finalized records");
    };
    if session.release_build_plan_digest != finalized.plan_hash
        || session.release_set_manifest_digest != release_set_manifest_digest
    {
        return Err(invalid(
            path,
            "session release-build evidence differs from its finalized authority",
        ));
    }

    Ok(Some(RecoveredFleetInstallAuthority {
        finalized_release_build: finalized,
        decision_release_build_id: session.decision_release_build_id,
        fresh_fleet_plan_digest: session.fresh_fleet_plan_digest.clone(),
        session,
    }))
}

fn session_matches_request(
    session: &FleetInstallSession,
    request: &PlanFleetInstallSessionRequest<'_>,
    release_build_plan_digest: [u8; 32],
    release_set_manifest_digest: [u8; 32],
) -> bool {
    session.schema_version == SESSION_SCHEMA_VERSION
        && session.fleet_name == request.fleet_name
        && session.fleet.fleet.canonical_network_id == request.canonical_network_id
        && session.fleet.app == request.app
        && session.release_build_id == request.finalized_release_build.record.release_build_id
        && session.release_build_plan_digest == release_build_plan_digest
        && session.release_set_manifest_digest == release_set_manifest_digest
        && session.decision_release_build_id == request.decision_release_build_id
        && session.fresh_fleet_plan_digest == request.fresh_fleet_plan_digest
}

fn encode_session(
    path: &Path,
    session: &FleetInstallSession,
) -> Result<Vec<u8>, FleetInstallSessionError> {
    validate_session(path, session)?;
    let mut bytes = serde_json::to_vec_pretty(session)
        .map_err(|error| invalid(path, format!("could not encode JSON: {error}")))?;
    bytes.push(b'\n');
    if bytes.len() > MAX_SESSION_BYTES {
        return Err(invalid(path, "session exceeds its byte bound"));
    }
    Ok(bytes)
}

fn load_optional_session(
    path: &Path,
) -> Result<Option<FleetInstallSession>, FleetInstallSessionError> {
    let bytes = match read_optional_regular_bytes(path) {
        Ok(bytes) => bytes,
        Err(RegularFileReadError::NotRegular) => {
            return Err(FleetInstallSessionError::UnsafeFile {
                path: path.to_path_buf(),
            });
        }
        Err(RegularFileReadError::Io(source)) => {
            return Err(FleetInstallSessionError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    let Some(bytes) = bytes else {
        return Ok(None);
    };
    if bytes.len() > MAX_SESSION_BYTES {
        return Err(invalid(path, "session exceeds its byte bound"));
    }
    let session = serde_json::from_slice::<FleetInstallSession>(&bytes)
        .map_err(|error| invalid(path, error.to_string()))?;
    validate_session(path, &session)?;
    if encode_session(path, &session)? != bytes {
        return Err(invalid(path, "JSON bytes are not canonical"));
    }
    Ok(Some(session))
}

fn validate_session(
    path: &Path,
    session: &FleetInstallSession,
) -> Result<(), FleetInstallSessionError> {
    if session.schema_version != SESSION_SCHEMA_VERSION {
        return Err(invalid(
            path,
            "unsupported session schema version; export with the matching Canic release before retrying",
        ));
    }
    if session.fleet_name.as_str().is_empty() {
        return Err(invalid(path, "Fleet name must not be empty"));
    }
    if session.operation_id == [0; 32] {
        return Err(invalid(path, "operation identity must not be zero"));
    }
    if !is_canonical_sha256(&session.fresh_fleet_plan_digest) {
        return Err(invalid(
            path,
            "fresh-Fleet plan digest must contain 64 lowercase hexadecimal characters",
        ));
    }
    if session
        .decision_release_build_id
        .is_some_and(|release_build_id| release_build_id != session.release_build_id)
    {
        return Err(invalid(
            path,
            "decision release build differs from finalized install release build",
        ));
    }
    Ok(())
}

fn is_canonical_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn validate_completion(
    path: &Path,
    completion: &FleetInstallCompletionV1,
    session: &FleetInstallSession,
) -> Result<(), FleetInstallSessionError> {
    let entry = &completion.catalog_entry;
    let exact = completion.schema_version == COMPLETION_SCHEMA_VERSION
        && completion.session_schema_version == session.schema_version
        && completion.fleet_name == session.fleet_name
        && completion.fleet == session.fleet
        && completion.release_build_id == session.release_build_id
        && completion.fresh_fleet_plan_digest == session.fresh_fleet_plan_digest
        && completion.fleet_install_operation_id == session.operation_id
        && completion.fleet_install_plan_digest != [0; 32]
        && completion.component_journal_schema_version == JOURNAL_SCHEMA_VERSION
        && completion.component_journal_sequence > 0
        && completion.component_journal_digest != [0; 32]
        && completion.component_provisioning_operation_id != [0; 32]
        && completion.component_provisioning_plan_hash != [0; 32]
        && completion.catalog_hash != [0; 32]
        && entry.canonical_network_id == session.fleet.fleet.canonical_network_id
        && entry.fleet_id == session.fleet.fleet.fleet_id
        && entry.fleet_name == session.fleet_name
        && entry.app == session.fleet.app
        && entry.release_build_id == session.release_build_id;
    if !exact {
        return Err(invalid(
            path,
            "completion differs from the exact terminal session and catalog authority",
        ));
    }
    Ok(())
}

fn encode_completion(
    path: &Path,
    completion: &FleetInstallCompletionV1,
) -> Result<Vec<u8>, FleetInstallSessionError> {
    encode_canonical_json(
        completion,
        CanonicalJsonStyle::PrettyNewline,
        MAX_COMPLETION_BYTES,
    )
    .map_err(|error| match error {
        CanonicalJsonEncodeError::Serialization(error) => invalid(path, error.to_string()),
        CanonicalJsonEncodeError::TooLarge => invalid(path, "completion exceeds its byte bound"),
    })
}

fn load_optional_completion(
    path: &Path,
) -> Result<Option<FleetInstallCompletionV1>, FleetInstallSessionError> {
    let bytes = match read_optional_bounded_regular_bytes(path, MAX_COMPLETION_BYTES) {
        Ok(bytes) => bytes,
        Err(BoundedRegularFileReadError::TooLarge) => {
            return Err(invalid(path, "completion exceeds its byte bound"));
        }
        Err(BoundedRegularFileReadError::Read(RegularFileReadError::NotRegular)) => {
            return Err(FleetInstallSessionError::UnsafeFile {
                path: path.to_path_buf(),
            });
        }
        Err(BoundedRegularFileReadError::Read(RegularFileReadError::Io(source))) => {
            return Err(FleetInstallSessionError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
        #[cfg(not(unix))]
        Err(BoundedRegularFileReadError::Read(RegularFileReadError::UnsupportedPlatform)) => {
            return Err(FleetInstallSessionError::Io {
                path: path.to_path_buf(),
                source: io::Error::new(
                    io::ErrorKind::Unsupported,
                    "Fleet install completion reads are unsupported",
                ),
            });
        }
    };
    let Some(bytes) = bytes else {
        return Ok(None);
    };
    let completion = serde_json::from_slice::<FleetInstallCompletionV1>(&bytes)
        .map_err(|error| invalid(path, error.to_string()))?;
    if encode_completion(path, &completion)? != bytes {
        return Err(invalid(path, "completion JSON bytes are not canonical"));
    }
    Ok(Some(completion))
}

fn session_path(
    root: &Path,
    canonical_network_id: CanonicalNetworkId,
    fleet_name: &FleetName,
) -> PathBuf {
    root.join(".canic")
        .join("recovery")
        .join("fleet-install-sessions")
        .join(canonical_network_id.to_string())
        .join(fleet_name.as_str())
        .join(SESSION_FILE)
}

fn completion_path(root: &Path, session: &FleetInstallSession) -> PathBuf {
    session_path(
        root,
        session.fleet.fleet.canonical_network_id,
        &session.fleet_name,
    )
    .with_file_name(COMPLETION_FILE)
}

fn lock_completion(path: &Path) -> Result<std::fs::File, FleetInstallSessionError> {
    let lock_path = path.with_file_name(COMPLETION_LOCK_FILE);
    lock_regular_file_with_parents(&lock_path).map_err(|error| match error {
        RegularFileLockError::NotRegular => FleetInstallSessionError::UnsafeLock {
            path: lock_path.clone(),
        },
        RegularFileLockError::Io(source) => FleetInstallSessionError::Io {
            path: lock_path.clone(),
            source,
        },
        #[cfg(windows)]
        RegularFileLockError::UnsupportedPlatform => FleetInstallSessionError::Io {
            path: lock_path,
            source: io::Error::new(
                io::ErrorKind::Unsupported,
                "Fleet install completion locking is unsupported on Windows",
            ),
        },
    })
}

fn lock_session(path: &Path) -> Result<std::fs::File, FleetInstallSessionError> {
    let lock_path = path.with_file_name(SESSION_LOCK_FILE);
    lock_regular_file_with_parents(&lock_path).map_err(|error| match error {
        RegularFileLockError::NotRegular => FleetInstallSessionError::UnsafeLock {
            path: lock_path.clone(),
        },
        RegularFileLockError::Io(source) => FleetInstallSessionError::Io {
            path: lock_path.clone(),
            source,
        },
        #[cfg(windows)]
        RegularFileLockError::UnsupportedPlatform => FleetInstallSessionError::Io {
            path: lock_path,
            source: io::Error::new(
                io::ErrorKind::Unsupported,
                "Fleet install session locking is unsupported on Windows",
            ),
        },
    })
}

fn random_identity_bytes() -> Result<[u8; 32], FleetInstallSessionError> {
    random_bytes_32().map_err(|error| match error {
        EntropyError::Io(source) => FleetInstallSessionError::Io {
            path: PathBuf::from("<operating-system entropy>"),
            source,
        },
        EntropyError::ShortRead { actual } => FleetInstallSessionError::ShortRandomRead { actual },
    })
}

fn invalid(path: &Path, reason: impl Into<String>) -> FleetInstallSessionError {
    FleetInstallSessionError::InvalidDocument {
        path: path.to_path_buf(),
        reason: reason.into(),
    }
}
