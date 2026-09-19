//! Module: release_set::fixture
//!
//! Responsibility: compile and retain opaque fixture artifacts under one release identity.
//! Does not own: application row encoding, complete release finalization, grants or network effects.
//! Boundary: authored chunk order is preserved; only exact retained bytes may populate publication.

mod configuration;
mod persistence;
#[cfg(test)]
mod tests;

use canic_control_plane::api::fixture_content::FixtureContentApi;
use canic_core::{
    CANIC_WASM_CHUNK_BYTES,
    bootstrap::compiled::ComponentTopology,
    dto::fixture_provisioning::{FixtureChunkDescriptor, FixtureDescriptor, FixtureStoreError},
    ids::{CanisterRole, ComponentTopologyDigest, ReleaseBuildId},
};
use serde::{Deserialize, Serialize};
use sha2_host::{Digest, Sha256};
use std::{collections::BTreeSet, io, path::PathBuf};
use thiserror::Error;

pub use configuration::{ConfiguredFixtureSources, load_configured_fixture_sources};
pub(crate) use persistence::read_chunk;
pub use persistence::{
    PersistedFixtureArtifactManifest, compile_and_persist_fixture_artifact_manifest,
    load_fixture_artifact_manifest, verify_fixture_artifacts,
};

///
/// FixtureSourceInput
///
/// Host build input for one role, with application-authored, independently decodable chunks.
/// Paths are relative to the supplied workspace root; their order carries application semantics.
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixtureSourceInput {
    pub role: CanisterRole,
    pub format_hash: [u8; 32],
    pub completion_summary: [u8; 32],
    pub chunk_paths: Vec<String>,
}

///
/// FixtureArtifactEntry
///
/// Release manifest binding between a declared application role and immutable opaque content.
/// Runtime target Principals and local source paths are deliberately absent.
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureArtifactEntry {
    pub role: CanisterRole,
    pub content_id: [u8; 32],
    pub descriptor: FixtureDescriptor,
}

///
/// FixtureArtifactManifest
///
/// Canonical host-owned fixture child manifest awaiting binding by the complete release owner.
/// Its digest alone does not authorize Store writes or target delivery.
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureArtifactManifest {
    pub schema_version: u16,
    pub release_build_id: ReleaseBuildId,
    pub component_topology_digest: ComponentTopologyDigest,
    pub entries: Vec<FixtureArtifactEntry>,
}

impl FixtureArtifactManifest {
    /// Reject unknown/duplicate roles, descriptor drift and a substituted release or topology.
    pub fn validate(
        &self,
        topology: &ComponentTopology,
        release_build_id: ReleaseBuildId,
    ) -> Result<(), FixtureArtifactError> {
        if self.schema_version != 1 || self.release_build_id != release_build_id {
            return Err(FixtureArtifactError::Authority);
        }
        let digest = topology
            .digest()
            .map_err(|_| FixtureArtifactError::Authority)?;
        if self.component_topology_digest != digest {
            return Err(FixtureArtifactError::Authority);
        }
        let roles = topology_roles(topology);
        let mut previous = None;
        for entry in &self.entries {
            if !roles.contains(&entry.role) || previous.is_some_and(|role| role >= &entry.role) {
                return Err(FixtureArtifactError::Role(entry.role.clone()));
            }
            previous = Some(&entry.role);
            if FixtureContentApi::content_id(&entry.descriptor)? != entry.content_id {
                return Err(FixtureArtifactError::Content);
            }
        }
        Ok(())
    }
}

///
/// FixtureArtifactError
///
/// Typed host build/publication refusal; callers retain the selected operation on failure.
///
#[derive(Debug, Error)]
pub enum FixtureArtifactError {
    #[error("fixture release or topology authority differs")]
    Authority,
    #[error("fixture inputs changed during the build")]
    ChangedInputs,
    #[error("fixture source configuration is invalid: {0}")]
    Configuration(String),
    #[error("fixture artifact bytes differ from selected content")]
    Content,
    #[error("fixture artifact already exists with different bytes: {0}")]
    Conflict(PathBuf),
    #[error("finalized release cannot acquire or repair fixture artifacts")]
    Finalized,
    #[error("fixture source path is not a regular workspace-relative file: {0}")]
    Path(PathBuf),
    #[error(
        "fixture artifact path contains a symlink at {0}; use real workspace-local files and directories, and keep .canic state independent between checkouts"
    )]
    Symlink(PathBuf),
    #[error("fixture role is duplicate, unordered or outside the application topology: {0}")]
    Role(CanisterRole),
    #[error("fixture descriptor or payload is invalid: {0:?}")]
    Store(FixtureStoreError),
    #[error("fixture artifact I/O failed at {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("fixture manifest encoding failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    ReleaseBuild(#[from] crate::release_build::ReleaseBuildPlanError),
}

impl From<FixtureStoreError> for FixtureArtifactError {
    fn from(error: FixtureStoreError) -> Self {
        Self::Store(error)
    }
}

fn topology_roles(topology: &ComponentTopology) -> BTreeSet<CanisterRole> {
    topology
        .component_specs
        .iter()
        .flat_map(|spec| {
            std::iter::once(spec.component_role.clone())
                .chain(spec.children.iter().map(|child| child.role.clone()))
        })
        .collect()
}

fn compile_entry(
    root: &std::path::Path,
    input: &FixtureSourceInput,
) -> Result<FixtureArtifactEntry, FixtureArtifactError> {
    if input.chunk_paths.len() > canic_core::ingress::payload::DEFAULT_UPDATE_INGRESS_MAX_BYTES / 32
    {
        return Err(FixtureStoreError::Bounds.into());
    }
    // Validate the complete command envelope before reading any payload. The temporary
    // lengths/digests have the same encoded shape as their eventual values.
    let mut descriptor = FixtureDescriptor {
        schema_version: 1,
        format_hash: input.format_hash,
        encoded_length: input.chunk_paths.len() as u64,
        chunks: vec![
            FixtureChunkDescriptor {
                digest: [0; 32],
                length: 1
            };
            input.chunk_paths.len()
        ],
        completion_summary: input.completion_summary,
    };
    FixtureContentApi::content_id(&descriptor)?;
    descriptor.encoded_length = 0;
    for (chunk, path) in descriptor.chunks.iter_mut().zip(&input.chunk_paths) {
        let bytes = persistence::read_workspace_file(root, path, CANIC_WASM_CHUNK_BYTES)?;
        chunk.length = u32::try_from(bytes.len()).map_err(|_| FixtureStoreError::Bounds)?;
        chunk.digest = Sha256::digest(&bytes).into();
        descriptor.encoded_length += u64::from(chunk.length);
    }
    let content_id = FixtureContentApi::content_id(&descriptor)?;
    Ok(FixtureArtifactEntry {
        role: input.role.clone(),
        content_id,
        descriptor,
    })
}
