//! Remove completed local execution authority only after preserving its exact historical bytes.
//!
//! The Fleet lock inode remains in place. The external intent survives every removal boundary.

#[cfg(test)]
mod tests;

use crate::{
    durable_io::read_optional_regular_bytes_bounded,
    fleet_ensure::{
        model::completed_operation::{CompletedOperationRetirementRecord, OperationArchiveRecord},
        ops::{
            EnsurePaths, EnsureStateError, is_sha256, operation_selection, read_current,
            write_current,
        },
    },
};
use canic_core::cdk::utils::hash::sha256_hex;
use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path, PathBuf},
};

const MAX_BYTES: usize = 32 * 1024 * 1024;

/// A reviewed bootstrap selects current artifacts before discarding completed local authority.
pub(in crate::fleet_ensure) fn prepare_bootstrap(
    paths: &EnsurePaths,
    desired: &crate::fleet_ensure::model::DesiredFleet,
) -> Result<(), EnsureStateError> {
    let _lock = crate::fleet_ensure::ops::lock_fleet_file(paths)?;
    let Some(_) = operation_selection::completed(paths, &desired.environment, &desired.fleet)?
    else {
        return Ok(());
    };
    let plan = super::read(&paths.plan)?.ok_or_else(invalid)?;
    if plan.get("scope").and_then(serde_json::Value::as_str) == Some("infrastructure_bootstrap") {
        return Ok(());
    }
    crate::fleet_ensure::ops::resolve_desired_artifacts(&paths.workspace, desired)?;
    let bytes = serde_json::to_vec(desired).map_err(|_| invalid())?;
    begin(
        paths,
        &desired.environment,
        &desired.fleet,
        &sha256_hex(&bytes),
    )
}

/// Caller owns the Fleet lock and has qualified the selected replacement before retiring history.
pub(in crate::fleet_ensure) fn begin(
    paths: &EnsurePaths,
    environment: &str,
    fleet: &str,
    replacement_sha256: &str,
) -> Result<(), EnsureStateError> {
    if !is_sha256(replacement_sha256) {
        return Err(invalid());
    }
    if let Some(record) = pending(paths)? {
        if record.replacement_sha256 != replacement_sha256 {
            return Err(invalid());
        }
        return recover(paths);
    }
    let Some(completed) = operation_selection::completed(paths, environment, fleet)? else {
        return Err(invalid());
    };
    if operation_selection::capacity_import_in_progress(paths)? {
        return Err(invalid());
    }
    require_terminal_side_effects(paths, &completed.operation_id)?;
    let archive_sha256 =
        operation_selection::archive::capture(paths, environment, fleet)?.ok_or_else(invalid)?;
    let record = CompletedOperationRetirementRecord {
        schema_version: 1,
        environment: environment.into(),
        fleet: fleet.into(),
        archive_sha256,
        replacement_sha256: replacement_sha256.into(),
    };
    write_current(&intent_path(paths)?, &record)?;
    recover(paths)
}

/// Inspect only side-operation completion metadata; no old executable payload is admitted.
pub(in crate::fleet_ensure) fn require_terminal_side_effects(
    paths: &EnsurePaths,
    operation: &str,
) -> Result<(), EnsureStateError> {
    use serde_json::Value;
    for name in [
        "completed-estate-publication.json",
        "activation-reset-adoption.json",
    ] {
        if let Some(record) = super::read(&paths.plan.with_file_name(name))?
            && record.get("complete").and_then(Value::as_bool) != Some(true)
        {
            return Err(invalid());
        }
    }
    if let Some(journal) = super::read(
        &paths
            .plan
            .with_file_name("completed-preparation-journal.json"),
    )? {
        let review = super::read(
            &paths
                .plan
                .with_file_name("completed-preparation-review.json"),
        )?
        .ok_or_else(invalid)?;
        let source = review
            .pointer("/source/operation_id")
            .and_then(Value::as_str)
            .ok_or_else(invalid)?;
        let review_digest = review
            .get("review_sha256")
            .and_then(Value::as_str)
            .filter(|digest| is_sha256(digest))
            .ok_or_else(invalid)?;
        if source == operation
            || journal.get("prepared").and_then(Value::as_bool) != Some(true)
            || journal.get("review_sha256").and_then(Value::as_str) != Some(review_digest)
        {
            return Err(invalid());
        }
        // The replacement's completed operation supersedes preparation of a different source.
        let publication = super::read(
            &paths
                .plan
                .with_file_name("completed-estate-publication.json"),
        )?
        .ok_or_else(invalid)?;
        if publication.get("complete").and_then(Value::as_bool) != Some(true) {
            return Err(invalid());
        }
    }
    Ok(())
}

