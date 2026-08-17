//! Module: install_root::operations::artifact
//!
//! Responsibility: resolve and validate one exact infrastructure Wasm for fresh installation.
//! Does not own: build output production, manifest persistence, or install sequencing.
//! Boundary: callers supply a validated infrastructure manifest, role, and release-build identity.

use crate::{
    durable_io::{RegularFileReadError, read_optional_regular_bytes},
    release_set::{
        CanicInfrastructureRole, PersistedCanicInfrastructureArtifactManifest,
        resolve_release_artifact_path,
    },
};
use canic_core::ids::ReleaseBuildId;
use sha2::{Digest, Sha256};
use std::{
    io,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
enum InstallArtifactError {
    #[error("{role} artifact {path} is missing")]
    Missing { role: &'static str, path: PathBuf },

    #[error("{role} artifact is not a regular no-follow file: {path}")]
    Unsafe { role: &'static str, path: PathBuf },

    #[error("failed to read {role} artifact {path}: {source}")]
    Read {
        role: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[cfg(not(unix))]
    #[error("{role} artifact reads are unsupported: {path}")]
    UnsupportedPlatform { role: &'static str, path: PathBuf },

    #[error("{role} artifact {path} has size {actual}, expected {expected}")]
    Size {
        role: &'static str,
        path: PathBuf,
        expected: u64,
        actual: usize,
    },

    #[error("{role} artifact {path} has SHA-256 {actual}, expected {expected}")]
    Hash {
        role: &'static str,
        path: PathBuf,
        expected: String,
        actual: String,
    },

    #[error("{role} artifact release build differs from the Fleet install plan")]
    ReleaseBuildMismatch { role: &'static str },
}

pub(in crate::install_root) struct InstallArtifact {
    pub wasm_path: PathBuf,
}

pub(in crate::install_root) fn resolve_install_artifact(
    icp_root: &Path,
    infrastructure_manifest: &PersistedCanicInfrastructureArtifactManifest,
    role: CanicInfrastructureRole,
    expected_release_build_id: ReleaseBuildId,
) -> Result<InstallArtifact, Box<dyn std::error::Error>> {
    let entry = infrastructure_manifest
        .manifest
        .entries
        .iter()
        .find(|entry| entry.role == role)
        .expect("validated infrastructure manifest contains every infrastructure role");
    let role = role.as_str();
    let wasm_path = resolve_release_artifact_path(icp_root, &entry.wasm_relative_path)?;
    let wasm = match read_optional_regular_bytes(&wasm_path) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => {
            return Err(InstallArtifactError::Missing {
                role,
                path: wasm_path,
            }
            .into());
        }
        Err(RegularFileReadError::NotRegular) => {
            return Err(InstallArtifactError::Unsafe {
                role,
                path: wasm_path,
            }
            .into());
        }
        Err(RegularFileReadError::Io(source)) => {
            return Err(InstallArtifactError::Read {
                role,
                path: wasm_path,
                source,
            }
            .into());
        }
        #[cfg(not(unix))]
        Err(RegularFileReadError::UnsupportedPlatform) => {
            return Err(InstallArtifactError::UnsupportedPlatform {
                role,
                path: wasm_path,
            }
            .into());
        }
    };
    if wasm.len() as u64 != entry.wasm_size_bytes {
        return Err(InstallArtifactError::Size {
            role,
            path: wasm_path,
            expected: entry.wasm_size_bytes,
            actual: wasm.len(),
        }
        .into());
    }
    let actual_hash = super::module_hash_text(Sha256::digest(&wasm).into());
    if actual_hash != entry.wasm_sha256_hex {
        return Err(InstallArtifactError::Hash {
            role,
            path: wasm_path,
            expected: entry.wasm_sha256_hex.clone(),
            actual: actual_hash,
        }
        .into());
    }
    if entry.release_build_id != expected_release_build_id {
        return Err(InstallArtifactError::ReleaseBuildMismatch { role }.into());
    }
    Ok(InstallArtifact { wasm_path })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::release_set::{
        CanicInfrastructureArtifactEntry, CanicInfrastructureArtifactManifest,
    };
    use canic_core::ids::ReleaseBuildNonce;
    use std::fs;

    #[test]
    fn resolves_exact_role_artifact_from_manifest() {
        let root = crate::test_support::temp_dir("canic-install-artifact-resolution");
        fs::create_dir_all(&root).expect("create artifact root");
        let wasm_path = root.join("coordinator.wasm");
        let wasm = b"\0asm";
        fs::write(&wasm_path, wasm).expect("write artifact");
        let release_build_id =
            ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([7; 32]));
        let expected_hash = super::super::module_hash_text(Sha256::digest(wasm).into());
        let manifest = PersistedCanicInfrastructureArtifactManifest {
            manifest: CanicInfrastructureArtifactManifest {
                release_build_id,
                entries: vec![CanicInfrastructureArtifactEntry {
                    role: CanicInfrastructureRole::FleetCoordinator,
                    package: "canic-control-plane".to_string(),
                    release_build_id,
                    wasm_relative_path: "coordinator.wasm".to_string(),
                    wasm_size_bytes: wasm.len() as u64,
                    wasm_sha256_hex: expected_hash,
                    wasm_gz_relative_path: "coordinator.wasm.gz".to_string(),
                    wasm_gz_size_bytes: 0,
                    wasm_gz_sha256_hex: String::new(),
                    candid_sha256: [3; 32],
                    protocol_profile_digest:
                        canic_core::role_contract::ProtocolProfileDigest::from_bytes([4; 32]),
                }],
            },
            digest: [0; 32],
            path: root.join("manifest.json"),
        };

        let artifact = resolve_install_artifact(
            &root,
            &manifest,
            CanicInfrastructureRole::FleetCoordinator,
            release_build_id,
        )
        .expect("resolve install artifact");

        assert_eq!(artifact.wasm_path, wasm_path.canonicalize().expect("path"));
        fs::remove_dir_all(root).expect("remove artifact root");
    }
}
