//! Immutable frontend publication and exact-digest verification.
//!
//! The manifest is published last; a retry may fill only identical missing files.

use crate::{
    durable_io::{create_new_bytes_with_parents, read_regular_bytes},
    frontend::{
        FrontendError,
        model::{FrontendFileRecord, FrontendManifestRecord},
        ops::{MAX_FRONTEND_BUNDLE_BYTES, MAX_FRONTEND_FILE_BYTES},
        view::FrontendBundleView,
    },
};
use canic_core::cdk::utils::hash::sha256_hex;
use std::{
    collections::BTreeSet,
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
    match create_new_bytes_with_parents(path, bytes) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if read_regular_bytes(path, MAX_FRONTEND_FILE_BYTES)? != bytes {
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
        files.extend([&role.candid, &role.javascript, &role.typescript]);
    }
    files
}

fn validate_prepared_bundle(
    directory: &Path,
    bundle: &FrontendBundleView,
) -> Result<(), FrontendError> {
    if manifest_digest(&bundle.manifest)? != bundle.manifest.manifest_sha256
        || bundle.manifest.schema_version != 1
    {
        return Err(FrontendError::Integrity);
    }
    let records = manifest_files(&bundle.manifest);
    if records.len() != bundle.files.len()
        || bundle.manifest.roles.is_empty()
        || bundle.manifest.roles.len() > crate::frontend::policy::MAX_EXPORTED_ROLES
    {
        return Err(FrontendError::Integrity);
    }
    let mut names = BTreeSet::new();
    let mut total = 0_usize;
    for file in records {
        safe_file(directory, &file.path)?;
        if file.path == "canic-frontend.json" || !names.insert(&file.path) {
            return Err(FrontendError::Integrity);
        }
        let bytes = bundle
            .files
            .get(&file.path)
            .ok_or(FrontendError::Integrity)?;
        total = total.saturating_add(bytes.len());
        if bytes.len() > MAX_FRONTEND_FILE_BYTES || total > MAX_FRONTEND_BUNDLE_BYTES {
            return Err(FrontendError::Bound("bundle bytes"));
        }
        if bytes.len() as u64 != file.bytes || sha256_hex(bytes) != file.sha256 {
            return Err(FrontendError::Integrity);
        }
    }
    Ok(())
}

/// Verify bounded files against an independently retained expected manifest digest.
pub fn verify_bundle(
    directory: &Path,
    expected_digest: &str,
) -> Result<FrontendManifestRecord, FrontendError> {
    let manifest: FrontendManifestRecord = serde_json::from_slice(&read_regular_bytes(
        &safe_file(directory, "canic-frontend.json")?,
        MAX_FRONTEND_FILE_BYTES,
    )?)?;
    if manifest.schema_version != 1 {
        return Err(FrontendError::Schema);
    }
    if manifest.manifest_sha256 != expected_digest || manifest_digest(&manifest)? != expected_digest
    {
        return Err(FrontendError::Integrity);
    }
    if manifest.roles.is_empty()
        || manifest.roles.len() > crate::frontend::policy::MAX_EXPORTED_ROLES
    {
        return Err(FrontendError::Bound("selected role count"));
    }
    let mut names = BTreeSet::new();
    let mut total = 0_usize;
    for file in manifest_files(&manifest) {
        if file.path == "canic-frontend.json" || !names.insert(&file.path) {
            return Err(FrontendError::Integrity);
        }
        let bytes =
            read_regular_bytes(&safe_file(directory, &file.path)?, MAX_FRONTEND_FILE_BYTES)?;
        total = total.saturating_add(bytes.len());
        if total > MAX_FRONTEND_BUNDLE_BYTES {
            return Err(FrontendError::Bound("bundle bytes"));
        }
        if bytes.len() as u64 != file.bytes || sha256_hex(&bytes) != file.sha256 {
            return Err(FrontendError::Integrity);
        }
    }
    Ok(manifest)
}
