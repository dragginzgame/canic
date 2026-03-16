use crate::{ids::AccessMetricKind, ops::runtime::metrics::access::AccessMetrics};

const AUTH_SIGNER_ENDPOINT: &str = "auth_signer";
const AUTH_SESSION_ENDPOINT: &str = "auth_session";
const AUTH_VERIFIER_ENDPOINT: &str = "auth_verifier";
const AUTH_ATTESTATION_VERIFIER_ENDPOINT: &str = "auth_attestation_verifier";

const PRED_ISSUE_WITHOUT_PROOF: &str = "issue_without_proof";
const PRED_DELEGATION_VERIFIER_TARGET_FAILED: &str = "delegation_verifier_target_failed";
const PRED_DELEGATION_VERIFIER_TARGET_MISSING: &str = "delegation_verifier_target_missing";
const PRED_DELEGATION_VERIFIER_TARGET_COUNT: &str = "delegation_verifier_target_count";
const PRED_DELEGATION_PROVISION_COMPLETE: &str = "delegation_provision_complete";
const PRED_DELEGATION_PROVISION_ATTEMPT_SIGNER: &str =
    "delegation_provision_attempt{role=\"signer\"}";
const PRED_DELEGATION_PROVISION_ATTEMPT_VERIFIER: &str =
    "delegation_provision_attempt{role=\"verifier\"}";
const PRED_DELEGATION_PROVISION_SUCCESS_SIGNER: &str =
    "delegation_provision_success{role=\"signer\"}";
const PRED_DELEGATION_PROVISION_SUCCESS_VERIFIER: &str =
    "delegation_provision_success{role=\"verifier\"}";
const PRED_DELEGATION_PROVISION_FAILED_SIGNER: &str =
    "delegation_provision_failed{role=\"signer\"}";
const PRED_DELEGATION_PROVISION_FAILED_VERIFIER: &str =
    "delegation_provision_failed{role=\"verifier\"}";
const PRED_SESSION_BOOTSTRAP_REJECTED_DISABLED: &str = "session_bootstrap_rejected_disabled";
const PRED_SESSION_BOOTSTRAP_REJECTED_SUBJECT_MISMATCH: &str =
    "session_bootstrap_rejected_subject_mismatch";
const PRED_SESSION_BOOTSTRAP_REJECTED_SUBJECT_REJECTED: &str =
    "session_bootstrap_rejected_subject_rejected";
const PRED_SESSION_BOOTSTRAP_REJECTED_REPLAY_CONFLICT: &str =
    "session_bootstrap_rejected_replay_conflict";
const PRED_SESSION_BOOTSTRAP_REJECTED_REPLAY_REUSED: &str =
    "session_bootstrap_rejected_replay_reused";
const PRED_SESSION_BOOTSTRAP_REJECTED_TOKEN_INVALID: &str =
    "session_bootstrap_rejected_token_invalid";
const PRED_SESSION_BOOTSTRAP_REJECTED_TTL_INVALID: &str = "session_bootstrap_rejected_ttl_invalid";
const PRED_SESSION_BOOTSTRAP_REJECTED_WALLET_CALLER_REJECTED: &str =
    "session_bootstrap_rejected_wallet_caller_rejected";
const PRED_SESSION_BOOTSTRAP_REPLAY_IDEMPOTENT: &str = "session_bootstrap_replay_idempotent";
const PRED_SESSION_CLEARED: &str = "session_cleared";
const PRED_SESSION_CREATED: &str = "session_created";
const PRED_SESSION_FALLBACK_INVALID_SUBJECT: &str = "session_fallback_invalid_subject";
const PRED_SESSION_FALLBACK_RAW_CALLER: &str = "session_fallback_raw_caller";
const PRED_SESSION_PRUNED: &str = "session_pruned";
const PRED_SESSION_REPLACED: &str = "session_replaced";
const PRED_PROOF_MISSING: &str = "token_rejected_proof_missing";
const PRED_PROOF_MISMATCH: &str = "token_rejected_proof_mismatch";
const PRED_CERT_EXPIRED: &str = "token_rejected_expired_cert";
const PRED_ATTESTATION_VERIFY_FAILED: &str = "attestation_verify_failed";
const PRED_ATTESTATION_UNKNOWN_KEY_ID: &str = "attestation_unknown_key_id";
const PRED_ATTESTATION_EPOCH_REJECTED: &str = "attestation_epoch_rejected";
const PRED_ATTESTATION_REFRESH_FAILED: &str = "attestation_refresh_failed";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelegationProvisionRole {
    Signer,
    Verifier,
}

