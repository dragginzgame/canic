use super::{
    AuthMetricOperation, AuthMetricOutcome, AuthMetricReason, AuthMetricSurface, AuthMetrics,
    attestation_epoch_rejected_predicate, attestation_refresh_failed_predicate,
    attestation_unknown_key_id_predicate, attestation_verify_failed_predicate,
    auth_attestation_verifier_endpoint,
};
use crate::{ids::AccessMetricKind, ops::runtime::metrics::access::AccessMetrics};

/// Record an attestation verification failure.
pub fn record_attestation_verify_failed() {
    record_attestation_metric(
        AuthMetricOperation::Verify,
        AuthMetricOutcome::Failed,
        AuthMetricReason::VerifyFailed,
        attestation_verify_failed_predicate(),
    );
}

/// Record an attestation verification failure caused by an unknown key id.
pub fn record_attestation_unknown_key_id() {
    record_attestation_metric(
        AuthMetricOperation::Verify,
        AuthMetricOutcome::Failed,
        AuthMetricReason::UnknownKeyId,
        attestation_unknown_key_id_predicate(),
    );
}

/// Record an attestation verification failure caused by an invalid epoch.
pub fn record_attestation_epoch_rejected() {
    record_attestation_metric(
        AuthMetricOperation::Verify,
        AuthMetricOutcome::Failed,
        AuthMetricReason::EpochRejected,
        attestation_epoch_rejected_predicate(),
    );
}

/// Record an attestation refresh failure.
pub fn record_attestation_refresh_failed() {
    record_attestation_metric(
        AuthMetricOperation::Refresh,
        AuthMetricOutcome::Failed,
        AuthMetricReason::RefreshFailed,
        attestation_refresh_failed_predicate(),
    );
}

// Record one attestation auth metric in both the dedicated Auth family and legacy Access rows.
fn record_attestation_metric(
    operation: AuthMetricOperation,
    outcome: AuthMetricOutcome,
    reason: AuthMetricReason,
    predicate: &'static str,
) {
    AuthMetrics::record(AuthMetricSurface::Attestation, operation, outcome, reason);
    AccessMetrics::increment(
        auth_attestation_verifier_endpoint(),
        AccessMetricKind::Auth,
        predicate,
    );
}
