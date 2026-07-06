//! Module: ops::runtime::metrics::auth
//!
//! Responsibility: record and snapshot low-cardinality runtime auth metrics.
//! Does not own: auth policy, session state, or endpoint DTOs.
//! Boundary: ops-layer counters consumed by metrics projection and auth recorders.

mod attestation;
mod sessions;

use std::{cell::RefCell, collections::HashMap};

pub use attestation::{record_attestation_epoch_rejected, record_attestation_verify_failed};
pub use sessions::{
    record_session_bootstrap_rejected_capacity, record_session_bootstrap_rejected_disabled,
    record_session_bootstrap_rejected_replay_conflict,
    record_session_bootstrap_rejected_replay_reused,
    record_session_bootstrap_rejected_subject_mismatch,
    record_session_bootstrap_rejected_subject_rejected,
    record_session_bootstrap_rejected_token_invalid, record_session_bootstrap_rejected_ttl_invalid,
    record_session_bootstrap_rejected_wallet_caller_rejected,
    record_session_bootstrap_replay_idempotent, record_session_cleared, record_session_created,
    record_session_fallback_invalid_subject, record_session_fallback_raw_caller,
    record_session_pruned, record_session_replaced,
};

thread_local! {
    static AUTH_METRICS: RefCell<HashMap<AuthMetricKey, u64>> = RefCell::new(HashMap::new());
}

///
/// AuthMetricSurface
///
/// Auth metric surface dimension used by public metrics projection.
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum AuthMetricSurface {
    Attestation,
    Session,
}

impl AuthMetricSurface {
    /// Return the stable public metrics label for this surface.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Attestation => "attestation",
            Self::Session => "session",
        }
    }
}

///
/// AuthMetricOperation
///
/// Auth metric operation dimension used by public metrics projection.
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum AuthMetricOperation {
    Bootstrap,
    IdentityFallback,
    Session,
    Verify,
}

impl AuthMetricOperation {
    /// Return the stable public metrics label for this operation.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Bootstrap => "bootstrap",
            Self::IdentityFallback => "identity_fallback",
            Self::Session => "session",
            Self::Verify => "verify",
        }
    }
}

///
/// AuthMetricOutcome
///
/// Auth metric outcome dimension used by public metrics projection.
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum AuthMetricOutcome {
    Completed,
    Failed,
    Idempotent,
    Rejected,
}

impl AuthMetricOutcome {
    /// Return the stable public metrics label for this outcome.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Idempotent => "idempotent",
            Self::Rejected => "rejected",
        }
    }
}

///
/// AuthMetricReason
///
/// Auth metric reason dimension used by public metrics projection.
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum AuthMetricReason {
    Capacity,
    Cleared,
    Created,
    Disabled,
    EpochRejected,
    InvalidSubject,
    Pruned,
    RawCaller,
    Replaced,
    Replay,
    ReplayConflict,
    ReplayReused,
    SubjectMismatch,
    SubjectRejected,
    TokenInvalid,
    TtlInvalid,
    VerifyFailed,
    WalletCallerRejected,
}

impl AuthMetricReason {
    /// Return the stable public metrics label for this reason.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Capacity => "capacity",
            Self::Cleared => "cleared",
            Self::Created => "created",
            Self::Disabled => "disabled",
            Self::EpochRejected => "epoch_rejected",
            Self::InvalidSubject => "invalid_subject",
            Self::Pruned => "pruned",
            Self::RawCaller => "raw_caller",
            Self::Replaced => "replaced",
            Self::Replay => "replay",
            Self::ReplayConflict => "replay_conflict",
            Self::ReplayReused => "replay_reused",
            Self::SubjectMismatch => "subject_mismatch",
            Self::SubjectRejected => "subject_rejected",
            Self::TokenInvalid => "token_invalid",
            Self::TtlInvalid => "ttl_invalid",
            Self::VerifyFailed => "verify_failed",
            Self::WalletCallerRejected => "wallet_caller_rejected",
        }
    }
}

///
/// AuthMetricKey
///
/// Composite key for one low-cardinality auth metric counter.
///

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct AuthMetricKey {
    pub surface: AuthMetricSurface,
    pub operation: AuthMetricOperation,
    pub outcome: AuthMetricOutcome,
    pub reason: AuthMetricReason,
}

///
/// AuthMetrics
///
/// Operations-layer recorder for auth runtime counters.
///

pub struct AuthMetrics;

