//! Module: fleet_ensure::policy::operator_mint::progress
//!
//! Responsibility: preserve known transaction locators while admitting reply observations.
//! Boundary: pure journal checks do not authenticate receipts or authorize calls/credit.

use crate::fleet_ensure::model::operator_mint::{
    OperatorMintNotificationOutcomeRecord, OperatorMintReviewRecord,
    OperatorMintTransferOutcomeRecord,
};

/// Return an ICP transaction locator, never proof of its contents.
#[must_use]
pub const fn transfer_block(outcome: &OperatorMintTransferOutcomeRecord) -> Option<u64> {
    match outcome {
        OperatorMintTransferOutcomeRecord::Accepted { block_index }
        | OperatorMintTransferOutcomeRecord::Duplicate { block_index } => Some(*block_index),
        _ => None,
    }
}

/// A later reply cannot discard or replace an already observed ICP transaction.
#[must_use]
pub fn admits_transfer(
    review: &OperatorMintReviewRecord,
    next: &OperatorMintTransferOutcomeRecord,
) -> bool {
    review.transfer_argument.is_some()
        && review
            .transfer_outcome
            .as_ref()
            .and_then(transfer_block)
            .is_none_or(|block| transfer_block(next) == Some(block))
}

/// Mint/refund locators are immutable; unresolved replies may advance to new observations.
#[must_use]
pub fn admits_notification(
    review: &OperatorMintReviewRecord,
    next: &OperatorMintNotificationOutcomeRecord,
) -> bool {
    let Some(notification) = &review.notification else {
        return false;
    };
    match &notification.outcome {
        Some(
            previous @ (OperatorMintNotificationOutcomeRecord::Minted { .. }
            | OperatorMintNotificationOutcomeRecord::Refunded { .. }),
        ) => previous == next,
        _ => true,
    }
}

/// Check ordering and block binding independently of wire serialization.
#[must_use]
pub fn valid(review: &OperatorMintReviewRecord) -> bool {
    if review.transfer_argument.is_none() {
        return review.transfer_outcome.is_none()
            && review.notification.is_none()
            && review.receipt.is_none();
    }
    review.notification.as_ref().is_none_or(|notification| {
        review.transfer_outcome.as_ref().and_then(transfer_block)
            == Some(notification.icp_block_index)
    })
}

/// A credited receipt must agree with both retained transaction locators.
#[must_use]
pub fn receipt_matches(review: &OperatorMintReviewRecord) -> bool {
    let Some(receipt) = &review.receipt else {
        return true;
    };
    if review.transfer_outcome.as_ref().and_then(transfer_block) != Some(receipt.icp_block_index) {
        return false;
    }
    matches!(review.notification.as_ref().and_then(|notification| notification.outcome.as_ref()),
        Some(OperatorMintNotificationOutcomeRecord::Minted { deposit_block_index, gross_minted_cycles, .. })
        if *deposit_block_index == receipt.deposit_block_index && *gross_minted_cycles == receipt.gross_minted_cycles)
}
