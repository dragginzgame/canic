//! Bind operator review to paired generator inputs and recover interrupted publication.
//!
//! The caller holds the Fleet lock. Root evidence must precede local publication and release.

#[cfg(test)]
mod tests;

use crate::{
    durable_io::{read_regular_bytes, write_bytes},
    fleet_ensure::{
        generate::capacity_import::{
            prepare_capacity_import_inventory, prepare_initial_import_inventory,
        },
        model::{
            EffectState,
            capacity_import::{
                CapacityImportJournalRecord,
                operation::{
                    CapacityImportDocumentRecord, CapacityImportOperationRecord,
                    CapacityImportOperationReviewRecord, CapacityImportPublicationKind,
                },
            },
        },
        ops::{
            EnsurePaths,
            capacity_import::{
                journal::{CapacityImportJournalError, CapacityImportJournalStore},
                validate_root_status,
            },
        },
    },
};
use canic_core::{
    cdk::utils::hash::{decode_hex, hex_bytes},
    dto::pool_import::{PoolImportPhase, PoolImportStatus},
};
use sha2_host::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Component, Path, PathBuf},
};

const MAX_DOCUMENT_BYTES: usize = 1024 * 1024;
pub(super) const MAX_STATUS_HEX_BYTES: usize = 512 * 1024;
const MAX_SUBMISSIONS: u32 = 2;
const MAX_INSPECTIONS: u32 = 4;

/// Attach exact original/replacement inputs before approval or any paid handoff.
pub fn bind(
    paths: &EnsurePaths,
    journal: &CapacityImportJournalRecord,
    policy: &Path,
    seed: &Path,
) -> Result<CapacityImportJournalRecord, CapacityImportJournalError> {
    bind_kind(
        paths,
        journal,
        policy,
        seed,
        CapacityImportPublicationKind::ExtendEstate,
    )
}

/// Bind the already-declared held capacity only after bootstrap setup has a terminal receipt.
pub(in crate::fleet_ensure) fn bind_initial(
    paths: &EnsurePaths,
    journal: &CapacityImportJournalRecord,
    policy: &Path,
    seed: &Path,
) -> Result<CapacityImportJournalRecord, CapacityImportJournalError> {
    bind_kind(
        paths,
        journal,
        policy,
        seed,
        CapacityImportPublicationKind::InitializeEstate,
    )
}

fn bind_kind(
    paths: &EnsurePaths,
    journal: &CapacityImportJournalRecord,
    policy: &Path,
    seed: &Path,
    publication_kind: CapacityImportPublicationKind,
) -> Result<CapacityImportJournalRecord, CapacityImportJournalError> {
    if journal.approved || journal.operation.is_some() {
        return Err(conflict());
    }
    let policy_path = relative(paths, policy)?;
    let seed_path = relative(paths, seed)?;
    if policy_path == seed_path {
        return Err(conflict());
    }
    let policy_bytes = read_regular_bytes(&target(paths, &policy_path)?, MAX_DOCUMENT_BYTES)?;
    let seed_bytes = read_regular_bytes(&target(paths, &seed_path)?, MAX_DOCUMENT_BYTES)?;
    let inventory =
        inventory_projection(&journal.plan, &policy_bytes, &seed_bytes, publication_kind)
            .map_err(|error| CapacityImportJournalError::InventoryProjection(Box::new(error)))?;
    let (environment, fleet) = labels(paths)?;
    let mut review = CapacityImportOperationReviewRecord {
        schema_version: 1,
        publication_kind,
        plan_sha256: journal.plan.plan_sha256,
        environment,
        fleet,
        policy: document(policy_path, policy_bytes, inventory.policy)?,
        seed: document(seed_path, seed_bytes, inventory.seed)?,
        maximum_submissions_per_step: MAX_SUBMISSIONS,
        maximum_management_observations_per_canister: MAX_INSPECTIONS,
        review_sha256: [0; 32],
    };
    review.review_sha256 = review_digest(&review)?;
    let mut result = journal.clone();
    result.operation = Some(CapacityImportOperationRecord {
        review,
        submissions: BTreeMap::new(),
        inspections: BTreeMap::new(),
        settled_status_candid_hex: None,
        publication_started: false,
        publication_complete: false,
        released_status_candid_hex: None,
    });
    validate(&result)?;
    Ok(result)
}

