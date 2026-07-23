//! Module: network
//!
//! Responsibility: enroll and resolve canonical IC network trust identities.
//! Does not own: gateway selection, Fleet identity, or trust-anchor rotation.
//! Boundary: verified trust bytes are authoritative; environment profiles are lookup pointers only.

#[cfg(test)]
mod tests;

use crate::{
    durable_io::create_new_bytes_with_parents,
    icp_config::{IcpConfigError, resolve_icp_build_network_from_root},
};
use canic_core::ids::{BuildNetwork, CanonicalNetworkId};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use sha2::{Digest, Sha256};
use std::{
    fmt::Write as _,
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    time::{SystemTime, SystemTimeError, UNIX_EPOCH},
};
use thiserror::Error as ThisError;

const CANIC_STATE_DIRECTORY: &str = ".canic";
const NETWORKS_DIRECTORY: &str = "networks";
const ENVIRONMENT_PROFILES_DIRECTORY: &str = "environment-profiles";
const ROOT_KEY_RELATIVE_PATH: &str = "trust/root-key.der";
const ENROLLMENT_FILE: &str = "enrollment.json";
const NETWORK_PROFILE_FILE: &str = "network.json";

///
/// NetworkEnrollmentRecord
///
/// Authoritative record binding one canonical network to its enrolled trust anchor.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkEnrollmentRecord {
    #[serde(with = "digest_hex")]
    pub root_key_digest: [u8; 32],
    /// Unix timestamp in seconds.
    pub enrolled_at: u64,
    pub source_profile: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct EnvironmentNetworkProfile {
    canonical_network_id: CanonicalNetworkId,
}

///
/// NetworkEnrollmentOptions
///
/// Inputs to one explicit trust-anchor enrollment.
///

#[derive(Clone, Copy, Debug)]
pub struct NetworkEnrollmentOptions<'a> {
    pub project_root: &'a Path,
    pub environment: &'a str,
    pub root_key: &'a Path,
    pub fingerprint: &'a str,
}

///
/// NetworkEnrollmentReport
///
/// Result of enrolling or confirming one network profile.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkEnrollmentReport {
    pub environment: String,
    pub canonical_network_id: CanonicalNetworkId,
    pub root_key_fingerprint: String,
    pub authority_directory: PathBuf,
    pub profile_path: PathBuf,
    pub created_profile: bool,
}

///
/// NetworkIdentityError
///
/// Typed failure while enrolling or resolving a canonical network identity.
///

#[derive(Debug, ThisError)]
pub enum NetworkIdentityError {
    #[error("invalid ICP environment name {name:?}")]
    InvalidEnvironmentName { name: String },

