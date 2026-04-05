use super::{
    attestation_epoch_rejected_predicate, attestation_refresh_failed_predicate,
    attestation_unknown_key_id_predicate, attestation_verify_failed_predicate,
    auth_attestation_verifier_endpoint,
};
use crate::{ids::AccessMetricKind, ops::runtime::metrics::access::AccessMetrics};

pub fn record_attestation_verify_failed() {
    AccessMetrics::increment(
        auth_attestation_verifier_endpoint(),
        AccessMetricKind::Auth,
        attestation_verify_failed_predicate(),
    );
}

pub fn record_attestation_unknown_key_id() {
    AccessMetrics::increment(
        auth_attestation_verifier_endpoint(),
        AccessMetricKind::Auth,
        attestation_unknown_key_id_predicate(),
    );
}

pub fn record_attestation_epoch_rejected() {
    AccessMetrics::increment(
        auth_attestation_verifier_endpoint(),
        AccessMetricKind::Auth,
        attestation_epoch_rejected_predicate(),
    );
}

pub fn record_attestation_refresh_failed() {
    AccessMetrics::increment(
        auth_attestation_verifier_endpoint(),
        AccessMetricKind::Auth,
        attestation_refresh_failed_predicate(),
    );
}
