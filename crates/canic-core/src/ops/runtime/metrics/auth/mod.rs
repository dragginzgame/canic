mod attestation;
mod labels;
mod sessions;

pub use attestation::{
    record_attestation_epoch_rejected, record_attestation_refresh_failed,
    record_attestation_unknown_key_id, record_attestation_verify_failed,
};
pub use sessions::{
    record_session_bootstrap_rejected_disabled, record_session_bootstrap_rejected_replay_conflict,
    record_session_bootstrap_rejected_replay_reused,
    record_session_bootstrap_rejected_subject_mismatch,
    record_session_bootstrap_rejected_subject_rejected,
    record_session_bootstrap_rejected_token_invalid, record_session_bootstrap_rejected_ttl_invalid,
    record_session_bootstrap_rejected_wallet_caller_rejected,
    record_session_bootstrap_replay_idempotent, record_session_cleared, record_session_created,
    record_session_fallback_invalid_subject, record_session_fallback_raw_caller,
    record_session_pruned, record_session_replaced,
};

use labels::{
    attestation_epoch_rejected_predicate, attestation_refresh_failed_predicate,
    attestation_unknown_key_id_predicate, attestation_verify_failed_predicate,
    auth_attestation_verifier_endpoint, auth_session_endpoint,
    session_bootstrap_rejected_disabled_predicate,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ids::AccessMetricKind, ops::runtime::metrics::access::AccessMetrics};

    fn metric_count(endpoint: &str, predicate: &str) -> u64 {
        AccessMetrics::snapshot()
            .entries
            .into_iter()
            .find_map(|(key, count)| {
                if key.endpoint == endpoint
                    && key.kind == AccessMetricKind::Auth
                    && key.predicate == predicate
                {
                    Some(count)
                } else {
                    None
                }
            })
            .unwrap_or(0)
    }

    fn assert_auth_metric_count(endpoint: &str, predicate: &str, expected: u64) {
        assert_eq!(metric_count(endpoint, predicate), expected);
    }

    #[test]
    fn session_metrics_increment_expected_predicates() {
        AccessMetrics::reset();

        for action in [
            record_session_created as fn(),
            record_session_replaced,
            record_session_cleared,
            record_session_fallback_raw_caller,
            record_session_fallback_invalid_subject,
            record_session_bootstrap_rejected_disabled,
            record_session_bootstrap_rejected_wallet_caller_rejected,
            record_session_bootstrap_rejected_subject_rejected,
            record_session_bootstrap_rejected_replay_conflict,
            record_session_bootstrap_rejected_replay_reused,
            record_session_bootstrap_rejected_token_invalid,
            record_session_bootstrap_rejected_subject_mismatch,
            record_session_bootstrap_rejected_ttl_invalid,
            record_session_bootstrap_replay_idempotent,
        ] {
            action();
        }
        record_session_pruned(2);

        for (predicate, expected) in [
            (session_created_predicate(), 1),
            (session_replaced_predicate(), 1),
            (session_cleared_predicate(), 1),
            (session_pruned_predicate(), 2),
            (session_fallback_raw_caller_predicate(), 1),
            (session_fallback_invalid_subject_predicate(), 1),
            (session_bootstrap_rejected_disabled_predicate(), 1),
            (
                session_bootstrap_rejected_wallet_caller_rejected_predicate(),
                1,
            ),
            (session_bootstrap_rejected_subject_rejected_predicate(), 1),
            (session_bootstrap_rejected_replay_conflict_predicate(), 1),
            (session_bootstrap_rejected_replay_reused_predicate(), 1),
            (session_bootstrap_rejected_token_invalid_predicate(), 1),
            (session_bootstrap_rejected_subject_mismatch_predicate(), 1),
            (session_bootstrap_rejected_ttl_invalid_predicate(), 1),
            (session_bootstrap_replay_idempotent_predicate(), 1),
        ] {
            assert_auth_metric_count(auth_session_endpoint(), predicate, expected);
        }
    }

    #[test]
    fn attestation_metrics_increment_expected_predicates() {
        AccessMetrics::reset();

        record_attestation_verify_failed();
        record_attestation_unknown_key_id();
        record_attestation_epoch_rejected();
        record_attestation_refresh_failed();

        for predicate in [
            attestation_verify_failed_predicate(),
            attestation_unknown_key_id_predicate(),
            attestation_epoch_rejected_predicate(),
            attestation_refresh_failed_predicate(),
        ] {
            assert_auth_metric_count(auth_attestation_verifier_endpoint(), predicate, 1);
        }
    }
}
