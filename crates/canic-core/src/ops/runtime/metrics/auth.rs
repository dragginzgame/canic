use crate::{
    config::schema::DelegationProofCacheProfile, dto::auth::DelegationProofInstallIntent,
    ids::AccessMetricKind, ops::runtime::metrics::access::AccessMetrics,
};
use std::borrow::Cow;

// Auth metric predicates are schema-owned here.
// Emitters should construct rollout-relevant predicates through `AuthMetricPredicate::as_str`
// rather than matching strings in multiple places.

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
const PRED_DELEGATION_PUSH_COMPLETE_PREWARM: &str = "delegation_push_complete{origin=\"prewarm\"}";
const PRED_DELEGATION_PUSH_COMPLETE_REPAIR: &str = "delegation_push_complete{origin=\"repair\"}";
const PRED_DELEGATION_PUSH_ATTEMPT_SIGNER_PREWARM: &str =
    "delegation_push_attempt{role=\"signer\",origin=\"prewarm\"}";
const PRED_DELEGATION_PUSH_ATTEMPT_VERIFIER_PREWARM: &str =
    "delegation_push_attempt{role=\"verifier\",origin=\"prewarm\"}";
const PRED_DELEGATION_PUSH_ATTEMPT_SIGNER_REPAIR: &str =
    "delegation_push_attempt{role=\"signer\",origin=\"repair\"}";
const PRED_DELEGATION_PUSH_ATTEMPT_VERIFIER_REPAIR: &str =
    "delegation_push_attempt{role=\"verifier\",origin=\"repair\"}";
const PRED_DELEGATION_PUSH_SUCCESS_SIGNER_PREWARM: &str =
    "delegation_push_success{role=\"signer\",origin=\"prewarm\"}";
const PRED_DELEGATION_PUSH_SUCCESS_VERIFIER_PREWARM: &str =
    "delegation_push_success{role=\"verifier\",origin=\"prewarm\"}";
const PRED_DELEGATION_PUSH_SUCCESS_SIGNER_REPAIR: &str =
    "delegation_push_success{role=\"signer\",origin=\"repair\"}";
const PRED_DELEGATION_PUSH_SUCCESS_VERIFIER_REPAIR: &str =
    "delegation_push_success{role=\"verifier\",origin=\"repair\"}";
const PRED_DELEGATION_PUSH_FAILED_SIGNER_PREWARM: &str =
    "delegation_push_failed{role=\"signer\",origin=\"prewarm\"}";
const PRED_DELEGATION_PUSH_FAILED_VERIFIER_PREWARM: &str =
    "delegation_push_failed{role=\"verifier\",origin=\"prewarm\"}";
const PRED_DELEGATION_PUSH_FAILED_SIGNER_REPAIR: &str =
    "delegation_push_failed{role=\"signer\",origin=\"repair\"}";
const PRED_DELEGATION_PUSH_FAILED_VERIFIER_REPAIR: &str =
    "delegation_push_failed{role=\"verifier\",origin=\"repair\"}";
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
const PRED_PROOF_MISS: &str = "token_rejected_proof_miss";
const PRED_PROOF_MISMATCH: &str = "token_rejected_proof_mismatch";
const PRED_CERT_EXPIRED: &str = "token_rejected_expired_cert";
const PRED_PROOF_CACHE_ACTIVE_EVICTION: &str = "proof_cache_evictions_total{class=\"active\"}";
const PRED_PROOF_CACHE_COLD_EVICTION: &str = "proof_cache_evictions_total{class=\"cold\"}";
const PRED_ATTESTATION_VERIFY_FAILED: &str = "attestation_verify_failed";
const PRED_ATTESTATION_UNKNOWN_KEY_ID: &str = "attestation_unknown_key_id";
const PRED_ATTESTATION_EPOCH_REJECTED: &str = "attestation_epoch_rejected";
const PRED_ATTESTATION_REFRESH_FAILED: &str = "attestation_refresh_failed";

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DelegationProvisionRole {
    Signer,
    Verifier,
}

