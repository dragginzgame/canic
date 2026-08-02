//! Module: api::auth::attestation
//!
//! Responsibility: adapt role-attestation endpoint calls.
//! Does not own: role-attestation signing, cache state, or verifier internals.
//! Boundary: endpoint adapters delegate role-attestation work to workflow.

use super::AuthApi;
use crate::{
    dto::{
        auth::{
            RoleAttestationGetRequest, RoleAttestationPrepareResponse, RoleAttestationRequest,
            SignedRoleAttestation,
        },
        error::Error,
    },
    workflow::runtime::auth::RuntimeAuthWorkflow,
};

impl AuthApi {
    /// Prepare a root-certified role attestation from the local root update path.
    pub fn prepare_component_role_attestation_root(
        request: RoleAttestationRequest,
        member: &crate::ids::ManagedCanisterBinding,
    ) -> Result<RoleAttestationPrepareResponse, Error> {
        RuntimeAuthWorkflow::prepare_component_role_attestation_root(request, member)
            .map_err(Self::map_auth_error)
    }

    /// Retrieve a prepared role attestation with its root canister-signature proof.
    pub fn get_role_attestation_root(
        request: RoleAttestationGetRequest,
    ) -> Result<SignedRoleAttestation, Error> {
        RuntimeAuthWorkflow::get_role_attestation_root(request).map_err(Self::map_auth_error)
    }

    /// Verify a role attestation locally from its embedded root proof.
    pub async fn verify_role_attestation(
        attestation: &SignedRoleAttestation,
        min_accepted_epoch: u64,
    ) -> Result<(), Error> {
        RuntimeAuthWorkflow::verify_role_attestation(attestation, min_accepted_epoch)
            .await
            .map_err(Self::map_auth_error)
    }

    /// Verify a role attestation that explicitly binds the caller to this live Subnet.
    pub async fn verify_local_subnet_role_attestation(
        attestation: &SignedRoleAttestation,
        min_accepted_epoch: u64,
    ) -> Result<(), Error> {
        RuntimeAuthWorkflow::verify_local_subnet_role_attestation(attestation, min_accepted_epoch)
            .await
            .map_err(Self::map_auth_error)
    }
}
