//! Immutable frontend publication and exact-digest verification.
//!
//! The manifest is published last; a retry may fill only identical missing files.

use crate::frontend::{
    FrontendError,
    model::{FrontendFileRecord, FrontendManifestRecord},
    ops::MAX_FRONTEND_FILE_BYTES,
    view::FrontendBundleView,
};
use ic_host_fs::durable::{PublicationMode, WriteOptions, write_bytes};
use ic_host_fs::read::read_file_no_follow;

use canic_core::cdk::utils::hash::sha256_hex;
use std::{
    fs,
    path::{Component, Path},
};

/// Digest all typed browser data and file identities, excluding the self digest.
pub fn manifest_digest(manifest: &FrontendManifestRecord) -> Result<String, FrontendError> {
    let mut canonical = manifest.clone();
    canonical.manifest_sha256.clear();
    let mut value = serde_json::to_value(canonical)?;
    value.sort_all_objects();
    Ok(sha256_hex(&serde_json::to_vec(&value)?))
}

fn safe_file(root: &Path, relative: &str) -> Result<std::path::PathBuf, FrontendError> {
    if fs::symlink_metadata(root).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err(FrontendError::Integrity);
    }
    let path = Path::new(relative);
    if relative.is_empty()
        || relative.contains(['\\', '?', '#', '%', ':'])
        || relative
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        || !path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(FrontendError::Integrity);
    }
    let mut selected = root.to_path_buf();
    for part in path.components() {
        selected.push(part);
        match fs::symlink_metadata(&selected) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(FrontendError::Integrity);
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(selected)
}

fn retain_file(path: &Path, bytes: &[u8]) -> Result<(), FrontendError> {
    match write_bytes(
        path,
        WriteOptions {
            mode: PublicationMode::CreateNew,
            permissions: 0o666,
        },
        bytes,
    )
    .map_err(crate::publication::ops::io_error)
    {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if read_file_no_follow(path, MAX_FRONTEND_FILE_BYTES).map_err(std::io::Error::from)?
                != bytes
            {
                return Err(FrontendError::Integrity);
            }
            Ok(())
        }
        Err(error) => Err(error.into()),
    }
}

/// Publish into a new or identical bundle directory; changed files never overwrite assets.
pub fn publish_bundle(directory: &Path, bundle: &FrontendBundleView) -> Result<(), FrontendError> {
    validate_prepared_bundle(directory, bundle)?;
    for (relative, bytes) in &bundle.files {
        retain_file(&safe_file(directory, relative)?, bytes)?;
    }
    retain_file(
        &safe_file(directory, "canic-frontend.json")?,
        &serde_json::to_vec_pretty(&bundle.manifest)?,
    )?;
    verify_bundle(directory, &bundle.manifest.manifest_sha256)?;
    Ok(())
}

pub(super) fn manifest_files(manifest: &FrontendManifestRecord) -> Vec<&FrontendFileRecord> {
    let mut files = vec![&manifest.alternative_origins];
    for role in &manifest.roles {
        files.extend([&role.wire_schema, &role.javascript, &role.typescript]);
        files.extend(&role.modules);
    }
    files
}

fn validate_prepared_bundle(
    directory: &Path,
    bundle: &FrontendBundleView,
) -> Result<(), FrontendError> {
    if manifest_files(&bundle.manifest).len() != bundle.files.len() {
        return Err(FrontendError::Integrity);
    }
    super::validation::validate(&bundle.manifest, &bundle.manifest.manifest_sha256, |file| {
        safe_file(directory, &file.path)?;
        bundle
            .files
            .get(&file.path)
            .map(Vec::as_slice)
            .ok_or(FrontendError::Integrity)
    })
}

/// Verify bounded files against an independently retained expected manifest digest.
pub fn verify_bundle(
    directory: &Path,
    expected_digest: &str,
) -> Result<FrontendManifestRecord, FrontendError> {
    let manifest: FrontendManifestRecord = serde_json::from_slice(
        &read_file_no_follow(
            &safe_file(directory, "canic-frontend.json")?,
            MAX_FRONTEND_FILE_BYTES,
        )
        .map_err(std::io::Error::from)?,
    )?;
    super::validation::validate(&manifest, expected_digest, |file| {
        read_file_no_follow(&safe_file(directory, &file.path)?, MAX_FRONTEND_FILE_BYTES)
            .map_err(std::io::Error::from)
            .map_err(FrontendError::from)
    })?;
    Ok(manifest)
}
