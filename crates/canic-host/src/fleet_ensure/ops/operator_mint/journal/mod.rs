//! Module: fleet_ensure::ops::operator_mint::journal
//!
//! Responsibility: construct and update conversion review records without effects.
//! Boundary: workflow owns locking, validation, approval and persistence.

use crate::fleet_ensure::{
    model::{
        FleetEnsureJournalRecord, FleetEnsurePlan, FundingReviewRecord,
        operator_mint::{
            OperatorMintAuthority, OperatorMintIntentRecord, OperatorMintNotificationOutcomeRecord,
            OperatorMintNotificationRecord, OperatorMintReceiptRecord, OperatorMintReviewRecord,
            OperatorMintTransferOutcomeRecord,
        },
    },
    ops::operator_mint::{
        OperatorMintWireError, notification_argument,
        receipts::{VerifiedCyclesDeposit, icp::VerifiedIcpTransfer},
        transfer_argument,
    },
};
use candid::Principal;
use canic_core::cdk::utils::hash::{decode_hex, sha256_hex};

pub(in crate::fleet_ensure) fn fresh_quote(
    plan: &FleetEnsurePlan,
    available_cycles: u128,
    rate: crate::fleet_ensure::view::operator_mint::OperatorMintRateQuote,
    estimated_icp_e8s: Option<u64>,
) -> Option<crate::fleet_ensure::view::operator_mint::FreshOperatorFundingQuote> {
    let desired = plan.reviewed_desired.as_ref()?.desired();
    Some(
        crate::fleet_ensure::view::operator_mint::FreshOperatorFundingQuote {
            plan_sha256: plan.plan_sha256.clone(),
            operator: desired.operator.clone(),
            cycles_ledger: desired.cycles_ledger.clone(),
            available_cycles,
            required_debit_cycles: plan.conservation.maximum_operator_debit_cycles,
            shortfall_cycles: plan
                .conservation
                .maximum_operator_debit_cycles
                .saturating_sub(available_cycles),
            estimated_icp_e8s,
            rate,
        },
    )
}

/// Resolve the persisted plan/review authority into an exact wire intent.
pub(in crate::fleet_ensure) fn reviewed_intent(
    plan: &FleetEnsurePlan,
    funding: &FundingReviewRecord,
    authority: OperatorMintAuthority,
    amount: u64,
    fee: u64,
    created_at: u64,
) -> Result<OperatorMintIntentRecord, OperatorMintWireError> {
    let desired = plan
        .reviewed_desired
        .as_ref()
        .ok_or(OperatorMintWireError::InvalidAuthority)?
        .desired();
    if Principal::from_text(&desired.operator).ok() != Some(authority.operator)
        || Principal::from_text(funding.pause.cycles_ledger()).ok() != Some(authority.cycles_ledger)
    {
        return Err(OperatorMintWireError::InvalidAuthority);
    }
    let intent = crate::fleet_ensure::ops::operator_mint::prepare_intent(
        authority, amount, fee, created_at,
    )?;
    if !binds_plan(&intent, plan, &funding.review_sha256) {
        return Err(OperatorMintWireError::InvalidAuthority);
    }
    Ok(intent)
}

pub(in crate::fleet_ensure) fn review(
    intent: OperatorMintIntentRecord,
) -> Result<OperatorMintReviewRecord, OperatorMintWireError> {
    transfer_argument(&intent)?;
    let digest = sha256_hex(
        &serde_json::to_vec(&("canic:operator-mint:review:v1", &intent))
            .expect("mint intent contains only serializable fixed fields"),
    );
    Ok(OperatorMintReviewRecord {
        intent,
        review_sha256: digest,
        transfer_argument: None,
        transfer_outcome: None,
        notification: None,
        receipt: None,
    })
}

/// Construct a credit only from authenticated evidence for the same exact intent.
pub(in crate::fleet_ensure) fn credit(
    review: &OperatorMintReviewRecord,
    transfer: &VerifiedIcpTransfer,
    deposit: &VerifiedCyclesDeposit,
) -> Option<OperatorMintReviewRecord> {
    if transfer.intent() != &review.intent || deposit.intent() != &review.intent {
        return None;
    }
    let mut updated = review.clone();
    updated.receipt = Some(OperatorMintReceiptRecord {
        intent: review.intent.clone(),
        icp_block_index: transfer.block_index(),
        deposit_block_index: deposit.block_index(),
        destination_owner: review.intent.authority.operator,
        destination_subaccount: None,
        deposit_memo: review.intent.deposit_memo,
        gross_minted_cycles: deposit
            .net_credit_cycles()
            .checked_add(deposit.deposit_fee_cycles())?,
        deposit_fee_cycles: deposit.deposit_fee_cycles(),
        net_credit_cycles: deposit.net_credit_cycles(),
    });
    Some(updated)
}

pub(in crate::fleet_ensure) fn transfer_outcome(
    review: &OperatorMintReviewRecord,
    outcome: OperatorMintTransferOutcomeRecord,
) -> OperatorMintReviewRecord {
    let mut updated = review.clone();
    updated.transfer_outcome = Some(outcome);
    updated
}

pub(in crate::fleet_ensure) fn notification(
    review: &OperatorMintReviewRecord,
    icp_block_index: u64,
) -> Result<OperatorMintReviewRecord, OperatorMintWireError> {
    let mut updated = review.clone();
    updated.notification = Some(OperatorMintNotificationRecord {
        icp_block_index,
        argument: notification_argument(&review.intent, icp_block_index)?,
        outcome: None,
    });
    Ok(updated)
}

pub(in crate::fleet_ensure) fn notification_outcome(
    review: &OperatorMintReviewRecord,
    outcome: OperatorMintNotificationOutcomeRecord,
) -> OperatorMintReviewRecord {
    let mut updated = review.clone();
    updated
        .notification
        .as_mut()
        .expect("workflow admitted a retained notification")
        .outcome = Some(outcome);
    updated
}

/// Validate the exact persisted request against its retained intent and block locator.
pub(in crate::fleet_ensure) fn notification_matches(review: &OperatorMintReviewRecord) -> bool {
    review.notification.as_ref().is_none_or(|notification| {
        notification_argument(&review.intent, notification.icp_block_index)
            .is_ok_and(|argument| argument == notification.argument)
    })
}

pub(in crate::fleet_ensure) fn approve(
    review: &OperatorMintReviewRecord,
) -> Result<OperatorMintReviewRecord, OperatorMintWireError> {
    let mut approved = review.clone();
    approved.transfer_argument = Some(transfer_argument(&review.intent)?);
    Ok(approved)
}

pub(in crate::fleet_ensure) fn retain(
    journal: &mut FleetEnsureJournalRecord,
    review: Option<OperatorMintReviewRecord>,
) {
    journal
        .funding_reviews
        .last_mut()
        .expect("workflow validated a funding review")
        .operator_mint = review;
}

/// Compare decoded exact identities without teaching policy about serialization.
pub(in crate::fleet_ensure) fn binds_plan(
    intent: &OperatorMintIntentRecord,
    plan: &FleetEnsurePlan,
    funding_review_sha256: &str,
) -> bool {
    [
        (&intent.authority.operation_id, plan.operation_id.as_str()),
        (&intent.authority.plan_sha256, plan.plan_sha256.as_str()),
        (
            &intent.authority.funding_review_sha256,
            funding_review_sha256,
        ),
    ]
    .into_iter()
    .all(|(expected, text)| decode_hex(text).is_ok_and(|actual| actual.as_slice() == expected))
}
