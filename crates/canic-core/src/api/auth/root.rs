//! Module: api::auth::root
//!
//! Responsibility: adapt root-only issuer policy, renewal, and chain-key proof calls.
//! Does not own: root timer execution, batch signing, or proof install state.
//! Boundary: verifies root context and delegates to auth workflow.

use super::AuthApi;
use crate::{
    cdk::types::Principal,
    dto::{
        auth::{
            RootChainKeyPublicKeyRequest, RootDelegationProofBatchProof,
            RootIssuerConfigureRequest, RootIssuerConfigureResponse,
            RootIssuerRenewalStatusRequest, RootIssuerRenewalStatusResponse,
        },
        error::Error,
    },
    ops::{ic::IcOps, runtime::env::EnvOps},
    workflow::runtime::auth::RuntimeAuthWorkflow,
};

impl AuthApi {
    /// Derive the public key for this Root without exposing signing authority.
    pub async fn root_chain_key_public_key(
        request: RootChainKeyPublicKeyRequest,
    ) -> Result<Vec<u8>, Error> {
        EnvOps::require_root().map_err(Error::from)?;
        RuntimeAuthWorkflow::root_chain_key_public_key(request)
            .await
            .map_err(Error::from)
    }

    /// Configure issuer authority and automatic renewal together.
    ///
    /// Fresh issuers fetch their proof during token preparation. Identical setup
    /// retries preserve the registry epoch and existing proof authority.
    pub fn configure_issuer_root(
        request: RootIssuerConfigureRequest,
    ) -> Result<RootIssuerConfigureResponse, Error> {
        EnvOps::require_root().map_err(Error::from)?;
        RuntimeAuthWorkflow::configure_root_issuer(request).map_err(Self::map_auth_error)
    }

    /// Report root-managed renewal template/state for one issuer.
    pub fn root_issuer_renewal_status_root(
        request: RootIssuerRenewalStatusRequest,
    ) -> Result<RootIssuerRenewalStatusResponse, Error> {
        EnvOps::require_root().map_err(Error::from)?;
        Ok(RuntimeAuthWorkflow::root_issuer_renewal_status(request))
    }

    /// Return or create a chain-key root delegation proof for the registered issuer caller.
    pub async fn get_or_create_chain_key_delegation_proof_root()
    -> Result<RootDelegationProofBatchProof, Error> {
        EnvOps::require_root().map_err(Error::from)?;
        RuntimeAuthWorkflow::get_or_create_chain_key_delegation_proof_for_issuer_root(
            IcOps::msg_caller(),
        )
        .await
        .map_err(Self::map_auth_error)
    }

    /// Create or reuse and install a chain-key delegation proof for one issuer.
    ///
    /// Root applications may call this after installing or reinstalling an
    /// issuer so delegated-token issuance is ready before the first login.
    pub async fn provision_chain_key_delegation_proof_for_issuer_root(
        issuer_pid: Principal,
    ) -> Result<(), Error> {
        EnvOps::require_root().map_err(Error::from)?;
        RuntimeAuthWorkflow::provision_chain_key_delegation_proof_for_issuer_root(issuer_pid)
            .await
            .map_err(Self::map_auth_error)
    }
}
