//! Shared validation of frontend manifests and their exact file bytes.
//!
//! Callers supply prepared bytes or bounded disk reads; publication remains bundle-owned.

use crate::frontend::{
    FrontendError,
    model::{FrontendFileRecord, FrontendManifestRecord},
    ops::{
        MAX_FRONTEND_BUNDLE_BYTES, MAX_FRONTEND_FILE_BYTES,
        bundle::{manifest_digest, manifest_files},
    },
};
use canic_core::cdk::utils::hash::sha256_hex;
use std::collections::BTreeSet;

pub(super) fn validate<B: AsRef<[u8]>>(
    manifest: &FrontendManifestRecord,
    expected_digest: &str,
    mut read: impl FnMut(&FrontendFileRecord) -> Result<B, FrontendError>,
) -> Result<(), FrontendError> {
    if manifest.schema_version != 1 {
        return Err(FrontendError::Schema);
    }
    if manifest.manifest_sha256 != expected_digest || manifest_digest(manifest)? != expected_digest
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
    for file in manifest_files(manifest) {
        if file.path == "canic-frontend.json" || !names.insert(&file.path) {
            return Err(FrontendError::Integrity);
        }
        let bytes = read(file)?;
        let bytes = bytes.as_ref();
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
