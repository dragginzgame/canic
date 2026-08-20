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
    record_application_session_cleanup, record_application_session_clear,
    record_application_session_created, record_application_session_establishment_started,
    record_application_session_expired_observation,
    record_application_session_generation_invalidation, record_application_session_idempotent,
    record_application_session_rejected, record_application_session_replaced,
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
    ApplicationSession,
    Attestation,
}

impl AuthMetricSurface {
    /// Return the stable public metrics label for this surface.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
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
    Cleanup,
    Clear,
    Establish,
    ExpiryObservation,
    GenerationInvalidation,
    Verify,
}

impl AuthMetricOperation {
    /// Return the stable public metrics label for this operation.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Cleanup => "cleanup",
            Self::Clear => "clear",
            Self::Establish => "establish",
            Self::ExpiryObservation => "expiry_observation",
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
    Completed,
    Failed,
    Idempotent,
    Rejected,
    Started,
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
            Self::Started => "started",
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
    AuthorityConflict,
    Capacity,
    Cleared,
    Created,
    EpochRejected,
    Expired,
    GenerationAdvanced,
    InvalidRequest,
    ProofInvalid,
    Replaced,
    Replay,
    ReplayConflict,
    Request,
    StateUnavailable,
    VerifyFailed,
}

impl AuthMetricReason {
    /// Return the stable public metrics label for this reason.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::AuthorityConflict => "authority_conflict",
            Self::Capacity => "capacity",
            Self::Cleared => "cleared",
            Self::Created => "created",
            Self::EpochRejected => "epoch_rejected",
            Self::Expired => "expired",
            Self::GenerationAdvanced => "generation_advanced",
            Self::InvalidRequest => "invalid_request",
            Self::ProofInvalid => "proof_invalid",
            Self::Replaced => "replaced",
            Self::Replay => "replay",
            Self::ReplayConflict => "replay_conflict",
            Self::Request => "request",
            Self::StateUnavailable => "state_unavailable",
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
            AuthMetricSurface::ApplicationSession,
            operation,
            outcome,
            reason,
            expected,
        );
    }

    #[test]
    fn application_session_metrics_use_only_bounded_outcome_dimensions() {
        AuthMetrics::reset();

        record_application_session_establishment_started();
        record_application_session_created();
        record_application_session_replaced();
        record_application_session_idempotent();
        record_application_session_rejected(AuthMetricReason::Capacity);
        record_application_session_clear(true);
        record_application_session_clear(false);
        record_application_session_expired_observation();
        record_application_session_cleanup(2);
        record_application_session_generation_invalidation();

        assert_session_metric_count(
            AuthMetricOperation::Establish,
            AuthMetricOutcome::Started,
            AuthMetricReason::Request,
            1,
        );
        assert_session_metric_count(
            AuthMetricOperation::Establish,
            AuthMetricOutcome::Completed,
            AuthMetricReason::Created,
            1,
        );
        assert_session_metric_count(
            AuthMetricOperation::Establish,
            AuthMetricOutcome::Idempotent,
            AuthMetricReason::Replay,
            1,
        );
        assert_session_metric_count(
            AuthMetricOperation::Cleanup,
            AuthMetricOutcome::Completed,
            AuthMetricReason::Expired,
            2,
        );
        assert_session_metric_count(
            AuthMetricOperation::Clear,
            AuthMetricOutcome::Idempotent,
            AuthMetricReason::Cleared,
            1,
        );
        assert_session_metric_count(
            AuthMetricOperation::GenerationInvalidation,
            AuthMetricOutcome::Completed,
            AuthMetricReason::GenerationAdvanced,
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
