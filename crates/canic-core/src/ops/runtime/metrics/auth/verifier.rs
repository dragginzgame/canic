use super::{
    AuthMetricPredicate, AuthProofCacheUtilizationBucket, VerifierProofCacheEvictionClass,
    auth_verifier_endpoint, cert_expired_predicate, proof_cache_active_size_predicate,
    proof_cache_active_window_predicate, proof_cache_capacity_predicate,
    proof_cache_size_predicate, record_auth_metric,
};
use crate::{
    config::schema::DelegationProofCacheProfile, ids::AccessMetricKind,
    ops::runtime::metrics::access::AccessMetrics,
};

pub fn record_verifier_proof_miss() {
    record_auth_metric(auth_verifier_endpoint(), AuthMetricPredicate::ProofMiss);
}

pub fn record_verifier_proof_mismatch() {
    record_auth_metric(auth_verifier_endpoint(), AuthMetricPredicate::ProofMismatch);
}

pub fn record_verifier_cert_expired() {
    AccessMetrics::increment(
        auth_verifier_endpoint(),
        AccessMetricKind::Auth,
        cert_expired_predicate(),
    );
}

pub fn record_verifier_proof_cache_stats(
    size: usize,
    active_count: usize,
    capacity: usize,
    profile: DelegationProofCacheProfile,
    active_window_secs: u64,
) {
    let size_predicate = proof_cache_size_predicate(size);
    AccessMetrics::increment(
        auth_verifier_endpoint(),
        AccessMetricKind::Auth,
        &size_predicate,
    );

    let active_predicate = proof_cache_active_size_predicate(active_count);
    AccessMetrics::increment(
        auth_verifier_endpoint(),
        AccessMetricKind::Auth,
        &active_predicate,
    );

    record_auth_metric(
        auth_verifier_endpoint(),
        AuthMetricPredicate::ProofCacheUtilization {
            bucket: AuthProofCacheUtilizationBucket::from_size_and_capacity(size, capacity),
        },
    );

    let capacity_predicate = proof_cache_capacity_predicate(profile, capacity);
    AccessMetrics::increment(
        auth_verifier_endpoint(),
        AccessMetricKind::Auth,
        &capacity_predicate,
    );

    let active_window_predicate = proof_cache_active_window_predicate(active_window_secs);
    AccessMetrics::increment(
        auth_verifier_endpoint(),
        AccessMetricKind::Auth,
        &active_window_predicate,
    );
}

pub fn record_verifier_proof_cache_eviction(class: VerifierProofCacheEvictionClass) {
    record_auth_metric(
        auth_verifier_endpoint(),
        AuthMetricPredicate::ProofCacheEviction { class },
    );
}
