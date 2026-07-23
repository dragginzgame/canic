//! Module: release_build
//!
//! Responsibility: create, validate, finalize, and hash one durable release-build plan.
//! Does not own: artifact compilation, release-set manifest construction, or deployment recovery.
//! Boundary: a random nonce is durable before building and finalization binds exact manifest bytes.

#[cfg(test)]
mod tests;

use crate::durable_io::{
    RegularFileReadError, create_new_bytes_with_parents, read_optional_regular_bytes, write_bytes,
};
use canic_core::ids::{ReleaseBuildId, ReleaseBuildNonce};
use ciborium::Value;
use sha2::{Digest, Sha256};
use std::{
    io,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

const PLAN_HASH_DOMAIN: &[u8] = b"canic:release-build:plan\0";
const RANDOM_ATTEMPTS: usize = 16;

///
/// ReleaseBuildPlanState
///
/// Monotonic state of one pre-build identity and its post-build manifest evidence.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseBuildPlanState {
    Planned,
    Finalized {
        release_set_manifest_digest: [u8; 32],
    },
}

///
/// ReleaseBuildPlanRecord
///
/// Canonical durable owner of the random release-build nonce.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReleaseBuildPlanRecord {
    pub nonce: ReleaseBuildNonce,
    pub release_build_id: ReleaseBuildId,
    pub state: ReleaseBuildPlanState,
}

///
/// PlannedReleaseBuild
///
/// Exact planned record and durable path supplied to the artifact builder.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedReleaseBuild {
    pub record: ReleaseBuildPlanRecord,
    pub path: PathBuf,
}

///
/// FinalizedReleaseBuild
///
/// Immutable post-build evidence admitted by fresh-install recovery.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FinalizedReleaseBuild {
    pub record: ReleaseBuildPlanRecord,
    pub plan_hash: [u8; 32],
    pub path: PathBuf,
}

///
/// ReleaseBuildPlanError
///
/// Typed failure at the durable release-build authority boundary.
///

#[derive(Debug, ThisError)]
pub enum ReleaseBuildPlanError {
    #[error("failed to access release-build plan {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("release-build plan is not a regular no-follow file: {path}")]
    UnsafeFile { path: PathBuf },

    #[error("release-build plan is missing: {path}")]
    Missing { path: PathBuf },

    #[error("invalid release-build plan {path}: {reason}")]
    InvalidDocument { path: PathBuf, reason: String },

    #[error(
        "release-build plan path identity {expected} does not match record identity {recorded}"
    )]
    PathIdentityMismatch {
        expected: ReleaseBuildId,
        recorded: ReleaseBuildId,
    },

    #[error(
        "release-build plan record identity {recorded} does not derive from nonce as {derived}"
    )]
    NonceIdentityMismatch {
        derived: ReleaseBuildId,
        recorded: ReleaseBuildId,
    },

    #[error(
        "release-build plan {release_build_id} is already finalized with a different manifest digest"
    )]
    ConflictingFinalization { release_build_id: ReleaseBuildId },

    #[error("cryptographic random source returned only {actual} of 32 required bytes")]
    ShortRandomRead { actual: usize },

    #[error("could not allocate a unique release-build identity after {RANDOM_ATTEMPTS} attempts")]
    IdentityAllocationExhausted,

    #[error("normal install cannot mutate a canister without a finalized release-build plan")]
    MissingFinalizedAuthority,
}

