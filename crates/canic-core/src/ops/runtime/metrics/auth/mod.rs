mod attestation;
mod labels;
mod sessions;

use std::{cell::RefCell, collections::HashMap};

pub use attestation::{
    record_attestation_epoch_rejected, record_attestation_refresh_failed,
    record_attestation_unknown_key_id, record_attestation_verify_failed,
};
pub use sessions::{
    record_session_bootstrap_rejected_disabled, record_session_bootstrap_rejected_replay_conflict,
    record_session_bootstrap_rejected_replay_reused,
    record_session_bootstrap_rejected_subject_mismatch,
    record_session_bootstrap_rejected_subject_rejected,
    record_session_bootstrap_rejected_token_invalid, record_session_bootstrap_rejected_ttl_invalid,
    record_session_bootstrap_rejected_wallet_caller_rejected,
    record_session_bootstrap_replay_idempotent, record_session_cleared, record_session_created,
    record_session_fallback_invalid_subject, record_session_fallback_raw_caller,
    record_session_pruned, record_session_replaced,
};

use labels::{
    attestation_epoch_rejected_predicate, attestation_refresh_failed_predicate,
    attestation_unknown_key_id_predicate, attestation_verify_failed_predicate,
    auth_attestation_verifier_endpoint, auth_session_endpoint,
    session_bootstrap_rejected_disabled_predicate,
    session_bootstrap_rejected_replay_conflict_predicate,
    session_bootstrap_rejected_replay_reused_predicate,
    session_bootstrap_rejected_subject_mismatch_predicate,
    session_bootstrap_rejected_subject_rejected_predicate,
    session_bootstrap_rejected_token_invalid_predicate,
    session_bootstrap_rejected_ttl_invalid_predicate,
    session_bootstrap_rejected_wallet_caller_rejected_predicate,
    session_bootstrap_replay_idempotent_predicate, session_cleared_predicate,
    session_created_predicate, session_fallback_invalid_subject_predicate,
    session_fallback_raw_caller_predicate, session_pruned_predicate, session_replaced_predicate,
};

thread_local! {
    static AUTH_METRICS: RefCell<HashMap<AuthMetricKey, u64>> = RefCell::new(HashMap::new());
}

///
/// AuthMetricSurface
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

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum AuthMetricOperation {
    Bootstrap,
    IdentityFallback,
    Refresh,
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
            Self::Refresh => "refresh",
            Self::Session => "session",
            Self::Verify => "verify",
        }
    }
}

///
/// AuthMetricOutcome
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

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum AuthMetricReason {
    Cleared,
    Created,
    Disabled,
    EpochRejected,
    InvalidSubject,
    Pruned,
    RawCaller,
    RefreshFailed,
    Replaced,
    Replay,
    ReplayConflict,
    ReplayReused,
    SubjectMismatch,
    SubjectRejected,
    TokenInvalid,
    TtlInvalid,
    UnknownKeyId,
    VerifyFailed,
    WalletCallerRejected,
}

impl AuthMetricReason {
    /// Return the stable public metrics label for this reason.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Cleared => "cleared",
            Self::Created => "created",
            Self::Disabled => "disabled",
            Self::EpochRejected => "epoch_rejected",
            Self::InvalidSubject => "invalid_subject",
            Self::Pruned => "pruned",
            Self::RawCaller => "raw_caller",
            Self::RefreshFailed => "refresh_failed",
            Self::Replaced => "replaced",
            Self::Replay => "replay",
            Self::ReplayConflict => "replay_conflict",
            Self::ReplayReused => "replay_reused",
            Self::SubjectMismatch => "subject_mismatch",
            Self::SubjectRejected => "subject_rejected",
            Self::TokenInvalid => "token_invalid",
            Self::TtlInvalid => "ttl_invalid",
            Self::UnknownKeyId => "unknown_key_id",
            Self::VerifyFailed => "verify_failed",
            Self::WalletCallerRejected => "wallet_caller_rejected",
        }
    }
}