/// Finish only the exact archived removals; never remove a changed or newly created file.
pub(in crate::fleet_ensure) fn recover(paths: &EnsurePaths) -> Result<(), EnsureStateError> {
    let Some(record) = pending(paths)? else {
        return Ok(());
    };
    remove_archived(
        paths,
        &record.environment,
        &record.fleet,
        &record.archive_sha256,
    )?;
    let history = history(paths)?;
    write_current(
        &history
            .join("retirements")
            .join(format!("{}.json", record.archive_sha256)),
        &record,
    )?;
    let intent = intent_path(paths)?;
    fs::remove_file(&intent).map_err(|source| io(&intent, source))?;
    sync(&history)
}

/// Remove only an exact archived snapshot; the caller owns its admitted durable intent and lock.
pub(in crate::fleet_ensure::ops) fn remove_archived(
    paths: &EnsurePaths,
    environment: &str,
    fleet: &str,
    archive_sha256: &str,
) -> Result<(), EnsureStateError> {
    let history = history(paths)?;
    let bytes = read(
        &history
            .join("operations")
            .join(format!("{archive_sha256}.json")),
    )?
    .ok_or_else(invalid)?;
    if sha256_hex(&bytes) != archive_sha256 {
        return Err(invalid());
    }
    let archive: OperationArchiveRecord = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
    if archive.schema_version != 1 || archive.environment != environment || archive.fleet != fleet {
        return Err(invalid());
    }
    operation_selection::archive::verify_remaining(paths, &archive)?;
    let directory = paths.plan.parent().ok_or_else(invalid)?;
    let mut removals = Vec::new();
    let mut directories = BTreeSet::new();
    for (name, digest) in &archive.files {
        if !is_sha256(digest) {
            return Err(invalid());
        }
        let object = read(&history.join("objects").join(digest))?.ok_or_else(invalid)?;
        if sha256_hex(&object) != *digest {
            return Err(invalid());
        }
        if let Some(relative) = name.strip_prefix("estate/") {
            let relative = Path::new(relative);
            if relative.as_os_str().is_empty()
                || relative
                    .components()
                    .any(|part| !matches!(part, Component::Normal(_)))
            {
                return Err(invalid());
            }
            let mut ancestor = directory.to_path_buf();
            for component in relative.parent().ok_or_else(invalid)?.components() {
                ancestor.push(component.as_os_str());
                match fs::symlink_metadata(&ancestor) {
                    Ok(metadata) if metadata.is_dir() => {}
                    Ok(_) => return Err(invalid()),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
                    Err(error) => return Err(io(&ancestor, error)),
                }
            }
            let path = directory.join(relative);
            if path == paths.lock {
                return Err(invalid());
            }
            if let Some(bytes) = read(&path)? {
                if sha256_hex(&bytes) != *digest {
                    return Err(invalid());
                }
                removals.push(path.clone());
            }
            let mut parent = path.parent();
            while let Some(current) = parent.filter(|parent| *parent != directory) {
                directories.insert(current.to_path_buf());
                parent = current.parent();
            }
        }
    }
    for path in removals {
        fs::remove_file(&path).map_err(|source| io(&path, source))?;
    }
    for path in directories.iter().rev() {
        match fs::remove_dir(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(io(path, error)),
        }
    }
    sync(directory)
}

pub(in crate::fleet_ensure) fn pending(
    paths: &EnsurePaths,
) -> Result<Option<CompletedOperationRetirementRecord>, EnsureStateError> {
    let record: Option<CompletedOperationRetirementRecord> = read_current(&intent_path(paths)?)?;
    if let Some(record) = &record
        && (record.schema_version != 1
            || !is_sha256(&record.archive_sha256)
            || !is_sha256(&record.replacement_sha256)
            || EnsurePaths::under(&paths.workspace, &record.environment, &record.fleet) != *paths)
    {
        return Err(invalid());
    }
    Ok(record)
}

pub(in crate::fleet_ensure::ops) fn history(
    paths: &EnsurePaths,
) -> Result<PathBuf, EnsureStateError> {
    let fleet = paths
        .plan
        .parent()
        .and_then(Path::file_name)
        .ok_or_else(invalid)?;
    let environment = paths
        .plan
        .parent()
        .and_then(Path::parent)
        .and_then(Path::file_name)
        .ok_or_else(invalid)?;
    Ok(paths
        .workspace
        .join(".canic/fleet-ensure/history")
        .join(environment)
        .join(fleet))
}

fn intent_path(paths: &EnsurePaths) -> Result<PathBuf, EnsureStateError> {
    Ok(history(paths)?.join("retirement-intent.json"))
}
fn read(path: &Path) -> Result<Option<Vec<u8>>, EnsureStateError> {
    read_optional_regular_bytes_bounded(path, MAX_BYTES)
        .map_err(|_| EnsureStateError::Unsafe { path: path.into() })
}
fn sync(path: &Path) -> Result<(), EnsureStateError> {
    fs::File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|source| io(path, source))
}
fn io(path: &Path, source: std::io::Error) -> EnsureStateError {
    EnsureStateError::Io {
        path: path.into(),
        source,
    }
}
const fn invalid() -> EnsureStateError {
    EnsureStateError::InvalidTerminalSource
}
