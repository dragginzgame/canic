//! Module: fleet_install_plan::authority
//!
//! Responsibility: load exact read-only config, Fleet-input, release, and source identities.
//! Does not own: planning policy, builds, report persistence, clocks, or IC effects.
//! Boundary: every returned digest is derived from regular local bytes or finalized authority.

#[cfg(test)]
mod tests;

use super::model::{
    FreshFleetDecisionAuthorityV1, FreshFleetExpectedArtifactV1,
    FreshFleetOperatorFundingEvidenceV1, FreshFleetReleaseSourceV1,
};
use crate::{
    fleet_install_input::ResolvedFleetInstallInput,
    release_build::{ReleaseBuildPlanError, ReleaseBuildPlanState, load_finalized_release_build},
    release_set::{AppConfigSnapshot, configured_release_roles_from_config},
};
use canic_core::{
    cdk::utils::hash::hex_bytes,
    ids::{CanisterRole, CanonicalNetworkId, ReleaseBuildId},
};
use sha2::{Digest, Sha256};
use std::{
    fs, io,
    path::{Component, Path, PathBuf},
    process::Command,
};
use thiserror::Error as ThisError;

const SOURCE_SNAPSHOT_DOMAIN: &[u8] = b"canic-workspace-release-source:v1\0";

/// Exact read-only inputs used to load one complete decision authority.
pub struct FreshFleetDecisionAuthorityRequest<'a> {
    pub workspace_root: &'a Path,
    pub icp_root: &'a Path,
    pub config: &'a AppConfigSnapshot,
    pub requested_environment: &'a str,
    pub canonical_network_id: CanonicalNetworkId,
    pub release_build_id: Option<ReleaseBuildId>,
    pub fleet_input: &'a ResolvedFleetInstallInput,
    pub operator: &'a FreshFleetOperatorFundingEvidenceV1,
}

/// Typed failure while loading complete pre-effect decision identity.
#[derive(Debug, ThisError)]
pub enum FreshFleetDecisionAuthorityError {
    #[error("App config cannot be canonically encoded: {0}")]
    ConfigEncoding(serde_json::Error),

    #[error("configured release role '{role}' has no exact package declaration")]
    MissingRolePackage { role: String },

    #[error("workspace source path is unsafe or non-canonical: {path}")]
    UnsafeSourcePath { path: PathBuf },

