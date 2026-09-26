//! Module: fleet_ensure::ops::reinstall::adoption::publication
//!
//! Responsibility: review and recover publication of current plan, journal and state as one set.
//! Does not own: live estate admission, plan compilation, remote effects or historical execution.
//! Boundary: callers must admit live custody/conservation before committing this local handoff.

mod archive;
#[cfg(test)]
mod tests;

use crate::{
    durable_io::{read_regular_bytes, write_bytes},
    fleet_ensure::{
        CompletedEstateCustodyView, json,
        model::{
            FleetEnsureCompletion, FleetEnsureJournalRecord, FleetEnsurePlan,
            FleetEnsureStateRecord, MAX_FLEET_ENSURE_CANISTERS, MAX_FLEET_ENSURE_PROTOCOL_STEPS,
            completed_handoff::{
                CompletedEstatePublicationRecord, CompletedEstatePublicationReviewRecord,
            },
        },
        ops::{
            EnsurePaths, EnsureStateError, is_sha256, lock_completed_source, read_document_bytes,
            reinstall::terminal::inventory::custody, retained_contract::inspect_completed_source,
            write_current,
        },
        policy::expected_plan_sha256,
    },
};
use canic_core::cdk::utils::hash::sha256_hex;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

/// Exact identity shared by the replacement plan and its fresh journal.
#[derive(Eq, PartialEq)]
struct OperationIdentity<'a> {
    schema_version: u16,
    operation_id: &'a str,
    plan_sha256: &'a str,
    fleet: &'a str,
}

/// Archive completed source evidence before a separately journaled preparation effect.
pub(in crate::fleet_ensure) fn retain_completed_source(
    paths: &EnsurePaths,
    source: &crate::fleet_ensure::CompletedSourceInspectionView,
) -> Result<std::collections::BTreeMap<String, String>, EnsureStateError> {
    archive::retain_source(paths, &source.inventory.receipts.documents)?;
    archive::retain_artifacts(paths, source)
}

/// Stage a separately reviewable local replacement without changing active source documents.
///
/// Requires fresh certified physical custody. Balances, mutation fencing and remote
/// effect admission remain with the workflow; this does not approve wipes or payments.
pub fn stage(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
    observed_custody: &CompletedEstateCustodyView,
) -> Result<CompletedEstatePublicationReviewRecord, EnsureStateError> {
    let _lock = lock_completed_source(paths)?;
    stage_locked(paths, plan, journal, state, observed_custody)
}

pub(in crate::fleet_ensure) fn stage_locked(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
    observed_custody: &CompletedEstateCustodyView,
) -> Result<CompletedEstatePublicationReviewRecord, EnsureStateError> {
    validate_target(plan, journal, state)?;
    require_paths(paths, &plan.environment, &plan.fleet)?;
    if marker(paths)?.is_some() {
        return Err(conflict());
    }
    let source = inspect_completed_source(&paths.workspace, &plan.environment, &plan.fleet)
        .map_err(|error| EnsureStateError::CompletedHandoffSource(Box::new(error)))?;
    let custody = custody::capture(observed_custody, &source.inventory.receipts.documents)
        .map_err(|error| EnsureStateError::CompletedHandoffCustody(Box::new(error)))?;
    let desired = plan
        .reviewed_desired
        .as_ref()
        .ok_or_else(conflict)?
        .desired();
    let source_ids = source
        .inventory
        .canisters
        .values()
        .map(|entry| entry.principal.to_text())
        .collect::<BTreeSet<_>>();
    let target_ids = state.principals.values().cloned().collect::<BTreeSet<_>>();
    if source_ids != target_ids
        || plan.operation_id == source.inventory.receipts.documents.operation_id
        || desired.operator != source.inventory.receipts.source_operator
        || desired.cycles_ledger != source.inventory.receipts.cycles_ledger
    {
        return Err(conflict());
    }
    let replacement = archive::retain_target(paths, plan, journal, state)?;
    archive::retain_source(paths, &source.inventory.receipts.documents)?;
    let source_artifacts = archive::retain_artifacts(paths, &source)?;
    let mut review = CompletedEstatePublicationReviewRecord {
        schema_version: 1,
        environment: plan.environment.clone(),
        fleet: plan.fleet.clone(),
        operation_id: plan.operation_id.clone(),
        plan_sha256: plan.plan_sha256.clone(),
        source: source.inventory.receipts.documents,
        custody,
        replacement,
        source_artifacts,
        review_sha256: String::new(),
    };
    review.review_sha256 = review_digest(&review)?;
    validate_review(&review)?;
    write_current(&review_path(paths), &review)?;
    Ok(review)
}

