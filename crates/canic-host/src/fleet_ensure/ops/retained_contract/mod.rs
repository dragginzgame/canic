//! Inspect local completion claims before decoding current executable contracts.
//!
//! This check grants no recovery authority and does not verify live completion.
//! Original documents are read without a lock, mutation, build or network call.

#[cfg(test)]
mod tests;

use crate::{
    durable_io::read_regular_bytes,
    fleet_ensure::{
        ops::{EnsurePaths, is_sha256},
        policy::{EnsurePolicyError, validate_path_labels},
    },
};
use serde_json::Value;
use std::{io, path::Path};
use thiserror::Error;

pub use crate::fleet_ensure::ops::reinstall::terminal::inventory::custody::{
    CompletedCustodyError, inspect as inspect_completed_custody,
};
pub use crate::fleet_ensure::ops::reinstall::terminal::inventory::ledger::CompletedLedgerError;
pub use crate::fleet_ensure::ops::reinstall::terminal::inventory::membership::{
    CompletedCoordinatorError, CompletedMembershipError, CompletedParentageError,
    inspect as inspect_completed_membership,
};

const MAXIMUM_DOCUMENT_BYTES: usize = 32 * 1024 * 1024;

/// Bind local source membership and all installed interfaces before any live inspection.
/// Uses the finalized source release; current-release interfaces cannot substitute for it.
pub fn inspect_completed_source(
    workspace: &Path,
    environment: &str,
    fleet: &str,
) -> Result<crate::fleet_ensure::CompletedSourceInspectionView, RetainedContractError> {
    let inventory = inspect_completed_inventory(workspace, environment, fleet)?;
    let source_protocols =
        crate::fleet_ensure::ops::reinstall::terminal::inventory::protocols::inspect(
            workspace, &inventory,
        )?;
    Ok(crate::fleet_ensure::CompletedSourceInspectionView {
        inventory,
        source_protocols,
    })
}

/// Cross-check recorded physical membership after receipt audit, without live calls or writes.
/// This inventory preserves allocated descendants; it does not authorize importing or wiping them.
pub fn inspect_completed_inventory(
    workspace: &Path,
    environment: &str,
    fleet: &str,
) -> Result<crate::fleet_ensure::CompletedEstateInventoryView, RetainedContractError> {
    validate_path_labels(environment, fleet)?;
    Ok(
        crate::fleet_ensure::ops::reinstall::terminal::inventory::inspect(
            &EnsurePaths::under(workspace, environment, fleet),
            environment,
            fleet,
        )?,
    )
}

/// Audit completed historical receipts without building, writing or contacting the IC.
/// Successful local audit still requires fresh live authority and cycle verification.
pub fn inspect_completed_receipts(
    workspace: &Path,
    environment: &str,
    fleet: &str,
) -> Result<crate::fleet_ensure::CompletedReceiptAuditView, RetainedContractError> {
    validate_path_labels(environment, fleet)?;
    Ok(
        crate::fleet_ensure::ops::reinstall::terminal::receipt_audit::inspect(
            &EnsurePaths::under(workspace, environment, fleet),
            environment,
            fleet,
        )?,
    )
}