fn document(
    relative_path: String,
    original: Vec<u8>,
    replacement: crate::fleet_ensure::view::capacity_import::CapacityImportDocumentView,
) -> Result<CapacityImportDocumentRecord, CapacityImportJournalError> {
    Ok(CapacityImportDocumentRecord {
        relative_path,
        original: String::from_utf8(original).map_err(|_| conflict())?,
        replacement: String::from_utf8(replacement.replacement).map_err(|_| conflict())?,
        before_sha256: replacement.before_sha256,
        after_sha256: replacement.after_sha256,
    })
}

/// Verify passive retained evidence without looking at subsequently edited operator inputs.
pub fn validate(journal: &CapacityImportJournalRecord) -> Result<(), CapacityImportJournalError> {
    let Some(operation) = &journal.operation else {
        return Ok(());
    };
    validate_review(journal, &operation.review)?;
    for (step, attempts) in &operation.submissions {
        if !journal.approved
            || !valid_step(journal, step)
            || !(1..=MAX_SUBMISSIONS).contains(attempts)
        {
            return Err(conflict());
        }
    }
    for (id, attempts) in &operation.inspections {
        let source = journal
            .plan
            .sources
            .iter()
            .any(|source| source.binding.canister_id.to_text() == *id);
        let root = journal.plan.authority.root.to_text() == *id;
        if !(source || root) || !(1..=MAX_INSPECTIONS).contains(attempts) {
            return Err(conflict());
        }
    }
    if let Some(bytes) = &operation.settled_status_candid_hex {
        let status = status(journal, bytes)?;
        if status.phase != PoolImportPhase::Ready
            || status.root_receipt.is_none()
            || !journal.approved
            || journal.reservation.is_none()
            || journal.handoffs.iter().any(|handoff| {
                handoff
                    .effect
                    .as_ref()
                    .is_none_or(|effect| effect.state != EffectState::Applied)
            })
        {
            return Err(conflict());
        }
    }
    let unreceipted_publication =
        operation.publication_started && operation.settled_status_candid_hex.is_none();
    let missing_publication_intent =
        operation.publication_complete && !operation.publication_started;
    if unreceipted_publication || missing_publication_intent {
        return Err(conflict());
    }
    if let Some(bytes) = &operation.released_status_candid_hex {
        let released = status(journal, bytes)?;
        let expected = PoolImportPhase::Released {
            publication_sha256: publication_digest(journal)?,
        };
        let mut settled = status(
            journal,
            operation
                .settled_status_candid_hex
                .as_deref()
                .ok_or_else(conflict)?,
        )?;
        settled.phase = expected;
        if !operation.publication_complete || released != settled {
            return Err(conflict());
        }
    }
    Ok(())
}

fn validate_review(
    journal: &CapacityImportJournalRecord,
    review: &CapacityImportOperationReviewRecord,
) -> Result<(), CapacityImportJournalError> {
    crate::fleet_ensure::policy::validate_path_labels(&review.environment, &review.fleet)
        .map_err(|_| conflict())?;
    if review.schema_version != 1
        || review.plan_sha256 != journal.plan.plan_sha256
        || review.review_sha256 != review_digest(review)?
        || review.maximum_submissions_per_step != MAX_SUBMISSIONS
        || review.maximum_management_observations_per_canister != MAX_INSPECTIONS
        || review.policy.relative_path == review.seed.relative_path
    {
        return Err(conflict());
    }
    let projection = inventory_projection(
        &journal.plan,
        review.policy.original.as_bytes(),
        review.seed.original.as_bytes(),
        review.publication_kind,
    )
    .map_err(|_| conflict())?;
    for (document, expected) in [
        (&review.policy, projection.policy),
        (&review.seed, projection.seed),
    ] {
        validate_path(&document.relative_path)?;
        if document.original.len() > MAX_DOCUMENT_BYTES
            || document.replacement.len() > MAX_DOCUMENT_BYTES
            || document.before_sha256 != expected.before_sha256
            || document.after_sha256 != expected.after_sha256
            || document.replacement.as_bytes() != expected.replacement
        {
            return Err(conflict());
        }
    }
    Ok(())
}