///
/// AuthMetricKey
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
    use crate::{ids::AccessMetricKind, ops::runtime::metrics::access::AccessMetrics};

    fn metric_count(endpoint: &str, predicate: &str) -> u64 {
        AccessMetrics::snapshot()
            .entries
            .into_iter()
            .find_map(|(key, count)| {
                if key.endpoint == endpoint
                    && key.kind == AccessMetricKind::Auth
                    && key.predicate == predicate
                {
                    Some(count)
                } else {
                    None
                }
            })
            .unwrap_or(0)
    }

    fn assert_auth_metric_count(endpoint: &str, predicate: &str, expected: u64) {
        assert_eq!(metric_count(endpoint, predicate), expected);
    }

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

    #[test]
    fn session_metrics_increment_expected_predicates() {
        AccessMetrics::reset();
        AuthMetrics::reset();

        for action in [
            record_session_created as fn(),
            record_session_replaced,
            record_session_cleared,
            record_session_fallback_raw_caller,
            record_session_fallback_invalid_subject,
            record_session_bootstrap_rejected_disabled,
            record_session_bootstrap_rejected_wallet_caller_rejected,
            record_session_bootstrap_rejected_subject_rejected,
            record_session_bootstrap_rejected_replay_conflict,
            record_session_bootstrap_rejected_replay_reused,
            record_session_bootstrap_rejected_token_invalid,
            record_session_bootstrap_rejected_subject_mismatch,
            record_session_bootstrap_rejected_ttl_invalid,
            record_session_bootstrap_replay_idempotent,
        ] {
            action();
        }
        record_session_pruned(2);

        for (predicate, expected) in [
            (session_created_predicate(), 1),
            (session_replaced_predicate(), 1),
            (session_cleared_predicate(), 1),
            (session_pruned_predicate(), 2),
            (session_fallback_raw_caller_predicate(), 1),
            (session_fallback_invalid_subject_predicate(), 1),
            (session_bootstrap_rejected_disabled_predicate(), 1),
            (
                session_bootstrap_rejected_wallet_caller_rejected_predicate(),
                1,
            ),
            (session_bootstrap_rejected_subject_rejected_predicate(), 1),
            (session_bootstrap_rejected_replay_conflict_predicate(), 1),
            (session_bootstrap_rejected_replay_reused_predicate(), 1),
            (session_bootstrap_rejected_token_invalid_predicate(), 1),
            (session_bootstrap_rejected_subject_mismatch_predicate(), 1),
            (session_bootstrap_rejected_ttl_invalid_predicate(), 1),
            (session_bootstrap_replay_idempotent_predicate(), 1),
        ] {
            assert_auth_metric_count(auth_session_endpoint(), predicate, expected);
        }

        assert_metric_count(
            AuthMetricSurface::Session,
            AuthMetricOperation::Session,
            AuthMetricOutcome::Completed,
            AuthMetricReason::Created,
            1,
        );
        assert_metric_count(
            AuthMetricSurface::Session,
            AuthMetricOperation::Session,
            AuthMetricOutcome::Completed,
            AuthMetricReason::Pruned,
            2,
        );
        assert_metric_count(
            AuthMetricSurface::Session,
            AuthMetricOperation::Bootstrap,
            AuthMetricOutcome::Rejected,
            AuthMetricReason::TokenInvalid,
            1,
        );
        assert_metric_count(
            AuthMetricSurface::Session,
            AuthMetricOperation::Bootstrap,
            AuthMetricOutcome::Idempotent,
            AuthMetricReason::Replay,
            1,
        );
    }

    #[test]
    fn attestation_metrics_increment_expected_predicates() {
        AccessMetrics::reset();
        AuthMetrics::reset();

        record_attestation_verify_failed();
        record_attestation_unknown_key_id();
        record_attestation_epoch_rejected();
        record_attestation_refresh_failed();

        for predicate in [
            attestation_verify_failed_predicate(),
            attestation_unknown_key_id_predicate(),
            attestation_epoch_rejected_predicate(),
            attestation_refresh_failed_predicate(),
        ] {
            assert_auth_metric_count(auth_attestation_verifier_endpoint(), predicate, 1);
        }

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
            AuthMetricReason::UnknownKeyId,
            1,
        );
        assert_metric_count(
            AuthMetricSurface::Attestation,
            AuthMetricOperation::Refresh,
            AuthMetricOutcome::Failed,
            AuthMetricReason::RefreshFailed,
            1,
        );
    }
}
