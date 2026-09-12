//! Module: release_set::fixture::persistence
//!
//! Responsibility: retain immutable bounded fixture artifacts under a release identity.
//! Does not own: complete release finalization, application decoding or network effects.
//! Boundary: the complete release owner must bind the returned manifest digest.

use super::{
    CANIC_WASM_CHUNK_BYTES, ComponentTopology, FixtureArtifactEntry, FixtureArtifactError,
    FixtureArtifactManifest, FixtureContentApi, FixtureSourceInput, ReleaseBuildId, compile_entry,
    topology_roles,
};
use crate::{
    durable_io::{
        BoundedRegularFileReadError, RegularFileReadError, create_new_bytes_with_parents,
        read_optional_regular_bytes_bounded,
    },
    release_build::{ReleaseBuildPlanState, load_release_build_plan},
    release_set::validate_release_artifact_relative_path,
};
use canic_core::{
    cdk::utils::hash::hex_bytes,
    dto::{
        fixture_provisioning::FixtureStoreError,
        root_store::ROOT_STORE_RELEASE_SET_MANIFEST_MAX_BYTES,
    },
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

///
/// PersistedFixtureArtifactManifest
///
/// Host receipt for one immutable fixture manifest, to be bound by reviewed release authority.
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersistedFixtureArtifactManifest {
    pub manifest: FixtureArtifactManifest,
    pub digest: [u8; 32],
    pub path: PathBuf,
}

/// Compile authored chunk boundaries and retain exact bytes before publishing the child manifest.
/// An interrupted copy can resume; neither existing artifacts nor finalized builds are overwritten.
pub fn compile_and_persist_fixture_artifact_manifest(
    root: &Path,
    topology: &ComponentTopology,
    release_build_id: ReleaseBuildId,
    inputs: &[FixtureSourceInput],
) -> Result<PersistedFixtureArtifactManifest, FixtureArtifactError> {
    let plan = load_release_build_plan(root, release_build_id)?;
    let finalized = matches!(plan.state, ReleaseBuildPlanState::Finalized { .. });
    let roles = topology_roles(topology);
    let mut sources = BTreeMap::new();
    for input in inputs {
        if !roles.contains(&input.role) || sources.insert(input.role.clone(), input).is_some() {
            return Err(FixtureArtifactError::Role(input.role.clone()));
        }
    }
    let entries = sources
        .values()
        .map(|input| compile_entry(root, input))
        .collect::<Result<Vec<_>, _>>()?;
    let manifest = FixtureArtifactManifest {
        schema_version: 1,
        release_build_id,
        component_topology_digest: topology
            .digest()
            .map_err(|_| FixtureArtifactError::Authority)?,
        entries,
    };
    manifest.validate(topology, release_build_id)?;
    let bytes = serde_json::to_vec(&manifest)?;
    if bytes.len() as u64 > ROOT_STORE_RELEASE_SET_MANIFEST_MAX_BYTES {
        return Err(FixtureStoreError::Bounds.into());
    }
    let relative_path = manifest_path(release_build_id);
    let path = root.join(&relative_path);
    // Reject a conflicting role/content selection before adding any payload files.
    match read_workspace_file(root, &relative_path, manifest_limit()) {
        Ok(existing) if existing == bytes => {}
        Ok(_) => return Err(FixtureArtifactError::Conflict(path)),
        Err(error) if is_missing(&error) && !finalized => {}
        Err(error) if is_missing(&error) => return Err(FixtureArtifactError::Finalized),
        Err(error) => return Err(error),
    }
    for entry in &manifest.entries {
        let input = sources[&entry.role];
        for (index, source) in input.chunk_paths.iter().enumerate() {
            let index = u32::try_from(index).map_err(|_| FixtureStoreError::Bounds)?;
            let payload = read_workspace_file(root, source, CANIC_WASM_CHUNK_BYTES)?;
            FixtureContentApi::verify_chunk(&entry.descriptor, index, &payload)?;
            retain_exact(
                root,
                &chunk_path(release_build_id, entry, index),
                &payload,
                finalized,
            )?;
        }
    }
    retain_exact(root, &relative_path, &bytes, finalized)?;
    Ok(PersistedFixtureArtifactManifest {
        digest: Sha256::digest(&bytes).into(),
        manifest,
        path,
    })
}

/// Load an exact selected child manifest without consulting mutable application source files.
/// Payload validation is bounded per chunk by publication or by `verify_fixture_artifacts`.
pub fn load_fixture_artifact_manifest(
    root: &Path,
    topology: &ComponentTopology,
    release_build_id: ReleaseBuildId,
    expected_digest: [u8; 32],
) -> Result<PersistedFixtureArtifactManifest, FixtureArtifactError> {
    load_release_build_plan(root, release_build_id)?;
    let relative = manifest_path(release_build_id);
    let bytes = read_workspace_file(root, &relative, manifest_limit())?;
    let digest: [u8; 32] = Sha256::digest(&bytes).into();
    if digest != expected_digest {
        return Err(FixtureArtifactError::Authority);
    }
    let manifest: FixtureArtifactManifest = serde_json::from_slice(&bytes)?;
    manifest.validate(topology, release_build_id)?;
    if serde_json::to_vec(&manifest)? != bytes {
        return Err(FixtureArtifactError::Content);
    }
    Ok(PersistedFixtureArtifactManifest {
        manifest,
        digest,
        path: root.join(relative),
    })
}

/// Verify retained source payloads before admitting their digest to a complete release or plan.
pub fn verify_fixture_artifacts(
    root: &Path,
    topology: &ComponentTopology,
    release_build_id: ReleaseBuildId,
    expected_digest: [u8; 32],
) -> Result<(), FixtureArtifactError> {
    let retained =
        load_fixture_artifact_manifest(root, topology, release_build_id, expected_digest)?;
    for entry in &retained.manifest.entries {
        for index in 0..entry.descriptor.chunks.len() {
            let index = u32::try_from(index).map_err(|_| FixtureStoreError::Bounds)?;
            read_chunk(root, release_build_id, entry, index)?;
        }
    }
    Ok(())
}

pub fn read_chunk(
    root: &Path,
    release: ReleaseBuildId,
    entry: &FixtureArtifactEntry,
    index: u32,
) -> Result<Vec<u8>, FixtureArtifactError> {
    let bytes = read_workspace_file(
        root,
        &chunk_path(release, entry, index),
        CANIC_WASM_CHUNK_BYTES,
    )?;
    FixtureContentApi::verify_chunk(&entry.descriptor, index, &bytes)?;
    Ok(bytes)
}

pub(super) fn read_workspace_file(
    root: &Path,
    relative: &str,
    limit: usize,
) -> Result<Vec<u8>, FixtureArtifactError> {
    let path = root.join(relative);
    validate_release_artifact_relative_path(relative)
        .map_err(|_| FixtureArtifactError::Path(path.clone()))?;
    let mut parent = root.to_path_buf();
    // Reject symlinked parent directories as well as final symlinks. Retained artifact
    // writes additionally use durable_io's atomic regular-file publication.
    for component in Path::new(relative).components() {
        parent.push(component);
        if fs::symlink_metadata(&parent).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
            return Err(FixtureArtifactError::Path(parent));
        }
    }
    match read_optional_regular_bytes_bounded(&path, limit) {
        Ok(Some(bytes)) => Ok(bytes),
        Ok(None) => Err(FixtureArtifactError::Io {
            path,
            source: io::ErrorKind::NotFound.into(),
        }),
        Err(BoundedRegularFileReadError::TooLarge) => Err(FixtureStoreError::Bounds.into()),
        Err(BoundedRegularFileReadError::Read(RegularFileReadError::Io(source))) => {
            Err(FixtureArtifactError::Io { path, source })
        }
        Err(_) => Err(FixtureArtifactError::Path(path)),
    }
}