/// Check the remaining submission budget before spending a rejection-recovery observation.
pub fn require_submission_allowance(
    journal: &CapacityImportJournalRecord,
    step: &str,
) -> Result<(), CapacityImportJournalError> {
    validate(journal)?;
    if !journal.approved || !valid_step(journal, step) || completed(journal) {
        return Err(conflict());
    }
    let operation = journal.operation.as_ref().ok_or_else(conflict)?;
    if operation.submissions.get(step).copied().unwrap_or(0) >= MAX_SUBMISSIONS {
        return Err(CapacityImportJournalError::BudgetExhausted {
            step: step.to_owned(),
        });
    }
    Ok(())
}

/// Bound host update retries before submission, including a lost transport reply.
pub fn reserve_submission(
    journal: &CapacityImportJournalRecord,
    step: &str,
) -> Result<CapacityImportJournalRecord, CapacityImportJournalError> {
    validate(journal)?;
    if !journal.approved || !valid_step(journal, step) || completed(journal) {
        return Err(conflict());
    }
    let mut result = journal.clone();
    let operation = result.operation.as_mut().ok_or_else(conflict)?;
    consume(&mut operation.submissions, step, MAX_SUBMISSIONS)?;
    Ok(result)
}

/// Bound paid source or Root management observations without resetting budgets on restart.
pub fn reserve_inspection(
    journal: &CapacityImportJournalRecord,
    canister: candid::Principal,
) -> Result<CapacityImportJournalRecord, CapacityImportJournalError> {
    validate(journal)?;
    let known = journal
        .plan
        .sources
        .iter()
        .any(|source| source.binding.canister_id == canister)
        || journal.plan.authority.root == canister;
    if !known || completed(journal) {
        return Err(conflict());
    }
    let mut result = journal.clone();
    let operation = result.operation.as_mut().ok_or_else(conflict)?;
    consume(
        &mut operation.inspections,
        &canister.to_text(),
        MAX_INSPECTIONS,
    )?;
    Ok(result)
}

fn consume(
    counts: &mut BTreeMap<String, u32>,
    step: &str,
    maximum: u32,
) -> Result<(), CapacityImportJournalError> {
    let count = counts.entry(step.to_owned()).or_default();
    if *count >= maximum {
        return Err(CapacityImportJournalError::BudgetExhausted {
            step: step.to_owned(),
        });
    }
    *count += 1;
    Ok(())
}

fn valid_step(journal: &CapacityImportJournalRecord, step: &str) -> bool {
    matches!(step, "reserve" | "settle" | "release")
        || journal.plan.sources.iter().enumerate().any(|(index, _)| {
            ["handoff", "controllers", "confirm", "uninstall", "cleared"]
                .iter()
                .any(|phase| step == format!("{index}:{phase}"))
        })
}

/// Retain authenticated terminal Root receipts before any local generator input changes.
pub fn retain_settled(
    journal: &CapacityImportJournalRecord,
    observed: &PoolImportStatus,
) -> Result<CapacityImportJournalRecord, CapacityImportJournalError> {
    validate_root_status(&journal.plan, observed)?;
    let mut result = journal.clone();
    let operation = result.operation.as_mut().ok_or_else(conflict)?;
    let encoded = hex_bytes(candid::encode_one(observed).map_err(|_| conflict())?);
    if operation
        .settled_status_candid_hex
        .as_ref()
        .is_some_and(|old| old != &encoded)
    {
        return Err(conflict());
    }
    operation.settled_status_candid_hex = Some(encoded);
    validate(&result)?;
    Ok(result)
}

