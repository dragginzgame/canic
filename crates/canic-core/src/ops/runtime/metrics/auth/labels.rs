const AUTH_SESSION_ENDPOINT: &str = "auth_session";
const AUTH_ATTESTATION_VERIFIER_ENDPOINT: &str = "auth_attestation_verifier";

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
const PRED_SESSION_BOOTSTRAP_REJECTED_CAPACITY: &str = "session_bootstrap_rejected_capacity";
const PRED_SESSION_BOOTSTRAP_REPLAY_IDEMPOTENT: &str = "session_bootstrap_replay_idempotent";
const PRED_SESSION_CLEARED: &str = "session_cleared";
const PRED_SESSION_CREATED: &str = "session_created";
const PRED_SESSION_FALLBACK_INVALID_SUBJECT: &str = "session_fallback_invalid_subject";
const PRED_SESSION_FALLBACK_RAW_CALLER: &str = "session_fallback_raw_caller";
const PRED_SESSION_PRUNED: &str = "session_pruned";
const PRED_SESSION_REPLACED: &str = "session_replaced";
const PRED_ATTESTATION_VERIFY_FAILED: &str = "attestation_verify_failed";
const PRED_ATTESTATION_UNKNOWN_KEY_ID: &str = "attestation_unknown_key_id";
const PRED_ATTESTATION_EPOCH_REJECTED: &str = "attestation_epoch_rejected";
const PRED_ATTESTATION_REFRESH_FAILED: &str = "attestation_refresh_failed";

pub(super) const fn auth_session_endpoint() -> &'static str {
    AUTH_SESSION_ENDPOINT
}

pub(super) const fn auth_attestation_verifier_endpoint() -> &'static str {
    AUTH_ATTESTATION_VERIFIER_ENDPOINT
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

pub(super) const fn session_bootstrap_rejected_capacity_predicate() -> &'static str {
    PRED_SESSION_BOOTSTRAP_REJECTED_CAPACITY
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