fn retain_exact(
    root: &Path,
    relative: &str,
    bytes: &[u8],
    finalized: bool,
) -> Result<(), FixtureArtifactError> {
    match read_workspace_file(root, relative, bytes.len()) {
        Ok(existing) if existing == bytes => return Ok(()),
        Ok(_) => return Err(FixtureArtifactError::Conflict(root.join(relative))),
        Err(error) if is_missing(&error) && !finalized => {}
        Err(error) if is_missing(&error) => return Err(FixtureArtifactError::Finalized),
        Err(error) => return Err(error),
    }
    let path = root.join(relative);
    if let Err(source) = create_new_bytes_with_parents(&path, bytes) {
        // A competing exact writer or a lost successful response may already have committed.
        if read_workspace_file(root, relative, bytes.len()).is_ok_and(|retained| retained == bytes)
        {
            return Ok(());
        }
        return Err(FixtureArtifactError::Io { path, source });
    }
    Ok(())
}

fn is_missing(error: &FixtureArtifactError) -> bool {
    matches!(error, FixtureArtifactError::Io { source, .. } if source.kind() == io::ErrorKind::NotFound)
}

fn manifest_limit() -> usize {
    usize::try_from(ROOT_STORE_RELEASE_SET_MANIFEST_MAX_BYTES)
        .expect("manifest limit fits host usize")
}

fn manifest_path(release: ReleaseBuildId) -> String {
    format!(".canic/release-builds/{release}/fixture-artifact-manifest.json")
}

fn chunk_path(release: ReleaseBuildId, entry: &FixtureArtifactEntry, index: u32) -> String {
    // Hash formatting here is path naming only. Content identity is compiled by the Store owner.
    let name = hex_bytes(entry.content_id);
    format!(".canic/release-builds/{release}/fixture-content/{name}/{index}.bin")
}