impl DelegationProvisionRole {
    const fn attempt_predicate(self, intent: DelegationProofInstallIntent) -> &'static str {
        match (self, intent) {
            (Self::Signer, DelegationProofInstallIntent::Provisioning) => {
                PRED_DELEGATION_PROVISION_ATTEMPT_SIGNER
            }
            (Self::Verifier, DelegationProofInstallIntent::Provisioning) => {
                PRED_DELEGATION_PROVISION_ATTEMPT_VERIFIER
            }
            (Self::Signer, DelegationProofInstallIntent::Prewarm) => {
                PRED_DELEGATION_PUSH_ATTEMPT_SIGNER_PREWARM
            }
            (Self::Verifier, DelegationProofInstallIntent::Prewarm) => {
                PRED_DELEGATION_PUSH_ATTEMPT_VERIFIER_PREWARM
            }
            (Self::Signer, DelegationProofInstallIntent::Repair) => {
                PRED_DELEGATION_PUSH_ATTEMPT_SIGNER_REPAIR
            }
            (Self::Verifier, DelegationProofInstallIntent::Repair) => {
                PRED_DELEGATION_PUSH_ATTEMPT_VERIFIER_REPAIR
            }
        }
    }

    const fn success_predicate(self, intent: DelegationProofInstallIntent) -> &'static str {
        match (self, intent) {
            (Self::Signer, DelegationProofInstallIntent::Provisioning) => {
                PRED_DELEGATION_PROVISION_SUCCESS_SIGNER
            }
            (Self::Verifier, DelegationProofInstallIntent::Provisioning) => {
                PRED_DELEGATION_PROVISION_SUCCESS_VERIFIER
            }
            (Self::Signer, DelegationProofInstallIntent::Prewarm) => {
                PRED_DELEGATION_PUSH_SUCCESS_SIGNER_PREWARM
            }
            (Self::Verifier, DelegationProofInstallIntent::Prewarm) => {
                PRED_DELEGATION_PUSH_SUCCESS_VERIFIER_PREWARM
            }
            (Self::Signer, DelegationProofInstallIntent::Repair) => {
                PRED_DELEGATION_PUSH_SUCCESS_SIGNER_REPAIR
            }
            (Self::Verifier, DelegationProofInstallIntent::Repair) => {
                PRED_DELEGATION_PUSH_SUCCESS_VERIFIER_REPAIR
            }
        }
    }

    const fn failed_predicate(self, intent: DelegationProofInstallIntent) -> &'static str {
        match (self, intent) {
            (Self::Signer, DelegationProofInstallIntent::Provisioning) => {
                PRED_DELEGATION_PROVISION_FAILED_SIGNER
            }
            (Self::Verifier, DelegationProofInstallIntent::Provisioning) => {
                PRED_DELEGATION_PROVISION_FAILED_VERIFIER
            }
            (Self::Signer, DelegationProofInstallIntent::Prewarm) => {
                PRED_DELEGATION_PUSH_FAILED_SIGNER_PREWARM
            }
            (Self::Verifier, DelegationProofInstallIntent::Prewarm) => {
                PRED_DELEGATION_PUSH_FAILED_VERIFIER_PREWARM
            }
            (Self::Signer, DelegationProofInstallIntent::Repair) => {
                PRED_DELEGATION_PUSH_FAILED_SIGNER_REPAIR
            }
            (Self::Verifier, DelegationProofInstallIntent::Repair) => {
                PRED_DELEGATION_PUSH_FAILED_VERIFIER_REPAIR
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DelegationInstallNormalizationRejectReason {
    SignerTarget,
    RootTarget,
    UnregisteredTarget,
    TargetNotInAudience,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DelegationInstallValidationFailureReason {
    CacheKeys,
    VerifyProof,
    RepairMissingLocal,
    RepairLocalMismatch,
    TargetNotInAudience,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum VerifierProofCacheEvictionClass {
    Cold,
    Active,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthMetricPredicate {
    DelegationPushFailed {
        role: DelegationProvisionRole,
        intent: DelegationProofInstallIntent,
    },
    DelegationInstallNormalizationRejected {
        intent: DelegationProofInstallIntent,
        reason: DelegationInstallNormalizationRejectReason,
    },
    DelegationInstallValidationFailed {
        intent: DelegationProofInstallIntent,
        reason: DelegationInstallValidationFailureReason,
    },
    ProofMiss,
    ProofMismatch,
    ProofCacheEviction {
        class: VerifierProofCacheEvictionClass,
    },
    ProofCacheUtilization {
        bucket: AuthProofCacheUtilizationBucket,
    },
}

impl AuthMetricPredicate {
    #[must_use]
    pub fn as_str(self) -> Cow<'static, str> {
        match self {
            Self::DelegationPushFailed { role, intent } => {
                Cow::Borrowed(role.failed_predicate(intent))
            }
            Self::DelegationInstallNormalizationRejected { intent, reason } => Cow::Owned(format!(
                "delegation_install_normalization_rejected{{intent=\"{}\",reason=\"{}\"}}",
                install_intent_label(intent),
                normalization_reject_reason_label(reason)
            )),
            Self::DelegationInstallValidationFailed { intent, reason } => Cow::Owned(format!(
                "delegation_install_validation_failed{{intent=\"{}\",stage=\"post_normalization\",reason=\"{}\"}}",
                install_intent_label(intent),
                validation_failure_reason_label(reason)
            )),
            Self::ProofMiss => Cow::Borrowed(PRED_PROOF_MISS),
            Self::ProofMismatch => Cow::Borrowed(PRED_PROOF_MISMATCH),
            Self::ProofCacheEviction { class } => {
                Cow::Borrowed(proof_cache_eviction_predicate(class))
            }
            Self::ProofCacheUtilization { bucket } => Cow::Owned(format!(
                "proof_cache_utilization{{bucket=\"{}\"}}",
                proof_cache_utilization_bucket_label(bucket)
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthProofCacheUtilizationBucket {
    ZeroToFortyNine,
    FiftyToEightyFour,
    EightyFiveToNinetyFour,
    NinetyFiveToOneHundred,
}

impl AuthProofCacheUtilizationBucket {
    const fn from_size_and_capacity(size: usize, capacity: usize) -> Self {
        if capacity == 0 {
            return Self::ZeroToFortyNine;
        }

        let percent = size.saturating_mul(100) / capacity;
        match percent {
            0..=49 => Self::ZeroToFortyNine,
            50..=84 => Self::FiftyToEightyFour,
            85..=94 => Self::EightyFiveToNinetyFour,
            _ => Self::NinetyFiveToOneHundred,
        }
    }
}

const fn complete_predicate(intent: DelegationProofInstallIntent) -> &'static str {
    match intent {
        DelegationProofInstallIntent::Provisioning => PRED_DELEGATION_PROVISION_COMPLETE,
        DelegationProofInstallIntent::Prewarm => PRED_DELEGATION_PUSH_COMPLETE_PREWARM,
        DelegationProofInstallIntent::Repair => PRED_DELEGATION_PUSH_COMPLETE_REPAIR,
    }
}

const fn install_intent_label(intent: DelegationProofInstallIntent) -> &'static str {
    match intent {
        DelegationProofInstallIntent::Provisioning => "provisioning",
        DelegationProofInstallIntent::Prewarm => "prewarm",
        DelegationProofInstallIntent::Repair => "repair",
    }
}

const fn normalization_reject_reason_label(
    reason: DelegationInstallNormalizationRejectReason,
) -> &'static str {
    match reason {
        DelegationInstallNormalizationRejectReason::SignerTarget => "signer_target",
        DelegationInstallNormalizationRejectReason::RootTarget => "root_target",
        DelegationInstallNormalizationRejectReason::UnregisteredTarget => "unregistered_target",
        DelegationInstallNormalizationRejectReason::TargetNotInAudience => "target_not_in_audience",
    }
}

const fn validation_failure_reason_label(
    reason: DelegationInstallValidationFailureReason,
) -> &'static str {
    match reason {
        DelegationInstallValidationFailureReason::CacheKeys => "cache_keys",
        DelegationInstallValidationFailureReason::VerifyProof => "verify_proof",
        DelegationInstallValidationFailureReason::RepairMissingLocal => "repair_missing_local",
        DelegationInstallValidationFailureReason::RepairLocalMismatch => "repair_local_mismatch",
        DelegationInstallValidationFailureReason::TargetNotInAudience => "target_not_in_audience",
    }
}

const fn fanout_bucket(target_count: usize) -> &'static str {
    match target_count {
        0 => "0",
        1 => "1",
        2..=4 => "2_4",
        5..=8 => "5_8",
        _ => "9_plus",
    }
}

const fn proof_cache_utilization_bucket_label(
    bucket: AuthProofCacheUtilizationBucket,
) -> &'static str {
    match bucket {
        AuthProofCacheUtilizationBucket::ZeroToFortyNine => "0_49",
        AuthProofCacheUtilizationBucket::FiftyToEightyFour => "50_84",
        AuthProofCacheUtilizationBucket::EightyFiveToNinetyFour => "85_94",
        AuthProofCacheUtilizationBucket::NinetyFiveToOneHundred => "95_100",
    }
}

const fn proof_cache_eviction_predicate(class: VerifierProofCacheEvictionClass) -> &'static str {
    match class {
        VerifierProofCacheEvictionClass::Cold => PRED_PROOF_CACHE_COLD_EVICTION,
        VerifierProofCacheEvictionClass::Active => PRED_PROOF_CACHE_ACTIVE_EVICTION,
    }
}

fn record_auth_metric(endpoint: &str, predicate: AuthMetricPredicate) {
    let predicate = predicate.as_str();
    AccessMetrics::increment(endpoint, AccessMetricKind::Auth, predicate.as_ref());
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

pub fn record_delegation_push_attempt(
    role: DelegationProvisionRole,
    intent: DelegationProofInstallIntent,
) {
    AccessMetrics::increment(
        AUTH_SIGNER_ENDPOINT,
        AccessMetricKind::Auth,
        role.attempt_predicate(intent),
    );
}

pub fn record_delegation_push_success(
    role: DelegationProvisionRole,
    intent: DelegationProofInstallIntent,
) {
    AccessMetrics::increment(
        AUTH_SIGNER_ENDPOINT,
        AccessMetricKind::Auth,
        role.success_predicate(intent),
    );
}

pub fn record_delegation_push_failed(
    role: DelegationProvisionRole,
    intent: DelegationProofInstallIntent,
) {
    record_auth_metric(
        AUTH_SIGNER_ENDPOINT,
        AuthMetricPredicate::DelegationPushFailed { role, intent },
    );
}

pub fn record_delegation_push_complete(intent: DelegationProofInstallIntent) {
    AccessMetrics::increment(
        AUTH_SIGNER_ENDPOINT,
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
    AccessMetrics::increment(AUTH_SIGNER_ENDPOINT, AccessMetricKind::Auth, &predicate);
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
        AccessMetrics::increment(AUTH_SIGNER_ENDPOINT, AccessMetricKind::Auth, &predicate);
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
    AccessMetrics::increment(AUTH_SIGNER_ENDPOINT, AccessMetricKind::Auth, &predicate);
}

pub fn record_delegation_install_normalization_rejected(
    intent: DelegationProofInstallIntent,
    reason: DelegationInstallNormalizationRejectReason,
) {
    record_auth_metric(
        AUTH_SIGNER_ENDPOINT,
        AuthMetricPredicate::DelegationInstallNormalizationRejected { intent, reason },
    );
}

pub fn record_delegation_install_validation_failed(
    intent: DelegationProofInstallIntent,
    reason: DelegationInstallValidationFailureReason,
) {
    record_auth_metric(
        AUTH_SIGNER_ENDPOINT,
        AuthMetricPredicate::DelegationInstallValidationFailed { intent, reason },
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

pub fn record_verifier_proof_miss() {
    record_auth_metric(AUTH_VERIFIER_ENDPOINT, AuthMetricPredicate::ProofMiss);
}

pub fn record_verifier_proof_mismatch() {
    record_auth_metric(AUTH_VERIFIER_ENDPOINT, AuthMetricPredicate::ProofMismatch);
}

pub fn record_verifier_cert_expired() {
    AccessMetrics::increment(
        AUTH_VERIFIER_ENDPOINT,
        AccessMetricKind::Auth,
        PRED_CERT_EXPIRED,
    );
}

pub fn record_verifier_proof_cache_stats(
    size: usize,
    active_count: usize,
    capacity: usize,
    profile: DelegationProofCacheProfile,
    active_window_secs: u64,
) {
    let size_predicate = format!("proof_cache_size{{size=\"{size}\"}}");
    AccessMetrics::increment(
        AUTH_VERIFIER_ENDPOINT,
        AccessMetricKind::Auth,
        &size_predicate,
    );

    let active_predicate = format!("proof_cache_active_size{{size=\"{active_count}\"}}");
    AccessMetrics::increment(
        AUTH_VERIFIER_ENDPOINT,
        AccessMetricKind::Auth,
        &active_predicate,
    );

    record_auth_metric(
        AUTH_VERIFIER_ENDPOINT,
        AuthMetricPredicate::ProofCacheUtilization {
            bucket: AuthProofCacheUtilizationBucket::from_size_and_capacity(size, capacity),
        },
    );

    let capacity_predicate = format!(
        "proof_cache_capacity{{profile=\"{}\",capacity=\"{capacity}\"}}",
        profile.as_str()
    );
    AccessMetrics::increment(
        AUTH_VERIFIER_ENDPOINT,
        AccessMetricKind::Auth,
        &capacity_predicate,
    );

    let active_window_predicate =
        format!("proof_cache_active_window_secs{{secs=\"{active_window_secs}\"}}");
    AccessMetrics::increment(
        AUTH_VERIFIER_ENDPOINT,
        AccessMetricKind::Auth,
        &active_window_predicate,
    );
}

pub fn record_verifier_proof_cache_eviction(class: VerifierProofCacheEvictionClass) {
    record_auth_metric(
        AUTH_VERIFIER_ENDPOINT,
        AuthMetricPredicate::ProofCacheEviction { class },
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

        record_delegation_push_attempt(
            DelegationProvisionRole::Signer,
            DelegationProofInstallIntent::Provisioning,
        );
        record_delegation_push_attempt(
            DelegationProvisionRole::Verifier,
            DelegationProofInstallIntent::Provisioning,
        );
        record_delegation_push_success(
            DelegationProvisionRole::Signer,
            DelegationProofInstallIntent::Provisioning,
        );
        record_delegation_push_success(
            DelegationProvisionRole::Verifier,
            DelegationProofInstallIntent::Provisioning,
        );
        record_delegation_push_failed(
            DelegationProvisionRole::Verifier,
            DelegationProofInstallIntent::Provisioning,
        );
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

    #[test]
    fn delegation_install_diagnostic_metrics_track_intent_and_stage() {
        AccessMetrics::reset();

        record_delegation_install_total(DelegationProofInstallIntent::Repair);
        record_delegation_install_normalized_target_count(DelegationProofInstallIntent::Repair, 3);
        record_delegation_install_fanout_bucket(DelegationProofInstallIntent::Repair, 3);
        record_delegation_install_normalization_rejected(
            DelegationProofInstallIntent::Repair,
            DelegationInstallNormalizationRejectReason::TargetNotInAudience,
        );
        record_delegation_install_validation_failed(
            DelegationProofInstallIntent::Repair,
            DelegationInstallValidationFailureReason::TargetNotInAudience,
        );

        assert_eq!(
            metric_count(
                AUTH_SIGNER_ENDPOINT,
                "delegation_install_total{intent=\"repair\"}"
            ),
            1
        );
        assert_eq!(
            metric_count(
                AUTH_SIGNER_ENDPOINT,
                "delegation_install_normalized_target_total{intent=\"repair\"}"
            ),
            3
        );
        assert_eq!(
            metric_count(
                AUTH_SIGNER_ENDPOINT,
                "delegation_install_fanout_bucket{intent=\"repair\",bucket=\"2_4\"}"
            ),
            1
        );
        assert_eq!(
            metric_count(
                AUTH_SIGNER_ENDPOINT,
                AuthMetricPredicate::DelegationInstallNormalizationRejected {
                    intent: DelegationProofInstallIntent::Repair,
                    reason: DelegationInstallNormalizationRejectReason::TargetNotInAudience,
                }
                .as_str()
                .as_ref()
            ),
            1
        );
        assert_eq!(
            metric_count(
                AUTH_SIGNER_ENDPOINT,
                AuthMetricPredicate::DelegationInstallValidationFailed {
                    intent: DelegationProofInstallIntent::Repair,
                    reason: DelegationInstallValidationFailureReason::TargetNotInAudience,
                }
                .as_str()
                .as_ref()
            ),
            1
        );
    }

    #[test]
    fn verifier_cache_metrics_track_size_utilization_and_eviction() {
        AccessMetrics::reset();

        record_verifier_proof_cache_stats(80, 4, 96, DelegationProofCacheProfile::Standard, 600);
        record_verifier_proof_cache_eviction(VerifierProofCacheEvictionClass::Cold);
        record_verifier_proof_cache_eviction(VerifierProofCacheEvictionClass::Active);

        assert_eq!(
            metric_count(AUTH_VERIFIER_ENDPOINT, "proof_cache_size{size=\"80\"}"),
            1
        );
        assert_eq!(
            metric_count(
                AUTH_VERIFIER_ENDPOINT,
                "proof_cache_active_size{size=\"4\"}"
            ),
            1
        );
        assert_eq!(
            metric_count(
                AUTH_VERIFIER_ENDPOINT,
                AuthMetricPredicate::ProofCacheUtilization {
                    bucket: AuthProofCacheUtilizationBucket::FiftyToEightyFour,
                }
                .as_str()
                .as_ref()
            ),
            1
        );
        assert_eq!(
            metric_count(
                AUTH_VERIFIER_ENDPOINT,
                "proof_cache_capacity{profile=\"standard\",capacity=\"96\"}"
            ),
            1
        );
        assert_eq!(
            metric_count(
                AUTH_VERIFIER_ENDPOINT,
                "proof_cache_active_window_secs{secs=\"600\"}"
            ),
            1
        );
        assert_eq!(
            metric_count(AUTH_VERIFIER_ENDPOINT, PRED_PROOF_CACHE_COLD_EVICTION),
            1
        );
        assert_eq!(
            metric_count(AUTH_VERIFIER_ENDPOINT, PRED_PROOF_CACHE_ACTIVE_EVICTION),
            1
        );
    }
}
