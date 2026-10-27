//! Verify decoded local-Subnet role attestations for endpoint access.
//! Proof issuance, trust configuration and transport decoding have separate owners.

use crate::{
    access::AccessError, cdk::types::Principal, dto::auth::SignedRoleAttestation,
    workflow::runtime::auth::RuntimeAuthWorkflow,
};

pub(super) fn verify_attestation(
    attestation: &SignedRoleAttestation,
    caller: Principal,
) -> Result<(), AccessError> {
    if attestation.payload.subject != caller {
        return Err(AccessError::RoleAttestationSubjectMismatch);
    }

    RuntimeAuthWorkflow::verify_local_subnet_role_attestation(attestation, 0)
        .map_err(AccessError::Internal)
}
