mod attestation;

use crate::{
    cdk::types::Principal,
    dto::auth::{AttestationKey, RoleAttestation},
    ops::auth::AuthOpsError,
};

// Route role-attestation verification through the attestation-focused verifier module.
pub(super) fn verify_role_attestation_claims(
    payload: &RoleAttestation,
    caller: Principal,
    self_pid: Principal,
    verifier_subnet: Option<Principal>,
    now_secs: u64,
    min_accepted_epoch: u64,
) -> Result<(), AuthOpsError> {
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
) -> Result<(), AuthOpsError> {
    attestation::verify_attestation_key_validity(key, now_secs)
}