/// Create and durably publish one new random release-build plan.
pub fn plan_release_build(root: &Path) -> Result<PlannedReleaseBuild, ReleaseBuildPlanError> {
    for _ in 0..RANDOM_ATTEMPTS {
        let nonce = random_nonce()?;
        match plan_release_build_with_nonce(root, nonce) {
            Ok(plan) => return Ok(plan),
            Err(ReleaseBuildPlanError::Io { source, .. })
                if source.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }

    Err(ReleaseBuildPlanError::IdentityAllocationExhausted)
}

/// Load and validate one exact planned or finalized release-build record.
pub fn load_release_build_plan(
    root: &Path,
    release_build_id: ReleaseBuildId,
) -> Result<ReleaseBuildPlanRecord, ReleaseBuildPlanError> {
    let path = release_build_plan_path(root, release_build_id);
    let bytes = read_plan_bytes(&path)?;
    let record = decode_record(&path, &bytes)?;
    validate_record_identity(release_build_id, &record)?;
    Ok(record)
}

/// Load one immutable finalized record for deployment-recovery admission.
pub fn load_finalized_release_build(
    root: &Path,
    release_build_id: ReleaseBuildId,
) -> Result<FinalizedReleaseBuild, ReleaseBuildPlanError> {
    let path = release_build_plan_path(root, release_build_id);
    let record = load_release_build_plan(root, release_build_id)?;
    finalized_record(path, record)
}

/// Finalize one planned build from the exact durable release-set manifest bytes.
pub fn finalize_release_build_from_manifest(
    root: &Path,
    release_build_id: ReleaseBuildId,
    manifest_path: &Path,
) -> Result<FinalizedReleaseBuild, ReleaseBuildPlanError> {
    let manifest_bytes = read_plan_bytes(manifest_path)?;
    let digest = Sha256::digest(&manifest_bytes).into();
    finalize_release_build(root, release_build_id, digest)
}

/// Return the canonical durable path for one release-build plan.
#[must_use]
pub fn release_build_plan_path(root: &Path, release_build_id: ReleaseBuildId) -> PathBuf {
    root.join(".canic")
        .join("release-builds")
        .join(release_build_id.to_string())
        .join("plan.cbor")
}

fn plan_release_build_with_nonce(
    root: &Path,
    nonce: ReleaseBuildNonce,
) -> Result<PlannedReleaseBuild, ReleaseBuildPlanError> {
    let release_build_id = ReleaseBuildId::from_nonce(nonce);
    let record = ReleaseBuildPlanRecord {
        nonce,
        release_build_id,
        state: ReleaseBuildPlanState::Planned,
    };
    let path = release_build_plan_path(root, release_build_id);
    let bytes = encode_record(record);
    create_new_bytes_with_parents(&path, &bytes).map_err(|source| ReleaseBuildPlanError::Io {
        path: path.clone(),
        source,
    })?;
    Ok(PlannedReleaseBuild { record, path })
}

fn finalize_release_build(
    root: &Path,
    release_build_id: ReleaseBuildId,
    release_set_manifest_digest: [u8; 32],
) -> Result<FinalizedReleaseBuild, ReleaseBuildPlanError> {
    let path = release_build_plan_path(root, release_build_id);
    let _plan_lock = lock_plan_file(&path)?;
    let mut record = load_release_build_plan(root, release_build_id)?;
    match record.state {
        ReleaseBuildPlanState::Planned => {}
        ReleaseBuildPlanState::Finalized {
            release_set_manifest_digest: observed,
        } if observed == release_set_manifest_digest => {
            return finalized_record(path, record);
        }
        ReleaseBuildPlanState::Finalized { .. } => {
            return Err(ReleaseBuildPlanError::ConflictingFinalization { release_build_id });
        }
    }

    record.state = ReleaseBuildPlanState::Finalized {
        release_set_manifest_digest,
    };
    let bytes = encode_record(record);
    if let Err(source) = write_bytes(&path, &bytes) {
        // A final parent-sync error may still have published the complete new
        // record. Re-read the authority before projecting failure.
        match load_release_build_plan(root, release_build_id) {
            Ok(observed) if observed == record => return finalized_record(path, observed),
            _ => {
                return Err(ReleaseBuildPlanError::Io {
                    path: path.clone(),
                    source,
                });
            }
        }
    }

    let observed = load_release_build_plan(root, release_build_id)?;
    if observed != record {
        return Err(ReleaseBuildPlanError::ConflictingFinalization { release_build_id });
    }
    finalized_record(path, observed)
}

fn finalized_record(
    path: PathBuf,
    record: ReleaseBuildPlanRecord,
) -> Result<FinalizedReleaseBuild, ReleaseBuildPlanError> {
    if !matches!(record.state, ReleaseBuildPlanState::Finalized { .. }) {
        return Err(ReleaseBuildPlanError::InvalidDocument {
            path,
            reason: "finalized release-build evidence remained planned".to_string(),
        });
    }
    let bytes = encode_record(record);
    Ok(FinalizedReleaseBuild {
        record,
        plan_hash: domain_hash(PLAN_HASH_DOMAIN, &bytes),
        path,
    })
}

fn validate_record_identity(
    expected: ReleaseBuildId,
    record: &ReleaseBuildPlanRecord,
) -> Result<(), ReleaseBuildPlanError> {
    let derived = ReleaseBuildId::from_nonce(record.nonce);
    if derived != record.release_build_id {
        return Err(ReleaseBuildPlanError::NonceIdentityMismatch {
            derived,
            recorded: record.release_build_id,
        });
    }
    if expected != record.release_build_id {
        return Err(ReleaseBuildPlanError::PathIdentityMismatch {
            expected,
            recorded: record.release_build_id,
        });
    }
    Ok(())
}

fn encode_record(record: ReleaseBuildPlanRecord) -> Vec<u8> {
    let state = match record.state {
        ReleaseBuildPlanState::Planned => Value::Array(vec![Value::Integer(0.into())]),
        ReleaseBuildPlanState::Finalized {
            release_set_manifest_digest,
        } => Value::Array(vec![
            Value::Integer(1.into()),
            Value::Bytes(release_set_manifest_digest.to_vec()),
        ]),
    };
    let value = Value::Array(vec![
        Value::Bytes(record.nonce.as_bytes().to_vec()),
        Value::Bytes(record.release_build_id.as_bytes().to_vec()),
        state,
    ]);
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(&value, &mut bytes)
        .expect("serializing a fixed release-build CBOR value cannot fail");
    bytes
}

fn decode_record(
    path: &Path,
    bytes: &[u8],
) -> Result<ReleaseBuildPlanRecord, ReleaseBuildPlanError> {
    let value: Value =
        ciborium::de::from_reader(bytes).map_err(|error| invalid(path, error.to_string()))?;
    let fields = exact_array(path, value, 3, "record")?;
    let nonce = ReleaseBuildNonce::from_random_bytes(exact_digest(path, &fields[0], "nonce")?);
    let release_build_id = exact_digest(path, &fields[1], "release_build_id")?
        .iter()
        .fold(String::with_capacity(64), |mut text, byte| {
            use std::fmt::Write as _;
            write!(text, "{byte:02x}").expect("writing to String cannot fail");
            text
        })
        .parse()
        .expect("lowercase digest text is a canonical release-build ID");
    let state_fields = exact_array_ref(path, &fields[2], "state")?;
    let discriminant = state_fields
        .first()
        .and_then(Value::as_integer)
        .and_then(|value| u8::try_from(value).ok())
        .ok_or_else(|| invalid(path, "state discriminant must be an unsigned integer"))?;
    let state = match (discriminant, state_fields) {
        (0, [_]) => ReleaseBuildPlanState::Planned,
        (1, [_, digest]) => ReleaseBuildPlanState::Finalized {
            release_set_manifest_digest: exact_digest(path, digest, "release_set_manifest_digest")?,
        },
        _ => return Err(invalid(path, "state has an unknown or invalid shape")),
    };
    let record = ReleaseBuildPlanRecord {
        nonce,
        release_build_id,
        state,
    };
    if encode_record(record) != bytes {
        return Err(invalid(path, "CBOR bytes are not canonical"));
    }
    Ok(record)
}

fn exact_array(
    path: &Path,
    value: Value,
    len: usize,
    field: &str,
) -> Result<Vec<Value>, ReleaseBuildPlanError> {
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
    field: &str,
) -> Result<&'a [Value], ReleaseBuildPlanError> {
    let Value::Array(values) = value else {
        return Err(invalid(path, format!("{field} must be an array")));
    };
    Ok(values)
}

