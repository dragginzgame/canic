//! Module: api::auth
//!
//! Responsibility: expose auth endpoint helpers and auth boundary adapters.
//! Does not own: stable auth records, proof verification internals, or runtime policy.
//! Boundary: endpoint layer maps public DTOs into ops/workflow auth calls.

use crate::{
    cdk::types::Principal,
    dto::{
        auth::{
            ActiveDelegationProofStatusResponse, DelegatedToken, DelegatedTokenGetRequest,
            DelegatedTokenPrepareRequest, DelegatedTokenPrepareResponse,
            InstallActiveDelegationProofRequest, InstallActiveDelegationProofResponse,
            RoleAttestationGetRequest, RoleAttestationPrepareResponse, RoleAttestationRequest,
            RootDelegationProofBatchProof, RootIssuerPolicyResponse, RootIssuerPolicyUpsertRequest,
            RootIssuerRenewalStatusRequest, RootIssuerRenewalStatusResponse,
            RootIssuerRenewalTemplateResponse, RootIssuerRenewalTemplateUpsertRequest,
            SignedRoleAttestation,
        },
        error::Error,
    },
    error::InternalErrorClass,
    ops::{
        auth::{AuthOps, VerifyDelegatedTokenRuntimeInput},
        config::ConfigOps,
        ic::IcOps,
        runtime::env::EnvOps,
    },
    workflow::runtime::auth::RuntimeAuthWorkflow,
};

// Internal auth pipeline:
// - `session` owns delegated-session ingress and replay/session state handling.
mod session;

///
/// AuthApi
///
/// Owns delegated-token helpers and root-signed role-attestation helpers.
/// Owned by the API layer and called by generated endpoint wrappers.
///

pub struct AuthApi;

impl AuthApi {
    const DELEGATED_TOKENS_DISABLED: &str =
        "delegated token auth disabled; set auth.delegated_tokens.enabled=true in canic.toml";
    const DELEGATED_TOKEN_ISSUER_DISABLED: &str = "delegated token issuer disabled for this canister; set subnets.<subnet>.canisters.<role>.auth.delegated_token_issuer=true in canic.toml";
    const MAX_DELEGATED_SESSION_TTL_SECS: u64 = 24 * 60 * 60;
    const SESSION_BOOTSTRAP_TOKEN_FINGERPRINT_DOMAIN: &[u8] =
        b"canic-session-bootstrap-token-fingerprint";

    // Map internal auth failures onto public endpoint errors.
    fn map_auth_error(err: crate::InternalError) -> Error {
        match err.class() {
            InternalErrorClass::Infra | InternalErrorClass::Ops | InternalErrorClass::Workflow => {
                Error::internal(err.to_string())
            }
            _ => Error::from(err),
        }
    }

    fn require_delegated_token_issuer_enabled() -> Result<(), Error> {
        let delegated_tokens_cfg =
            ConfigOps::delegated_tokens_config().map_err(Self::map_auth_error)?;
        if !delegated_tokens_cfg.enabled {
            return Err(Error::invalid(Self::DELEGATED_TOKENS_DISABLED));
        }

        let canister_cfg = ConfigOps::current_canister().map_err(Self::map_auth_error)?;
        if !canister_cfg.auth.delegated_token_issuer {
            return Err(Error::forbidden(Self::DELEGATED_TOKEN_ISSUER_DISABLED));
        }

        Ok(())
    }

    // Verify delegated-token material and return the token subject.
    //
    // This is intentionally private: endpoint authorization must also bind the
    // verified subject to the caller before dispatch.
    fn verify_token_material(
        token: &DelegatedToken,
        max_cert_ttl_ns: u64,
        max_token_ttl_ns: u64,
        required_scopes: &[String],
        now_ns: u64,
    ) -> Result<Principal, Error> {
        AuthOps::verify_token(VerifyDelegatedTokenRuntimeInput {
            token,
            caller: IcOps::msg_caller(),
            max_cert_ttl_ns,
            max_token_ttl_ns,
            required_scopes,
            now_ns,
        })
        .map(|verified| verified.subject)
        .map_err(Self::map_auth_error)
    }

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

    /// Prepare a root-certified role attestation from the local root update path.
    pub fn prepare_role_attestation_root(
        request: RoleAttestationRequest,
    ) -> Result<RoleAttestationPrepareResponse, Error> {
        RuntimeAuthWorkflow::prepare_role_attestation_root(request).map_err(Self::map_auth_error)
    }

    /// Retrieve a prepared role attestation with its root canister-signature proof.
    pub fn get_role_attestation_root(
        request: RoleAttestationGetRequest,
    ) -> Result<SignedRoleAttestation, Error> {
        EnvOps::require_root().map_err(Error::from)?;
        AuthOps::get_role_attestation(IcOps::msg_caller(), request.payload_hash)
            .map_err(Self::map_auth_error)
    }

    /// Verify a role attestation locally from its embedded root proof.
    pub async fn verify_role_attestation(
        attestation: &SignedRoleAttestation,
        min_accepted_epoch: u64,
    ) -> Result<(), Error> {
        crate::workflow::runtime::auth::RuntimeAuthWorkflow::verify_role_attestation(
            attestation,
            min_accepted_epoch,
        )
        .await
        .map_err(Self::map_auth_error)
    }

    // Resolve the delegated-token TTL ceiling for endpoint auth/session callers.
    fn delegated_token_max_ttl_ns() -> Result<u64, Error> {
        let cfg = ConfigOps::delegated_tokens_config().map_err(Error::from)?;
        if !cfg.enabled {
            return Err(Error::forbidden(Self::DELEGATED_TOKENS_DISABLED));
        }

        let max_ttl_secs = cfg
            .max_ttl_secs
            .unwrap_or(Self::MAX_DELEGATED_SESSION_TTL_SECS);
        max_ttl_secs.checked_mul(1_000_000_000).ok_or_else(|| {
            Error::invalid("auth.delegated_tokens.max_ttl_secs overflows nanoseconds")
        })
    }
}
