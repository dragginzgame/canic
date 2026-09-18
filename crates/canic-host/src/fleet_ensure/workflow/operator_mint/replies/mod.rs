//! Module: fleet_ensure::workflow::operator_mint::replies
//!
//! Responsibility: durably retain reply observations and intent before notification.
//! Boundary: no IC calls, receipt authentication, retry scheduling or balance credit.

use crate::fleet_ensure::{
    model::operator_mint::{OperatorMintAuthority, OperatorMintReviewRecord},
    ops::{
        EnsurePaths, lock_operation,
        operator_mint::{decode_notification_reply, decode_transfer_reply, journal as records},
        write_journal,
    },
    policy::operator_mint::progress,
    workflow::{
        EnsureWorkflowError,
        operator_mint::{load, selected, verify},
    },
};
use std::convert::Infallible;

/// Retain a reply to the approved transfer without treating its block as verified credit.
pub fn record_transfer_reply(
    paths: &EnsurePaths,
    authority: &OperatorMintAuthority,
    reviewed_sha256: &str,
    reply: &[u8],
) -> Result<OperatorMintReviewRecord, EnsureWorkflowError<Infallible>> {
    update(paths, authority, reviewed_sha256, |retained| {
        if retained.transfer_argument.is_none() {
            return Err(EnsureWorkflowError::OperatorMintReviewConflict);
        }
        let outcome = decode_transfer_reply(reply)?;
        if !progress::admits_transfer(retained, &outcome) {
            return Err(EnsureWorkflowError::OperatorMintReplyConflict);
        }
        if retained
            .transfer_outcome
            .as_ref()
            .and_then(progress::transfer_block)
            .is_some()
        {
            return Ok(retained.clone());
        }
        Ok(records::transfer_outcome(retained, outcome))
    })
}

/// Persist notification bytes against the known ICP block before any notification call.
pub fn prepare_notification(
    paths: &EnsurePaths,
    authority: &OperatorMintAuthority,
    reviewed_sha256: &str,
) -> Result<OperatorMintReviewRecord, EnsureWorkflowError<Infallible>> {
    update(paths, authority, reviewed_sha256, |retained| {
        if retained.notification.is_some() {
            return Ok(retained.clone());
        }
        let block = retained
            .transfer_outcome
            .as_ref()
            .and_then(progress::transfer_block)
            .ok_or(EnsureWorkflowError::OperatorMintReviewConflict)?;
        Ok(records::notification(retained, block)?)
    })
}

/// Retain CMC outcomes as locators/unresolved evidence, never an account-credit receipt.
pub fn record_notification_reply(
    paths: &EnsurePaths,
    authority: &OperatorMintAuthority,
    reviewed_sha256: &str,
    reply: &[u8],
) -> Result<OperatorMintReviewRecord, EnsureWorkflowError<Infallible>> {
    update(paths, authority, reviewed_sha256, |retained| {
        if retained.notification.is_none() {
            return Err(EnsureWorkflowError::OperatorMintReviewConflict);
        }
        let outcome = decode_notification_reply(reply)?;
        if !progress::admits_notification(retained, &outcome) {
            return Err(EnsureWorkflowError::OperatorMintReplyConflict);
        }
        Ok(records::notification_outcome(retained, outcome))
    })
}

fn update(
    paths: &EnsurePaths,
    authority: &OperatorMintAuthority,
    reviewed_sha256: &str,
    derive: impl FnOnce(
        &OperatorMintReviewRecord,
    ) -> Result<OperatorMintReviewRecord, EnsureWorkflowError<Infallible>>,
) -> Result<OperatorMintReviewRecord, EnsureWorkflowError<Infallible>> {
    let _lock = lock_operation(paths)?;
    let (plan, mut journal) = load(paths)?;
    let retained = selected(&journal, authority, reviewed_sha256)?;
    let updated = derive(retained)?;
    if updated == *retained {
        return Ok(updated);
    }
    records::retain(&mut journal, Some(updated.clone()));
    verify(&plan, &journal)?;
    write_journal(paths, &journal)?;
    Ok(updated)
}
