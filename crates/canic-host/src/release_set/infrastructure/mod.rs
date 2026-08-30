//! Module: release_set::infrastructure
//!
//! Responsibility: compile and validate the exact Canic infrastructure artifact manifest.
//! Does not own: Cargo execution, placement, installation, or application release sets.
//! Boundary: derives immutable artifact evidence from one qualified release build.

mod persistence;
#[cfg(test)]
mod tests;

use std::{collections::BTreeSet, io::Read};

use canic_core::{
    cdk::utils::hash::{decode_hex, sha256_hex},
    ids::{CanisterRole, ReleaseBuildId},
    role_contract::{ProtocolProfileDigest, RoleCapabilityKey},
};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error as ThisError;

use super::{GZIP_MAGIC, WASM_MAGIC, valid_package_name, validate_release_artifact_relative_path};

pub use persistence::verify_persisted_canic_infrastructure_artifact;
pub use persistence::{
    CanicInfrastructureArtifactBuildOutput, CanicInfrastructureArtifactPersistenceError,
    PersistedCanicInfrastructureArtifactManifest,
    compile_and_persist_canic_infrastructure_artifact_manifest,
    load_persisted_canic_infrastructure_artifact_manifest,
};

const SHA_256_HEX_BYTES: usize = 64;
const MAX_ARTIFACT_PATH_BYTES: usize = 4_096;
const REQUIRED_INFRASTRUCTURE_ROLES: [CanicInfrastructureRole; 4] = [
    CanicInfrastructureRole::FleetCoordinator,
    CanicInfrastructureRole::FleetSubnetRoot,
    CanicInfrastructureRole::PoolLedgerRecovery,
    CanicInfrastructureRole::WasmStore,
];

///
/// CanicInfrastructureRole
///
/// Exact infrastructure artifact roles installed outside Component topology.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CanicInfrastructureRole {
    FleetCoordinator,
    FleetSubnetRoot,
    PoolLedgerRecovery,
    WasmStore,
}

impl CanicInfrastructureRole {
    /// Return the canonical role identity used in artifact paths and evidence.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FleetCoordinator => "fleet_coordinator",
            Self::FleetSubnetRoot => "fleet_subnet_root",
            Self::PoolLedgerRecovery => "pool_ledger_recovery",
            Self::WasmStore => "wasm_store",
        }
    }

    /// Return the canonical compiled protocol role selected by this artifact.
    #[must_use]
    pub const fn protocol_role_name(self) -> &'static str {
        match self {
            Self::FleetCoordinator => "fleet_coordinator",
            Self::FleetSubnetRoot => "root",
            Self::PoolLedgerRecovery => "pool_ledger_recovery",
            Self::WasmStore => "wasm_store",
        }
    }
}

///
/// CanicInfrastructureArtifactInput
///
/// One current-build artifact supplied to the manifest compiler.
///

#[derive(Clone, Copy, Debug)]
pub struct CanicInfrastructureArtifactInput<'a> {
    pub role: CanicInfrastructureRole,
    pub package: &'a str,
    pub protocol_release_identity: &'a str,
    pub protocol_role: &'a CanisterRole,
    pub protocol_capabilities: &'a BTreeSet<RoleCapabilityKey>,
    pub release_build_id: ReleaseBuildId,
    pub wasm_relative_path: &'a str,
    pub wasm: &'a [u8],
    pub wasm_gz_relative_path: &'a str,
    pub wasm_gz: &'a [u8],
    pub candid_sha256: [u8; 32],
    pub protocol_profile_digest: ProtocolProfileDigest,
}

///
/// CanicInfrastructureArtifactManifest
///
/// Canonical infrastructure and temporary-support artifact authority for one release build.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CanicInfrastructureArtifactManifest {
    pub release_build_id: ReleaseBuildId,
    pub entries: Vec<CanicInfrastructureArtifactEntry>,
}

