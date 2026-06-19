//! Module: ops::auth::verify
//!
//! Responsibility: route auth claim verification to focused verifier helpers.
//! Does not own: proof preparation, storage, or endpoint authorization.
//! Boundary: private auth-ops verification dispatch.

mod attestation;

use crate::{cdk::types::Principal, dto::auth::RoleAttestation, ops::auth::AuthOpsError};

// Route role-attestation verification through the attestation-focused verifier module.
pub(super) fn verify_role_attestation_claims(
    payload: &RoleAttestation,
    caller: Principal,
    self_pid: Principal,
    verifier_subnet: Option<Principal>,
    now_ns: u64,
    min_accepted_epoch: u64,
) -> Result<(), AuthOpsError> {
    attestation::verify_role_attestation_claims(
        payload,
        caller,
        self_pid,
        verifier_subnet,
        now_ns,
        min_accepted_epoch,
    )
}