impl DelegationProvisionRole {
    const fn attempt_predicate(self) -> &'static str {
        match self {
            Self::Signer => PRED_DELEGATION_PROVISION_ATTEMPT_SIGNER,
            Self::Verifier => PRED_DELEGATION_PROVISION_ATTEMPT_VERIFIER,
        }
    }

    const fn success_predicate(self) -> &'static str {
        match self {
            Self::Signer => PRED_DELEGATION_PROVISION_SUCCESS_SIGNER,
            Self::Verifier => PRED_DELEGATION_PROVISION_SUCCESS_VERIFIER,
        }
    }

    const fn failed_predicate(self) -> &'static str {
        match self {
            Self::Signer => PRED_DELEGATION_PROVISION_FAILED_SIGNER,
            Self::Verifier => PRED_DELEGATION_PROVISION_FAILED_VERIFIER,
        }
    }
}

pub fn record_signer_issue_without_proof() {
    AccessMetrics::increment(
        AUTH_SIGNER_ENDPOINT,
        AccessMetricKind::Auth,
        PRED_ISSUE_WITHOUT_PROOF,
    );
}

pub fn record_delegation_verifier_target_failed() {
    AccessMetrics::increment(
        AUTH_SIGNER_ENDPOINT,
        AccessMetricKind::Auth,
        PRED_DELEGATION_VERIFIER_TARGET_FAILED,
    );
}

pub fn record_delegation_verifier_target_missing() {
    AccessMetrics::increment(
        AUTH_SIGNER_ENDPOINT,
        AccessMetricKind::Auth,
        PRED_DELEGATION_VERIFIER_TARGET_MISSING,
    );
}

pub fn record_delegation_verifier_target_count(target_count: usize) {
    for _ in 0..target_count {
        AccessMetrics::increment(
            AUTH_SIGNER_ENDPOINT,
            AccessMetricKind::Auth,
            PRED_DELEGATION_VERIFIER_TARGET_COUNT,
        );
    }
}

pub fn record_delegation_provision_attempt(role: DelegationProvisionRole) {
    AccessMetrics::increment(
        AUTH_SIGNER_ENDPOINT,
        AccessMetricKind::Auth,
        role.attempt_predicate(),
    );
}

pub fn record_delegation_provision_success(role: DelegationProvisionRole) {
    AccessMetrics::increment(
        AUTH_SIGNER_ENDPOINT,
        AccessMetricKind::Auth,
        role.success_predicate(),
    );
}

pub fn record_delegation_provision_failed(role: DelegationProvisionRole) {
    AccessMetrics::increment(
        AUTH_SIGNER_ENDPOINT,
        AccessMetricKind::Auth,
        role.failed_predicate(),
    );
}

pub fn record_delegation_provision_complete() {
    AccessMetrics::increment(
        AUTH_SIGNER_ENDPOINT,
        AccessMetricKind::Auth,
        PRED_DELEGATION_PROVISION_COMPLETE,
    );
}

pub fn record_session_bootstrap_rejected_disabled() {
    AccessMetrics::increment(
        AUTH_SESSION_ENDPOINT,
        AccessMetricKind::Auth,
        PRED_SESSION_BOOTSTRAP_REJECTED_DISABLED,
    );
}

pub fn record_session_bootstrap_rejected_wallet_caller_rejected() {
    AccessMetrics::increment(
        AUTH_SESSION_ENDPOINT,
        AccessMetricKind::Auth,
        PRED_SESSION_BOOTSTRAP_REJECTED_WALLET_CALLER_REJECTED,
    );
}

pub fn record_session_bootstrap_rejected_subject_rejected() {
    AccessMetrics::increment(
        AUTH_SESSION_ENDPOINT,
        AccessMetricKind::Auth,
        PRED_SESSION_BOOTSTRAP_REJECTED_SUBJECT_REJECTED,
    );
}

pub fn record_session_bootstrap_rejected_replay_conflict() {
    AccessMetrics::increment(
        AUTH_SESSION_ENDPOINT,
        AccessMetricKind::Auth,
        PRED_SESSION_BOOTSTRAP_REJECTED_REPLAY_CONFLICT,
    );
}

pub fn record_session_bootstrap_rejected_replay_reused() {
    AccessMetrics::increment(
        AUTH_SESSION_ENDPOINT,
        AccessMetricKind::Auth,
        PRED_SESSION_BOOTSTRAP_REJECTED_REPLAY_REUSED,
    );
}

pub fn record_session_bootstrap_rejected_token_invalid() {
    AccessMetrics::increment(
        AUTH_SESSION_ENDPOINT,
        AccessMetricKind::Auth,
        PRED_SESSION_BOOTSTRAP_REJECTED_TOKEN_INVALID,
    );
}

pub fn record_session_bootstrap_rejected_subject_mismatch() {
    AccessMetrics::increment(
        AUTH_SESSION_ENDPOINT,
        AccessMetricKind::Auth,
        PRED_SESSION_BOOTSTRAP_REJECTED_SUBJECT_MISMATCH,
    );
}

