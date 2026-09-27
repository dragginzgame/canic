//! Retire consumed local approvals before reviewing another completed-estate reset.
//!
//! Completed documents are archived as exact bytes. This owner neither decodes
//! their executable contracts nor changes the current operation or its effects.

#[cfg(test)]
mod tests;

use crate::fleet_ensure::{
    model::completed_handoff::CompletedAuthorityRetirementRecord,
    ops::{
        EnsurePaths, EnsureStateError, completed_handoff, is_sha256, read_current,
        read_document_bytes, reinstall::adoption as storage, write_current,
    },
};
use canic_core::cdk::utils::hash::sha256_hex;
use std::{collections::BTreeMap, fs, path::PathBuf};

const FILES: [&str; 4] = [
    "completed-estate-publication.json",
    "completed-estate-review.json",
    "completed-preparation-journal.json",
    "completed-preparation-review.json",
];

/// Release consumed approvals under the Fleet lock, only after the current work completed.
pub(in crate::fleet_ensure) fn retire(paths: &EnsurePaths) -> Result<(), EnsureStateError> {
    recover(paths)?;
    let Some(completed) = completed_handoff::completed(paths)? else {
        return Ok(());
    };
    let journal: serde_json::Value = read_current(&paths.journal)?.ok_or_else(conflict)?;
    if journal
        .get("completion")
        .and_then(serde_json::Value::as_str)
        != Some("converged")
    {
        // Publication completion retires its preparation, not the replacement's paid work.
        return Ok(());
    }
    if journal.get("fleet").and_then(serde_json::Value::as_str) != Some(&completed.fleet)
        || !completed_handoff::consumed_preparation(paths)?
    {
        return Err(conflict());
    }
    let mut files = BTreeMap::new();
    for name in FILES {
        let path = paths.plan.with_file_name(name);
        if let Some(bytes) = read_document_bytes(&path)? {
            let digest = sha256_hex(&bytes);
            storage::retain(paths, &digest, &bytes)?;
            files.insert(name.into(), digest);
        }
    }
    let record = CompletedAuthorityRetirementRecord {
        schema_version: 1,
        review_sha256: completed.review_sha256,
        files,
    };
    write_current(&intent_path(paths), &record)?;
    recover(paths)
}

/// Resume a local archive/removal transaction before selecting any operation.
pub(in crate::fleet_ensure) fn recover(paths: &EnsurePaths) -> Result<(), EnsureStateError> {
    let Some(record): Option<CompletedAuthorityRetirementRecord> =
        read_current(&intent_path(paths))?
    else {
        return Ok(());
    };
    if record.schema_version != 1
        || !is_sha256(&record.review_sha256)
        || record.files.is_empty()
        || record
            .files
            .keys()
            .any(|name| !FILES.contains(&name.as_str()))
        || record.files.values().any(|digest| !is_sha256(digest))
    {
        return Err(conflict());
    }
    // Verify the whole archive and every remaining active byte before removing any file.
    for (name, digest) in &record.files {
        storage::exact_bytes(&storage::object_path(paths, digest), digest)?;
        if let Some(bytes) = read_document_bytes(&paths.plan.with_file_name(name))?
            && sha256_hex(&bytes) != *digest
        {
            return Err(conflict());
        }
    }
    let history = paths
        .plan
        .with_file_name("completed-authority-history")
        .join(format!("{}.json", record.review_sha256));
    write_current(&history, &record)?;
    for name in record.files.keys() {
        remove(&paths.plan.with_file_name(name))?;
    }
    sync_directory(paths)?;
    remove(&intent_path(paths))?;
    sync_directory(paths)
}

pub(in crate::fleet_ensure) fn pending(paths: &EnsurePaths) -> bool {
    intent_path(paths).exists()
}

fn intent_path(paths: &EnsurePaths) -> PathBuf {
    paths
        .plan
        .with_file_name("completed-authority-retirement.json")
}

fn remove(path: &std::path::Path) -> Result<(), EnsureStateError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(storage::io_error(path, error)),
    }
}

fn sync_directory(paths: &EnsurePaths) -> Result<(), EnsureStateError> {
    let directory = paths.plan.parent().ok_or_else(conflict)?;
    fs::File::open(directory)
        .and_then(|file| file.sync_all())
        .map_err(|error| storage::io_error(directory, error))
}

const fn conflict() -> EnsureStateError {
    EnsureStateError::CompletedHandoffConflict
}