fn exact_digest(
    path: &Path,
    value: &Value,
    field: &str,
) -> Result<[u8; 32], ReleaseBuildPlanError> {
    let Value::Bytes(bytes) = value else {
        return Err(invalid(path, format!("{field} must be a byte string")));
    };
    bytes
        .as_slice()
        .try_into()
        .map_err(|_| invalid(path, format!("{field} must contain exactly 32 bytes")))
}

fn read_plan_bytes(path: &Path) -> Result<Vec<u8>, ReleaseBuildPlanError> {
    match read_optional_regular_bytes(path) {
        Ok(Some(bytes)) => Ok(bytes),
        Ok(None) => Err(ReleaseBuildPlanError::Missing {
            path: path.to_path_buf(),
        }),
        Err(RegularFileReadError::NotRegular) => Err(ReleaseBuildPlanError::UnsafeFile {
            path: path.to_path_buf(),
        }),
        Err(RegularFileReadError::Io(source)) => Err(ReleaseBuildPlanError::Io {
            path: path.to_path_buf(),
            source,
        }),
        #[cfg(not(unix))]
        Err(RegularFileReadError::UnsupportedPlatform) => Err(ReleaseBuildPlanError::Io {
            path: path.to_path_buf(),
            source: io::Error::new(
                io::ErrorKind::Unsupported,
                "no-follow release-build reads are unsupported on this platform",
            ),
        }),
    }
}

