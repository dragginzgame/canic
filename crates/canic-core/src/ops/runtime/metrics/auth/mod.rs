//! Module: ops::runtime::metrics::auth
//!
//! Responsibility: record and snapshot low-cardinality runtime auth metrics.
//! Does not own: auth policy, session state, or endpoint DTOs.
//! Boundary: ops-layer counters consumed by metrics projection and auth recorders.

mod attestation;
#[cfg(any(test, feature = "auth-local-application-authorization"))]
mod sessions;

use ic_metrics::MeasurementCounter;
use std::{cell::RefCell, collections::HashMap};

pub use attestation::{record_attestation_epoch_rejected, record_attestation_verify_failed};
#[cfg(any(test, feature = "auth-local-application-authorization"))]
pub use sessions::{
    record_application_session_cleanup, record_application_session_generation_invalidation,
};

thread_local! {
    static AUTH_METRICS: RefCell<HashMap<AuthMetricKey, MeasurementCounter>> = RefCell::new(HashMap::new());
}

///
/// AuthMetricSurface
///
/// Auth metric surface dimension used by public metrics projection.
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum AuthMetricSurface {
    #[cfg(any(test, feature = "auth-local-application-authorization"))]
    ApplicationSession,
    Attestation,
}

impl AuthMetricSurface {
    /// Return the stable public metrics label for this surface.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            #[cfg(any(test, feature = "auth-local-application-authorization"))]
            Self::ApplicationSession => "application_session",
            Self::Attestation => "attestation",
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
    #[cfg(any(test, feature = "auth-local-application-authorization"))]
    Cleanup,
    #[cfg(any(test, feature = "auth-local-application-authorization"))]
    GenerationInvalidation,
    Verify,
}

impl AuthMetricOperation {
    /// Return the stable public metrics label for this operation.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            #[cfg(any(test, feature = "auth-local-application-authorization"))]
            Self::Cleanup => "cleanup",
            #[cfg(any(test, feature = "auth-local-application-authorization"))]
            Self::GenerationInvalidation => "generation_invalidation",
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
    #[cfg(any(test, feature = "auth-local-application-authorization"))]
    Completed,
    Failed,
}

impl AuthMetricOutcome {
    /// Return the stable public metrics label for this outcome.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            #[cfg(any(test, feature = "auth-local-application-authorization"))]
            Self::Completed => "completed",
            Self::Failed => "failed",
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
    EpochRejected,
    #[cfg(any(test, feature = "auth-local-application-authorization"))]
    Expired,
    #[cfg(any(test, feature = "auth-local-application-authorization"))]
    GenerationAdvanced,
    VerifyFailed,
}

impl AuthMetricReason {
    /// Return the stable public metrics label for this reason.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::EpochRejected => "epoch_rejected",
            #[cfg(any(test, feature = "auth-local-application-authorization"))]
            Self::Expired => "expired",
            #[cfg(any(test, feature = "auth-local-application-authorization"))]
            Self::GenerationAdvanced => "generation_advanced",
            Self::VerifyFailed => "verify_failed",
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
            let entry = counts.entry(key).or_default();
            entry.add(1);
        });
    }

    /// Snapshot the current auth metric table as stable rows.
    #[must_use]
    pub fn snapshot() -> Vec<(AuthMetricKey, MeasurementCounter)> {
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
    ) -> u128 {
        AuthMetrics::snapshot()
            .into_iter()
            .find_map(|(key, count)| {
                if key.surface == surface
                    && key.operation == operation
                    && key.outcome == outcome
                    && key.reason == reason
                {
                    Some(count.exact_total().unwrap())
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
        expected: u128,
    ) {
        assert_eq!(
            auth_metric_count(surface, operation, outcome, reason),
            expected
        );
    }

    #[test]
    fn recorder_preserves_exact_cap_and_sticky_overflow_until_reset() {
        AuthMetrics::reset();
        let key = AuthMetricKey {
            surface: AuthMetricSurface::Attestation,
            operation: AuthMetricOperation::Verify,
            outcome: AuthMetricOutcome::Failed,
            reason: AuthMetricReason::VerifyFailed,
        };
        AUTH_METRICS.with_borrow_mut(|counts| {
            counts.insert(
                key,
                MeasurementCounter::from_parts(u128::MAX - 1, false).unwrap(),
            );
        });
        record_attestation_verify_failed();
        let exact = AuthMetrics::snapshot()[0].1;
        assert_eq!(exact.exact_total(), Ok(u128::MAX));
        record_attestation_verify_failed();
        let clipped = AuthMetrics::snapshot()[0].1;
        assert_eq!(clipped.total(), u128::MAX);
        assert!(clipped.total_overflowed());
        record_attestation_verify_failed();
        assert!(AuthMetrics::snapshot()[0].1.total_overflowed());
        AuthMetrics::reset();
        assert!(AuthMetrics::snapshot().is_empty());
        record_attestation_verify_failed();
        assert_eq!(AuthMetrics::snapshot()[0].1.exact_total(), Ok(1));
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
