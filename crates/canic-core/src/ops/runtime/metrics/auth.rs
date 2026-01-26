use crate::{ids::AccessMetricKind, ops::runtime::metrics::access::AccessMetrics};

const AUTH_SIGNER_ENDPOINT: &str = "auth_signer";
const AUTH_VERIFIER_ENDPOINT: &str = "auth_verifier";

const PRED_MINT_WITHOUT_PROOF: &str = "mint_without_proof";
const PRED_PROOF_MISSING: &str = "token_rejected_proof_missing";
const PRED_PROOF_MISMATCH: &str = "token_rejected_proof_mismatch";
const PRED_CERT_EXPIRED: &str = "token_rejected_expired_cert";

pub fn record_signer_mint_without_proof() {
    AccessMetrics::increment(
        AUTH_SIGNER_ENDPOINT,
        AccessMetricKind::Auth,
        PRED_MINT_WITHOUT_PROOF,
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
