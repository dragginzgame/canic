//! Module: ops::runtime::metrics::auth::attestation
//!
//! Responsibility: record auth metrics for role-attestation verification outcomes.
//! Does not own: attestation verification, access policy, or endpoint DTOs.
//! Boundary: auth workflow calls these ops helpers after typed outcomes are known.

use crate::ops::runtime::metrics::auth::{
    AuthMetricOperation, AuthMetricOutcome, AuthMetricReason, AuthMetricSurface, AuthMetrics,
};

/// Record an attestation verification failure.
pub fn record_attestation_verify_failed() {
    record_attestation_metric(
        AuthMetricOperation::Verify,
        AuthMetricOutcome::Failed,
        AuthMetricReason::VerifyFailed,
    );
}

/// Record an attestation verification failure caused by an invalid epoch.
pub fn record_attestation_epoch_rejected() {
    record_attestation_metric(
        AuthMetricOperation::Verify,
        AuthMetricOutcome::Failed,
        AuthMetricReason::EpochRejected,
    );
}

fn record_attestation_metric(
    operation: AuthMetricOperation,
    outcome: AuthMetricOutcome,
    reason: AuthMetricReason,
) {
    AuthMetrics::record(AuthMetricSurface::Attestation, operation, outcome, reason);
}
