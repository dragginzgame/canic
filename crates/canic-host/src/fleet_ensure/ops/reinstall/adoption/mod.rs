//! Module: fleet_ensure::ops::reinstall::adoption
//!
//! Responsibility: retain a separate review and atomically recover its local journal handoff.
//! Does not own: reset admission, remote effects or source-plan execution.
//! Boundary: exact source documents are archived before replacement intent is committed.

#[cfg(test)]
pub(in crate::fleet_ensure::ops) mod tests;

use crate::{
    durable_io::{create_new_bytes_with_parents, write_bytes},
    fleet_ensure::{
        model::{ActivationResetAdoptionRecord, FleetEnsureJournalRecord, FleetEnsurePlan},
        ops::{
            EnsurePaths, EnsureStateError, is_sha256, read_current, read_document_bytes, read_plan,
            write_current, write_plan,
        },
        policy::expected_plan_sha256,
    },
};
use canic_core::cdk::utils::hash::sha256_hex;
use std::{io, path::Path};

/// Retain a new review without replacing the active source plan, journal or state.
pub(in crate::fleet_ensure) fn stage(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
) -> Result<(), EnsureStateError> {
    validate_plan(plan)?;
    let source = &plan
        .reinstall
        .as_ref()
        .ok_or_else(conflict)?
        .activation_reset
        .as_ref()
        .ok_or_else(conflict)?
        .source;
    let current = super::source::read(paths, &plan.environment, &plan.fleet)?;
    if current != *source {
        return Err(conflict());
    }
    let pending = review_paths(paths);
    write_plan(&pending, plan)
}

/// Read only a still-applicable review. An adopted review never supersedes later progress.
pub(in crate::fleet_ensure) fn review(
    paths: &EnsurePaths,
) -> Result<Option<FleetEnsurePlan>, EnsureStateError> {
    let Some(plan) = read_plan(&review_paths(paths))? else {
        return Ok(None);
    };
    validate_plan(&plan)?;
    let source = &plan
        .reinstall
        .as_ref()
        .ok_or_else(conflict)?
        .activation_reset
        .as_ref()
        .ok_or_else(conflict)?
        .source;
    if let Some(marker) = marker(paths)? {
        if !marker.complete {
            return Err(conflict());
        }
        let bytes = read_document_bytes(&review_paths(paths).plan)?.ok_or_else(conflict)?;
        if marker.replacement_plan_sha256 == sha256_hex(&bytes) {
            return Ok(None);
        }
    }
    let current = super::source::read(paths, &plan.environment, &plan.fleet)?;
    if current != *source {
        return Err(conflict());
    }
    Ok(Some(plan))
}

/// Commit a validated fresh journal and its reviewed plan under the caller's operation lock.
pub(in crate::fleet_ensure) fn adopt(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
) -> Result<(), EnsureStateError> {
    if review(paths)?.as_ref() != Some(plan)
        || journal.operation_id != plan.operation_id
        || journal.plan_sha256 != plan.plan_sha256
        || journal.fleet != plan.fleet
        || journal.completion != crate::fleet_ensure::model::FleetEnsureCompletion::InProgress
        || !journal.effects.is_empty()
        || !journal.successor_phases.is_empty()
        || !journal.funding_reviews.is_empty()
        || journal.estate_funding_required.is_some()
    {
        return Err(conflict());
    }
    let source = &plan
        .reinstall
        .as_ref()
        .ok_or_else(conflict)?
        .activation_reset
        .as_ref()
        .ok_or_else(conflict)?
        .source;
    let plan_bytes = read_document_bytes(&review_paths(paths).plan)?.ok_or_else(conflict)?;
    let journal_bytes =
        serde_json::to_vec_pretty(journal).map_err(|source| EnsureStateError::Decode {
            path: paths.journal.clone(),
            source,
        })?;
    let intent = ActivationResetAdoptionRecord {
        source_plan_sha256: source.plan_document_sha256.clone(),
        source_journal_sha256: source.journal_document_sha256.clone(),
        source_state_sha256: source.state_document_sha256.clone(),
        replacement_plan_sha256: sha256_hex(&plan_bytes),
        replacement_journal_sha256: sha256_hex(&journal_bytes),
        complete: false,
    };
    for (path, digest) in source_documents(paths, &intent) {
        let bytes = exact_bytes(path, digest)?;
        retain(paths, digest, &bytes)?;
    }
    retain(paths, &intent.replacement_plan_sha256, &plan_bytes)?;
    retain(paths, &intent.replacement_journal_sha256, &journal_bytes)?;
    write_current(&marker_path(paths), &intent)?;
    recover(paths)
}

/// Finish only a committed local handoff, before the journal driver can issue any effect.
pub(in crate::fleet_ensure) fn recover(paths: &EnsurePaths) -> Result<(), EnsureStateError> {
    let Some(mut intent) = marker(paths)? else {
        return Ok(());
    };
    if intent.complete {
        return Ok(());
    }
    // Validate the entire source archive and both active files before replacing either file.
    for (_, digest) in source_documents(paths, &intent) {
        exact_bytes(&object_path(paths, digest), digest)?;
    }
    exact_bytes(&paths.state, &intent.source_state_sha256)?;
    let replacements = [
        (
            &paths.plan,
            &intent.source_plan_sha256,
            &intent.replacement_plan_sha256,
        ),
        (
            &paths.journal,
            &intent.source_journal_sha256,
            &intent.replacement_journal_sha256,
        ),
    ];
    let mut documents = Vec::new();
    for (path, before, after) in replacements {
        let current = read_document_bytes(path)?.ok_or_else(conflict)?;
        let digest = sha256_hex(&current);
        if digest != *before && digest != *after {
            return Err(conflict());
        }
        documents.push((path, exact_bytes(&object_path(paths, after), after)?));
    }
    for (path, bytes) in documents {
        write_bytes(path, &bytes).map_err(|source| io_error(path, source))?;
    }
    intent.complete = true;
    write_current(&marker_path(paths), &intent)
}

