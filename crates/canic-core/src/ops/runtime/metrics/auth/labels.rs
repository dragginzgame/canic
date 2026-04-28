use super::{
    DelegationInstallNormalizationRejectReason, DelegationInstallValidationFailureReason,
    VerifierProofCacheEvictionClass,
};
use crate::dto::auth::DelegationProofInstallIntent;

const AUTH_SIGNER_ENDPOINT: &str = "auth_signer";
const AUTH_SESSION_ENDPOINT: &str = "auth_session";
const AUTH_VERIFIER_ENDPOINT: &str = "auth_verifier";
const AUTH_ATTESTATION_VERIFIER_ENDPOINT: &str = "auth_attestation_verifier";

const PRED_ISSUE_WITHOUT_PROOF: &str = "issue_without_proof";
const PRED_DELEGATION_VERIFIER_TARGET_FAILED: &str = "delegation_verifier_target_failed";
const PRED_DELEGATION_VERIFIER_TARGET_MISSING: &str = "delegation_verifier_target_missing";
const PRED_DELEGATION_VERIFIER_TARGET_COUNT: &str = "delegation_verifier_target_count";
const PRED_DELEGATION_PROVISION_COMPLETE: &str = "delegation_provision_complete";
const PRED_DELEGATION_PROVISION_ATTEMPT_VERIFIER: &str =
    "delegation_provision_attempt{role=\"verifier\"}";
const PRED_DELEGATION_PROVISION_SUCCESS_VERIFIER: &str =
    "delegation_provision_success{role=\"verifier\"}";
const PRED_DELEGATION_PROVISION_FAILED_VERIFIER: &str =
    "delegation_provision_failed{role=\"verifier\"}";
const PRED_DELEGATION_PUSH_COMPLETE_REPAIR: &str = "delegation_push_complete{origin=\"repair\"}";
const PRED_DELEGATION_PUSH_ATTEMPT_VERIFIER_REPAIR: &str =
    "delegation_push_attempt{role=\"verifier\",origin=\"repair\"}";
const PRED_DELEGATION_PUSH_SUCCESS_VERIFIER_REPAIR: &str =
    "delegation_push_success{role=\"verifier\",origin=\"repair\"}";
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

pub(super) const fn auth_signer_endpoint() -> &'static str {
    AUTH_SIGNER_ENDPOINT
}

pub(super) const fn auth_session_endpoint() -> &'static str {
    AUTH_SESSION_ENDPOINT
}

pub(super) const fn auth_verifier_endpoint() -> &'static str {
    AUTH_VERIFIER_ENDPOINT
}

pub(super) const fn auth_attestation_verifier_endpoint() -> &'static str {
    AUTH_ATTESTATION_VERIFIER_ENDPOINT
}

pub(super) const fn signer_issue_without_proof_predicate() -> &'static str {
    PRED_ISSUE_WITHOUT_PROOF
}

pub(super) const fn verifier_target_failed_predicate() -> &'static str {
    PRED_DELEGATION_VERIFIER_TARGET_FAILED
}

pub(super) const fn verifier_target_missing_predicate() -> &'static str {
    PRED_DELEGATION_VERIFIER_TARGET_MISSING
}

pub(super) const fn verifier_target_count_predicate() -> &'static str {
    PRED_DELEGATION_VERIFIER_TARGET_COUNT
}

pub(super) const fn delegation_provision_attempt_verifier_predicate() -> &'static str {
    PRED_DELEGATION_PROVISION_ATTEMPT_VERIFIER
}

pub(super) const fn delegation_provision_success_verifier_predicate() -> &'static str {
    PRED_DELEGATION_PROVISION_SUCCESS_VERIFIER
}

pub(super) const fn delegation_provision_failed_verifier_predicate() -> &'static str {
    PRED_DELEGATION_PROVISION_FAILED_VERIFIER
}

pub(super) const fn delegation_push_attempt_verifier_repair_predicate() -> &'static str {
    PRED_DELEGATION_PUSH_ATTEMPT_VERIFIER_REPAIR
}

pub(super) const fn delegation_push_success_verifier_repair_predicate() -> &'static str {
    PRED_DELEGATION_PUSH_SUCCESS_VERIFIER_REPAIR
}

pub(super) const fn delegation_push_failed_verifier_repair_predicate() -> &'static str {
    PRED_DELEGATION_PUSH_FAILED_VERIFIER_REPAIR
}

pub(super) const fn session_bootstrap_rejected_disabled_predicate() -> &'static str {
    PRED_SESSION_BOOTSTRAP_REJECTED_DISABLED
}

pub(super) const fn session_bootstrap_rejected_subject_mismatch_predicate() -> &'static str {
    PRED_SESSION_BOOTSTRAP_REJECTED_SUBJECT_MISMATCH
}

pub(super) const fn session_bootstrap_rejected_subject_rejected_predicate() -> &'static str {
    PRED_SESSION_BOOTSTRAP_REJECTED_SUBJECT_REJECTED
}

pub(super) const fn session_bootstrap_rejected_replay_conflict_predicate() -> &'static str {
    PRED_SESSION_BOOTSTRAP_REJECTED_REPLAY_CONFLICT
}

pub(super) const fn session_bootstrap_rejected_replay_reused_predicate() -> &'static str {
    PRED_SESSION_BOOTSTRAP_REJECTED_REPLAY_REUSED
}

pub(super) const fn session_bootstrap_rejected_token_invalid_predicate() -> &'static str {
    PRED_SESSION_BOOTSTRAP_REJECTED_TOKEN_INVALID
}

