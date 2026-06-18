//! Module: api::auth
//!
//! Responsibility: expose auth endpoint helpers and auth boundary adapters.
//! Does not own: stable auth records, proof verification internals, or runtime policy.
//! Boundary: endpoint layer maps public DTOs into ops/workflow auth calls.

use crate::{
    cdk::types::Principal,
    domain::policy::auth::{
        RootDelegatedRoleGrantPolicy, RootDelegationAudiencePolicy, RootIssuerPolicy,
    },
    dto::{
        auth::{
            ActiveDelegationProofStatusResponse, DelegatedRoleGrant, DelegatedToken,
            DelegatedTokenGetRequest, DelegatedTokenPrepareRequest, DelegatedTokenPrepareResponse,
            DelegationAudience, InstallActiveDelegationProofRequest,
            InstallActiveDelegationProofResponse, RoleAttestationGetRequest,
            RoleAttestationPrepareResponse, RoleAttestationRequest,
            RootDelegationProofBatchGetRequest, RootDelegationProofBatchGetResponse,
            RootDelegationProofBatchInstallRequest, RootDelegationProofBatchInstallResponse,
            RootDelegationProofBatchPrepareRequest, RootDelegationProofBatchPrepareResponse,
            RootIssuerPolicyResponse, RootIssuerPolicyUpsertRequest, RootIssuerPolicyView,
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
        storage::auth::AuthStateOps,
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
    pub fn prepare_delegated_token(
        request: DelegatedTokenPrepareRequest,
    ) -> Result<DelegatedTokenPrepareResponse, Error> {
        Self::require_delegated_token_issuer_enabled()?;
        RuntimeAuthWorkflow::prepare_delegated_token(request).map_err(Self::map_auth_error)
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

    /// Report non-secret issuer-local active proof lifecycle status for provisioners.
    pub fn active_delegation_proof_status() -> Result<ActiveDelegationProofStatusResponse, Error> {
        Self::require_delegated_token_issuer_enabled()?;
        Ok(AuthOps::active_delegation_proof_status(IcOps::now_nanos()))
    }

    /// Upsert root issuer policy from the local root controller path.
    pub fn upsert_root_issuer_policy_root(
        request: RootIssuerPolicyUpsertRequest,
    ) -> Result<RootIssuerPolicyResponse, Error> {
        EnvOps::require_root().map_err(Error::from)?;
        validate_root_issuer_policy_upsert_request(&request)?;

        let policy = root_issuer_policy_from_request(request);
        AuthStateOps::upsert_root_issuer_policy(policy.clone());

        Ok(RootIssuerPolicyResponse {
            issuer: root_issuer_policy_view(&policy),
        })
    }

    /// Install root issuer policy in explicit delegation-material test builds.
    #[cfg(canic_test_delegation_material)]
    pub fn test_upsert_root_issuer_policy(
        issuer_pid: Principal,
        allowed_audiences: Vec<DelegationAudience>,
        allowed_grants: Vec<DelegatedRoleGrant>,
        max_cert_ttl_ns: u64,
        refresh_after_ratio_bps: u16,
    ) -> Result<(), Error> {
        Self::upsert_root_issuer_policy_root(RootIssuerPolicyUpsertRequest {
            issuer_pid,
            enabled: true,
            allowed_audiences,
            allowed_grants,
            max_cert_ttl_ns,
            refresh_after_ratio_bps,
        })
        .map(|_| ())
    }

    /// Prepare root delegation proof batch metadata from the local root update path.
    pub fn prepare_delegation_proof_batch_root(
        request: RootDelegationProofBatchPrepareRequest,
    ) -> Result<RootDelegationProofBatchPrepareResponse, Error> {
        EnvOps::require_root().map_err(Error::from)?;
        let max_cert_ttl_ns = Self::delegated_token_max_ttl_ns()?;
        AuthOps::prepare_delegation_proof_batch(request, max_cert_ttl_ns, IcOps::now_nanos())
            .map_err(Self::map_auth_error)
    }

    /// Retrieve root delegation proofs from the local direct root query path.
    pub fn get_delegation_proof_batch_root(
        request: RootDelegationProofBatchGetRequest,
    ) -> Result<RootDelegationProofBatchGetResponse, Error> {
        EnvOps::require_root().map_err(Error::from)?;
        AuthOps::get_delegation_proof_batch(request).map_err(Self::map_auth_error)
    }

    /// Install retrieved root delegation proof batches from the local root update path.
    pub async fn install_delegation_proof_batch_root(
        request: RootDelegationProofBatchInstallRequest,
    ) -> Result<RootDelegationProofBatchInstallResponse, Error> {
        EnvOps::require_root().map_err(Error::from)?;
        RuntimeAuthWorkflow::install_delegation_proof_batch_root(request)
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

fn validate_root_issuer_policy_upsert_request(
    request: &RootIssuerPolicyUpsertRequest,
) -> Result<(), Error> {
    if request.max_cert_ttl_ns == 0 {
        return Err(Error::invalid(
            "root issuer max certificate TTL must be greater than zero",
        ));
    }
    if request.refresh_after_ratio_bps == 0 || request.refresh_after_ratio_bps >= 10_000 {
        return Err(Error::invalid(
            "root issuer refresh ratio must be between 1 and 9999 basis points",
        ));
    }
    if request.enabled && request.allowed_audiences.is_empty() {
        return Err(Error::invalid(
            "enabled root issuer policy must allow at least one audience",
        ));
    }
    if request.enabled && request.allowed_grants.is_empty() {
        return Err(Error::invalid(
            "enabled root issuer policy must allow at least one grant",
        ));
    }
    Ok(())
}

fn root_issuer_policy_from_request(request: RootIssuerPolicyUpsertRequest) -> RootIssuerPolicy {
    RootIssuerPolicy {
        issuer_pid: request.issuer_pid,
        enabled: request.enabled,
        allowed_audiences: request
            .allowed_audiences
            .iter()
            .map(root_delegation_audience_policy)
            .collect(),
        allowed_grants: request
            .allowed_grants
            .iter()
            .map(root_delegated_role_grant_policy)
            .collect(),
        max_cert_ttl_ns: request.max_cert_ttl_ns,
        refresh_after_ratio_bps: request.refresh_after_ratio_bps,
    }
}

fn root_issuer_policy_view(policy: &RootIssuerPolicy) -> RootIssuerPolicyView {
    RootIssuerPolicyView {
        issuer_pid: policy.issuer_pid,
        enabled: policy.enabled,
        allowed_audiences: policy
            .allowed_audiences
            .iter()
            .map(root_delegation_audience_view)
            .collect(),
        allowed_grants: policy
            .allowed_grants
            .iter()
            .map(root_delegated_role_grant_view)
            .collect(),
        max_cert_ttl_ns: policy.max_cert_ttl_ns,
        refresh_after_ratio_bps: policy.refresh_after_ratio_bps,
    }
}

fn root_delegation_audience_policy(audience: &DelegationAudience) -> RootDelegationAudiencePolicy {
    match audience {
        DelegationAudience::Canister(canister) => RootDelegationAudiencePolicy::Canister(*canister),
        DelegationAudience::CanicSubnet(subnet) => {
            RootDelegationAudiencePolicy::CanicSubnet(*subnet)
        }
        DelegationAudience::Project(project) => {
            RootDelegationAudiencePolicy::Project(project.clone())
        }
    }
}

fn root_delegated_role_grant_policy(grant: &DelegatedRoleGrant) -> RootDelegatedRoleGrantPolicy {
    RootDelegatedRoleGrantPolicy {
        target: grant.target.clone(),
        scopes: grant.scopes.clone(),
    }
}

fn root_delegation_audience_view(policy: &RootDelegationAudiencePolicy) -> DelegationAudience {
    match policy {
        RootDelegationAudiencePolicy::Canister(canister) => DelegationAudience::Canister(*canister),
        RootDelegationAudiencePolicy::CanicSubnet(subnet) => {
            DelegationAudience::CanicSubnet(*subnet)
        }
        RootDelegationAudiencePolicy::Project(project) => {
            DelegationAudience::Project(project.clone())
        }
    }
}

fn root_delegated_role_grant_view(policy: &RootDelegatedRoleGrantPolicy) -> DelegatedRoleGrant {
    DelegatedRoleGrant {
        target: policy.target.clone(),
        scopes: policy.scopes.clone(),
    }
}
