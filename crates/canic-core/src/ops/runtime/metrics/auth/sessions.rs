use super::{
    auth_session_endpoint, session_bootstrap_rejected_disabled_predicate,
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

pub fn record_session_bootstrap_rejected_disabled() {
    AccessMetrics::increment(
        auth_session_endpoint(),
        AccessMetricKind::Auth,
        session_bootstrap_rejected_disabled_predicate(),
    );
}

pub fn record_session_bootstrap_rejected_wallet_caller_rejected() {
    AccessMetrics::increment(
        auth_session_endpoint(),
        AccessMetricKind::Auth,
        session_bootstrap_rejected_wallet_caller_rejected_predicate(),
    );
}

pub fn record_session_bootstrap_rejected_subject_rejected() {
    AccessMetrics::increment(
        auth_session_endpoint(),
        AccessMetricKind::Auth,
        session_bootstrap_rejected_subject_rejected_predicate(),
    );
}

pub fn record_session_bootstrap_rejected_replay_conflict() {
    AccessMetrics::increment(
        auth_session_endpoint(),
        AccessMetricKind::Auth,
        session_bootstrap_rejected_replay_conflict_predicate(),
    );
}

pub fn record_session_bootstrap_rejected_replay_reused() {
    AccessMetrics::increment(
        auth_session_endpoint(),
        AccessMetricKind::Auth,
        session_bootstrap_rejected_replay_reused_predicate(),
    );
}

pub fn record_session_bootstrap_rejected_token_invalid() {
    AccessMetrics::increment(
        auth_session_endpoint(),
        AccessMetricKind::Auth,
        session_bootstrap_rejected_token_invalid_predicate(),
    );
}

pub fn record_session_bootstrap_rejected_subject_mismatch() {
    AccessMetrics::increment(
        auth_session_endpoint(),
        AccessMetricKind::Auth,
        session_bootstrap_rejected_subject_mismatch_predicate(),
    );
}

pub fn record_session_bootstrap_rejected_ttl_invalid() {
    AccessMetrics::increment(
        auth_session_endpoint(),
        AccessMetricKind::Auth,
        session_bootstrap_rejected_ttl_invalid_predicate(),
    );
}

pub fn record_session_bootstrap_replay_idempotent() {
    AccessMetrics::increment(
        auth_session_endpoint(),
        AccessMetricKind::Auth,
        session_bootstrap_replay_idempotent_predicate(),
    );
}

pub fn record_session_created() {
    AccessMetrics::increment(
        auth_session_endpoint(),
        AccessMetricKind::Auth,
        session_created_predicate(),
    );
}

pub fn record_session_replaced() {
    AccessMetrics::increment(
        auth_session_endpoint(),
        AccessMetricKind::Auth,
        session_replaced_predicate(),
    );
}

pub fn record_session_cleared() {
    AccessMetrics::increment(
        auth_session_endpoint(),
        AccessMetricKind::Auth,
        session_cleared_predicate(),
    );
}

pub fn record_session_pruned(removed: usize) {
    for _ in 0..removed {
        AccessMetrics::increment(
            auth_session_endpoint(),
            AccessMetricKind::Auth,
            session_pruned_predicate(),
        );
    }
}

pub fn record_session_fallback_raw_caller() {
    AccessMetrics::increment(
        auth_session_endpoint(),
        AccessMetricKind::Auth,
        session_fallback_raw_caller_predicate(),
    );
}

pub fn record_session_fallback_invalid_subject() {
    AccessMetrics::increment(
        auth_session_endpoint(),
        AccessMetricKind::Auth,
        session_fallback_invalid_subject_predicate(),
    );
}