impl CanicInfrastructureArtifactManifest {
    /// Compile exact artifact evidence without accepting caller-supplied hashes.
    pub fn compile(
        release_build_id: ReleaseBuildId,
        inputs: &[CanicInfrastructureArtifactInput<'_>],
    ) -> Result<Self, CanicInfrastructureArtifactManifestError> {
        let mut entries = inputs
            .iter()
            .map(|input| compile_entry(release_build_id, input))
            .collect::<Result<Vec<_>, _>>()?;
        entries.sort_unstable_by_key(|entry| entry.role);

        let manifest = Self {
            release_build_id,
            entries,
        };
        manifest.validate()?;
        Ok(manifest)
    }

    /// Validate the canonical role, build, package, path, size, and digest shape.
    pub fn validate(&self) -> Result<(), CanicInfrastructureArtifactManifestError> {
        let actual_roles = self
            .entries
            .iter()
            .map(|entry| entry.role)
            .collect::<Vec<_>>();
        if actual_roles != REQUIRED_INFRASTRUCTURE_ROLES {
            return Err(
                CanicInfrastructureArtifactManifestError::InfrastructureRoleSet {
                    actual: actual_roles,
                },
            );
        }

        let mut paths = BTreeSet::new();
        for entry in &self.entries {
            validate_entry(self.release_build_id, entry)?;
            for path in [&entry.wasm_relative_path, &entry.wasm_gz_relative_path] {
                if !paths.insert(path.as_str()) {
                    return Err(
                        CanicInfrastructureArtifactManifestError::DuplicateArtifactPath {
                            path: path.clone(),
                        },
                    );
                }
            }
        }

        Ok(())
    }

    /// Encode the validated manifest into deterministic compact JSON bytes.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, CanicInfrastructureArtifactManifestError> {
        self.validate()?;
        serde_json::to_vec(self).map_err(CanicInfrastructureArtifactManifestError::Serialization)
    }

    /// Hash the exact canonical manifest bytes.
    pub fn digest(&self) -> Result<[u8; 32], CanicInfrastructureArtifactManifestError> {
        Ok(Sha256::digest(self.canonical_bytes()?).into())
    }
}

///
/// CanicInfrastructureArtifactEntry
///
/// Exact package, release-build, path, size, and digest evidence for one role.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CanicInfrastructureArtifactEntry {
    pub role: CanicInfrastructureRole,
    pub package: String,
    pub protocol_release_identity: String,
    pub protocol_role: CanisterRole,
    pub protocol_capabilities: BTreeSet<RoleCapabilityKey>,
    pub release_build_id: ReleaseBuildId,
    pub wasm_relative_path: String,
    pub wasm_size_bytes: u64,
    pub wasm_sha256_hex: String,
    pub wasm_gz_relative_path: String,
    pub wasm_gz_size_bytes: u64,
    pub wasm_gz_sha256_hex: String,
    pub candid_sha256: [u8; 32],
    pub protocol_profile_digest: ProtocolProfileDigest,
}

///
/// CanicInfrastructureArtifactManifestError
///
/// Typed rejection at the infrastructure artifact authority boundary.
///

#[derive(Debug, ThisError)]
pub enum CanicInfrastructureArtifactManifestError {
    #[error("infrastructure artifact {role:?} {kind} size cannot be represented")]
    ArtifactSizeOverflow {
        role: CanicInfrastructureRole,
        kind: &'static str,
    },

    #[error("infrastructure artifact path is duplicated: {path}")]
    DuplicateArtifactPath { path: String },

    #[error("infrastructure artifact {role:?} has an empty {kind} payload")]
    EmptyArtifact {
        role: CanicInfrastructureRole,
        kind: &'static str,
    },

    #[error("infrastructure artifact {role:?} has invalid gzip Wasm: {source}")]
    InvalidGzip {
        role: CanicInfrastructureRole,
        #[source]
        source: std::io::Error,
    },

