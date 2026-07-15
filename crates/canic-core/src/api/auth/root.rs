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
            RootDelegationProofBatchProof, RootIssuerPolicyResponse, RootIssuerPolicyUpsertRequest,
            RootIssuerRenewalStatusRequest, RootIssuerRenewalStatusResponse,
            RootIssuerRenewalTemplateResponse, RootIssuerRenewalTemplateUpsertRequest,
        },
        error::Error,
    },
    ops::{ic::IcOps, runtime::env::EnvOps},
    workflow::runtime::auth::RuntimeAuthWorkflow,
};

impl AuthApi {
    /// Upsert root issuer policy from the local root controller path.
    pub fn upsert_root_issuer_policy_root(
        request: RootIssuerPolicyUpsertRequest,
    ) -> Result<RootIssuerPolicyResponse, Error> {
        EnvOps::require_root().map_err(Error::from)?;
        RuntimeAuthWorkflow::upsert_root_issuer_policy(request).map_err(Self::map_auth_error)
    }

    /// Upsert root-managed renewal template from the local root controller path.
    pub fn upsert_root_issuer_renewal_template_root(
        request: RootIssuerRenewalTemplateUpsertRequest,
    ) -> Result<RootIssuerRenewalTemplateResponse, Error> {
        EnvOps::require_root().map_err(Error::from)?;
        RuntimeAuthWorkflow::upsert_root_issuer_renewal_template(request)
            .map_err(Self::map_auth_error)
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