/// Return the staged publication review until its exact handoff has completed.
pub fn review(
    paths: &EnsurePaths,
) -> Result<Option<CompletedEstatePublicationReviewRecord>, EnsureStateError> {
    let Some(review) = read_review(&review_path(paths))? else {
        return Ok(None);
    };
    if let Some(marker) = marker(paths)? {
        if marker.review_sha256 != review.review_sha256 || !marker.complete {
            return Err(conflict());
        }
        return Ok(None);
    }
    require_paths(paths, &review.environment, &review.fleet)?;
    archive::verify(paths, &review)?;
    verify_active_source(paths, &review)?;
    Ok(Some(review))
}

/// Commit approved local authority after the workflow's live custody/conservation admission.
/// An exact completed replay performs no writes and never resets later operation progress.
pub fn adopt(
    paths: &EnsurePaths,
    approved_review_sha256: &str,
    observed_custody: Option<&CompletedEstateCustodyView>,
) -> Result<(), EnsureStateError> {
    let _lock = lock_completed_source(paths)?;
    if let Some(marker) = marker(paths)? {
        if marker.review_sha256 != approved_review_sha256 {
            return Err(conflict());
        }
        return recover(paths);
    }
    let review = review(paths)?.ok_or_else(conflict)?;
    if review.review_sha256 != approved_review_sha256 {
        return Err(conflict());
    }
    let observed = observed_custody.ok_or(EnsureStateError::CompletedHandoffCustodyRequired)?;
    let fresh = custody::capture(observed, &review.source)
        .map_err(|error| EnsureStateError::CompletedHandoffCustody(Box::new(error)))?;
    if !custody::same_authority(&review.custody, &fresh) {
        return Err(conflict());
    }
    let source = inspect_completed_source(&paths.workspace, &review.environment, &review.fleet)
        .map_err(|error| EnsureStateError::CompletedHandoffSource(Box::new(error)))?;
    if source.inventory.receipts.documents != review.source {
        return Err(conflict());
    }
    archive::verify_source_artifacts(paths, &review)?;
    commit(paths, &review)
}

/// Read verified committed publication evidence without consulting predecessor executable state.
pub fn committed(
    paths: &EnsurePaths,
) -> Result<Option<CompletedEstatePublicationReviewRecord>, EnsureStateError> {
    marker(paths)?
        .map(|marker| committed_review(paths, &marker))
        .transpose()
}

/// The archived current plan is immutable even after its published journal advances.
pub fn replacement_plan(
    paths: &EnsurePaths,
    review: &CompletedEstatePublicationReviewRecord,
) -> Result<FleetEnsurePlan, EnsureStateError> {
    archive::verify(paths, review)?;
    let bytes = super::exact_bytes(
        &super::object_path(paths, &review.replacement.plan_sha256),
        &review.replacement.plan_sha256,
    )?;
    serde_json::from_slice(&bytes).map_err(|_| conflict())
}

/// Inspect an interrupted committed publication without touching active documents.
/// Readiness uses this before any executable source decode; only a locked operation recovers it.
pub(in crate::fleet_ensure) fn pending(
    paths: &EnsurePaths,
) -> Result<Option<CompletedEstatePublicationReviewRecord>, EnsureStateError> {
    let Some(marker) = marker(paths)?.filter(|marker| !marker.complete) else {
        return Ok(None);
    };
    Ok(Some(committed_review(paths, &marker)?))
}

/// Commit only the local intent. All archives and active source bytes have been checked first.
fn commit(
    paths: &EnsurePaths,
    review: &CompletedEstatePublicationReviewRecord,
) -> Result<(), EnsureStateError> {
    archive::verify(paths, review)?;
    verify_active_source(paths, review)?;
    let bytes = serialize(review, &review_path(paths))?;
    let review_document_sha256 = sha256_hex(&bytes);
    super::retain(paths, &review_document_sha256, &bytes)?;
    let marker = CompletedEstatePublicationRecord {
        schema_version: 1,
        review_sha256: review.review_sha256.clone(),
        review_document_sha256,
        complete: false,
    };
    write_current(&marker_path(paths), &marker)?;
    recover(paths)
}

