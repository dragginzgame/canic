//! Module: install_root::fleet_install_session
//!
//! Responsibility: own the durable Fleet identity and install operation for one fresh install.
//! Does not own: Coordinator, Fleet Subnet Root, Store, Registry, or activation effects.
//! Boundary: exact same-release retries recover one immutable session before any paid effect.

#[cfg(test)]
mod tests;

use crate::{
    durable_io::{
        RegularFileLockError, RegularFileReadError, create_new_bytes_with_parents,
        lock_regular_file_with_parents, read_optional_regular_bytes,
    },
    entropy::{EntropyError, random_bytes_32},
    release_build::{
        FinalizedReleaseBuild, ReleaseBuildPlanError, ReleaseBuildPlanState,
        load_finalized_release_build,
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

/// Recover the finalized release-build authority retained by an existing exact session.
pub(super) fn recover_fleet_install_session_release_build(
    root: &Path,
    canonical_network_id: CanonicalNetworkId,
    fleet_name: &FleetName,
    app: &AppId,
) -> Result<Option<FinalizedReleaseBuild>, FleetInstallSessionError> {
    let path = session_path(root, canonical_network_id, fleet_name);
    let _lock = lock_session(&path)?;
    let Some(session) = load_optional_session(&path)? else {
        return Ok(None);
    };
    if session.fleet_name != *fleet_name
        || session.fleet.fleet.canonical_network_id != canonical_network_id
        || session.fleet.app != *app
    {
        return Err(FleetInstallSessionError::ConflictingAuthority { path });
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
            &path,
            "session release-build evidence differs from its finalized authority",
        ));
    }

    Ok(Some(finalized))
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
        return Err(invalid(path, "unsupported schema version"));
    }
    if session.fleet_name.as_str().is_empty() {
        return Err(invalid(path, "Fleet name must not be empty"));
    }
    if session.operation_id == [0; 32] {
        return Err(invalid(path, "operation identity must not be zero"));
    }
    Ok(())
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
