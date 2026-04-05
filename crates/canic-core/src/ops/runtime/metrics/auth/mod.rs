use crate::{
    dto::auth::DelegationProofInstallIntent, ids::AccessMetricKind,
    ops::runtime::metrics::access::AccessMetrics,
};
use std::borrow::Cow;

mod attestation;
mod delegation;
mod labels;
mod sessions;
mod verifier;

pub use attestation::{
    record_attestation_epoch_rejected, record_attestation_refresh_failed,
    record_attestation_unknown_key_id, record_attestation_verify_failed,
};
pub use delegation::{
    record_delegation_install_fanout_bucket, record_delegation_install_normalization_rejected,
    record_delegation_install_normalized_target_count, record_delegation_install_total,
    record_delegation_install_validation_failed, record_delegation_provision_complete,
    record_delegation_push_attempt, record_delegation_push_complete, record_delegation_push_failed,
    record_delegation_push_success, record_delegation_verifier_target_count,
    record_delegation_verifier_target_failed, record_delegation_verifier_target_missing,
    record_signer_issue_without_proof,
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
pub use verifier::{
    record_verifier_cert_expired, record_verifier_proof_cache_eviction,
    record_verifier_proof_cache_stats, record_verifier_proof_mismatch, record_verifier_proof_miss,
};

use labels::{
    attestation_epoch_rejected_predicate, attestation_refresh_failed_predicate,
    attestation_unknown_key_id_predicate, attestation_verify_failed_predicate,
    auth_attestation_verifier_endpoint, auth_session_endpoint, auth_signer_endpoint,
    auth_verifier_endpoint, cert_expired_predicate, complete_predicate,
    delegation_provision_attempt_signer_predicate, delegation_provision_attempt_verifier_predicate,
    delegation_provision_failed_verifier_predicate, delegation_provision_success_signer_predicate,
    delegation_provision_success_verifier_predicate, fanout_bucket, install_intent_label,
    normalization_reject_reason_label, proof_cache_active_size_predicate,
    proof_cache_active_window_predicate, proof_cache_capacity_predicate,
    proof_cache_eviction_predicate, proof_cache_size_predicate,
    proof_cache_utilization_bucket_label, session_bootstrap_rejected_disabled_predicate,
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
    signer_issue_without_proof_predicate, validation_failure_reason_label,
    verifier_target_count_predicate, verifier_target_failed_predicate,
    verifier_target_missing_predicate,
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DelegationProvisionRole {
    Signer,
    Verifier,
}

impl DelegationProvisionRole {
    const fn attempt_predicate(self, intent: DelegationProofInstallIntent) -> &'static str {
        match (self, intent) {
            (Self::Signer, DelegationProofInstallIntent::Provisioning) => {
                delegation_provision_attempt_signer_predicate()
            }
            (Self::Verifier, DelegationProofInstallIntent::Provisioning) => {
                delegation_provision_attempt_verifier_predicate()
            }
            (Self::Signer, DelegationProofInstallIntent::Prewarm) => {
                labels::delegation_push_attempt_signer_prewarm_predicate()
            }
            (Self::Verifier, DelegationProofInstallIntent::Prewarm) => {
                labels::delegation_push_attempt_verifier_prewarm_predicate()
            }
            (Self::Signer, DelegationProofInstallIntent::Repair) => {
                labels::delegation_push_attempt_signer_repair_predicate()
            }
            (Self::Verifier, DelegationProofInstallIntent::Repair) => {
                labels::delegation_push_attempt_verifier_repair_predicate()
            }
        }
    }

    const fn success_predicate(self, intent: DelegationProofInstallIntent) -> &'static str {
        match (self, intent) {
            (Self::Signer, DelegationProofInstallIntent::Provisioning) => {
                delegation_provision_success_signer_predicate()
            }
            (Self::Verifier, DelegationProofInstallIntent::Provisioning) => {
                delegation_provision_success_verifier_predicate()
            }
            (Self::Signer, DelegationProofInstallIntent::Prewarm) => {
                labels::delegation_push_success_signer_prewarm_predicate()
            }
            (Self::Verifier, DelegationProofInstallIntent::Prewarm) => {
                labels::delegation_push_success_verifier_prewarm_predicate()
            }
            (Self::Signer, DelegationProofInstallIntent::Repair) => {
                labels::delegation_push_success_signer_repair_predicate()
            }
            (Self::Verifier, DelegationProofInstallIntent::Repair) => {
                labels::delegation_push_success_verifier_repair_predicate()
            }
        }
    }

    const fn failed_predicate(self, intent: DelegationProofInstallIntent) -> &'static str {
        match (self, intent) {
            (Self::Signer, DelegationProofInstallIntent::Provisioning) => {
                labels::delegation_provision_failed_signer_predicate()
            }
            (Self::Verifier, DelegationProofInstallIntent::Provisioning) => {
                delegation_provision_failed_verifier_predicate()
            }
            (Self::Signer, DelegationProofInstallIntent::Prewarm) => {
                labels::delegation_push_failed_signer_prewarm_predicate()
            }
            (Self::Verifier, DelegationProofInstallIntent::Prewarm) => {
                labels::delegation_push_failed_verifier_prewarm_predicate()
            }
            (Self::Signer, DelegationProofInstallIntent::Repair) => {
                labels::delegation_push_failed_signer_repair_predicate()
            }
            (Self::Verifier, DelegationProofInstallIntent::Repair) => {
                labels::delegation_push_failed_verifier_repair_predicate()
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
            Self::ProofMiss => Cow::Borrowed(labels::proof_miss_predicate()),
            Self::ProofMismatch => Cow::Borrowed(labels::proof_mismatch_predicate()),
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

fn record_auth_metric(endpoint: &str, predicate: AuthMetricPredicate) {
    let predicate = predicate.as_str();
    AccessMetrics::increment(endpoint, AccessMetricKind::Auth, predicate.as_ref());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::DelegationProofCacheProfile;

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
                auth_signer_endpoint(),
                labels::delegation_provision_attempt_signer_predicate()
            ),
            1
        );
        assert_eq!(
            metric_count(
                auth_signer_endpoint(),
                labels::delegation_provision_attempt_verifier_predicate()
            ),
            1
        );
        assert_eq!(
            metric_count(
                auth_signer_endpoint(),
                labels::delegation_provision_success_signer_predicate()
            ),
            1
        );
        assert_eq!(
            metric_count(
                auth_signer_endpoint(),
                labels::delegation_provision_success_verifier_predicate()
            ),
            1
        );
        assert_eq!(
            metric_count(
                auth_signer_endpoint(),
                labels::delegation_provision_failed_verifier_predicate()
            ),
            1
        );
        assert_eq!(
            metric_count(auth_signer_endpoint(), verifier_target_failed_predicate()),
            1
        );
        assert_eq!(
            metric_count(auth_signer_endpoint(), verifier_target_missing_predicate()),
            1
        );
        assert_eq!(
            metric_count(auth_signer_endpoint(), verifier_target_count_predicate()),
            3
        );
        assert_eq!(
            metric_count(
                auth_signer_endpoint(),
                complete_predicate(DelegationProofInstallIntent::Provisioning)
            ),
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
                auth_signer_endpoint(),
                "delegation_install_total{intent=\"repair\"}"
            ),
            1
        );
        assert_eq!(
            metric_count(
                auth_signer_endpoint(),
                "delegation_install_normalized_target_total{intent=\"repair\"}"
            ),
            3
        );
        assert_eq!(
            metric_count(
                auth_signer_endpoint(),
                "delegation_install_fanout_bucket{intent=\"repair\",bucket=\"2_4\"}"
            ),
            1
        );
        assert_eq!(
            metric_count(
                auth_signer_endpoint(),
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
                auth_signer_endpoint(),
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
            metric_count(auth_verifier_endpoint(), "proof_cache_size{size=\"80\"}"),
            1
        );
        assert_eq!(
            metric_count(
                auth_verifier_endpoint(),
                "proof_cache_active_size{size=\"4\"}"
            ),
            1
        );
        assert_eq!(
            metric_count(
                auth_verifier_endpoint(),
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
                auth_verifier_endpoint(),
                "proof_cache_capacity{profile=\"standard\",capacity=\"96\"}"
            ),
            1
        );
        assert_eq!(
            metric_count(
                auth_verifier_endpoint(),
                "proof_cache_active_window_secs{secs=\"600\"}"
            ),
            1
        );
        assert_eq!(
            metric_count(
                auth_verifier_endpoint(),
                proof_cache_eviction_predicate(VerifierProofCacheEvictionClass::Cold)
            ),
            1
        );
        assert_eq!(
            metric_count(
                auth_verifier_endpoint(),
                proof_cache_eviction_predicate(VerifierProofCacheEvictionClass::Active)
            ),
            1
        );
    }
}
