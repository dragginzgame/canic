use super::{
    AuthMetricPredicate, DelegationInstallNormalizationRejectReason,
    DelegationInstallValidationFailureReason, auth_signer_endpoint, complete_predicate,
    fanout_bucket, install_intent_label, push_attempt_predicate, push_success_predicate,
    record_auth_metric, signer_issue_without_proof_predicate, verifier_target_count_predicate,
    verifier_target_failed_predicate, verifier_target_missing_predicate,
};
use crate::{
    dto::auth::DelegationProofInstallIntent, ids::AccessMetricKind,
    ops::runtime::metrics::access::AccessMetrics,
};

pub fn record_signer_issue_without_proof() {
    AccessMetrics::increment(
        auth_signer_endpoint(),
        AccessMetricKind::Auth,
        signer_issue_without_proof_predicate(),
    );
}

pub fn record_delegation_verifier_target_failed() {
    AccessMetrics::increment(
        auth_signer_endpoint(),
        AccessMetricKind::Auth,
        verifier_target_failed_predicate(),
    );
}

pub fn record_delegation_verifier_target_missing() {
    AccessMetrics::increment(
        auth_signer_endpoint(),
        AccessMetricKind::Auth,
        verifier_target_missing_predicate(),
    );
}

pub fn record_delegation_verifier_target_count(target_count: usize) {
    for _ in 0..target_count {
        AccessMetrics::increment(
            auth_signer_endpoint(),
            AccessMetricKind::Auth,
            verifier_target_count_predicate(),
        );
    }
}

pub fn record_delegation_push_attempt(intent: DelegationProofInstallIntent) {
    AccessMetrics::increment(
        auth_signer_endpoint(),
        AccessMetricKind::Auth,
        push_attempt_predicate(intent),
    );
}

pub fn record_delegation_push_success(intent: DelegationProofInstallIntent) {
    AccessMetrics::increment(
        auth_signer_endpoint(),
        AccessMetricKind::Auth,
        push_success_predicate(intent),
    );
}

pub fn record_delegation_push_failed(intent: DelegationProofInstallIntent) {
    record_auth_metric(
        auth_signer_endpoint(),
        AuthMetricPredicate::DelegationPushFailed { intent },
    );
}

pub fn record_delegation_push_complete(intent: DelegationProofInstallIntent) {
    AccessMetrics::increment(
        auth_signer_endpoint(),
        AccessMetricKind::Auth,
        complete_predicate(intent),
    );
}

pub fn record_delegation_provision_complete() {
    record_delegation_push_complete(DelegationProofInstallIntent::Provisioning);
}

pub fn record_delegation_install_total(intent: DelegationProofInstallIntent) {
    let predicate = format!(
        "delegation_install_total{{intent=\"{}\"}}",
        install_intent_label(intent)
    );
    AccessMetrics::increment(auth_signer_endpoint(), AccessMetricKind::Auth, &predicate);
}

pub fn record_delegation_install_normalized_target_count(
    intent: DelegationProofInstallIntent,
    target_count: usize,
) {
    let predicate = format!(
        "delegation_install_normalized_target_total{{intent=\"{}\"}}",
        install_intent_label(intent)
    );
    for _ in 0..target_count {
        AccessMetrics::increment(auth_signer_endpoint(), AccessMetricKind::Auth, &predicate);
    }
}

pub fn record_delegation_install_fanout_bucket(
    intent: DelegationProofInstallIntent,
    target_count: usize,
) {
    let predicate = format!(
        "delegation_install_fanout_bucket{{intent=\"{}\",bucket=\"{}\"}}",
        install_intent_label(intent),
        fanout_bucket(target_count)
    );
    AccessMetrics::increment(auth_signer_endpoint(), AccessMetricKind::Auth, &predicate);
}

pub fn record_delegation_install_normalization_rejected(
    intent: DelegationProofInstallIntent,
    reason: DelegationInstallNormalizationRejectReason,
) {
    record_auth_metric(
        auth_signer_endpoint(),
        AuthMetricPredicate::DelegationInstallNormalizationRejected { intent, reason },
    );
}

pub fn record_delegation_install_validation_failed(
    intent: DelegationProofInstallIntent,
    reason: DelegationInstallValidationFailureReason,
) {
    record_auth_metric(
        auth_signer_endpoint(),
        AuthMetricPredicate::DelegationInstallValidationFailed { intent, reason },
    );
}