/// A local source contract cannot be used as current execution authority.
#[derive(Debug, Error)]
pub enum RetainedContractError {
    #[error(
        "Fleet has a reviewed completed-source reset {review_sha256} (target plan {plan_sha256}). Review its wipe and funding scope, then use fleet ensure with the same environment and --apply {review_sha256}. Source evidence remains unchanged until that approval."
    )]
    CompletedResetReviewRequired {
        review_sha256: String,
        plan_sha256: String,
    },

    #[error("completed-source preparation evidence is invalid: {0}")]
    Preparation(
        #[source] Box<crate::fleet_ensure::ops::completed_preparation::CompletedPreparationError>,
    ),
    #[error(
        "Fleet has retained completed-source preparation {review_sha256} (preparation complete: {prepared}). Preserve its evidence and use fleet ensure with the same environment and --apply {review_sha256} to resume or replay preparation. This is not deployment completion or authority for a new build/reset."
    )]
    PreparationRecoveryRequired {
        review_sha256: String,
        prepared: bool,
    },
    #[error(
        "Fleet local authority publication was interrupted after approval (review {review_sha256}, target plan {plan_sha256}). Preserve the records and resume the already reviewed apply; the Fleet lock recovers publication before current state decoding. No new build or source reset is needed."
    )]
    PublicationRecoveryRequired {
        review_sha256: String,
        plan_sha256: String,
    },
    #[error("completed source infrastructure interface verification failed: {0}")]
    SourceProtocol(#[from] crate::fleet_ensure::CompletedSourceProtocolError),
    #[error("completed source evidence audit failed; preserve its original evidence: {0}")]
    ReceiptAudit(#[from] crate::fleet_ensure::ops::EnsureStateError),
    #[error(transparent)]
    Policy(#[from] EnsurePolicyError),
    #[error("cannot read retained Fleet contract before build: {0}")]
    Read(#[from] io::Error),
    #[error("cannot inspect retained Fleet JSON before build: {0}")]
    Decode(#[from] serde_json::Error),
    #[error("retained Fleet completion identity is inconsistent; preserve its evidence")]
    IdentityMismatch,
    #[error(
        "Fleet {fleet} retains completed operation {operation_id} under a different generated authority contract (plan {plan_sha256}). Verified {effect_count} local action receipts across {phase_count} successor phases, cross-checked {canister_count} recorded physical canisters and bound infrastructure interfaces to the finalized source release. The source has no recovery-controller declaration. Updating operator configuration cannot rewrite this evidence. Preserve the original desired document, plan, journal, state, phases and artifacts. A reviewed completed-estate hard-cut handoff is required before current Fleet execution; this check does not authorize a reset or validate live completion."
    )]
    CompletedAuthorityContract {
        fleet: String,
        operation_id: String,
        plan_sha256: String,
        effect_count: usize,
        phase_count: usize,
        canister_count: usize,
    },
}

/// Reject a completed authority-contract boundary before current-schema decoding.
///
/// This is a diagnostic preflight. Passing it does not establish completion,
/// controller authority, receipt integrity or permission to replace local files.
pub fn check(
    workspace: &Path,
    environment: &str,
    fleet: &str,
) -> Result<(), RetainedContractError> {
    validate_path_labels(environment, fleet)?;
    let paths = EnsurePaths::under(workspace, environment, fleet);
    crate::fleet_ensure::ops::capacity_import::journal::require_no_approved_import(&paths)?;
    if let Some(review) = crate::fleet_ensure::ops::completed_handoff::pending(&paths)? {
        return Err(RetainedContractError::PublicationRecoveryRequired {
            review_sha256: review.review_sha256,
            plan_sha256: review.plan_sha256,
        });
    }
    if crate::fleet_ensure::ops::completed_handoff::committed(&paths)?.is_none()
        && let Some(review) = crate::fleet_ensure::ops::completed_handoff::review(&paths)?
    {
        return Err(RetainedContractError::CompletedResetReviewRequired {
            review_sha256: review.review_sha256,
            plan_sha256: review.plan_sha256,
        });
    }
    let preparation = crate::fleet_ensure::ops::completed_preparation::review(&paths)
        .map_err(|error| RetainedContractError::Preparation(Box::new(error)))?;
    if let Some(review) = preparation
        && crate::fleet_ensure::ops::completed_handoff::committed(&paths)?.is_none()
    {
        if let Some(journal) =
            crate::fleet_ensure::ops::completed_preparation::journal(&paths, &review)
                .map_err(|error| RetainedContractError::Preparation(Box::new(error)))?
        {
            return Err(RetainedContractError::PreparationRecoveryRequired {
                review_sha256: review.review_sha256,
                prepared: journal.prepared,
            });
        }
    } else {
        crate::fleet_ensure::ops::completed_preparation::require_no_intent(&paths)?;
    }
    let Some(journal) = read(&paths.journal)? else {
        return Ok(());
    };
    if journal.get("completion").and_then(Value::as_str) != Some("converged") {
        return Ok(());
    }
    let plan = read(&paths.plan)?.ok_or(RetainedContractError::IdentityMismatch)?;
    let Some(bootstrap) = plan.pointer("/reviewed_desired/desired/bootstrap") else {
        return Ok(());
    };
    // Current reviews may replace the plan while the completed source journal
    // remains until apply. Their identity belongs to the current workflow guard.
    if !bootstrap.is_object() || bootstrap.get("recovery_controllers").is_some() {
        return Ok(());
    }
    let operation_id = text(&journal, "operation_id")?;
    let plan_sha256 = text(&journal, "plan_sha256")?;
    let matching_identity = journal.get("schema_version") == Some(&Value::from(1))
        && plan.get("schema_version") == Some(&Value::from(1))
        && text(&journal, "fleet")? == fleet
        && text(&plan, "fleet")? == fleet
        && text(&plan, "environment")? == environment
        && text(&plan, "operation_id")? == operation_id
        && text(&plan, "plan_sha256")? == plan_sha256;
    if !matching_identity || !is_sha256(operation_id) || !is_sha256(plan_sha256) {
        return Err(RetainedContractError::IdentityMismatch);
    }
    let source = inspect_completed_source(workspace, environment, fleet)?;
    let audit = &source.inventory.receipts;
    // Bind the audit to the first observation as well as its own coherent snapshot.
    if audit.documents.operation_id != operation_id || audit.documents.plan_sha256 != plan_sha256 {
        return Err(RetainedContractError::IdentityMismatch);
    }
    Err(RetainedContractError::CompletedAuthorityContract {
        fleet: fleet.to_string(),
        operation_id: operation_id.to_string(),
        plan_sha256: plan_sha256.to_string(),
        effect_count: audit.effect_count,
        phase_count: audit.phase_count,
        canister_count: source.inventory.canisters.len(),
    })
}

fn read(path: &Path) -> Result<Option<Value>, RetainedContractError> {
    match read_regular_bytes(path, MAXIMUM_DOCUMENT_BYTES) {
        Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn text<'a>(value: &'a Value, field: &str) -> Result<&'a str, RetainedContractError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or(RetainedContractError::IdentityMismatch)
}