pub fn record_session_bootstrap_rejected_ttl_invalid() {
    AccessMetrics::increment(
        AUTH_SESSION_ENDPOINT,
        AccessMetricKind::Auth,
        PRED_SESSION_BOOTSTRAP_REJECTED_TTL_INVALID,
    );
}

pub fn record_session_bootstrap_replay_idempotent() {
    AccessMetrics::increment(
        AUTH_SESSION_ENDPOINT,
        AccessMetricKind::Auth,
        PRED_SESSION_BOOTSTRAP_REPLAY_IDEMPOTENT,
    );
}

pub fn record_session_created() {
    AccessMetrics::increment(
        AUTH_SESSION_ENDPOINT,
        AccessMetricKind::Auth,
        PRED_SESSION_CREATED,
    );
}

pub fn record_session_replaced() {
    AccessMetrics::increment(
        AUTH_SESSION_ENDPOINT,
        AccessMetricKind::Auth,
        PRED_SESSION_REPLACED,
    );
}

pub fn record_session_cleared() {
    AccessMetrics::increment(
        AUTH_SESSION_ENDPOINT,
        AccessMetricKind::Auth,
        PRED_SESSION_CLEARED,
    );
}

pub fn record_session_pruned(removed: usize) {
    for _ in 0..removed {
        AccessMetrics::increment(
            AUTH_SESSION_ENDPOINT,
            AccessMetricKind::Auth,
            PRED_SESSION_PRUNED,
        );
    }
}

pub fn record_session_fallback_raw_caller() {
    AccessMetrics::increment(
        AUTH_SESSION_ENDPOINT,
        AccessMetricKind::Auth,
        PRED_SESSION_FALLBACK_RAW_CALLER,
    );
}

pub fn record_session_fallback_invalid_subject() {
    AccessMetrics::increment(
        AUTH_SESSION_ENDPOINT,
        AccessMetricKind::Auth,
        PRED_SESSION_FALLBACK_INVALID_SUBJECT,
    );
}

pub fn record_verifier_proof_missing() {
    AccessMetrics::increment(
        AUTH_VERIFIER_ENDPOINT,
        AccessMetricKind::Auth,
        PRED_PROOF_MISSING,
    );
}

pub fn record_verifier_proof_mismatch() {
    AccessMetrics::increment(
        AUTH_VERIFIER_ENDPOINT,
        AccessMetricKind::Auth,
        PRED_PROOF_MISMATCH,
    );
}

pub fn record_verifier_cert_expired() {
    AccessMetrics::increment(
        AUTH_VERIFIER_ENDPOINT,
        AccessMetricKind::Auth,
        PRED_CERT_EXPIRED,
    );
}

pub fn record_attestation_verify_failed() {
    AccessMetrics::increment(
        AUTH_ATTESTATION_VERIFIER_ENDPOINT,
        AccessMetricKind::Auth,
        PRED_ATTESTATION_VERIFY_FAILED,
    );
}

pub fn record_attestation_unknown_key_id() {
    AccessMetrics::increment(
        AUTH_ATTESTATION_VERIFIER_ENDPOINT,
        AccessMetricKind::Auth,
        PRED_ATTESTATION_UNKNOWN_KEY_ID,
    );
}

pub fn record_attestation_epoch_rejected() {
    AccessMetrics::increment(
        AUTH_ATTESTATION_VERIFIER_ENDPOINT,
        AccessMetricKind::Auth,
        PRED_ATTESTATION_EPOCH_REJECTED,
    );
}

