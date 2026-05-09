use super::{
    AuthMetricOperation, AuthMetricOutcome, AuthMetricReason, AuthMetricSurface, AuthMetrics,
    auth_session_endpoint, session_bootstrap_rejected_capacity_predicate,
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
use crate::{ids::AccessMetricKind, ops::runtime::metrics::access::AccessMetrics};

/// Record a rejected session bootstrap when delegated-token auth is disabled.
pub fn record_session_bootstrap_rejected_disabled() {
    record_session_metric(
        AuthMetricOperation::Bootstrap,
        AuthMetricOutcome::Rejected,
        AuthMetricReason::Disabled,
        session_bootstrap_rejected_disabled_predicate(),
    );
}

/// Record a rejected session bootstrap when session state capacity is exhausted.
pub fn record_session_bootstrap_rejected_capacity() {
    record_session_metric(
        AuthMetricOperation::Bootstrap,
        AuthMetricOutcome::Rejected,
        AuthMetricReason::Capacity,
        session_bootstrap_rejected_capacity_predicate(),
    );
}

/// Record a rejected session bootstrap when the wallet caller is not accepted.
pub fn record_session_bootstrap_rejected_wallet_caller_rejected() {
    record_session_metric(
        AuthMetricOperation::Bootstrap,
        AuthMetricOutcome::Rejected,
        AuthMetricReason::WalletCallerRejected,
        session_bootstrap_rejected_wallet_caller_rejected_predicate(),
    );
}

/// Record a rejected session bootstrap when the requested subject is not accepted.
pub fn record_session_bootstrap_rejected_subject_rejected() {
    record_session_metric(
        AuthMetricOperation::Bootstrap,
        AuthMetricOutcome::Rejected,
        AuthMetricReason::SubjectRejected,
        session_bootstrap_rejected_subject_rejected_predicate(),
    );
}

/// Record a rejected session bootstrap when a replay id conflicts.
pub fn record_session_bootstrap_rejected_replay_conflict() {
    record_session_metric(
        AuthMetricOperation::Bootstrap,
        AuthMetricOutcome::Rejected,
        AuthMetricReason::ReplayConflict,
        session_bootstrap_rejected_replay_conflict_predicate(),
    );
}

/// Record a rejected session bootstrap when a replay id was already consumed.
pub fn record_session_bootstrap_rejected_replay_reused() {
    record_session_metric(
        AuthMetricOperation::Bootstrap,
        AuthMetricOutcome::Rejected,
        AuthMetricReason::ReplayReused,
        session_bootstrap_rejected_replay_reused_predicate(),
    );
}

/// Record a rejected session bootstrap when token verification fails.
pub fn record_session_bootstrap_rejected_token_invalid() {
    record_session_metric(
        AuthMetricOperation::Bootstrap,
        AuthMetricOutcome::Rejected,
        AuthMetricReason::TokenInvalid,
        session_bootstrap_rejected_token_invalid_predicate(),
    );
}

/// Record a rejected session bootstrap when token subject and request subject differ.
pub fn record_session_bootstrap_rejected_subject_mismatch() {
    record_session_metric(
        AuthMetricOperation::Bootstrap,
        AuthMetricOutcome::Rejected,
        AuthMetricReason::SubjectMismatch,
        session_bootstrap_rejected_subject_mismatch_predicate(),
    );
}

/// Record a rejected session bootstrap when requested session ttl is invalid.
pub fn record_session_bootstrap_rejected_ttl_invalid() {
    record_session_metric(
        AuthMetricOperation::Bootstrap,
        AuthMetricOutcome::Rejected,
        AuthMetricReason::TtlInvalid,
        session_bootstrap_rejected_ttl_invalid_predicate(),
    );
}

/// Record an idempotent session bootstrap replay.
pub fn record_session_bootstrap_replay_idempotent() {
    record_session_metric(
        AuthMetricOperation::Bootstrap,
        AuthMetricOutcome::Idempotent,
        AuthMetricReason::Replay,
        session_bootstrap_replay_idempotent_predicate(),
    );
}

/// Record creation of a new session.
pub fn record_session_created() {
    record_session_metric(
        AuthMetricOperation::Session,
        AuthMetricOutcome::Completed,
        AuthMetricReason::Created,
        session_created_predicate(),
    );
}

/// Record replacement of an existing session.
pub fn record_session_replaced() {
    record_session_metric(
        AuthMetricOperation::Session,
        AuthMetricOutcome::Completed,
        AuthMetricReason::Replaced,
        session_replaced_predicate(),
    );
}

/// Record clearing of an existing session.
pub fn record_session_cleared() {
    record_session_metric(
        AuthMetricOperation::Session,
        AuthMetricOutcome::Completed,
        AuthMetricReason::Cleared,
        session_cleared_predicate(),
    );
}

/// Record pruning of expired or stale sessions.
pub fn record_session_pruned(removed: usize) {
    for _ in 0..removed {
        record_session_metric(
            AuthMetricOperation::Session,
            AuthMetricOutcome::Completed,
            AuthMetricReason::Pruned,
            session_pruned_predicate(),
        );
    }
}

/// Record identity fallback to the raw caller principal.
pub fn record_session_fallback_raw_caller() {
    record_session_metric(
        AuthMetricOperation::IdentityFallback,
        AuthMetricOutcome::Completed,
        AuthMetricReason::RawCaller,
        session_fallback_raw_caller_predicate(),
    );
}

/// Record identity fallback after an invalid session subject.
pub fn record_session_fallback_invalid_subject() {
    record_session_metric(
        AuthMetricOperation::IdentityFallback,
        AuthMetricOutcome::Completed,
        AuthMetricReason::InvalidSubject,
        session_fallback_invalid_subject_predicate(),
    );
}

// Record one session auth metric in both the dedicated Auth family and legacy Access rows.
fn record_session_metric(
    operation: AuthMetricOperation,
    outcome: AuthMetricOutcome,
    reason: AuthMetricReason,
    predicate: &'static str,
) {
    AuthMetrics::record(AuthMetricSurface::Session, operation, outcome, reason);
    AccessMetrics::increment(auth_session_endpoint(), AccessMetricKind::Auth, predicate);
}