/// Resume committed local intent under the Fleet lock, before current state decoding.
pub(in crate::fleet_ensure) fn recover(paths: &EnsurePaths) -> Result<(), EnsureStateError> {
    let Some(mut marker) = marker(paths)? else {
        return Ok(());
    };
    if marker.complete {
        return Ok(());
    }
    let review = committed_review(paths, &marker)?;
    // Validate every current file and all replacements before writing even the first file.
    let mut pending = Vec::new();
    for (path, before, after) in documents(paths, &review) {
        let current = read_document_bytes(path)?.ok_or_else(conflict)?;
        let digest = sha256_hex(&current);
        if digest != before && digest != after {
            return Err(conflict());
        }
        if digest != after {
            let bytes = super::exact_bytes(&super::object_path(paths, after), after)?;
            pending.push((path, bytes));
        }
    }
    for (path, bytes) in pending {
        write_bytes(path, &bytes).map_err(|error| super::io_error(path, error))?;
    }
    marker.complete = true;
    write_current(&marker_path(paths), &marker)
}

fn committed_review(
    paths: &EnsurePaths,
    marker: &CompletedEstatePublicationRecord,
) -> Result<CompletedEstatePublicationReviewRecord, EnsureStateError> {
    let bytes = super::exact_bytes(
        &super::object_path(paths, &marker.review_document_sha256),
        &marker.review_document_sha256,
    )?;
    let review: CompletedEstatePublicationReviewRecord =
        serde_json::from_slice(&bytes).map_err(|_| conflict())?;
    validate_review(&review)?;
    require_paths(paths, &review.environment, &review.fleet)?;
    if review.review_sha256 != marker.review_sha256 {
        return Err(conflict());
    }
    archive::verify(paths, &review)?;
    Ok(review)
}

fn validate_target(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
) -> Result<(), EnsureStateError> {
    let desired = plan
        .reviewed_desired
        .as_ref()
        .ok_or_else(conflict)?
        .desired();
    let expected_identity = OperationIdentity {
        schema_version: 1,
        operation_id: &plan.operation_id,
        plan_sha256: &plan.plan_sha256,
        fleet: &plan.fleet,
    };
    let journal_identity = OperationIdentity {
        schema_version: journal.schema_version,
        operation_id: &journal.operation_id,
        plan_sha256: &journal.plan_sha256,
        fleet: &journal.fleet,
    };
    let identity_matches = journal_identity == expected_identity
        && (
            plan.schema_version,
            state.schema_version,
            desired.schema_version,
        ) == (1, 1, 1)
        && (&state.fleet, &desired.fleet, &desired.environment)
            == (&plan.fleet, &plan.fleet, &plan.environment);
    let fresh_journal = journal.completion == FleetEnsureCompletion::InProgress
        && journal.effects.is_empty()
        && journal.successor_phases.is_empty()
        && journal.funding_reviews.is_empty()
        && journal.funding_observations.is_empty()
        && journal.estate_funding_required.is_none()
        && journal.stalled_observations == 0;
    let fresh_state = state.active_registry.is_none()
        && state.pending_principals.is_empty()
        && state.completed_reinstalls.is_empty()
        && state.completed_reinstall_action_sha256.is_empty()
        && state.completed_reinstall_operation_id.is_none()
        && state.topology.is_empty()
        && state.retained_cycles_by_principal.is_empty();
    if !identity_matches
        || !fresh_journal
        || !fresh_state
        || desired.bootstrap.is_none()
        || desired.protocol.is_none()
        || !is_sha256(&plan.operation_id)
        || expected_plan_sha256(plan) != plan.plan_sha256
        || journal.initial_controlled_cycles != plan.conservation.observed_controlled_cycles
        || journal.initial_operator_cycles < plan.conservation.maximum_operator_debit_cycles
    {
        return Err(conflict());
    }
    let configured = desired
        .canisters
        .iter()
        .map(|entry| {
            entry
                .principal
                .as_ref()
                .map(|principal| (entry.name.clone(), principal.clone()))
                .ok_or_else(conflict)
        })
        .collect::<Result<std::collections::BTreeMap<_, _>, _>>()?;
    let unique = configured.values().collect::<BTreeSet<_>>();
    if configured.is_empty()
        || configured.len() != desired.canisters.len()
        || unique.len() != configured.len()
        || configured != state.principals
    {
        return Err(conflict());
    }
    Ok(())
}

