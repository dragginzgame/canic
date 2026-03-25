mod attestation;
mod proof_state;
mod token_chain;

use crate::{
    InternalError,
    cdk::types::Principal,
    dto::auth::{AttestationKey, DelegatedToken, DelegationCert, DelegationProof, RoleAttestation},
    ops::auth::{DelegatedTokenOpsError, DelegationExpiryError, TokenGrant, TokenLifetime},
};

// Route role-attestation verification through the attestation-focused verifier module.
pub(super) fn verify_role_attestation_claims(
    payload: &RoleAttestation,
    caller: Principal,
    self_pid: Principal,
    verifier_subnet: Option<Principal>,
    now_secs: u64,
    min_accepted_epoch: u64,
) -> Result<(), DelegatedTokenOpsError> {
    attestation::verify_role_attestation_claims(
        payload,
        caller,
        self_pid,
        verifier_subnet,
        now_secs,
        min_accepted_epoch,
    )
}

// Route attestation-key time validity checks through the attestation verifier module.
pub(super) fn verify_attestation_key_validity(
    key: &AttestationKey,
    now_secs: u64,
) -> Result<(), DelegatedTokenOpsError> {
    attestation::verify_attestation_key_validity(key, now_secs)
}

// Route verifier-local proof-state checks through the proof-state verifier module.
#[cfg(test)]
pub(super) fn verify_current_proof(proof: &DelegationProof) -> Result<(), InternalError> {
    proof_state::verify_current_proof(proof)
}

// Route token claim-vs-cert grant checks through the token-chain verifier module.
pub(super) fn validate_claims_against_cert(
    grant: TokenGrant<'_>,
    cert: &DelegationCert,
) -> Result<(), InternalError> {
    token_chain::validate_claims_against_cert(grant, cert)
}

// Route delegation signature checks through the token-chain verifier module.
pub(super) fn verify_delegation_signature(proof: &DelegationProof) -> Result<(), InternalError> {
    token_chain::verify_delegation_signature(proof)
}

// Route max TTL checks through the token-chain verifier module.
pub(super) fn verify_max_ttl(
    lifetime: TokenLifetime,
    max_ttl_secs: u64,
) -> Result<(), DelegationExpiryError> {
    token_chain::verify_max_ttl(lifetime, max_ttl_secs)
}

// Route the full canonical delegated-token trust chain through the token-chain verifier module.
pub(super) fn verify_token_trust_chain(
    token: &DelegatedToken,
    authority_pid: Principal,
    now_secs: u64,
    self_pid: Principal,
) -> Result<(), InternalError> {
    token_chain::verify_token_trust_chain(token, authority_pid, now_secs, self_pid)
}

// Expose the trust-chain trace seam for unit tests without widening production visibility.
#[cfg(test)]
pub(super) fn trace_token_trust_chain(
    token: &DelegatedToken,
    authority_pid: Principal,
    now_secs: u64,
    self_pid: Principal,
) -> (Vec<&'static str>, Result<(), InternalError>) {
    token_chain::trace_token_trust_chain(token, authority_pid, now_secs, self_pid)
}

// Expose the forced proof-failure trust-chain seam for unit tests only.
#[cfg(test)]
pub(super) fn trace_token_trust_chain_with_forced_current_proof_failure(
    token: &DelegatedToken,
    authority_pid: Principal,
    now_secs: u64,
    self_pid: Principal,
    err: InternalError,
) -> (Vec<&'static str>, Result<(), InternalError>) {
    token_chain::trace_token_trust_chain_with_forced_current_proof_failure(
        token,
        authority_pid,
        now_secs,
        self_pid,
        err,
    )
}
