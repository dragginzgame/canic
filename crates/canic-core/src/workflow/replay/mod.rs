//! Module: workflow::replay
//!
//! Responsibility: preserve primary workflow failures when replay cleanup also fails.
//! Does not own: replay decisions, receipt storage, or command-specific recovery policy.
//! Boundary: workflow callers supply an approved cleanup or recovery transition.

use crate::{
    InternalError,
    model::replay::RecoveryReason,
    ops::replay::receipt::{ReplayReceiptToken, abort_reserved_receipt, mark_recovery_required},
};

/// Abort a pre-effect reservation without replacing the primary typed failure.
#[must_use]
pub fn abort_reserved_receipt_after_failure(
    token: &ReplayReceiptToken,
    mut error: InternalError,
    context: &'static str,
) -> InternalError {
    if let Err(cleanup_error) = abort_reserved_receipt(token) {
        error = error.with_diagnostic_context(format!("{context}: {cleanup_error}"));
    }
    error
}

/// Record required recovery without replacing the primary typed failure.
#[must_use]
pub fn mark_recovery_required_after_failure(
    token: &ReplayReceiptToken,
    reason: RecoveryReason,
    now_ns: u64,
    mut error: InternalError,
    context: &'static str,
) -> InternalError {
    if let Err(recovery_error) = mark_recovery_required(token, reason, now_ns) {
        error = error.with_diagnostic_context(format!("{context}: {recovery_error}"));
    }
    error
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::types::Principal,
        dto::error::{Error, ErrorCode},
        model::replay::{CommandKind, OperationId, ReplayActor},
        ops::{
            replay::receipt::{
                ReplayReceiptDecision, ReplayReceiptReserveInput, reserve_or_replay_receipt,
            },
            storage::replay::ReplayReceiptOps,
        },
    };

    fn reserved_token() -> ReplayReceiptToken {
        ReplayReceiptOps::reset_for_tests();
        let input = ReplayReceiptReserveInput::new(
            CommandKind::new("test.failure-preservation.v1").expect("command kind"),
            OperationId::from_bytes([7; 32]),
            ReplayActor::direct_caller(Principal::from_slice(&[1; 29])),
            [9; 32],
            10,
        );
        match reserve_or_replay_receipt(input).expect("reserve receipt") {
            ReplayReceiptDecision::Fresh(token) => token,
            other => panic!("expected fresh receipt, got {other:?}"),
        }
    }

    fn corrupt_receipt(token: &ReplayReceiptToken) {
        let key = token.key();
        let mut record = ReplayReceiptOps::get(key).expect("reserved receipt");
        record.schema_version = u32::MAX;
        ReplayReceiptOps::upsert(key, record);
    }

    fn primary_error() -> InternalError {
        InternalError::public(Error::conflict("primary failure"))
    }

    fn assert_primary_error_preserved(error: &InternalError) {
        assert_eq!(
            error.public_error().map(|error| error.code),
            Some(ErrorCode::Conflict)
        );
    }

    #[test]
    fn abort_failure_preserves_primary_error_projection() {
        let token = reserved_token();
        corrupt_receipt(&token);

        let error = abort_reserved_receipt_after_failure(
            &token,
            primary_error(),
            "test replay cleanup failed",
        );

        assert_primary_error_preserved(&error);
    }

    #[test]
    fn recovery_marker_failure_preserves_primary_error_projection() {
        let token = reserved_token();
        corrupt_receipt(&token);

        let error = mark_recovery_required_after_failure(
            &token,
            RecoveryReason::ExternalEffectStatusUnknown,
            11,
            primary_error(),
            "test replay recovery marker failed",
        );

        assert_primary_error_preserved(&error);
    }
}
