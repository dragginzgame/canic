//! Module: ops::runtime::metrics::auth::sessions
//!
//! Responsibility: record auth metrics for session lifecycle and bootstrap outcomes.
//! Does not own: session storage, delegated-token verification, or endpoint DTOs.
//! Boundary: auth workflow calls these ops helpers after typed outcomes are known.

use crate::ops::runtime::metrics::auth::{
    AuthMetricOperation, AuthMetricOutcome, AuthMetricReason, AuthMetricSurface, AuthMetrics,
};

/// Record a rejected session bootstrap when delegated-token auth is disabled.
pub fn record_session_bootstrap_rejected_disabled() {
    record_session_metric(
        AuthMetricOperation::Bootstrap,
        AuthMetricOutcome::Rejected,
        AuthMetricReason::Disabled,
    );
}

/// Record a rejected session bootstrap when session state capacity is exhausted.
pub fn record_session_bootstrap_rejected_capacity() {
    record_session_metric(
        AuthMetricOperation::Bootstrap,
        AuthMetricOutcome::Rejected,
        AuthMetricReason::Capacity,
    );
}

/// Record a rejected session bootstrap when the wallet caller is not accepted.
pub fn record_session_bootstrap_rejected_wallet_caller_rejected() {
    record_session_metric(
        AuthMetricOperation::Bootstrap,
        AuthMetricOutcome::Rejected,
        AuthMetricReason::WalletCallerRejected,
    );
}

/// Record a rejected session bootstrap when the requested subject is not accepted.
pub fn record_session_bootstrap_rejected_subject_rejected() {
    record_session_metric(
        AuthMetricOperation::Bootstrap,
        AuthMetricOutcome::Rejected,
        AuthMetricReason::SubjectRejected,
    );
}

/// Record a rejected session bootstrap when a replay id conflicts.
pub fn record_session_bootstrap_rejected_replay_conflict() {
    record_session_metric(
        AuthMetricOperation::Bootstrap,
        AuthMetricOutcome::Rejected,
        AuthMetricReason::ReplayConflict,
    );
}

/// Record a rejected session bootstrap when a replay id was already consumed.
pub fn record_session_bootstrap_rejected_replay_reused() {
    record_session_metric(
        AuthMetricOperation::Bootstrap,
        AuthMetricOutcome::Rejected,
        AuthMetricReason::ReplayReused,
    );
}

/// Record a rejected session bootstrap when token verification fails.
pub fn record_session_bootstrap_rejected_token_invalid() {
    record_session_metric(
        AuthMetricOperation::Bootstrap,
        AuthMetricOutcome::Rejected,
        AuthMetricReason::TokenInvalid,
    );
}

/// Record a rejected session bootstrap when token subject and request subject differ.
pub fn record_session_bootstrap_rejected_subject_mismatch() {
    record_session_metric(
        AuthMetricOperation::Bootstrap,
        AuthMetricOutcome::Rejected,
        AuthMetricReason::SubjectMismatch,
    );
}

/// Record a rejected session bootstrap when requested session ttl is invalid.
pub fn record_session_bootstrap_rejected_ttl_invalid() {
    record_session_metric(
        AuthMetricOperation::Bootstrap,
        AuthMetricOutcome::Rejected,
        AuthMetricReason::TtlInvalid,
    );
}

/// Record an idempotent session bootstrap replay.
pub fn record_session_bootstrap_replay_idempotent() {
    record_session_metric(
        AuthMetricOperation::Bootstrap,
        AuthMetricOutcome::Idempotent,
        AuthMetricReason::Replay,
    );
}

/// Record creation of a new session.
pub fn record_session_created() {
    record_session_metric(
        AuthMetricOperation::Session,
        AuthMetricOutcome::Completed,
        AuthMetricReason::Created,
    );
}

/// Record replacement of an existing session.
pub fn record_session_replaced() {
    record_session_metric(
        AuthMetricOperation::Session,
        AuthMetricOutcome::Completed,
        AuthMetricReason::Replaced,
    );
}

/// Record clearing of an existing session.
pub fn record_session_cleared() {
    record_session_metric(
        AuthMetricOperation::Session,
        AuthMetricOutcome::Completed,
        AuthMetricReason::Cleared,
    );
}

/// Record pruning of expired or stale sessions.
pub fn record_session_pruned(removed: usize) {
    for _ in 0..removed {
        record_session_metric(
            AuthMetricOperation::Session,
            AuthMetricOutcome::Completed,
            AuthMetricReason::Pruned,
        );
    }
}

/// Record identity fallback to the raw caller principal.
pub fn record_session_fallback_raw_caller() {
    record_session_metric(
        AuthMetricOperation::IdentityFallback,
        AuthMetricOutcome::Completed,
        AuthMetricReason::RawCaller,
    );
}

/// Record identity fallback after an invalid session subject.
pub fn record_session_fallback_invalid_subject() {
    record_session_metric(
        AuthMetricOperation::IdentityFallback,
        AuthMetricOutcome::Completed,
        AuthMetricReason::InvalidSubject,
    );
}

fn record_session_metric(
    operation: AuthMetricOperation,
    outcome: AuthMetricOutcome,
    reason: AuthMetricReason,
) {
    AuthMetrics::record(AuthMetricSurface::Session, operation, outcome, reason);
}