    #[error(
        "ICP environment {environment:?} resolves to the public IC, whose root trust anchor is compiled into Canic and cannot be enrolled"
    )]
    PublicIcEnrollment { environment: String },

    #[error("root-key fingerprint must contain exactly 64 lowercase hexadecimal characters")]
    InvalidFingerprint,

    #[error(
        "root-key fingerprint mismatch: expected {expected}, observed {observed}; no enrollment was written"
    )]
    FingerprintMismatch { expected: String, observed: String },

    #[error("root key is not a regular non-symlink file: {}", path.display())]
    RootKeyNotRegular { path: PathBuf },

    #[error("root key is not a valid DER-encoded IC root public key: {reason}")]
    InvalidRootKeyDer { reason: String },

    #[error("required network profile is missing: {}", path.display())]
    MissingProfile { path: PathBuf },

    #[error("network profile is not a regular non-symlink file: {}", path.display())]
    ProfileNotRegular { path: PathBuf },

    #[error("required network authority file is missing: {}", path.display())]
    MissingAuthority { path: PathBuf },

    #[error("network authority file is not a regular non-symlink file: {}", path.display())]
    AuthorityNotRegular { path: PathBuf },

    #[error(
        "environment profile {environment:?} is already bound to network {existing}, not {requested}"
    )]
    ProfileConflict {
        environment: String,
        existing: CanonicalNetworkId,
        requested: CanonicalNetworkId,
    },

    #[error("network trust anchor conflicts with the authority at {}", path.display())]
    TrustAnchorConflict { path: PathBuf },

    #[error("network authority is incomplete or contradictory: {reason}")]
    ContradictoryAuthority { reason: String },

    #[error("could not decode network document {}: {source}", path.display())]
    Decode {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("could not encode network document: {0}")]
    Encode(#[from] serde_json::Error),

    #[error("network filesystem operation failed for {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(transparent)]
    IcpConfig(#[from] IcpConfigError),

    #[error("system clock is before the Unix epoch: {0}")]
    Clock(#[from] SystemTimeError),

    #[error("secure network trust files are unsupported on platform {0}")]
    UnsupportedPlatform(&'static str),
}

/// Enroll an exact non-public trust anchor and publish its environment profile.
pub fn enroll_network(
    options: NetworkEnrollmentOptions<'_>,
) -> Result<NetworkEnrollmentReport, NetworkIdentityError> {
    validate_environment_name(options.environment)?;
    if resolve_icp_build_network_from_root(options.project_root, options.environment)?
        == BuildNetwork::Ic
    {
        return Err(NetworkIdentityError::PublicIcEnrollment {
            environment: options.environment.to_string(),
        });
    }

    let expected_digest = parse_fingerprint(options.fingerprint)?;
    let root_key = read_regular_file(options.root_key, FilePurpose::EnrollmentInput)?;
    let canonical_network_id =
        CanonicalNetworkId::from_der_root_trust_anchor(&root_key).map_err(|error| {
            NetworkIdentityError::InvalidRootKeyDer {
                reason: error.to_string(),
            }
        })?;
    let observed_digest = sha256_digest(&root_key);
    if observed_digest != expected_digest {
        return Err(NetworkIdentityError::FingerprintMismatch {
            expected: encode_digest(expected_digest),
            observed: encode_digest(observed_digest),
        });
    }

    let paths = NetworkPaths::new(
        options.project_root,
        options.environment,
        canonical_network_id,
    );
    let existing_profile = read_optional_profile(&paths.profile)?;
    if let Some(profile) = &existing_profile
        && profile.canonical_network_id != canonical_network_id
    {
        return Err(NetworkIdentityError::ProfileConflict {
            environment: options.environment.to_string(),
            existing: profile.canonical_network_id,
            requested: canonical_network_id,
        });
    }

    let existing_root_key = read_optional_regular_file(&paths.root_key)?;
    if existing_root_key
        .as_deref()
        .is_some_and(|existing| existing != root_key)
    {
        return Err(NetworkIdentityError::TrustAnchorConflict {
            path: paths.root_key,
        });
    }
    let existing_enrollment = read_optional_json::<NetworkEnrollmentRecord>(&paths.enrollment)?;
    validate_existing_authority(
        &paths,
        observed_digest,
        canonical_network_id,
        existing_root_key.as_deref(),
        existing_enrollment.as_ref(),
        existing_profile.as_ref(),
    )?;

    if existing_root_key.is_none() {
        create_new(&paths.root_key, &root_key)?;
    }
    if existing_enrollment.is_none() {
        let enrollment = NetworkEnrollmentRecord {
            root_key_digest: observed_digest,
            enrolled_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            source_profile: options.environment.to_string(),
        };
        create_new_enrollment(&paths, &enrollment)?;
    }

    let created_profile = existing_profile.is_none();
    if created_profile {
        create_new_profile(&paths, options.environment)?;
    }

    Ok(NetworkEnrollmentReport {
        environment: options.environment.to_string(),
        canonical_network_id,
        root_key_fingerprint: encode_digest(observed_digest),
        authority_directory: paths.authority_directory,
        profile_path: paths.profile,
        created_profile,
    })
}

/// Resolve an environment profile to its verified canonical network identity.
pub fn resolve_canonical_network_id_from_root(
    project_root: &Path,
    environment: &str,
) -> Result<CanonicalNetworkId, NetworkIdentityError> {
    validate_environment_name(environment)?;
    let build_network = resolve_icp_build_network_from_root(project_root, environment)?;
    let profile_path = environment_profile_path(project_root, environment);

    if build_network == BuildNetwork::Ic {
        let expected = CanonicalNetworkId::public_ic();
        if let Some(profile) = read_optional_profile(&profile_path)?
            && profile.canonical_network_id != expected
        {
            return Err(NetworkIdentityError::ProfileConflict {
                environment: environment.to_string(),
                existing: profile.canonical_network_id,
                requested: expected,
            });
        }
        return Ok(expected);
    }

    let profile = read_required_profile(&profile_path)?;
    let paths = NetworkPaths::new(project_root, environment, profile.canonical_network_id);
    let root_key = read_required_regular_file(&paths.root_key)?;
    let observed_network_id =
        CanonicalNetworkId::from_der_root_trust_anchor(&root_key).map_err(|error| {
            NetworkIdentityError::InvalidRootKeyDer {
                reason: error.to_string(),
            }
        })?;
    let observed_digest = sha256_digest(&root_key);
    let enrollment = read_required_json::<NetworkEnrollmentRecord>(&paths.enrollment)?;
    validate_environment_name(&enrollment.source_profile)?;
    validate_complete_authority(
        profile.canonical_network_id,
        observed_digest,
        observed_network_id,
        &enrollment,
        &paths,
    )?;
    Ok(profile.canonical_network_id)
}

fn validate_existing_authority(
    paths: &NetworkPaths,
    expected_digest: [u8; 32],
    observed_network_id: CanonicalNetworkId,
    root_key: Option<&[u8]>,
    enrollment: Option<&NetworkEnrollmentRecord>,
    profile: Option<&EnvironmentNetworkProfile>,
) -> Result<(), NetworkIdentityError> {
    if enrollment.is_some() && root_key.is_none() {
        return Err(NetworkIdentityError::ContradictoryAuthority {
            reason: format!(
                "{} exists without {}",
                paths.enrollment.display(),
                paths.root_key.display()
            ),
        });
    }
    if profile.is_some() && (root_key.is_none() || enrollment.is_none()) {
        return Err(NetworkIdentityError::ContradictoryAuthority {
            reason: format!(
                "{} is visible without a complete authority",
                paths.profile.display()
            ),
        });
    }
    if let Some(enrollment) = enrollment {
        validate_environment_name(&enrollment.source_profile)?;
        validate_complete_authority(
            paths.canonical_network_id,
            expected_digest,
            observed_network_id,
            enrollment,
            paths,
        )?;
    }
    Ok(())
}

fn validate_complete_authority(
    canonical_network_id: CanonicalNetworkId,
    root_key_digest: [u8; 32],
    observed_network_id: CanonicalNetworkId,
    enrollment: &NetworkEnrollmentRecord,
    paths: &NetworkPaths,
) -> Result<(), NetworkIdentityError> {
    if enrollment.root_key_digest != root_key_digest {
        return Err(NetworkIdentityError::ContradictoryAuthority {
            reason: format!(
                "{} does not match the exact root trust anchor",
                paths.enrollment.display()
            ),
        });
    }
    if observed_network_id != canonical_network_id {
        return Err(NetworkIdentityError::ContradictoryAuthority {
            reason: format!(
                "{} derives network {observed_network_id}, not {canonical_network_id}",
                paths.root_key.display()
            ),
        });
    }
    Ok(())
}

fn validate_environment_name(name: &str) -> Result<(), NetworkIdentityError> {
    if !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        Ok(())
    } else {
        Err(NetworkIdentityError::InvalidEnvironmentName {
            name: name.to_string(),
        })
    }
}

fn sha256_digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn parse_fingerprint(value: &str) -> Result<[u8; 32], NetworkIdentityError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(NetworkIdentityError::InvalidFingerprint);
    }
    let mut digest = [0; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        digest[index] = (decode_nibble(pair[0]) << 4) | decode_nibble(pair[1]);
    }
    Ok(digest)
}

