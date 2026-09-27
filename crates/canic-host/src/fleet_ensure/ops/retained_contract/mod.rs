//! Inspect local completion claims before decoding current executable contracts.
//!
//! This check grants no recovery authority and does not verify live completion.
//! Original documents are read without a lock, mutation, build or network call.

#[cfg(test)]
mod tests;

use crate::fleet_ensure::{
    model::{FleetEnsureCompletion, FleetEnsurePlanScope},
    ops::{EnsurePaths, read_journal, read_plan},
    policy::{EnsurePolicyError, validate_path_labels},
};
use std::path::Path;
use thiserror::Error;

pub use crate::fleet_ensure::ops::reinstall::terminal::inventory::custody::{
    CompletedCustodyError, inspect as inspect_completed_custody,
};
pub use crate::fleet_ensure::ops::reinstall::terminal::inventory::ledger::CompletedLedgerError;
pub use crate::fleet_ensure::ops::reinstall::terminal::inventory::membership::{
    CompletedCoordinatorError, CompletedMembershipError, CompletedParentageError,
    inspect as inspect_completed_membership,
};

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

/// Audit completed current-schema receipts without building, writing or contacting the IC.
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
    #[error("retained Fleet completion identity is inconsistent; preserve its evidence")]
    IdentityMismatch,
}

/// Require pending publication and preparation to use their existing recovery owner.
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
    Ok(())
}

/// Select explicit preparation only for a completed operation in the current schema.
/// Receipt, custody and conservation checks still belong to the preparation workflow.
pub fn completed_source_available(
    workspace: &Path,
    environment: &str,
    fleet: &str,
) -> Result<bool, RetainedContractError> {
    validate_path_labels(environment, fleet)?;
    let paths = EnsurePaths::under(workspace, environment, fleet);
    let Some(journal) = read_journal(&paths)? else {
        return Ok(false);
    };
    if journal.completion != FleetEnsureCompletion::Converged {
        return Ok(false);
    }
    let Some(plan) = read_plan(&paths)? else {
        return Err(RetainedContractError::IdentityMismatch);
    };
    let same_operation =
        plan.operation_id == journal.operation_id && plan.plan_sha256 == journal.plan_sha256;
    let same_scope =
        plan.fleet == fleet && journal.fleet == fleet && plan.environment == environment;
    Ok(same_operation
        && same_scope
        && plan.scope == FleetEnsurePlanScope::Full
        && plan.reinstall.is_none()
        && plan.continuation.is_some()
        && plan.reviewed_desired.as_ref().is_some_and(|reviewed| {
            reviewed
                .desired()
                .bootstrap
                .as_ref()
                .is_some_and(|bootstrap| !bootstrap.fresh_estate)
        }))
}
