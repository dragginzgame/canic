//! Effect-free review and digest-approved continuation of retained Host attempt counters.
//!
//! The existing Fleet lock owns all writes; original plans, balances and Root caps do not change.

pub(in crate::fleet_ensure) mod allowance;
mod owner;
#[cfg(test)]
mod tests;

use crate::fleet_ensure::{
    model::attempt_recovery::AttemptRecoveryReviewRecord,
    ops::{EnsurePaths, capacity_import::journal::CapacityImportJournalError},
};
use ic_host_fs::durable::write_bytes;
use ic_host_fs::read::read_file_no_follow;

use canic_core::cdk::utils::hash::hex_bytes;
use sha2_host::{Digest, Sha256};

/// Retain exact exhausted resources for operator review without extending any allowance.
pub(in crate::fleet_ensure) fn review(
    paths: &EnsurePaths,
    environment: &str,
    fleet: &str,
) -> Result<AttemptRecoveryReviewRecord, CapacityImportJournalError> {
    let owners = owner::discover(paths)?;
    if owners.is_empty() {
        return Err(CapacityImportJournalError::RequestInvalid);
    }
    let mut review = AttemptRecoveryReviewRecord {
        schema_version: 1,
        environment: environment.into(),
        fleet: fleet.into(),
        owners,
        review_sha256: [0; 32],
    };
    review.review_sha256 = digest(&review)?;
    let path = review_path(paths, review.review_sha256);
    let bytes = serde_json::to_vec_pretty(&review)?;
    if bytes.len() > owner::MAX_BYTES {
        return Err(CapacityImportJournalError::Integrity);
    }
    match ic_host_fs::read::read_optional_file_no_follow(&path, owner::MAX_BYTES)
        .map_err(|_| CapacityImportJournalError::Integrity)?
    {
        Some(bytes) if serde_json::from_slice::<AttemptRecoveryReviewRecord>(&bytes)? != review => {
            return Err(CapacityImportJournalError::Integrity);
        }
        Some(_) => {}
        None => write_bytes(&path, &bytes).map_err(crate::publication::ops::io_error)?,
    }
    Ok(review)
}

/// Apply only exact reviewed snapshots; a retained grant makes replay effect-free.
pub(in crate::fleet_ensure) fn apply(
    paths: &EnsurePaths,
    environment: &str,
    fleet: &str,
    approved: [u8; 32],
) -> Result<AttemptRecoveryReviewRecord, CapacityImportJournalError> {
    let bytes = read_file_no_follow(&review_path(paths, approved), owner::MAX_BYTES)
        .map_err(std::io::Error::from)?;
    let review: AttemptRecoveryReviewRecord = serde_json::from_slice(&bytes)?;
    let owners = review
        .owners
        .iter()
        .map(|owner| &owner.relative_path)
        .collect::<std::collections::BTreeSet<_>>();
    if review.schema_version != 1
        || review.environment != environment
        || review.fleet != fleet
        || review.review_sha256 != approved
        || digest(&review)? != approved
        || review.owners.is_empty()
        || owners.len() != review.owners.len()
    {
        return Err(CapacityImportJournalError::Integrity);
    }
    // Admit every changed owner before publishing any grant. An interrupted local
    // publication may already contain some exact grants, which are skipped on replay.
    let updates = review
        .owners
        .iter()
        .map(|target| owner::prepare(paths, target))
        .collect::<Result<Vec<_>, _>>()?;
    for update in updates.into_iter().flatten() {
        write_bytes(&update.0, &update.1).map_err(crate::publication::ops::io_error)?;
    }
    Ok(review)
}

fn review_path(paths: &EnsurePaths, digest: [u8; 32]) -> std::path::PathBuf {
    paths
        .plan
        .with_file_name("attempt-recovery-reviews")
        .join(format!("{}.json", hex_bytes(digest)))
}

fn digest(review: &AttemptRecoveryReviewRecord) -> Result<[u8; 32], CapacityImportJournalError> {
    let mut review = review.clone();
    review.review_sha256 = [0; 32];
    let mut hash = Sha256::new();
    hash.update(b"canic:host-attempt-recovery-review:v1\0");
    hash.update(serde_json::to_vec(&review)?);
    Ok(hash.finalize().into())
}