    #[error("failed to read release-source input {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(transparent)]
    ReleaseBuild(#[from] ReleaseBuildPlanError),
}

/// Load one exact complete authority without allocating release or install state.
pub fn load_fresh_fleet_decision_authority(
    request: FreshFleetDecisionAuthorityRequest<'_>,
) -> Result<FreshFleetDecisionAuthorityV1, FreshFleetDecisionAuthorityError> {
    let app_config_sha256 = hex_bytes(Sha256::digest(
        serde_json::to_vec(request.config.model())
            .map_err(FreshFleetDecisionAuthorityError::ConfigEncoding)?,
    ));
    let expected_artifacts = expected_artifacts(request.config)?;
    let release_source = match request.release_build_id {
        Some(release_build_id) => {
            finalized_release_source(request.icp_root, release_build_id, expected_artifacts)?
        }
        None => workspace_release_source(request.workspace_root, expected_artifacts)?,
    };

    Ok(FreshFleetDecisionAuthorityV1 {
        app_config_sha256,
        requested_environment: request.requested_environment.to_string(),
        canonical_network_id: request.canonical_network_id,
        fleet_input_schema_version: request.fleet_input.schema_version,
        fleet_input_sha256: request.fleet_input.canonical_sha256.clone(),
        release_source,
        catalog: request.fleet_input.catalog.clone(),
        operator: request.operator.clone(),
    })
}

fn expected_artifacts(
    config: &AppConfigSnapshot,
) -> Result<Vec<FreshFleetExpectedArtifactV1>, FreshFleetDecisionAuthorityError> {
    let mut roles = vec![CanisterRole::ROOT.as_str().to_string()];
    roles.extend(configured_release_roles_from_config(config.model()));
    let mut artifacts = roles
        .into_iter()
        .map(|role| {
            let package = config
                .model()
                .roles
                .get(&CanisterRole::owned(role.clone()))
                .ok_or_else(|| FreshFleetDecisionAuthorityError::MissingRolePackage {
                    role: role.clone(),
                })?
                .package
                .clone();
            Ok::<_, FreshFleetDecisionAuthorityError>(FreshFleetExpectedArtifactV1 {
                role,
                package,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    artifacts.extend([
        FreshFleetExpectedArtifactV1 {
            role: "fleet_coordinator".to_string(),
            package: "canic-fleet-coordinator".to_string(),
        },
        FreshFleetExpectedArtifactV1 {
            role: "wasm_store".to_string(),
            package: "canic-wasm-store".to_string(),
        },
    ]);
    artifacts.sort();
    artifacts.dedup();
    Ok(artifacts)
}

fn finalized_release_source(
    icp_root: &Path,
    release_build_id: ReleaseBuildId,
    expected_artifacts: Vec<FreshFleetExpectedArtifactV1>,
) -> Result<FreshFleetReleaseSourceV1, FreshFleetDecisionAuthorityError> {
    let finalized = load_finalized_release_build(icp_root, release_build_id)?;
    let ReleaseBuildPlanState::Finalized {
        release_set_manifest_digest,
    } = finalized.record.state
    else {
        unreachable!("finalized loader admits only finalized release authority");
    };
    Ok(FreshFleetReleaseSourceV1::Finalized {
        release_build_id,
        builder_version: finalized.record.builder_version,
        release_build_plan_sha256: hex_bytes(finalized.plan_hash),
        release_set_manifest_sha256: hex_bytes(release_set_manifest_digest),
        expected_artifacts,
    })
}

fn workspace_release_source(
    workspace_root: &Path,
    expected_artifacts: Vec<FreshFleetExpectedArtifactV1>,
) -> Result<FreshFleetReleaseSourceV1, FreshFleetDecisionAuthorityError> {
    let cargo_lock = workspace_root.join("Cargo.lock");
    let cargo_lock_sha256 = regular_file_sha256(&cargo_lock)?;
    let source_snapshot_sha256 = workspace_source_snapshot_sha256(workspace_root)?;
    Ok(FreshFleetReleaseSourceV1::Workspace {
        builder_version: env!("CARGO_PKG_VERSION").to_string(),
        cargo_lock_sha256,
        source_snapshot_sha256,
        expected_artifacts,
    })
}

fn workspace_source_snapshot_sha256(
    workspace_root: &Path,
) -> Result<String, FreshFleetDecisionAuthorityError> {
    let output = Command::new("git")
        .current_dir(workspace_root)
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .output()
        .map_err(|source| FreshFleetDecisionAuthorityError::Io {
            path: workspace_root.to_path_buf(),
            source,
        })?;
    let mut paths = if output.status.success() {
        output
            .stdout
            .split(|byte| *byte == 0)
            .filter(|bytes| !bytes.is_empty())
            .map(|bytes| {
                std::str::from_utf8(bytes).map(PathBuf::from).map_err(|_| {
                    FreshFleetDecisionAuthorityError::UnsafeSourcePath {
                        path: PathBuf::from("<non-UTF-8>"),
                    }
                })
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        recursive_source_paths(workspace_root)?
    };
    paths.retain(|path| included_source_path(path));
    paths.sort();
    paths.dedup();

    let mut hasher = Sha256::new();
    hasher.update(SOURCE_SNAPSHOT_DOMAIN);
    for relative in paths {
        validate_relative_source_path(&relative)?;
        let path = workspace_root.join(&relative);
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(source) if source.kind() == io::ErrorKind::NotFound => continue,
            Err(source) => {
                return Err(FreshFleetDecisionAuthorityError::Io {
                    path: path.clone(),
                    source,
                });
            }
        };
        if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
            return Err(FreshFleetDecisionAuthorityError::UnsafeSourcePath { path });
        }
        let bytes = fs::read(&path).map_err(|source| FreshFleetDecisionAuthorityError::Io {
            path: path.clone(),
            source,
        })?;
        let relative = relative.to_str().ok_or_else(|| {
            FreshFleetDecisionAuthorityError::UnsafeSourcePath { path: path.clone() }
        })?;
        hash_field(&mut hasher, relative.as_bytes());
        hash_field(&mut hasher, &bytes);
    }
    Ok(hex_bytes(hasher.finalize()))
}

fn recursive_source_paths(
    workspace_root: &Path,
) -> Result<Vec<PathBuf>, FreshFleetDecisionAuthorityError> {
    let mut pending = vec![PathBuf::new()];
    let mut files = Vec::new();
    while let Some(relative_directory) = pending.pop() {
        let directory = workspace_root.join(&relative_directory);
        let entries =
            fs::read_dir(&directory).map_err(|source| FreshFleetDecisionAuthorityError::Io {
                path: directory.clone(),
                source,
            })?;
        for entry in entries {
            let entry = entry.map_err(|source| FreshFleetDecisionAuthorityError::Io {
                path: directory.clone(),
                source,
            })?;
            let relative = relative_directory.join(entry.file_name());
            if !included_source_path(&relative) {
                continue;
            }
            let file_type =
                entry
                    .file_type()
                    .map_err(|source| FreshFleetDecisionAuthorityError::Io {
                        path: entry.path(),
                        source,
                    })?;
            if file_type.is_symlink() {
                return Err(FreshFleetDecisionAuthorityError::UnsafeSourcePath {
                    path: entry.path(),
                });
            }
            if file_type.is_dir() {
                pending.push(relative);
            } else if file_type.is_file() {
                files.push(relative);
            } else {
                return Err(FreshFleetDecisionAuthorityError::UnsafeSourcePath {
                    path: entry.path(),
                });
            }
        }
    }
    Ok(files)
}

fn validate_relative_source_path(path: &Path) -> Result<(), FreshFleetDecisionAuthorityError> {
    let valid = !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir));
    if valid {
        Ok(())
    } else {
        Err(FreshFleetDecisionAuthorityError::UnsafeSourcePath {
            path: path.to_path_buf(),
        })
    }
}

fn included_source_path(path: &Path) -> bool {
    let mut components = path.components();
    let Some(Component::Normal(first)) = components.next() else {
        return false;
    };
    if matches!(
        first.to_str(),
        Some(".cargo" | "apps" | "canisters" | "crates")
    ) {
        return true;
    }
    components.next().is_none()
        && matches!(
            first.to_str(),
            Some("Cargo.lock" | "Cargo.toml" | "rust-toolchain.toml")
        )
}

fn hash_field(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update(u64::try_from(bytes.len()).unwrap_or(u64::MAX).to_be_bytes());
    hasher.update(bytes);
}

fn regular_file_sha256(path: &Path) -> Result<String, FreshFleetDecisionAuthorityError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|source| FreshFleetDecisionAuthorityError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(FreshFleetDecisionAuthorityError::UnsafeSourcePath {
            path: path.to_path_buf(),
        });
    }
    let bytes = fs::read(path).map_err(|source| FreshFleetDecisionAuthorityError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(hex_bytes(Sha256::digest(bytes)))
}
