//! Module: release_set::fixture::configuration
//!
//! Responsibility: discover package-declared fixture sources and freeze their build inputs.
//! Does not own: Cargo execution, release finalization or remote publication.
//! Boundary: package metadata selects a local source manifest; every observed file is hash-bound.

use super::*;
use crate::release_set::AppConfigSnapshot;
use canic_core::{
    cdk::utils::hash::{decode_hex, sha256_hex},
    dto::root_store::ROOT_STORE_RELEASE_SET_MANIFEST_MAX_BYTES,
};
use std::{collections::BTreeMap, path::Path};

///
/// ConfiguredFixtureSources
///
/// Host-owned pre-build selection and exact source-file fingerprints, including packages without fixtures.
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfiguredFixtureSources {
    pub inputs: Vec<FixtureSourceInput>,
    pub source_files: BTreeMap<PathBuf, String>,
    entries: Vec<FixtureArtifactEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureSourceDocument {
    format_hash: String,
    completion_summary: String,
    chunk_paths: Vec<String>,
}

impl ConfiguredFixtureSources {
    /// Bind retained payload descriptors to the exact bytes observed before the build.
    pub fn verify_retained(
        &self,
        retained: &PersistedFixtureArtifactManifest,
    ) -> Result<(), FixtureArtifactError> {
        if self.entries != retained.manifest.entries {
            return Err(FixtureArtifactError::ChangedInputs);
        }
        Ok(())
    }

    /// Reject a source addition, removal or changed byte before finalizing the selected build.
    pub fn verify_unchanged(
        &self,
        root: &Path,
        config_path: &Path,
    ) -> Result<(), FixtureArtifactError> {
        if *self != load_configured_fixture_sources(root, config_path)? {
            return Err(FixtureArtifactError::ChangedInputs);
        }
        Ok(())
    }
}

/// Read `[package.metadata.canic].fixture` for the roles selected by Component topology.
/// Source document paths are package-relative; chunk paths are source-document-relative.
pub fn load_configured_fixture_sources(
    root: &Path,
    config_path: &Path,
) -> Result<ConfiguredFixtureSources, FixtureArtifactError> {
    let root = root
        .canonicalize()
        .map_err(|source| FixtureArtifactError::Io {
            path: root.into(),
            source,
        })?;
    let config_path = config_path
        .canonicalize()
        .map_err(|source| FixtureArtifactError::Io {
            path: config_path.into(),
            source,
        })?;
    let mut selected = ConfiguredFixtureSources {
        inputs: Vec::new(),
        source_files: BTreeMap::new(),
        entries: Vec::new(),
    };
    let before = observe(
        &root,
        &config_path,
        document_limit(),
        &mut selected.source_files,
    )?;
    let config = AppConfigSnapshot::load(&config_path)
        .map_err(|error| FixtureArtifactError::Configuration(error.to_string()))?;
    for role in topology_roles(config.component_topology()) {
        load_role_source(&root, &config_path, &config, role, &mut selected)?;
    }
    let after = persistence::read_workspace_file(
        &root,
        &relative_to(&root, &config_path)?,
        document_limit(),
    )?;
    if before != after {
        return Err(FixtureArtifactError::ChangedInputs);
    }
    Ok(selected)
}

fn load_role_source(
    root: &Path,
    config_path: &Path,
    config: &AppConfigSnapshot,
    role: CanisterRole,
    selected: &mut ConfiguredFixtureSources,
) -> Result<(), FixtureArtifactError> {
    let declaration = config
        .model()
        .roles
        .get(&role)
        .ok_or_else(|| FixtureArtifactError::Role(role.clone()))?;
    let package = declaration
        .package
        .as_ref()
        .ok_or_else(|| FixtureArtifactError::Role(role.clone()))?;
    let mut manifest = config_path
        .parent()
        .ok_or(FixtureArtifactError::Authority)?
        .join(package);
    if manifest.file_name().is_none_or(|name| name != "Cargo.toml") {
        manifest.push("Cargo.toml");
    }
    // Existing role packages can use parent-relative selectors. Resolve their directory
    // once, then require all fixture inputs to stay beneath this workspace.
    let package_root = manifest
        .parent()
        .ok_or(FixtureArtifactError::Authority)?
        .canonicalize()
        .map_err(|source| FixtureArtifactError::Io {
            path: manifest.clone(),
            source,
        })?;
    let manifest = package_root.join("Cargo.toml");
    let bytes =
        crate::durable_io::read_regular_bytes(&manifest, document_limit()).map_err(|source| {
            FixtureArtifactError::Io {
                path: manifest.clone(),
                source,
            }
        })?;
    let hash = sha256_hex(&bytes);
    if selected
        .source_files
        .insert(manifest, hash.clone())
        .is_some_and(|before| before != hash)
    {
        return Err(FixtureArtifactError::ChangedInputs);
    }
    let cargo: toml::Value =
        toml::from_str(std::str::from_utf8(&bytes).map_err(|_| FixtureArtifactError::Content)?)
            .map_err(|error| FixtureArtifactError::Configuration(error.to_string()))?;
    let Some(fixture) = cargo
        .get("package")
        .and_then(|value| value.get("metadata"))
        .and_then(|value| value.get("canic"))
        .and_then(|value| value.get("fixture"))
    else {
        return Ok(());
    };
    let relative = fixture.as_str().ok_or_else(|| {
        FixtureArtifactError::Configuration(
            "package.metadata.canic.fixture must be a relative source-manifest path".into(),
        )
    })?;
    let source_path = relative_file(&package_root, relative)?;
    let bytes = observe(
        root,
        &source_path,
        document_limit(),
        &mut selected.source_files,
    )?;
    compile_configured_source(root, &source_path, role, &bytes, selected)
}

fn compile_configured_source(
    root: &Path,
    source_path: &Path,
    role: CanisterRole,
    bytes: &[u8],
    selected: &mut ConfiguredFixtureSources,
) -> Result<(), FixtureArtifactError> {
    let document: FixtureSourceDocument = serde_json::from_slice(bytes)?;
    let parent = source_path
        .parent()
        .ok_or(FixtureArtifactError::Authority)?;
    let mut input = FixtureSourceInput {
        role,
        format_hash: digest(&document.format_hash)?,
        completion_summary: digest(&document.completion_summary)?,
        chunk_paths: Vec::new(),
    };
    if document.chunk_paths.len()
        > canic_core::ingress::payload::DEFAULT_UPDATE_INGRESS_MAX_BYTES / 32
    {
        return Err(FixtureStoreError::Bounds.into());
    }
    let mut descriptor = FixtureDescriptor {
        schema_version: 1,
        format_hash: input.format_hash,
        completion_summary: input.completion_summary,
        encoded_length: 0,
        chunks: Vec::new(),
    };
    for path in document.chunk_paths {
        let absolute = relative_file(parent, &path)?;
        let bytes = observe(
            root,
            &absolute,
            CANIC_WASM_CHUNK_BYTES,
            &mut selected.source_files,
        )?;
        descriptor.encoded_length += bytes.len() as u64;
        descriptor.chunks.push(FixtureChunkDescriptor {
            digest: Sha256::digest(&bytes).into(),
            length: u32::try_from(bytes.len()).map_err(|_| FixtureStoreError::Bounds)?,
        });
        input.chunk_paths.push(relative_to(root, &absolute)?);
    }
    selected.entries.push(FixtureArtifactEntry {
        role: input.role.clone(),
        content_id: FixtureContentApi::content_id(&descriptor)?,
        descriptor,
    });
    selected.inputs.push(input);
    Ok(())
}

fn observe(
    root: &Path,
    path: &Path,
    limit: usize,
    files: &mut BTreeMap<PathBuf, String>,
) -> Result<Vec<u8>, FixtureArtifactError> {
    let bytes = persistence::read_workspace_file(root, &relative_to(root, path)?, limit)?;
    let hash = sha256_hex(&bytes);
    if files
        .insert(path.into(), hash.clone())
        .is_some_and(|prior| prior != hash)
    {
        return Err(FixtureArtifactError::ChangedInputs);
    }
    Ok(bytes)
}

fn relative_file(parent: &Path, relative: &str) -> Result<PathBuf, FixtureArtifactError> {
    crate::release_set::validate_release_artifact_relative_path(relative)
        .map_err(|_| FixtureArtifactError::Path(parent.join(relative)))?;
    Ok(parent.join(relative))
}

fn relative_to(root: &Path, path: &Path) -> Result<String, FixtureArtifactError> {
    path.strip_prefix(root)
        .ok()
        .and_then(Path::to_str)
        .map(str::to_owned)
        .ok_or_else(|| FixtureArtifactError::Path(path.into()))
}

fn digest(value: &str) -> Result<[u8; 32], FixtureArtifactError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(FixtureArtifactError::Content);
    }
    decode_hex(value)
        .map_err(|_| FixtureArtifactError::Content)?
        .try_into()
        .map_err(|_| FixtureArtifactError::Content)
}

fn document_limit() -> usize {
    usize::try_from(ROOT_STORE_RELEASE_SET_MANIFEST_MAX_BYTES)
        .expect("host document limit fits usize")
}