pub(super) const fn session_bootstrap_rejected_ttl_invalid_predicate() -> &'static str {
    PRED_SESSION_BOOTSTRAP_REJECTED_TTL_INVALID
}

pub(super) const fn session_bootstrap_rejected_wallet_caller_rejected_predicate() -> &'static str {
    PRED_SESSION_BOOTSTRAP_REJECTED_WALLET_CALLER_REJECTED
}

pub(super) const fn session_bootstrap_replay_idempotent_predicate() -> &'static str {
    PRED_SESSION_BOOTSTRAP_REPLAY_IDEMPOTENT
}

pub(super) const fn session_cleared_predicate() -> &'static str {
    PRED_SESSION_CLEARED
}

pub(super) const fn session_created_predicate() -> &'static str {
    PRED_SESSION_CREATED
}

pub(super) const fn session_fallback_invalid_subject_predicate() -> &'static str {
    PRED_SESSION_FALLBACK_INVALID_SUBJECT
}

pub(super) const fn session_fallback_raw_caller_predicate() -> &'static str {
    PRED_SESSION_FALLBACK_RAW_CALLER
}

pub(super) const fn session_pruned_predicate() -> &'static str {
    PRED_SESSION_PRUNED
}

pub(super) const fn session_replaced_predicate() -> &'static str {
    PRED_SESSION_REPLACED
}

pub(super) const fn proof_miss_predicate() -> &'static str {
    PRED_PROOF_MISS
}

pub(super) const fn proof_mismatch_predicate() -> &'static str {
    PRED_PROOF_MISMATCH
}

pub(super) const fn cert_expired_predicate() -> &'static str {
    PRED_CERT_EXPIRED
}

pub(super) const fn attestation_verify_failed_predicate() -> &'static str {
    PRED_ATTESTATION_VERIFY_FAILED
}

pub(super) const fn attestation_unknown_key_id_predicate() -> &'static str {
    PRED_ATTESTATION_UNKNOWN_KEY_ID
}

pub(super) const fn attestation_epoch_rejected_predicate() -> &'static str {
    PRED_ATTESTATION_EPOCH_REJECTED
}

pub(super) const fn attestation_refresh_failed_predicate() -> &'static str {
    PRED_ATTESTATION_REFRESH_FAILED
}

pub(super) const fn complete_predicate(intent: DelegationProofInstallIntent) -> &'static str {
    match intent {
        DelegationProofInstallIntent::Provisioning => PRED_DELEGATION_PROVISION_COMPLETE,
        DelegationProofInstallIntent::Repair => PRED_DELEGATION_PUSH_COMPLETE_REPAIR,
    }
}

pub(super) const fn install_intent_label(intent: DelegationProofInstallIntent) -> &'static str {
    match intent {
        DelegationProofInstallIntent::Provisioning => "provisioning",
        DelegationProofInstallIntent::Repair => "repair",
    }
}

pub(super) const fn normalization_reject_reason_label(
    reason: DelegationInstallNormalizationRejectReason,
) -> &'static str {
    match reason {
        DelegationInstallNormalizationRejectReason::SignerTarget => "signer_target",
        DelegationInstallNormalizationRejectReason::RootTarget => "root_target",
        DelegationInstallNormalizationRejectReason::UnregisteredTarget => "unregistered_target",
        DelegationInstallNormalizationRejectReason::TargetNotInAudience => "target_not_in_audience",
    }
}

pub(super) const fn validation_failure_reason_label(
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

pub(super) const fn fanout_bucket(target_count: usize) -> &'static str {
    match target_count {
        0 => "0",
        1 => "1",
        2..=4 => "2_4",
        5..=8 => "5_8",
        _ => "9_plus",
    }
}

pub(super) const fn proof_cache_utilization_bucket_label(
    bucket: super::AuthProofCacheUtilizationBucket,
) -> &'static str {
    match bucket {
        super::AuthProofCacheUtilizationBucket::ZeroToFortyNine => "0_49",
        super::AuthProofCacheUtilizationBucket::FiftyToEightyFour => "50_84",
        super::AuthProofCacheUtilizationBucket::EightyFiveToNinetyFour => "85_94",
        super::AuthProofCacheUtilizationBucket::NinetyFiveToOneHundred => "95_100",
    }
}

pub(super) const fn proof_cache_eviction_predicate(
    class: VerifierProofCacheEvictionClass,
) -> &'static str {
    match class {
        VerifierProofCacheEvictionClass::Cold => PRED_PROOF_CACHE_COLD_EVICTION,
        VerifierProofCacheEvictionClass::Active => PRED_PROOF_CACHE_ACTIVE_EVICTION,
    }
}

pub(super) fn proof_cache_size_predicate(size: usize) -> String {
    format!("proof_cache_size{{size=\"{size}\"}}")
}

pub(super) fn proof_cache_active_size_predicate(active_count: usize) -> String {
    format!("proof_cache_active_size{{size=\"{active_count}\"}}")
}

pub(super) fn proof_cache_capacity_predicate(
    profile: crate::config::schema::DelegationProofCacheProfile,
    capacity: usize,
) -> String {
    format!(
        "proof_cache_capacity{{profile=\"{}\",capacity=\"{capacity}\"}}",
        profile.as_str()
    )
}

pub(super) fn proof_cache_active_window_predicate(active_window_secs: u64) -> String {
    format!("proof_cache_active_window_secs{{secs=\"{active_window_secs}\"}}")
}
