//! Opt-in funding over the installed Blob journal, without endpoints or scheduling.
//!
//! Call only from authenticated IC updates. The actual IC caller must be the
//! installed Blob operator; controller status does not substitute that authority.
//! Applications own evidence acquisition, account-wide completeness and economic
//! policy. Evidence callbacks must establish current facts synchronously, never
//! copy caller flags or cached balance observations. Missing upstream evidence
//! fields remain unknown and block payment. No borrow survives a dispatch await.
//!
//! Use the upstream types directly from `ic-blob-storage`; this module introduces
//! no request schema, second journal, provider receipt or automatic retry.

use crate::ops::{Host, Workflow};
use ic_blob_storage::{
    model::billing::journal::{FundingIntent, renewal::FundingBudgetRenewal},
    ops::service::funding::{FundingJournalError, access::FundingJournalAccess},
    policy::billing::admission::journal::FundingHostEvidence,
    workflow::{
        funding::{
            self, FundingPreparationResult,
            dispatch::{FundingDispatchEvidence, FundingDispatchResult},
        },
        service::ServiceHost,
    },
};
use ic_blob_storage_contracts::upload::binding::UploadContext;

fn context() -> UploadContext {
    UploadContext {
        service: ic_platform::cdk::api::canister_self(),
        actor: ic_platform::cdk::api::msg_caller(),
    }
}

/// Reserve a new exact intent after authenticated, synchronous evidence acquisition.
///
/// Unknown evidence produces the upstream blocked assessment, without reservation.
/// This does not send cycles or authorize a later dispatch from cached evidence.
/// # Errors
/// Returns the shared authority, binding, identity and storage refusals.
/// # Panics
/// Platform and stable traps propagate; never catch them to continue a payment.
pub fn prepare(
    intent: FundingIntent,
    observe: impl FnOnce() -> FundingHostEvidence,
) -> Result<FundingPreparationResult, FundingJournalError> {
    funding::prepare(&Workflow::ACCESS, context(), intent, observe)
}

/// Attempt one retained reservation with fresh evidence and settle its exact callback.
///
/// A lost response never authorizes resending. Inspect the original operation via
/// Blob's mounted funding views. The upstream engine refuses attempted identities.
/// # Errors
/// Returns shared authority, identity, transport-binding and settlement failures.
/// # Panics
/// Platform/stable traps propagate, retaining uncertainty rather than inventing retry.
pub async fn dispatch(
    intent: FundingIntent,
    observe: impl FnOnce() -> FundingDispatchEvidence,
) -> Result<FundingDispatchResult, FundingJournalError> {
    funding::dispatch::dispatch(
        &Workflow::ACCESS,
        context(),
        intent,
        observe,
        Host::limits().account,
    )
    .await
}

/// Apply operator-authorized budget renewal within the installed ceiling.
///
/// Applications own grant authorization; a renewal neither sends cycles nor
/// clears uncertainty, releases restore fences or replenishes lifetime slots.
/// # Errors
/// Returns shared authority, reconciliation, identity and budget-bound refusals.
/// # Panics
/// Stable traps propagate for whole-message rollback.
pub fn renew(input: FundingBudgetRenewal) -> Result<bool, FundingJournalError> {
    let execution = context();
    Workflow::ACCESS
        .with_funding_journal(|journal| funding::renewal::increase(journal, execution, input))
}
