//! Module: release_set::artifact
//!
//! Responsibility: resolve, validate, and describe release-set Wasm artifacts.
//! Does not own: manifest persistence or build orchestration.
//! Boundary: keeps every artifact read within the canonical ICP project root.

use crate::release_set::{GZIP_MAGIC, ReleaseSetEntry, WASM_MAGIC};
use std::{
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
};

use canic_core::{CANIC_WASM_CHUNK_BYTES, cdk::utils::hash::wasm_hash_hex};
use flate2::read::GzDecoder;

// Build one release-set entry from one built ordinary role artifact.
pub(in crate::release_set) fn build_release_set_entry(
    icp_root: &Path,
    role_name: &str,
    artifact_path: &Path,
) -> Result<ReleaseSetEntry, Box<dyn std::error::Error>> {
    let artifact_relative_path = artifact_path
        .strip_prefix(icp_root)
        .map_err(|_| {
            format!(
                "artifact {} is not under ICP root {}",
                artifact_path.display(),
                icp_root.display()
            )
        })?
        .to_string_lossy()
        .to_string();
    let artifact_path = resolve_release_artifact_path(icp_root, &artifact_relative_path)?;
    let artifact = read_release_artifact(&artifact_path)?;

    let chunk_hashes = artifact
        .chunks(CANIC_WASM_CHUNK_BYTES)
        .map(wasm_hash_hex)
        .collect::<Vec<_>>();

    Ok(ReleaseSetEntry {
        role: role_name.to_string(),
        template_id: format!("embedded:{role_name}"),
        artifact_relative_path,
        payload_size_bytes: u64::try_from(artifact.len())?,
        payload_sha256_hex: wasm_hash_hex(&artifact),
        chunk_size_bytes: u64::try_from(CANIC_WASM_CHUNK_BYTES)?,
        chunk_sha256_hex: chunk_hashes,
    })
}

/// Validate the lexical path contract shared by manifest admission and
/// filesystem resolution.
pub(in crate::release_set) fn validate_release_artifact_relative_path(
    artifact_relative_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let relative_path = Path::new(artifact_relative_path);
    if relative_path.as_os_str().is_empty()
        || relative_path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "release artifact path must be relative to the ICP root: {artifact_relative_path}"
        )
        .into());
    }

    Ok(())
}

/// Resolve one manifest artifact path and prove that its canonical target is
/// contained by the canonical ICP project root.
pub fn resolve_release_artifact_path(
    icp_root: &Path,
    artifact_relative_path: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    validate_release_artifact_relative_path(artifact_relative_path)?;
    let relative_path = Path::new(artifact_relative_path);

    let canonical_root = icp_root.canonicalize()?;
    let canonical_artifact = canonical_root.join(relative_path).canonicalize()?;
    if !canonical_artifact.starts_with(&canonical_root) {
        return Err(format!(
            "release artifact path escapes ICP root: {}",
            canonical_artifact.display()
        )
        .into());
    }

    Ok(canonical_artifact)
}

// Read one staged release artifact and validate that it is a non-empty gzip stream
// whose decompressed payload is a real wasm module.
pub(in crate::release_set) fn read_release_artifact(
    path: &Path,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let artifact = fs::read(path)?;

    if artifact.is_empty() {
        return Err(format!("release artifact is empty: {}", path.display()).into());
    }

    if !artifact.starts_with(&GZIP_MAGIC) {
        return Err(format!(
            "release artifact is not gzip-compressed: {}",
            path.display()
        )
        .into());
    }

    let mut decoder = GzDecoder::new(&artifact[..]);
    let mut wasm = Vec::new();
    decoder
        .read_to_end(&mut wasm)
        .map_err(|err| format!("failed to decompress {}: {err}", path.display()))?;

    if wasm.is_empty() {
        return Err(format!(
            "release artifact decompresses to zero bytes: {}",
            path.display()
        )
        .into());
    }

    if !wasm.starts_with(&WASM_MAGIC) {
        return Err(format!(
            "release artifact does not decompress to a wasm module: {}",
            path.display()
        )
        .into());
    }

    Ok(artifact)
}