pub fn record_attestation_refresh_failed() {
    AccessMetrics::increment(
        AUTH_ATTESTATION_VERIFIER_ENDPOINT,
        AccessMetricKind::Auth,
        PRED_ATTESTATION_REFRESH_FAILED,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn session_metrics_increment_expected_predicates() {
        AccessMetrics::reset();

        record_session_created();
        record_session_replaced();
        record_session_cleared();
        record_session_pruned(2);
        record_session_fallback_raw_caller();
        record_session_fallback_invalid_subject();
        record_session_bootstrap_rejected_disabled();
        record_session_bootstrap_rejected_wallet_caller_rejected();
        record_session_bootstrap_rejected_subject_rejected();
        record_session_bootstrap_rejected_replay_conflict();
        record_session_bootstrap_rejected_replay_reused();
        record_session_bootstrap_rejected_token_invalid();
        record_session_bootstrap_rejected_subject_mismatch();
        record_session_bootstrap_rejected_ttl_invalid();
        record_session_bootstrap_replay_idempotent();

        assert_eq!(metric_count(AUTH_SESSION_ENDPOINT, PRED_SESSION_CREATED), 1);
        assert_eq!(
            metric_count(AUTH_SESSION_ENDPOINT, PRED_SESSION_REPLACED),
            1
        );
        assert_eq!(metric_count(AUTH_SESSION_ENDPOINT, PRED_SESSION_CLEARED), 1);
        assert_eq!(metric_count(AUTH_SESSION_ENDPOINT, PRED_SESSION_PRUNED), 2);
        assert_eq!(
            metric_count(AUTH_SESSION_ENDPOINT, PRED_SESSION_FALLBACK_RAW_CALLER),
            1
        );
        assert_eq!(
            metric_count(AUTH_SESSION_ENDPOINT, PRED_SESSION_FALLBACK_INVALID_SUBJECT),
            1
        );
        assert_eq!(
            metric_count(
                AUTH_SESSION_ENDPOINT,
                PRED_SESSION_BOOTSTRAP_REJECTED_DISABLED
            ),
            1
        );
        assert_eq!(
            metric_count(
                AUTH_SESSION_ENDPOINT,
                PRED_SESSION_BOOTSTRAP_REJECTED_WALLET_CALLER_REJECTED
            ),
            1
        );
        assert_eq!(
            metric_count(
                AUTH_SESSION_ENDPOINT,
                PRED_SESSION_BOOTSTRAP_REJECTED_SUBJECT_REJECTED
            ),
            1
        );
        assert_eq!(
            metric_count(
                AUTH_SESSION_ENDPOINT,
                PRED_SESSION_BOOTSTRAP_REJECTED_REPLAY_CONFLICT
            ),
            1
        );
        assert_eq!(
            metric_count(
                AUTH_SESSION_ENDPOINT,
                PRED_SESSION_BOOTSTRAP_REJECTED_REPLAY_REUSED
            ),
            1
        );
        assert_eq!(
            metric_count(
                AUTH_SESSION_ENDPOINT,
                PRED_SESSION_BOOTSTRAP_REJECTED_TOKEN_INVALID
            ),
            1
        );
        assert_eq!(
            metric_count(
                AUTH_SESSION_ENDPOINT,
                PRED_SESSION_BOOTSTRAP_REJECTED_SUBJECT_MISMATCH
            ),
            1
        );
        assert_eq!(
            metric_count(
                AUTH_SESSION_ENDPOINT,
                PRED_SESSION_BOOTSTRAP_REJECTED_TTL_INVALID
            ),
            1
        );
        assert_eq!(
            metric_count(
                AUTH_SESSION_ENDPOINT,
                PRED_SESSION_BOOTSTRAP_REPLAY_IDEMPOTENT
            ),
            1
        );
    }

    #[test]
    fn delegation_provision_metrics_increment_expected_predicates() {
        AccessMetrics::reset();

        record_delegation_provision_attempt(DelegationProvisionRole::Signer);
        record_delegation_provision_attempt(DelegationProvisionRole::Verifier);
        record_delegation_provision_success(DelegationProvisionRole::Signer);
        record_delegation_provision_success(DelegationProvisionRole::Verifier);
        record_delegation_provision_failed(DelegationProvisionRole::Verifier);
        record_delegation_verifier_target_failed();
        record_delegation_verifier_target_missing();
        record_delegation_verifier_target_count(3);
        record_delegation_provision_complete();

        assert_eq!(
            metric_count(
                AUTH_SIGNER_ENDPOINT,
                PRED_DELEGATION_PROVISION_ATTEMPT_SIGNER
            ),
            1
        );
        assert_eq!(
            metric_count(
                AUTH_SIGNER_ENDPOINT,
                PRED_DELEGATION_PROVISION_ATTEMPT_VERIFIER
            ),
            1
        );
        assert_eq!(
            metric_count(
                AUTH_SIGNER_ENDPOINT,
                PRED_DELEGATION_PROVISION_SUCCESS_SIGNER
            ),
            1
        );
        assert_eq!(
            metric_count(
                AUTH_SIGNER_ENDPOINT,
                PRED_DELEGATION_PROVISION_SUCCESS_VERIFIER
            ),
            1
        );
        assert_eq!(
            metric_count(
                AUTH_SIGNER_ENDPOINT,
                PRED_DELEGATION_PROVISION_FAILED_VERIFIER
            ),
            1
        );
        assert_eq!(
            metric_count(AUTH_SIGNER_ENDPOINT, PRED_DELEGATION_VERIFIER_TARGET_FAILED),
            1
        );
        assert_eq!(
            metric_count(
                AUTH_SIGNER_ENDPOINT,
                PRED_DELEGATION_VERIFIER_TARGET_MISSING
            ),
            1
        );
        assert_eq!(
            metric_count(AUTH_SIGNER_ENDPOINT, PRED_DELEGATION_VERIFIER_TARGET_COUNT),
            3
        );
        assert_eq!(
            metric_count(AUTH_SIGNER_ENDPOINT, PRED_DELEGATION_PROVISION_COMPLETE),
            1
        );
    }
}