/// Recover either partial write; unrelated edits reject before either input is replaced.
pub fn publish(
    store: &CapacityImportJournalStore,
    paths: &EnsurePaths,
) -> Result<CapacityImportJournalRecord, CapacityImportJournalError> {
    let mut journal = store.read()?.ok_or_else(conflict)?;
    validate(&journal)?;
    let operation = journal.operation.as_ref().ok_or_else(conflict)?;
    if operation.publication_complete {
        return Ok(journal);
    }
    if operation.settled_status_candid_hex.is_none() {
        return Err(conflict());
    }
    verify_inputs(paths, &journal, operation.publication_started)?;
    journal
        .operation
        .as_mut()
        .ok_or_else(conflict)?
        .publication_started = true;
    store.save(&journal)?;
    let review = &journal.operation.as_ref().ok_or_else(conflict)?.review;
    for document in [&review.policy, &review.seed] {
        let path = target(paths, &document.relative_path)?;
        let current = read_regular_bytes(&path, MAX_DOCUMENT_BYTES)?;
        if current == document.replacement.as_bytes() {
            continue;
        }
        if current != document.original.as_bytes() {
            return Err(conflict());
        }
        write_bytes(&path, document.replacement.as_bytes())?;
    }
    journal
        .operation
        .as_mut()
        .ok_or_else(conflict)?
        .publication_complete = true;
    store.save(&journal)?;
    Ok(journal)
}

/// Recheck the exact reviewed files before remote effects or resumed local publication.
pub fn verify_inputs(
    paths: &EnsurePaths,
    journal: &CapacityImportJournalRecord,
    allow_replacement: bool,
) -> Result<(), CapacityImportJournalError> {
    let review = &journal.operation.as_ref().ok_or_else(conflict)?.review;
    if labels(paths)? != (review.environment.clone(), review.fleet.clone()) {
        return Err(conflict());
    }
    for document in [&review.policy, &review.seed] {
        let bytes =
            read_regular_bytes(&target(paths, &document.relative_path)?, MAX_DOCUMENT_BYTES)?;
        if bytes != document.original.as_bytes()
            && !(allow_replacement && bytes == document.replacement.as_bytes())
        {
            return Err(conflict());
        }
    }
    Ok(())
}

/// Bind Root's allocation release to exact reviewed local bytes and terminal conservation.
pub fn publication_digest(
    journal: &CapacityImportJournalRecord,
) -> Result<[u8; 32], CapacityImportJournalError> {
    let operation = journal.operation.as_ref().ok_or_else(conflict)?;
    let settled = operation
        .settled_status_candid_hex
        .as_ref()
        .ok_or_else(conflict)?;
    let mut hash = Sha256::new();
    hash.update(b"canic:capacity-import:publication:v1\0");
    hash.update(operation.review.review_sha256);
    hash.update(Sha256::digest(settled.as_bytes()));
    Ok(hash.finalize().into())
}

/// Preserve release evidence locally so later replay does not query reassigned capacity.
pub fn retain_released(
    journal: &CapacityImportJournalRecord,
    observed: &PoolImportStatus,
) -> Result<CapacityImportJournalRecord, CapacityImportJournalError> {
    let mut result = journal.clone();
    result
        .operation
        .as_mut()
        .ok_or_else(conflict)?
        .released_status_candid_hex = Some(hex_bytes(
        candid::encode_one(observed).map_err(|_| conflict())?,
    ));
    validate(&result)?;
    Ok(result)
}

/// Report durable release evidence after the caller validates the journal.
#[must_use]
pub fn completed(journal: &CapacityImportJournalRecord) -> bool {
    journal
        .operation
        .as_ref()
        .is_some_and(|operation| operation.released_status_candid_hex.is_some())
}