fn lock_plan_file(path: &Path) -> Result<std::fs::File, ReleaseBuildPlanError> {
    #[cfg(not(windows))]
    {
        use rustix::{
            fd::OwnedFd,
            fs::{FileType, FlockOperation, Mode, OFlags, flock, fstat, open},
        };

        let fd: OwnedFd = open(
            path,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|source| ReleaseBuildPlanError::Io {
            path: path.to_path_buf(),
            source: io::Error::from_raw_os_error(source.raw_os_error()),
        })?;
        let metadata = fstat(&fd).map_err(|source| ReleaseBuildPlanError::Io {
            path: path.to_path_buf(),
            source: io::Error::from_raw_os_error(source.raw_os_error()),
        })?;
        if FileType::from_raw_mode(metadata.st_mode) != FileType::RegularFile {
            return Err(ReleaseBuildPlanError::UnsafeFile {
                path: path.to_path_buf(),
            });
        }
        let file = std::fs::File::from(fd);
        flock(&file, FlockOperation::LockExclusive).map_err(|source| {
            ReleaseBuildPlanError::Io {
                path: path.to_path_buf(),
                source: io::Error::from_raw_os_error(source.raw_os_error()),
            }
        })?;
        Ok(file)
    }

    #[cfg(windows)]
    {
        Err(ReleaseBuildPlanError::Io {
            path: path.to_path_buf(),
            source: io::Error::new(
                io::ErrorKind::Unsupported,
                "release-build plan locking is unsupported on Windows",
            ),
        })
    }
}

fn random_nonce() -> Result<ReleaseBuildNonce, ReleaseBuildPlanError> {
    #[cfg(not(windows))]
    {
        use rustix::rand::{GetRandomFlags, getrandom};

        let mut bytes = [0; 32];
        let mut filled = 0;
        while filled < bytes.len() {
            let current =
                getrandom(&mut bytes[filled..], GetRandomFlags::empty()).map_err(|source| {
                    ReleaseBuildPlanError::Io {
                        path: PathBuf::from("<operating-system random source>"),
                        source: io::Error::from_raw_os_error(source.raw_os_error()),
                    }
                })?;
            if current == 0 {
                return Err(ReleaseBuildPlanError::ShortRandomRead { actual: filled });
            }
            filled += current;
        }
        Ok(ReleaseBuildNonce::from_random_bytes(bytes))
    }

    #[cfg(windows)]
    {
        Err(ReleaseBuildPlanError::Io {
            path: PathBuf::from("<operating-system random source>"),
            source: io::Error::new(
                io::ErrorKind::Unsupported,
                "release-build planning is unsupported on Windows",
            ),
        })
    }
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

fn invalid(path: &Path, reason: impl Into<String>) -> ReleaseBuildPlanError {
    ReleaseBuildPlanError::InvalidDocument {
        path: path.to_path_buf(),
        reason: reason.into(),
    }
}