fn decode_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => unreachable!("fingerprint was validated before decoding"),
    }
}

fn encode_digest(digest: [u8; 32]) -> String {
    digest
        .iter()
        .fold(String::with_capacity(64), |mut encoded, byte| {
            write!(encoded, "{byte:02x}").expect("writing to a String cannot fail");
            encoded
        })
}

fn create_new(path: &Path, bytes: &[u8]) -> Result<(), NetworkIdentityError> {
    match create_new_bytes_with_parents(path, bytes) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {
            let existing = read_required_regular_file(path)?;
            if existing == bytes {
                Ok(())
            } else {
                Err(NetworkIdentityError::TrustAnchorConflict {
                    path: path.to_path_buf(),
                })
            }
        }
        Err(source) => Err(NetworkIdentityError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn create_new_enrollment(
    paths: &NetworkPaths,
    enrollment: &NetworkEnrollmentRecord,
) -> Result<(), NetworkIdentityError> {
    let path = &paths.enrollment;
    let bytes = encode_json(enrollment)?;
    match create_new_bytes_with_parents(path, &bytes) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {
            let existing = read_required_json::<NetworkEnrollmentRecord>(path)?;
            validate_environment_name(&existing.source_profile)?;
            validate_complete_authority(
                paths.canonical_network_id,
                enrollment.root_key_digest,
                paths.canonical_network_id,
                &existing,
                paths,
            )
        }
        Err(source) => Err(NetworkIdentityError::Io {
            path: path.clone(),
            source,
        }),
    }
}

fn create_new_profile(paths: &NetworkPaths, environment: &str) -> Result<(), NetworkIdentityError> {
    let path = &paths.profile;
    let profile = EnvironmentNetworkProfile {
        canonical_network_id: paths.canonical_network_id,
    };
    let bytes = encode_json(&profile)?;
    match create_new_bytes_with_parents(path, &bytes) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {
            let existing = read_required_profile(path)?;
            if existing == profile {
                Ok(())
            } else {
                Err(NetworkIdentityError::ProfileConflict {
                    environment: environment.to_string(),
                    existing: existing.canonical_network_id,
                    requested: profile.canonical_network_id,
                })
            }
        }
        Err(source) => Err(NetworkIdentityError::Io {
            path: path.clone(),
            source,
        }),
    }
}