fn validate_review(
    review: &CompletedEstatePublicationReviewRecord,
) -> Result<(), EnsureStateError> {
    let source = &review.source;
    let digests = [
        &review.operation_id,
        &review.plan_sha256,
        &source.operation_id,
        &source.plan_sha256,
        &source.plan_document_sha256,
        &source.journal_document_sha256,
        &source.state_document_sha256,
        &review.replacement.plan_sha256,
        &review.replacement.journal_sha256,
        &review.replacement.state_sha256,
    ];
    if review.schema_version != 1
        || review.operation_id == source.operation_id
        || digests.into_iter().any(|hash| !is_sha256(hash))
        || review_digest(review)? != review.review_sha256
        || source
            .phase_document_sha256
            .iter()
            .any(|(label, hash)| !is_sha256(label) || !is_sha256(hash))
        || review.source_artifacts.is_empty()
        || review.custody.canisters.is_empty()
        || review.custody.canisters.len() > MAX_FLEET_ENSURE_CANISTERS
        || review.source_artifacts.len() > MAX_FLEET_ENSURE_CANISTERS + 3
        || source.phase_document_sha256.len() > MAX_FLEET_ENSURE_PROTOCOL_STEPS
        || review
            .source_artifacts
            .values()
            .any(|hash| !is_sha256(hash))
    {
        return Err(conflict());
    }
    crate::fleet_ensure::policy::validate_path_labels(&review.environment, &review.fleet)
        .map_err(|_| conflict())?;
    Ok(())
}

fn review_digest(
    review: &CompletedEstatePublicationReviewRecord,
) -> Result<String, EnsureStateError> {
    let mut canonical = review.clone();
    canonical.review_sha256.clear();
    let mut bytes = b"canic:completed-estate-publication:v1\0".to_vec();
    bytes.extend(json::to_vec(&canonical).map_err(|_| conflict())?);
    Ok(sha256_hex(&bytes))
}

fn verify_active_source(
    paths: &EnsurePaths,
    review: &CompletedEstatePublicationReviewRecord,
) -> Result<(), EnsureStateError> {
    for (path, before, _) in documents(paths, review) {
        super::exact_bytes(path, before)?;
    }
    Ok(())
}

fn documents<'a>(
    paths: &'a EnsurePaths,
    review: &'a CompletedEstatePublicationReviewRecord,
) -> [(&'a Path, &'a str, &'a str); 3] {
    [
        (
            &paths.plan,
            &review.source.plan_document_sha256,
            &review.replacement.plan_sha256,
        ),
        (
            &paths.journal,
            &review.source.journal_document_sha256,
            &review.replacement.journal_sha256,
        ),
        (
            &paths.state,
            &review.source.state_document_sha256,
            &review.replacement.state_sha256,
        ),
    ]
}

fn marker(
    paths: &EnsurePaths,
) -> Result<Option<CompletedEstatePublicationRecord>, EnsureStateError> {
    let marker: Option<CompletedEstatePublicationRecord> = read_bounded(&marker_path(paths), 4096)?;
    if let Some(record) = &marker
        && (record.schema_version != 1
            || !is_sha256(&record.review_sha256)
            || !is_sha256(&record.review_document_sha256))
    {
        return Err(conflict());
    }
    Ok(marker)
}
fn read_review(
    path: &Path,
) -> Result<Option<CompletedEstatePublicationReviewRecord>, EnsureStateError> {
    let value: Option<CompletedEstatePublicationReviewRecord> =
        read_bounded(path, 4 * 1024 * 1024)?;
    if let Some(review) = &value {
        validate_review(review)?;
    }
    Ok(value)
}
fn read_bounded<T: serde::de::DeserializeOwned>(
    path: &Path,
    maximum_bytes: usize,
) -> Result<Option<T>, EnsureStateError> {
    let bytes = match read_regular_bytes(path, maximum_bytes) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(super::io_error(path, error)),
    };
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|source| EnsureStateError::Decode {
            path: path.to_path_buf(),
            source,
        })
}

fn require_paths(
    paths: &EnsurePaths,
    environment: &str,
    fleet: &str,
) -> Result<(), EnsureStateError> {
    crate::fleet_ensure::policy::validate_path_labels(environment, fleet)
        .map_err(|_| conflict())?;
    if *paths != EnsurePaths::under(&paths.workspace, environment, fleet) {
        return Err(conflict());
    }
    Ok(())
}
fn serialize(value: &impl serde::Serialize, path: &Path) -> Result<Vec<u8>, EnsureStateError> {
    serde_json::to_vec_pretty(value).map_err(|source| EnsureStateError::Decode {
        path: path.to_path_buf(),
        source,
    })
}
fn review_path(paths: &EnsurePaths) -> PathBuf {
    paths.plan.with_file_name("completed-estate-review.json")
}
fn marker_path(paths: &EnsurePaths) -> PathBuf {
    paths
        .plan
        .with_file_name("completed-estate-publication.json")
}
const fn conflict() -> EnsureStateError {
    EnsureStateError::CompletedHandoffConflict
}