    #[error(
        "infrastructure manifest must contain each exact role once in canonical order: {actual:?}"
    )]
    InfrastructureRoleSet {
        actual: Vec<CanicInfrastructureRole>,
    },

    #[error("infrastructure artifact {role:?} has invalid package identity: {package}")]
    InvalidPackage {
        role: CanicInfrastructureRole,
        package: String,
    },

    #[error("infrastructure artifact {role:?} has an empty protocol release identity")]
    MissingProtocolReleaseIdentity { role: CanicInfrastructureRole },

    #[error(
        "infrastructure artifact {role:?} protocol role {protocol_role} does not match its artifact role"
    )]
    ProtocolRoleMismatch {
        role: CanicInfrastructureRole,
        protocol_role: CanisterRole,
    },

    #[error("infrastructure artifact {role:?} has invalid {kind} path: {path}")]
    InvalidPath {
        role: CanicInfrastructureRole,
        kind: &'static str,
        path: String,
    },

    #[error("infrastructure artifact {role:?} has invalid {kind} SHA-256: {value}")]
    InvalidSha256 {
        role: CanicInfrastructureRole,
        kind: &'static str,
        value: String,
    },

    #[error("infrastructure artifact {role:?} has invalid raw Wasm bytes")]
    InvalidWasm { role: CanicInfrastructureRole },

    #[error(
        "infrastructure artifact {role:?} release build {actual} does not match manifest release build {expected}"
    )]
    ReleaseBuildMismatch {
        role: CanicInfrastructureRole,
        expected: ReleaseBuildId,
        actual: ReleaseBuildId,
    },

    #[error("infrastructure artifact {role:?} raw and gzip Wasm representations differ")]
    RepresentationMismatch { role: CanicInfrastructureRole },

    #[error("failed to serialize infrastructure artifact manifest: {0}")]
    Serialization(serde_json::Error),

    #[error("infrastructure artifact {role:?} {kind} size must be nonzero")]
    ZeroSize {
        role: CanicInfrastructureRole,
        kind: &'static str,
    },
}

fn compile_entry(
    release_build_id: ReleaseBuildId,
    input: &CanicInfrastructureArtifactInput<'_>,
) -> Result<CanicInfrastructureArtifactEntry, CanicInfrastructureArtifactManifestError> {
    if input.release_build_id != release_build_id {
        return Err(
            CanicInfrastructureArtifactManifestError::ReleaseBuildMismatch {
                role: input.role,
                expected: release_build_id,
                actual: input.release_build_id,
            },
        );
    }
    if input.wasm.is_empty() {
        return Err(CanicInfrastructureArtifactManifestError::EmptyArtifact {
            role: input.role,
            kind: "raw Wasm",
        });
    }
    if !input.wasm.starts_with(&WASM_MAGIC) {
        return Err(CanicInfrastructureArtifactManifestError::InvalidWasm { role: input.role });
    }
    if input.wasm_gz.is_empty() {
        return Err(CanicInfrastructureArtifactManifestError::EmptyArtifact {
            role: input.role,
            kind: "gzip Wasm",
        });
    }
    if !input.wasm_gz.starts_with(&GZIP_MAGIC) {
        return Err(CanicInfrastructureArtifactManifestError::InvalidGzip {
            role: input.role,
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, "missing gzip header"),
        });
    }

    let wasm_size_bytes = u64::try_from(input.wasm.len()).map_err(|_| {
        CanicInfrastructureArtifactManifestError::ArtifactSizeOverflow {
            role: input.role,
            kind: "raw Wasm",
        }
    })?;
    let wasm_gz_size_bytes = u64::try_from(input.wasm_gz.len()).map_err(|_| {
        CanicInfrastructureArtifactManifestError::ArtifactSizeOverflow {
            role: input.role,
            kind: "gzip Wasm",
        }
    })?;
    let mut decoded = Vec::new();
    GzDecoder::new(input.wasm_gz)
        .take(wasm_size_bytes.saturating_add(1))
        .read_to_end(&mut decoded)
        .map_err(
            |source| CanicInfrastructureArtifactManifestError::InvalidGzip {
                role: input.role,
                source,
            },
        )?;
    if decoded != input.wasm {
        return Err(
            CanicInfrastructureArtifactManifestError::RepresentationMismatch { role: input.role },
        );
    }

    let entry = CanicInfrastructureArtifactEntry {
        role: input.role,
        package: input.package.to_string(),
        protocol_release_identity: input.protocol_release_identity.to_string(),
        protocol_role: input.protocol_role.clone(),
        protocol_capabilities: input.protocol_capabilities.clone(),
        release_build_id: input.release_build_id,
        wasm_relative_path: input.wasm_relative_path.to_string(),
        wasm_size_bytes,
        wasm_sha256_hex: sha256_hex(input.wasm),
        wasm_gz_relative_path: input.wasm_gz_relative_path.to_string(),
        wasm_gz_size_bytes,
        wasm_gz_sha256_hex: sha256_hex(input.wasm_gz),
        candid_sha256: input.candid_sha256,
        protocol_profile_digest: input.protocol_profile_digest,
    };
    validate_entry(release_build_id, &entry)?;
    Ok(entry)
}

