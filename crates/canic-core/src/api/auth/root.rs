//! Module: api::auth::root
//!
//! Responsibility: adapt root-only issuer policy, renewal, and chain-key proof calls.
//! Does not own: root timer execution, batch signing, or proof install state.
//! Boundary: verifies root context and delegates to auth ops/workflow.

use super::AuthApi;
use crate::{
    dto::{
        auth::{
            RootDelegationProofBatchProof, RootIssuerPolicyResponse, RootIssuerPolicyUpsertRequest,
            RootIssuerRenewalStatusRequest, RootIssuerRenewalStatusResponse,
            RootIssuerRenewalTemplateResponse, RootIssuerRenewalTemplateUpsertRequest,
        },
        error::Error,
    },
    ops::{auth::AuthOps, ic::IcOps, runtime::env::EnvOps},
    workflow::runtime::auth::RuntimeAuthWorkflow,
};

impl AuthApi {
    /// Upsert root issuer policy from the local root controller path.
    pub fn upsert_root_issuer_policy_root(
        request: RootIssuerPolicyUpsertRequest,
    ) -> Result<RootIssuerPolicyResponse, Error> {
        EnvOps::require_root().map_err(Error::from)?;
        AuthOps::upsert_root_issuer_policy(request, IcOps::now_nanos())
            .map_err(Self::map_auth_error)
    }

    /// Upsert root-managed renewal template from the local root controller path.
    pub fn upsert_root_issuer_renewal_template_root(
        request: RootIssuerRenewalTemplateUpsertRequest,
    ) -> Result<RootIssuerRenewalTemplateResponse, Error> {
        EnvOps::require_root().map_err(Error::from)?;
        let response = AuthOps::upsert_root_issuer_renewal_template(request, IcOps::now_nanos())
            .map_err(Self::map_auth_error)?;
        if response.template.enabled {
            RuntimeAuthWorkflow::start_root_delegation_renewal_timer_soon_if_configured()
                .map_err(Self::map_auth_error)?;
        }
        Ok(response)
    }

    /// Report root-managed renewal template/state for one issuer.
    pub fn root_issuer_renewal_status_root(
        request: RootIssuerRenewalStatusRequest,
    ) -> Result<RootIssuerRenewalStatusResponse, Error> {
        EnvOps::require_root().map_err(Error::from)?;
        Ok(AuthOps::root_issuer_renewal_status(request))
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
}