fn validate_plan(plan: &FleetEnsurePlan) -> Result<(), EnsureStateError> {
    let intent = plan.reinstall.as_ref().ok_or_else(conflict)?;
    let source = &intent
        .activation_reset
        .as_ref()
        .ok_or_else(conflict)?
        .source;
    if expected_plan_sha256(plan) != plan.plan_sha256
        || plan.scope != crate::fleet_ensure::model::FleetEnsurePlanScope::ReinstallPreparation
        || intent.operation_id != plan.operation_id
        || intent.source_operation_id != source.operation_id
        || intent.operation_id == source.operation_id
    {
        return Err(conflict());
    }
    Ok(())
}

/// Archive a completed current preparation before replacing its review document.
pub(in crate::fleet_ensure) fn retain_preparation(
    paths: &EnsurePaths,
) -> Result<crate::fleet_ensure::model::ActivationPreparationEvidenceRecord, EnsureStateError> {
    let plan = read_document_bytes(&paths.plan)?.ok_or_else(conflict)?;
    let journal = read_document_bytes(&paths.journal)?.ok_or_else(conflict)?;
    let evidence = crate::fleet_ensure::model::ActivationPreparationEvidenceRecord {
        plan_document_sha256: sha256_hex(&plan),
        journal_document_sha256: sha256_hex(&journal),
    };
    retain(paths, &evidence.plan_document_sha256, &plan)?;
    retain(paths, &evidence.journal_document_sha256, &journal)?;
    Ok(evidence)
}

/// Read exact current preparation documents; source documents never enter this decoder.
pub(in crate::fleet_ensure) fn read_preparation(
    paths: &EnsurePaths,
    evidence: &crate::fleet_ensure::model::ActivationPreparationEvidenceRecord,
) -> Result<(FleetEnsurePlan, FleetEnsureJournalRecord), EnsureStateError> {
    if !is_sha256(&evidence.plan_document_sha256) || !is_sha256(&evidence.journal_document_sha256) {
        return Err(conflict());
    }
    let mut retained = paths.clone();
    retained.plan = object_path(paths, &evidence.plan_document_sha256);
    retained.journal = object_path(paths, &evidence.journal_document_sha256);
    exact_bytes(&retained.plan, &evidence.plan_document_sha256)?;
    exact_bytes(&retained.journal, &evidence.journal_document_sha256)?;
    Ok((
        read_plan(&retained)?.ok_or_else(conflict)?,
        crate::fleet_ensure::ops::read_journal(&retained)?.ok_or_else(conflict)?,
    ))
}

fn marker(paths: &EnsurePaths) -> Result<Option<ActivationResetAdoptionRecord>, EnsureStateError> {
    let value: Option<ActivationResetAdoptionRecord> = read_current(&marker_path(paths))?;
    if let Some(record) = &value {
        let valid = [
            &record.source_plan_sha256,
            &record.source_journal_sha256,
            &record.source_state_sha256,
            &record.replacement_plan_sha256,
            &record.replacement_journal_sha256,
        ]
        .into_iter()
        .all(|digest| is_sha256(digest));
        if !valid {
            return Err(conflict());
        }
    }
    Ok(value)
}

fn source_documents<'a>(
    paths: &'a EnsurePaths,
    intent: &'a ActivationResetAdoptionRecord,
) -> [(&'a Path, &'a str); 3] {
    [
        (&paths.plan, &intent.source_plan_sha256),
        (&paths.journal, &intent.source_journal_sha256),
        (&paths.state, &intent.source_state_sha256),
    ]
}

fn review_paths(paths: &EnsurePaths) -> EnsurePaths {
    let mut review = paths.clone();
    review.plan = paths.plan.with_file_name("activation-reset-review.json");
    review
}

fn marker_path(paths: &EnsurePaths) -> std::path::PathBuf {
    paths.plan.with_file_name("activation-reset-adoption.json")
}

fn object_path(paths: &EnsurePaths, digest: &str) -> std::path::PathBuf {
    paths
        .plan
        .with_file_name("activation-reset-evidence")
        .join(digest)
}

fn exact_bytes(path: &Path, digest: &str) -> Result<Vec<u8>, EnsureStateError> {
    let bytes = read_document_bytes(path)?.ok_or_else(conflict)?;
    if sha256_hex(&bytes) != digest {
        return Err(conflict());
    }
    Ok(bytes)
}

fn retain(paths: &EnsurePaths, digest: &str, bytes: &[u8]) -> Result<(), EnsureStateError> {
    if !is_sha256(digest) || sha256_hex(bytes) != digest {
        return Err(conflict());
    }
    let path = object_path(paths, digest);
    match create_new_bytes_with_parents(&path, bytes) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            exact_bytes(&path, digest).map(|_| ())
        }
        Err(error) => Err(io_error(&path, error)),
    }
}

fn io_error(path: &Path, source: io::Error) -> EnsureStateError {
    EnsureStateError::Io {
        path: path.to_path_buf(),
        source,
    }
}

const fn conflict() -> EnsureStateError {
    EnsureStateError::ActivationResetAdoptionConflict
}
