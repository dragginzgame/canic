use crate::{ids::AccessMetricKind, ops::runtime::metrics::access::AccessMetrics};

const AUTH_SIGNER_ENDPOINT: &str = "auth_signer";
const AUTH_VERIFIER_ENDPOINT: &str = "auth_verifier";
const AUTH_ATTESTATION_VERIFIER_ENDPOINT: &str = "auth_attestation_verifier";

const PRED_ISSUE_WITHOUT_PROOF: &str = "issue_without_proof";
const PRED_PROOF_MISSING: &str = "token_rejected_proof_missing";
const PRED_PROOF_MISMATCH: &str = "token_rejected_proof_mismatch";
const PRED_CERT_EXPIRED: &str = "token_rejected_expired_cert";
const PRED_ATTESTATION_VERIFY_FAILED: &str = "attestation_verify_failed";
const PRED_ATTESTATION_UNKNOWN_KEY_ID: &str = "attestation_unknown_key_id";
const PRED_ATTESTATION_EPOCH_REJECTED: &str = "attestation_epoch_rejected";
const PRED_ATTESTATION_REFRESH_FAILED: &str = "attestation_refresh_failed";

pub fn record_signer_issue_without_proof() {
    AccessMetrics::increment(
        AUTH_SIGNER_ENDPOINT,
        AccessMetricKind::Auth,
        PRED_ISSUE_WITHOUT_PROOF,
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
