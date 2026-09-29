//! Cancel only an unapproved reset review, preserving its original bytes before local removal.
//!
//! The ordinary Fleet lock fences every writer; exact-digest retry owns interrupted archival.

#[cfg(test)]
mod tests;

use crate::fleet_ensure::{
    model::{
        clean_reinstall::CleanReinstallCancellationRecord,
        completed_operation::OperationArchiveRecord,
        infrastructure_bootstrap::InfrastructureBootstrapInspectionRecord,
    },
    ops::{
        EnsurePaths, EnsureStateError, is_sha256, lock_fleet_file_without_recovery,
        operation_selection::{self, archive, retirement},
        read_current, write_current,
    },
    view::readiness::InfrastructureFundingUnavailable,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Cancel a digest only while its review has no execution or side-operation authority.
pub(in crate::fleet_ensure) fn cancel(
    paths: &EnsurePaths,
    environment: &str,
    fleet: &str,
    digest: &str,
) -> Result<CleanReinstallCancellationRecord, EnsureStateError> {
    if !is_sha256(digest) {
        return Err(EnsureStateError::ResetReviewConflict);
    }
    let _lock = lock_fleet_file_without_recovery(paths)?;
    if retirement::pending(paths)?.is_some() {
        return Err(EnsureStateError::ResetReviewConflict);
    }
    if let Some(record) = pending(paths)? {
        if record.plan_sha256 != digest {
            return Err(EnsureStateError::ResetReviewConflict);
        }
        finish(paths, &record)?;
        return Ok(record);
    }
    let plan = operation_selection::read(&paths.plan)?;
    if let Some(plan) = &plan
        && plan.get("plan_sha256").and_then(serde_json::Value::as_str) != Some(digest)
    {
        return Err(EnsureStateError::ResetReviewConflict);
    }
    if plan.is_none() {
        let record = completed(paths)?.ok_or(EnsureStateError::ResetReviewConflict)?;
        if record.plan_sha256 != digest {
            return Err(EnsureStateError::ResetReviewConflict);
        }
        require_review_files(paths, digest)?;
        if !cleared(paths)? {
            return Err(EnsureStateError::ResetReviewConflict);
        }
        return Ok(record);
    }
    require_review_files(paths, digest)?;
    let InfrastructureFundingUnavailable::RetainedInfrastructureReview {
        operation_id,
        plan_sha256,
    } = operation_selection::infrastructure_funding_unavailable(paths, environment, fleet)?
    else {
        return Err(EnsureStateError::ResetReviewEffectEvidence {
            path: paths.journal.clone(),
        });
    };
    if plan_sha256 != digest
        || operation_selection::read(&paths.plan.with_file_name("clean-reinstall.json"))?.is_none()
    {
        return Err(EnsureStateError::ResetReviewConflict);
    }
    require_review_files(paths, digest)?;
    let archive_sha256 = archive::capture_review(paths, environment, fleet, &operation_id, digest)?;
    let record = CleanReinstallCancellationRecord {
        schema_version: 1,
        environment: environment.into(),
        fleet: fleet.into(),
        operation_id,
        plan_sha256,
        archive_sha256,
    };
    write_current(&intent_path(paths)?, &record)?;
    finish(paths, &record)?;
    Ok(record)
}

/// Other Fleet writers cannot turn an interrupted cancellation into new execution authority.
pub(in crate::fleet_ensure) fn require_no_pending(
    paths: &EnsurePaths,
) -> Result<(), EnsureStateError> {
    if let Some(record) = pending(paths)? {
        return Err(EnsureStateError::ResetReviewCancellationPending {
            plan_sha256: record.plan_sha256,
        });
    }
    Ok(())
}

/// A cancelled unpaid selection can be replaced using current explicit inventory and artifacts.
pub(in crate::fleet_ensure) fn ready_for_review(
    paths: &EnsurePaths,
) -> Result<bool, EnsureStateError> {
    require_no_pending(paths)?;
    Ok(operation_selection::read(&paths.plan)?.is_none()
        && operation_selection::read(&paths.journal)?.is_none()
        && completed(paths)?.is_some()
        && cleared(paths)?)
}

fn cleared(paths: &EnsurePaths) -> Result<bool, EnsureStateError> {
    let directory = paths
        .plan
        .parent()
        .ok_or(EnsureStateError::ResetReviewConflict)?;
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(true),
        Err(error) => return Err(io(directory, error)),
    };
    for entry in entries {
        let entry = entry.map_err(|source| io(directory, source))?;
        if entry.path() != paths.lock
            || !entry
                .file_type()
                .map_err(|source| io(&entry.path(), source))?
                .is_file()
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn pending(
    paths: &EnsurePaths,
) -> Result<Option<CleanReinstallCancellationRecord>, EnsureStateError> {
    read_record(paths, &intent_path(paths)?)
}

fn completed(
    paths: &EnsurePaths,
) -> Result<Option<CleanReinstallCancellationRecord>, EnsureStateError> {
    read_record(
        paths,
        &retirement::history(paths)?.join("cancelled-review.json"),
    )
}

fn read_record(
    paths: &EnsurePaths,
    path: &Path,
) -> Result<Option<CleanReinstallCancellationRecord>, EnsureStateError> {
    let record: Option<CleanReinstallCancellationRecord> = read_current(path)?;
    if let Some(record) = &record {
        let valid = record.schema_version == 1
            && EnsurePaths::under(&paths.workspace, &record.environment, &record.fleet) == *paths
            && [
                &record.operation_id,
                &record.plan_sha256,
                &record.archive_sha256,
            ]
            .into_iter()
            .all(|digest| is_sha256(digest));
        if !valid {
            return Err(EnsureStateError::ResetReviewConflict);
        }
    }
    Ok(record)
}

fn finish(
    paths: &EnsurePaths,
    record: &CleanReinstallCancellationRecord,
) -> Result<(), EnsureStateError> {
    require_review_files(paths, &record.plan_sha256)?;
    let archive: OperationArchiveRecord = read_current(
        &retirement::history(paths)?
            .join("operations")
            .join(format!("{}.json", record.archive_sha256)),
    )?
    .ok_or(EnsureStateError::ResetReviewConflict)?;
    if archive.operation_id != record.operation_id || archive.plan_sha256 != record.plan_sha256 {
        return Err(EnsureStateError::ResetReviewConflict);
    }
    retirement::remove_archived(
        paths,
        &record.environment,
        &record.fleet,
        &record.archive_sha256,
    )?;
    let history = retirement::history(paths)?;
    write_current(
        &history
            .join("cancelled-reviews")
            .join(format!("{}.json", record.plan_sha256)),
        record,
    )?;
    write_current(&history.join("cancelled-review.json"), record)?;
    let intent = intent_path(paths)?;
    fs::remove_file(&intent).map_err(|source| io(&intent, source))?;
    fs::File::open(&history)
        .and_then(|file| file.sync_all())
        .map_err(|source| io(&history, source))
}

// Only the review owner's known observation files may accompany an unpaid review.
// Any journal, state, import, mint, publication or unknown side owner fails closed.
fn require_review_files(paths: &EnsurePaths, digest: &str) -> Result<(), EnsureStateError> {
    let directory = paths
        .plan
        .parent()
        .ok_or(EnsureStateError::ResetReviewConflict)?;
    for entry in fs::read_dir(directory).map_err(|source| io(directory, source))? {
        let entry = entry.map_err(|source| io(directory, source))?;
        let path = entry.path();
        if path == paths.lock {
            continue;
        }
        let kind = entry.file_type().map_err(|source| io(&path, source))?;
        let name = entry.file_name();
        match name.to_str() {
            Some(
                "plan.json"
                | "clean-reinstall.json"
                | "clean-reinstall-desired.json"
                | "clean-reinstall-custody.json"
                | "infrastructure-bootstrap-survey.json",
            ) if kind.is_file() => {}
            Some("infrastructure-bootstrap-surveys" | "infrastructure-bootstrap-inspections")
                if kind.is_dir() =>
            {
                require_observations(
                    &path,
                    digest,
                    name == "infrastructure-bootstrap-inspections",
                )?;
            }
            _ => return Err(EnsureStateError::ResetReviewEffectEvidence { path }),
        }
    }
    Ok(())
}

fn require_observations(
    directory: &Path,
    digest: &str,
    inspections: bool,
) -> Result<(), EnsureStateError> {
    for (index, entry) in fs::read_dir(directory)
        .map_err(|source| io(directory, source))?
        .enumerate()
    {
        if index >= 8_192 {
            return Err(EnsureStateError::ResetReviewConflict);
        }
        let entry = entry.map_err(|source| io(directory, source))?;
        let path = entry.path();
        let kind = entry.file_type().map_err(|source| io(&path, source))?;
        let named = path
            .file_stem()
            .and_then(|name| name.to_str())
            .is_some_and(is_sha256)
            && path
                .extension()
                .is_some_and(|extension| extension == "json");
        if !kind.is_file() || !named {
            return Err(EnsureStateError::ResetReviewEffectEvidence { path });
        }
        if inspections {
            let record: InfrastructureBootstrapInspectionRecord =
                read_current(&path)?.ok_or(EnsureStateError::ResetReviewConflict)?;
            let no_execution_rounds = [
                record.apply_attempts,
                record.terminal_attempts,
                record.registration_attempts,
            ]
            .into_iter()
            .all(|count| count == 0);
            let no_effect_observations =
                record.effect_observations.values().all(|count| *count == 0);
            let unissued = no_execution_rounds
                && no_effect_observations
                && record.registration_plan_sha256.is_none();
            if record.schema_version != 1 || record.plan_sha256 != digest || !unissued {
                return Err(EnsureStateError::ResetReviewEffectEvidence { path });
            }
        }
    }
    Ok(())
}

fn intent_path(paths: &EnsurePaths) -> Result<PathBuf, EnsureStateError> {
    Ok(retirement::history(paths)?.join("review-cancellation-intent.json"))
}

fn io(path: &Path, source: std::io::Error) -> EnsureStateError {
    EnsureStateError::Io {
        path: path.into(),
        source,
    }
}