fn encode_json<T: Serialize>(value: &T) -> Result<Vec<u8>, NetworkIdentityError> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn read_required_json<T: for<'de> Deserialize<'de>>(
    path: &Path,
) -> Result<T, NetworkIdentityError> {
    let bytes = read_required_regular_file(path)?;
    serde_json::from_slice(&bytes).map_err(|source| NetworkIdentityError::Decode {
        path: path.to_path_buf(),
        source,
    })
}

fn read_optional_json<T: for<'de> Deserialize<'de>>(
    path: &Path,
) -> Result<Option<T>, NetworkIdentityError> {
    let Some(bytes) = read_optional_regular_file(path)? else {
        return Ok(None);
    };
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|source| NetworkIdentityError::Decode {
            path: path.to_path_buf(),
            source,
        })
}

fn read_required_profile(path: &Path) -> Result<EnvironmentNetworkProfile, NetworkIdentityError> {
    read_optional_profile(path)?.ok_or_else(|| NetworkIdentityError::MissingProfile {
        path: path.to_path_buf(),
    })
}

fn read_optional_profile(
    path: &Path,
) -> Result<Option<EnvironmentNetworkProfile>, NetworkIdentityError> {
    let bytes = match read_regular_file(path, FilePurpose::Profile) {
        Ok(bytes) => bytes,
        Err(NetworkIdentityError::Io { source, .. })
            if source.kind() == io::ErrorKind::NotFound =>
        {
            return Ok(None);
        }
        Err(error) => return Err(error),
    };
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|source| NetworkIdentityError::Decode {
            path: path.to_path_buf(),
            source,
        })
}

