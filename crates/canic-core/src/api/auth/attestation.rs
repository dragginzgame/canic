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
    ///
    /// `min_accepted_epoch` can tighten the configured per-role epoch floor;
    /// the effective floor is the greater of the application and configured values.
    ///
    /// After verification, the receiving application must compare
    /// `attestation.payload.role` with its allowed roles before performing effects.
    /// Successful proof verification establishes bindings and freshness; the
    /// receiver owns authorization for the attested role.
    pub fn verify_role_attestation(
        attestation: &SignedRoleAttestation,
        min_accepted_epoch: u64,
    ) -> Result<(), Error> {
        RuntimeAuthWorkflow::verify_role_attestation(attestation, min_accepted_epoch)
            .map_err(Self::map_auth_error)
    }

    /// Verify a role attestation that explicitly binds the caller to this live Subnet.
    ///
    /// `min_accepted_epoch` can tighten the configured per-role epoch floor;
    /// the effective floor is the greater of the application and configured values.
    ///
    /// The receiving application must also authorize `attestation.payload.role`
    /// against its allowed roles before performing effects. A valid caller and
    /// live-Subnet binding does not select the roles permitted by the application.
    pub fn verify_local_subnet_role_attestation(
        attestation: &SignedRoleAttestation,
        min_accepted_epoch: u64,
    ) -> Result<(), Error> {
        RuntimeAuthWorkflow::verify_local_subnet_role_attestation(attestation, min_accepted_epoch)
            .map_err(Self::map_auth_error)
    }
}
