//! Module: ops::runtime::metrics::auth::sessions
//!
//! Responsibility: record low-cardinality local application-session outcomes.
//! Does not own: session storage, delegated-token verification, or endpoint DTOs.
//! Boundary: auth API/workflow records typed outcomes without identity or scope labels.

use crate::ops::runtime::metrics::auth::{
    AuthMetricOperation, AuthMetricOutcome, AuthMetricReason, AuthMetricSurface, AuthMetrics,
};

/// Record one local application-session establishment attempt.
pub fn record_application_session_establishment_started() {
    record(
        AuthMetricOperation::Establish,
        AuthMetricOutcome::Started,
        AuthMetricReason::Request,
    );
}

/// Record a newly created local application session.
pub fn record_application_session_created() {
    record(
        AuthMetricOperation::Establish,
        AuthMetricOutcome::Completed,
        AuthMetricReason::Created,
    );
}

/// Record an atomic local application-session replacement.
pub fn record_application_session_replaced() {
    record(
        AuthMetricOperation::Establish,
        AuthMetricOutcome::Completed,
        AuthMetricReason::Replaced,
    );
}

/// Record a byte-identical establishment retry.
pub fn record_application_session_idempotent() {
    record(
        AuthMetricOperation::Establish,
        AuthMetricOutcome::Idempotent,
        AuthMetricReason::Replay,
    );
}

/// Record a bounded establishment rejection class.
pub fn record_application_session_rejected(reason: AuthMetricReason) {
    record(
        AuthMetricOperation::Establish,
        AuthMetricOutcome::Rejected,
        reason,
    );
}

/// Record caller-scoped clear, distinguishing missing-state idempotence.
pub fn record_application_session_clear(removed: bool) {
    record(
        AuthMetricOperation::Clear,
        if removed {
            AuthMetricOutcome::Completed
        } else {
            AuthMetricOutcome::Idempotent
        },
        AuthMetricReason::Cleared,
    );
}

/// Record one strict-expiry observation outside the pure authorization function.
pub fn record_application_session_expired_observation() {
    record(
        AuthMetricOperation::ExpiryObservation,
        AuthMetricOutcome::Completed,
        AuthMetricReason::Expired,
    );
}

/// Record the bounded number of expired records removed by one cleanup call.
pub fn record_application_session_cleanup(removed: usize) {
    for _ in 0..removed {
        record(
            AuthMetricOperation::Cleanup,
            AuthMetricOutcome::Completed,
            AuthMetricReason::Expired,
        );
    }
}

/// Record one local authority-generation invalidation.
pub fn record_application_session_generation_invalidation() {
    record(
        AuthMetricOperation::GenerationInvalidation,
        AuthMetricOutcome::Completed,
        AuthMetricReason::GenerationAdvanced,
    );
}

fn record(operation: AuthMetricOperation, outcome: AuthMetricOutcome, reason: AuthMetricReason) {
    AuthMetrics::record(
        AuthMetricSurface::ApplicationSession,
        operation,
        outcome,
        reason,
    );
}