impl AuthMetrics {
    /// Record one auth runtime event.
    pub fn record(
        surface: AuthMetricSurface,
        operation: AuthMetricOperation,
        outcome: AuthMetricOutcome,
        reason: AuthMetricReason,
    ) {
        AUTH_METRICS.with_borrow_mut(|counts| {
            let key = AuthMetricKey {
                surface,
                operation,
                outcome,
                reason,
            };
            let entry = counts.entry(key).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    /// Snapshot the current auth metric table as stable rows.
    #[must_use]
    pub fn snapshot() -> Vec<(AuthMetricKey, u64)> {
        AUTH_METRICS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .collect()
    }

    /// Test-only helper: clear all auth metrics.
    #[cfg(test)]
    pub fn reset() {
        AUTH_METRICS.with_borrow_mut(HashMap::clear);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn auth_metric_count(
        surface: AuthMetricSurface,
        operation: AuthMetricOperation,
        outcome: AuthMetricOutcome,
        reason: AuthMetricReason,
    ) -> u64 {
        AuthMetrics::snapshot()
            .into_iter()
            .find_map(|(key, count)| {
                if key.surface == surface
                    && key.operation == operation
                    && key.outcome == outcome
                    && key.reason == reason
                {
                    Some(count)
                } else {
                    None
                }
            })
            .unwrap_or(0)
    }

    fn assert_metric_count(
        surface: AuthMetricSurface,
        operation: AuthMetricOperation,
        outcome: AuthMetricOutcome,
        reason: AuthMetricReason,
        expected: u64,
    ) {
        assert_eq!(
            auth_metric_count(surface, operation, outcome, reason),
            expected
        );
    }

    fn assert_session_metric_count(
        operation: AuthMetricOperation,
        outcome: AuthMetricOutcome,
        reason: AuthMetricReason,
        expected: u64,
    ) {
        assert_metric_count(
            AuthMetricSurface::Session,
            operation,
            outcome,
            reason,
            expected,
        );
    }

    #[test]
    fn session_lifecycle_metrics_increment_expected_auth_dimensions() {
        AuthMetrics::reset();

        record_session_created();
        record_session_replaced();
        record_session_cleared();
        record_session_pruned(2);

        for (reason, expected) in [
            (AuthMetricReason::Created, 1),
            (AuthMetricReason::Replaced, 1),
            (AuthMetricReason::Cleared, 1),
            (AuthMetricReason::Pruned, 2),
        ] {
            assert_session_metric_count(
                AuthMetricOperation::Session,
                AuthMetricOutcome::Completed,
                reason,
                expected,
            );
        }
    }

    #[test]
    fn session_identity_fallback_metrics_increment_expected_auth_dimensions() {
        AuthMetrics::reset();

        record_session_fallback_raw_caller();
        record_session_fallback_invalid_subject();

        for reason in [
            AuthMetricReason::RawCaller,
            AuthMetricReason::InvalidSubject,
        ] {
            assert_session_metric_count(
                AuthMetricOperation::IdentityFallback,
                AuthMetricOutcome::Completed,
                reason,
                1,
            );
        }
    }

    #[test]
    fn session_bootstrap_metrics_increment_expected_auth_dimensions() {
        AuthMetrics::reset();

        for action in [
            record_session_bootstrap_rejected_capacity as fn(),
            record_session_bootstrap_rejected_disabled,
            record_session_bootstrap_rejected_wallet_caller_rejected,
            record_session_bootstrap_rejected_subject_rejected,
            record_session_bootstrap_rejected_replay_conflict,
            record_session_bootstrap_rejected_replay_reused,
            record_session_bootstrap_rejected_token_invalid,
            record_session_bootstrap_rejected_subject_mismatch,
            record_session_bootstrap_rejected_ttl_invalid,
        ] {
            action();
        }
        record_session_bootstrap_replay_idempotent();

        for reason in [
            AuthMetricReason::Capacity,
            AuthMetricReason::Disabled,
            AuthMetricReason::WalletCallerRejected,
            AuthMetricReason::SubjectRejected,
            AuthMetricReason::ReplayConflict,
            AuthMetricReason::ReplayReused,
            AuthMetricReason::TokenInvalid,
            AuthMetricReason::SubjectMismatch,
            AuthMetricReason::TtlInvalid,
        ] {
            assert_session_metric_count(
                AuthMetricOperation::Bootstrap,
                AuthMetricOutcome::Rejected,
                reason,
                1,
            );
        }
        assert_session_metric_count(
            AuthMetricOperation::Bootstrap,
            AuthMetricOutcome::Idempotent,
            AuthMetricReason::Replay,
            1,
        );
    }

    #[test]
    fn attestation_metrics_increment_expected_auth_dimensions() {
        AuthMetrics::reset();

        record_attestation_verify_failed();
        record_attestation_epoch_rejected();

        assert_metric_count(
            AuthMetricSurface::Attestation,
            AuthMetricOperation::Verify,
            AuthMetricOutcome::Failed,
            AuthMetricReason::VerifyFailed,
            1,
        );
        assert_metric_count(
            AuthMetricSurface::Attestation,
            AuthMetricOperation::Verify,
            AuthMetricOutcome::Failed,
            AuthMetricReason::EpochRejected,
            1,
        );
    }
}