fn read_required_regular_file(path: &Path) -> Result<Vec<u8>, NetworkIdentityError> {
    read_optional_regular_file(path)?.ok_or_else(|| NetworkIdentityError::MissingAuthority {
        path: path.to_path_buf(),
    })
}

fn read_optional_regular_file(path: &Path) -> Result<Option<Vec<u8>>, NetworkIdentityError> {
    match read_regular_file(path, FilePurpose::Authority) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(NetworkIdentityError::Io { source, .. })
            if source.kind() == io::ErrorKind::NotFound =>
        {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

#[derive(Clone, Copy)]
enum FilePurpose {
    EnrollmentInput,
    Authority,
    Profile,
}

fn read_regular_file(path: &Path, purpose: FilePurpose) -> Result<Vec<u8>, NetworkIdentityError> {
    #[cfg(unix)]
    {
        use rustix::{
            fd::OwnedFd,
            fs::{FileType, Mode, OFlags},
        };

        let metadata = fs::symlink_metadata(path).map_err(|source| NetworkIdentityError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        if !metadata.file_type().is_file() {
            return Err(non_regular_file_error(path, purpose));
        }

        let fd: OwnedFd = rustix::fs::open(
            path,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|error| NetworkIdentityError::Io {
            path: path.to_path_buf(),
            source: io::Error::from_raw_os_error(error.raw_os_error()),
        })?;
        let metadata = rustix::fs::fstat(&fd).map_err(|error| NetworkIdentityError::Io {
            path: path.to_path_buf(),
            source: io::Error::from_raw_os_error(error.raw_os_error()),
        })?;
        if FileType::from_raw_mode(metadata.st_mode) != FileType::RegularFile {
            return Err(non_regular_file_error(path, purpose));
        }
        let mut file = fs::File::from(fd);
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|source| NetworkIdentityError::Io {
                path: path.to_path_buf(),
                source,
            })?;
        Ok(bytes)
    }

    #[cfg(not(unix))]
    {
        let _ = (path, purpose);
        Err(NetworkIdentityError::UnsupportedPlatform(
            std::env::consts::OS,
        ))
    }
}

fn non_regular_file_error(path: &Path, purpose: FilePurpose) -> NetworkIdentityError {
    match purpose {
        FilePurpose::EnrollmentInput => NetworkIdentityError::RootKeyNotRegular {
            path: path.to_path_buf(),
        },
        FilePurpose::Authority => NetworkIdentityError::AuthorityNotRegular {
            path: path.to_path_buf(),
        },
        FilePurpose::Profile => NetworkIdentityError::ProfileNotRegular {
            path: path.to_path_buf(),
        },
    }
}

fn environment_profile_path(project_root: &Path, environment: &str) -> PathBuf {
    project_root
        .join(CANIC_STATE_DIRECTORY)
        .join(ENVIRONMENT_PROFILES_DIRECTORY)
        .join(environment)
        .join(NETWORK_PROFILE_FILE)
}

struct NetworkPaths {
    canonical_network_id: CanonicalNetworkId,
    authority_directory: PathBuf,
    root_key: PathBuf,
    enrollment: PathBuf,
    profile: PathBuf,
}

impl NetworkPaths {
    fn new(
        project_root: &Path,
        environment: &str,
        canonical_network_id: CanonicalNetworkId,
    ) -> Self {
        let authority_directory = project_root
            .join(CANIC_STATE_DIRECTORY)
            .join(NETWORKS_DIRECTORY)
            .join(canonical_network_id.to_string());
        Self {
            canonical_network_id,
            root_key: authority_directory.join(ROOT_KEY_RELATIVE_PATH),
            enrollment: authority_directory.join(ENROLLMENT_FILE),
            profile: environment_profile_path(project_root, environment),
            authority_directory,
        }
    }
}

mod digest_hex {
    use super::*;

    pub fn serialize<S>(digest: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&encode_digest(*digest))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        parse_fingerprint(&value).map_err(de::Error::custom)
    }
}