fn validate_entry(
    release_build_id: ReleaseBuildId,
    entry: &CanicInfrastructureArtifactEntry,
) -> Result<(), CanicInfrastructureArtifactManifestError> {
    if entry.release_build_id != release_build_id {
        return Err(
            CanicInfrastructureArtifactManifestError::ReleaseBuildMismatch {
                role: entry.role,
                expected: release_build_id,
                actual: entry.release_build_id,
            },
        );
    }
    if !valid_package_name(&entry.package) {
        return Err(CanicInfrastructureArtifactManifestError::InvalidPackage {
            role: entry.role,
            package: entry.package.clone(),
        });
    }
    if entry.protocol_release_identity.trim().is_empty() {
        return Err(
            CanicInfrastructureArtifactManifestError::MissingProtocolReleaseIdentity {
                role: entry.role,
            },
        );
    }
    if entry.protocol_role.as_str() != entry.role.protocol_role_name() {
        return Err(
            CanicInfrastructureArtifactManifestError::ProtocolRoleMismatch {
                role: entry.role,
                protocol_role: entry.protocol_role.clone(),
            },
        );
    }
    validate_path(entry.role, "raw Wasm", &entry.wasm_relative_path)?;
    validate_path(entry.role, "gzip Wasm", &entry.wasm_gz_relative_path)?;
    if entry.wasm_size_bytes == 0 {
        return Err(CanicInfrastructureArtifactManifestError::ZeroSize {
            role: entry.role,
            kind: "raw Wasm",
        });
    }
    if entry.wasm_gz_size_bytes == 0 {
        return Err(CanicInfrastructureArtifactManifestError::ZeroSize {
            role: entry.role,
            kind: "gzip Wasm",
        });
    }
    validate_sha256(entry.role, "raw Wasm", &entry.wasm_sha256_hex)?;
    validate_sha256(entry.role, "gzip Wasm", &entry.wasm_gz_sha256_hex)
}

fn validate_path(
    role: CanicInfrastructureRole,
    kind: &'static str,
    path: &str,
) -> Result<(), CanicInfrastructureArtifactManifestError> {
    if path.len() > MAX_ARTIFACT_PATH_BYTES
        || validate_release_artifact_relative_path(path).is_err()
    {
        return Err(CanicInfrastructureArtifactManifestError::InvalidPath {
            role,
            kind,
            path: path.to_string(),
        });
    }
    Ok(())
}

fn validate_sha256(
    role: CanicInfrastructureRole,
    kind: &'static str,
    value: &str,
) -> Result<(), CanicInfrastructureArtifactManifestError> {
    let canonical = value.len() == SHA_256_HEX_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && decode_hex(value).is_ok();
    if canonical {
        Ok(())
    } else {
        Err(CanicInfrastructureArtifactManifestError::InvalidSha256 {
            role,
            kind,
            value: value.to_string(),
        })
    }
}
