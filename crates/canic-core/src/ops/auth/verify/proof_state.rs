use crate::{
    InternalError,
    ops::{
        ic::IcOps,
        runtime::metrics::auth::{
            record_verifier_proof_cache_stats, record_verifier_proof_mismatch,
            record_verifier_proof_miss,
        },
        storage::auth::DelegationStateOps,
    },
};
use crate::{dto::auth::DelegationProof, ops::auth::DelegationValidationError};

// Enforce verifier-local proof presence and equality before token signatures count.
pub(super) fn verify_current_proof(proof: &DelegationProof) -> Result<(), InternalError> {
    let now_secs = IcOps::now_secs();
    let Some(stored) = DelegationStateOps::matching_proof_dto(proof)? else {
        let stats = DelegationStateOps::proof_cache_stats(now_secs)?;
        record_verifier_proof_cache_stats(
            stats.size,
            stats.active_count,
            stats.capacity,
            stats.profile,
            stats.active_window_secs,
        );
        record_verifier_proof_miss();
        let local = IcOps::canister_self();
        crate::log!(
            crate::log::Topic::Auth,
            Warn,
            "delegation proof miss local={} shard={}",
            local,
            proof.cert.shard_pid
        );
        return Err(DelegationValidationError::ProofMiss.into());
    };

    if proofs_equal(proof, &stored) {
        let _ = DelegationStateOps::mark_matching_proof_verified(proof, now_secs)?;
        let stats = DelegationStateOps::proof_cache_stats(now_secs)?;
        record_verifier_proof_cache_stats(
            stats.size,
            stats.active_count,
            stats.capacity,
            stats.profile,
            stats.active_window_secs,
        );
        Ok(())
    } else {
        let stats = DelegationStateOps::proof_cache_stats(now_secs)?;
        record_verifier_proof_cache_stats(
            stats.size,
            stats.active_count,
            stats.capacity,
            stats.profile,
            stats.active_window_secs,
        );
        record_verifier_proof_mismatch();
        let local = IcOps::canister_self();
        crate::log!(
            crate::log::Topic::Auth,
            Warn,
            "delegation proof mismatch local={} shard={} stored_shard={}",
            local,
            proof.cert.shard_pid,
            stored.cert.shard_pid
        );
        Err(DelegationValidationError::ProofMismatch.into())
    }
}

// Compare the full delegation proof payload that defines verifier-local trust.
pub(super) fn proofs_equal(a: &DelegationProof, b: &DelegationProof) -> bool {
    let a_cert = &a.cert;
    let b_cert = &b.cert;

    a_cert.root_pid == b_cert.root_pid
        && a_cert.shard_pid == b_cert.shard_pid
        && a_cert.issued_at == b_cert.issued_at
        && a_cert.expires_at == b_cert.expires_at
        && a_cert.scopes == b_cert.scopes
        && a_cert.aud == b_cert.aud
        && a.cert_sig == b.cert_sig
}
