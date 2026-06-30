//! Module: api::auth::token
//!
//! Responsibility: adapt issuer-local delegated-token endpoint calls.
//! Does not own: endpoint authorization, token verification internals, or stable records.
//! Boundary: checks issuer feature/config gates and delegates to auth ops/workflow.

use super::AuthApi;
use crate::{
    dto::{
        auth::{
            ActiveDelegationProofStatusResponse, DelegatedToken, DelegatedTokenGetRequest,
            DelegatedTokenPrepareRequest, DelegatedTokenPrepareResponse,
            InstallActiveDelegationProofRequest, InstallActiveDelegationProofResponse,
        },
        error::Error,
    },
    ops::{auth::AuthOps, ic::IcOps},
    workflow::runtime::auth::RuntimeAuthWorkflow,
};

impl AuthApi {
    /// Prepare a delegated token from the issuer-local active delegation proof.
    pub async fn prepare_delegated_token(
        request: DelegatedTokenPrepareRequest,
    ) -> Result<DelegatedTokenPrepareResponse, Error> {
        Self::require_delegated_token_issuer_enabled()?;
        RuntimeAuthWorkflow::prepare_delegated_token(request)
            .await
            .map_err(Self::map_auth_error)
    }

    /// Retrieve a prepared delegated token with its issuer canister-signature proof.
    pub fn get_delegated_token(request: DelegatedTokenGetRequest) -> Result<DelegatedToken, Error> {
        Self::require_delegated_token_issuer_enabled()?;

        AuthOps::get_delegated_token_issuer_proof(request.claims_hash, IcOps::msg_caller())
            .map_err(Self::map_auth_error)
    }

    /// Install validated root-certified delegation material for issuer-local token issuance.
    pub fn install_active_delegation_proof(
        request: InstallActiveDelegationProofRequest,
    ) -> Result<InstallActiveDelegationProofResponse, Error> {
        Self::require_delegated_token_issuer_enabled()?;

        let active_proof =
            AuthOps::install_active_delegation_proof(request.proof, IcOps::msg_caller())
                .map_err(Self::map_auth_error)?;

        Ok(InstallActiveDelegationProofResponse { active_proof })
    }

    /// Report non-secret issuer-local active proof lifecycle status for operators.
    pub fn active_delegation_proof_status() -> Result<ActiveDelegationProofStatusResponse, Error> {
        Self::require_delegated_token_issuer_enabled()?;
        Ok(AuthOps::active_delegation_proof_status(IcOps::now_nanos()))
    }
}
