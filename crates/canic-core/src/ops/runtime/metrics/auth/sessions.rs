//! Module: ops::runtime::metrics::auth::sessions
//!
//! Responsibility: record low-cardinality local application-session outcomes.
//! Does not own: session storage, delegated-token verification, or endpoint DTOs.
//! Boundary: auth API/workflow records typed outcomes without identity or scope labels.

use crate::ops::runtime::metrics::auth::{
    AuthMetricOperation, AuthMetricOutcome, AuthMetricReason, AuthMetricSurface, AuthMetrics,
};

/// Record the bounded number of expired records removed by one cleanup call.
#[cfg(any(test, feature = "auth-local-application-authorization"))]
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
#[cfg(any(test, feature = "auth-local-application-authorization"))]
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