pub(super) fn monotonic(
    before: Option<&CapacityImportOperationRecord>,
    after: Option<&CapacityImportOperationRecord>,
) -> bool {
    match (before, after) {
        (None, None) => true,
        (Some(old), Some(new)) => {
            old.review == new.review
                && old
                    .submissions
                    .iter()
                    .all(|(key, value)| new.submissions.get(key).is_some_and(|next| next >= value))
                && old
                    .inspections
                    .iter()
                    .all(|(key, value)| new.inspections.get(key).is_some_and(|next| next >= value))
                && (old.settled_status_candid_hex.is_none()
                    || old.settled_status_candid_hex == new.settled_status_candid_hex)
                && (!old.publication_started || new.publication_started)
                && (!old.publication_complete || new.publication_complete)
                && (old.released_status_candid_hex.is_none() || old == new)
        }
        _ => false,
    }
}

fn status(
    journal: &CapacityImportJournalRecord,
    value: &str,
) -> Result<PoolImportStatus, CapacityImportJournalError> {
    if value.len() > MAX_STATUS_HEX_BYTES {
        return Err(conflict());
    }
    let bytes = decode_hex(value).map_err(|_| conflict())?;
    let status = candid::decode_one(&bytes).map_err(|_| conflict())?;
    validate_root_status(&journal.plan, &status)?;
    Ok(status)
}

fn review_digest(
    review: &CapacityImportOperationReviewRecord,
) -> Result<[u8; 32], CapacityImportJournalError> {
    let mut review = review.clone();
    review.review_sha256 = [0; 32];
    let mut hash = Sha256::new();
    hash.update(b"canic:capacity-import:operation-review:v1\0");
    hash.update(serde_json::to_vec(&review)?);
    Ok(hash.finalize().into())
}

fn relative(paths: &EnsurePaths, path: &Path) -> Result<String, CapacityImportJournalError> {
    let workspace = paths.workspace.canonicalize()?;
    let path = if path.is_absolute() {
        path.to_owned()
    } else {
        workspace.join(path)
    };
    let relative = path
        .strip_prefix(&workspace)
        .map_err(|_| conflict())?
        .to_str()
        .ok_or_else(conflict)?
        .to_owned();
    target(paths, &relative)?;
    Ok(relative)
}

fn labels(paths: &EnsurePaths) -> Result<(String, String), CapacityImportJournalError> {
    let directory = paths.plan.parent().ok_or_else(conflict)?;
    let fleet = directory
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(conflict)?
        .to_owned();
    let environment = directory
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .ok_or_else(conflict)?
        .to_owned();
    if EnsurePaths::under(&paths.workspace, &environment, &fleet).plan != paths.plan {
        return Err(conflict());
    }
    Ok((environment, fleet))
}

fn validate_path(path: &str) -> Result<(), CapacityImportJournalError> {
    if path.is_empty()
        || !Path::new(path)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(conflict());
    }
    Ok(())
}

fn target(paths: &EnsurePaths, relative: &str) -> Result<PathBuf, CapacityImportJournalError> {
    validate_path(relative)?;
    let workspace = paths.workspace.canonicalize()?;
    let target = workspace.join(relative);
    if target.canonicalize()? != target {
        return Err(conflict());
    }
    Ok(target)
}

const fn conflict() -> CapacityImportJournalError {
    CapacityImportJournalError::PublicationConflict
}

fn inventory_projection(
    plan: &crate::fleet_ensure::model::capacity_import::CapacityImportPlanRecord,
    policy: &[u8],
    seed: &[u8],
    kind: CapacityImportPublicationKind,
) -> Result<
    crate::fleet_ensure::view::capacity_import::CapacityImportInventoryView,
    crate::fleet_ensure::generate::capacity_import::CapacityImportInventoryError,
> {
    match kind {
        CapacityImportPublicationKind::ExtendEstate => {
            prepare_capacity_import_inventory(plan, policy, seed)
        }
        CapacityImportPublicationKind::InitializeEstate => {
            prepare_initial_import_inventory(plan, policy, seed)
        }
    }
}
