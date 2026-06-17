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
            DelegatedTokenPrepareRequest, DelegatedTokenPrepareResponse, DelegationProof,
            DelegationProofGetRequest, DelegationProofIssueRequest, DelegationProofPrepareResponse,
            InstallActiveDelegationProofRequest, InstallActiveDelegationProofResponse,
            RoleAttestationGetRequest, RoleAttestationPrepareResponse, RoleAttestationRequest,
            RootDelegationProofBatchGetRequest, RootDelegationProofBatchGetResponse,
            RootDelegationProofBatchInstallRequest, RootDelegationProofBatchInstallResponse,
            RootDelegationProofBatchPrepareRequest, RootDelegationProofBatchPrepareResponse,
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
    const ROOT_DELEGATION_PROOF_SELF_PROVISIONING_DISABLED: &str = "issuer-initiated root delegation proof provisioning is unsupported; use root hard-cut provisioning";
    const ROOT_DELEGATION_PROOF_BATCH_PROVISIONING_UNAVAILABLE: &str =
        "root delegation proof batch provisioning is not implemented yet";
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

    /// Reject the issuer-initiated root delegation proof provisioning path.
    pub fn prepare_delegation_proof(
        _request: DelegationProofIssueRequest,
    ) -> Result<DelegationProofPrepareResponse, Error> {
        Err(Error::forbidden(
            Self::ROOT_DELEGATION_PROOF_SELF_PROVISIONING_DISABLED,
        ))
    }

    /// Prepare a root-certified delegation proof from the local root update path.
    pub fn prepare_delegation_proof_root(
        request: DelegationProofIssueRequest,
    ) -> Result<DelegationProofPrepareResponse, Error> {
        RuntimeAuthWorkflow::prepare_delegation_proof_root(request).map_err(Self::map_auth_error)
    }

    /// Retrieve a prepared self-contained delegation proof from the local root query path.
    pub fn get_delegation_proof_root(
        request: DelegationProofGetRequest,
    ) -> Result<DelegationProof, Error> {
        EnvOps::require_root().map_err(Error::from)?;
        let caller = IcOps::msg_caller();
        AuthOps::get_delegation_proof(caller, request.cert_hash).map_err(Self::map_auth_error)
    }

    /// Prepare root delegation proof batch metadata from the local root update path.
    pub fn prepare_delegation_proof_batch_root(
        request: RootDelegationProofBatchPrepareRequest,
    ) -> Result<RootDelegationProofBatchPrepareResponse, Error> {
        EnvOps::require_root().map_err(Error::from)?;
        let max_cert_ttl_ns = Self::delegated_token_max_ttl_ns()?;
        AuthOps::preflight_delegation_proof_batch_prepare_request(
            &request,
            max_cert_ttl_ns,
            IcOps::now_nanos(),
        )
        .map_err(Self::map_auth_error)?;
        Err(Error::unavailable(
            Self::ROOT_DELEGATION_PROOF_BATCH_PROVISIONING_UNAVAILABLE,
        ))
    }

    /// Retrieve root delegation proofs from the local direct root query path.
    pub fn get_delegation_proof_batch_root(
        _request: RootDelegationProofBatchGetRequest,
    ) -> Result<RootDelegationProofBatchGetResponse, Error> {
        EnvOps::require_root().map_err(Error::from)?;
        Err(Error::unavailable(
            Self::ROOT_DELEGATION_PROOF_BATCH_PROVISIONING_UNAVAILABLE,
        ))
    }

    /// Install retrieved root delegation proof batches from the local root update path.
    pub fn install_delegation_proof_batch_root(
        _request: RootDelegationProofBatchInstallRequest,
    ) -> Result<RootDelegationProofBatchInstallResponse, Error> {
        EnvOps::require_root().map_err(Error::from)?;
        Err(Error::unavailable(
            Self::ROOT_DELEGATION_PROOF_BATCH_PROVISIONING_UNAVAILABLE,
        ))
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

#[cfg(test)]
mod tests {
    use super::AuthApi;
    use crate::{
        cdk::types::Principal,
        dto::{
            auth::{DelegatedRoleGrant, DelegationAudience, DelegationProofIssueRequest},
            error::ErrorCode,
        },
        ids::CanisterRole,
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn delegation_request() -> DelegationProofIssueRequest {
        DelegationProofIssueRequest {
            metadata: None,
            issuer_pid: p(2),
            aud: DelegationAudience::Project("test".to_string()),
            grants: vec![DelegatedRoleGrant {
                target: CanisterRole::owned("user_shard".to_string()),
                scopes: vec!["canic.issue".to_string()],
            }],
            cert_ttl_ns: 60_000_000_000,
        }
    }

    #[test]
    fn issuer_root_delegation_prepare_is_hard_cut() {
        let err = AuthApi::prepare_delegation_proof(delegation_request())
            .expect_err("issuer-initiated root proof prepare must be hard-cut");

        assert_eq!(err.code, ErrorCode::Forbidden);
        assert!(
            err.message.contains("root hard-cut provisioning"),
            "unexpected error message: {}",
            err.message
        );
    }
}
