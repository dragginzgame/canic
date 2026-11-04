//! Module: api::auth::issuer_proof
//!
//! Responsibility: adapt issuer-local Root delegation proof installation and reporting.
//! Does not own: endpoint authorization, proof verification internals, or stable records.
//! Boundary: checks issuer feature/config gates and delegates to auth ops/workflow.

use super::AuthApi;
use crate::{
    dto::{
        auth::{
            ActiveDelegationProofStatusResponse, InstallActiveDelegationProofRequest,
            InstallActiveDelegationProofResponse,
        },
        error::Error,
    },
    ops::{auth::AuthOps, ic::IcOps},
};

impl AuthApi {
    /// Install validated root-certified delegation material for issuer-local signature proof support.
    pub fn install_active_delegation_proof(
        request: InstallActiveDelegationProofRequest,
    ) -> Result<InstallActiveDelegationProofResponse, Error> {
        Self::require_issuer_proof_support()?;

        let active_proof =
            AuthOps::install_active_delegation_proof(request.proof, IcOps::msg_caller())
                .map_err(Self::map_auth_error)?;

        Ok(InstallActiveDelegationProofResponse { active_proof })
    }

    /// Report non-secret issuer-local active proof lifecycle status for operators.
    pub fn active_delegation_proof_status() -> Result<ActiveDelegationProofStatusResponse, Error> {
        Self::require_issuer_proof_support()?;
        AuthOps::active_delegation_proof_status(IcOps::now_nanos()).map_err(Self::map_auth_error)
    }
}
