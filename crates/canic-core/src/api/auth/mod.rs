use crate::{
    cdk::types::Principal,
    dto::{
        auth::{
            AttestationKeySet, DelegatedToken, DelegatedTokenIssueRequest,
            DelegatedTokenMintRequest, DelegationProof, DelegationProofIssueRequest,
            InternalInvocationProofRequest, RoleAttestationRequest,
            SignedInternalInvocationProofV1, SignedRoleAttestation,
        },
        error::{Error, ErrorCode},
        rpc::{Request as RootRequest, Response as RootCapabilityResponse},
    },
    error::InternalErrorClass,
    ids::CanisterRole,
    log,
    log::Topic,
    ops::{
        auth::{
            AuthExpiryError, AuthOps, AuthOpsError, AuthValidationError, SignDelegatedTokenInput,
            SignDelegationProofInput, VerifyDelegatedTokenRuntimeInput,
        },
        config::ConfigOps,
        ic::IcOps,
        runtime::env::EnvOps,
        runtime::metrics::auth::record_attestation_refresh_failed,
    },
    workflow::rpc::request::handler::RootResponseWorkflow,
};
use root_client::RootAuthMaterialClient;

// Internal auth pipeline:
// - `session` owns delegated-session ingress and replay/session state handling.
// - `metadata` owns root request metadata construction.
// - `verify_flow` owns verifier-side attestation refresh behavior.
mod metadata;
mod root_client;
mod session;
mod verify_flow;

///
/// AuthApi
///
/// Owns delegated-token helpers and root-signed role-attestation helpers.
///

pub struct AuthApi;

impl AuthApi {
    const DELEGATED_TOKENS_DISABLED: &str =
        "delegated token auth disabled; set auth.delegated_tokens.enabled=true in canic.toml";
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

    fn map_internal_invocation_verify_error(err: AuthOpsError) -> Error {
        match err {
            AuthOpsError::Validation(AuthValidationError::AttestationUnknownKeyId { .. }) => {
                Error::new(ErrorCode::AuthKeyUnknown, err.to_string())
            }
            AuthOpsError::Expiry(AuthExpiryError::AttestationEpochRejected { .. }) => {
                Error::new(ErrorCode::AuthMaterialStale, err.to_string())
            }
            AuthOpsError::Expiry(
                AuthExpiryError::AttestationExpired { .. }
                | AuthExpiryError::AttestationNotYetValid { .. },
            ) => Error::new(ErrorCode::AuthProofExpired, err.to_string()),
            _ => Error::unauthorized(err.to_string()),
        }
    }

    // Verify delegated-token material and return the token subject.
    //
    // This is intentionally private: endpoint authorization must also bind the
    // verified subject to the caller and consume update tokens once.
    fn verify_token_material(
        token: &DelegatedToken,
        max_cert_ttl_secs: u64,
        max_token_ttl_secs: u64,
        required_scopes: &[String],
        now_secs: u64,
    ) -> Result<Principal, Error> {
        AuthOps::verify_token(VerifyDelegatedTokenRuntimeInput {
            token,
            max_cert_ttl_secs,
            max_token_ttl_secs,
            required_scopes,
            now_secs,
        })
        .map(|verified| verified.subject)
        .map_err(Self::map_auth_error)
    }

    /// Resolve the local shard public key in SEC1 encoding.
    pub async fn local_shard_public_key_sec1() -> Result<Vec<u8>, Error> {
        AuthOps::local_shard_public_key_sec1(IcOps::canister_self())
            .await
            .map_err(Self::map_auth_error)
    }

    /// Issue a delegated token from an explicit self-contained proof.
    pub async fn issue_token(request: DelegatedTokenIssueRequest) -> Result<DelegatedToken, Error> {
        AuthOps::sign_token(SignDelegatedTokenInput {
            proof: request.proof,
            subject: request.subject,
            audience: request.aud,
            scopes: request.scopes,
            ttl_secs: request.ttl_secs,
            nonce: request.nonce,
        })
        .await
        .map_err(Self::map_auth_error)
    }

    /// Request a root proof, then issue a self-contained delegated token.
    pub async fn mint_token(request: DelegatedTokenMintRequest) -> Result<DelegatedToken, Error> {
        let proof = Self::request_delegation(DelegationProofIssueRequest {
            shard_pid: IcOps::canister_self(),
            scopes: request.scopes.clone(),
            aud: request.aud.clone(),
            cert_ttl_secs: request.cert_ttl_secs,
        })
        .await?;

        Self::issue_token(DelegatedTokenIssueRequest {
            proof,
            subject: request.subject,
            aud: request.aud,
            scopes: request.scopes,
            ttl_secs: request.token_ttl_secs,
            nonce: request.nonce,
        })
        .await
    }

    /// Request a self-contained delegation proof from root over RPC.
    pub async fn request_delegation(
        request: DelegationProofIssueRequest,
    ) -> Result<DelegationProof, Error> {
        Self::request_delegation_remote(request).await
    }

    /// Issue a self-contained delegation proof from the local root.
    pub async fn issue_delegation_proof(
        request: DelegationProofIssueRequest,
    ) -> Result<DelegationProof, Error> {
        EnvOps::require_root().map_err(Error::from)?;
        let max_cert_ttl_secs = Self::delegated_token_max_ttl_secs()?;
        let max_token_ttl_secs = request.cert_ttl_secs.min(max_cert_ttl_secs);
        AuthOps::sign_delegation_proof(SignDelegationProofInput {
            audience: request.aud,
            scopes: request.scopes,
            shard_pid: request.shard_pid,
            cert_ttl_secs: request.cert_ttl_secs,
            max_token_ttl_secs,
            max_cert_ttl_secs,
            issued_at: IcOps::now_secs(),
        })
        .await
        .map_err(Self::map_auth_error)
    }

    /// Request a signed role attestation from root over RPC.
    pub async fn request_role_attestation(
        request: RoleAttestationRequest,
    ) -> Result<SignedRoleAttestation, Error> {
        let request = metadata::with_root_attestation_request_metadata(request);
        Self::request_role_attestation_remote(request).await
    }

    /// Request a method-scoped internal invocation proof from root over RPC.
    pub async fn request_internal_invocation_proof(
        request: InternalInvocationProofRequest,
    ) -> Result<SignedInternalInvocationProofV1, Error> {
        let request = metadata::with_internal_invocation_proof_request_metadata(request);
        Self::request_internal_invocation_proof_remote(request).await
    }

    /// Return the current root role-attestation key set.
    pub async fn attestation_key_set() -> Result<AttestationKeySet, Error> {
        AuthOps::attestation_key_set()
            .await
            .map_err(Self::map_auth_error)
    }

    /// Publish root auth material into subnet state and warm root-owned keys once.
    pub async fn publish_root_auth_material() -> Result<(), Error> {
        EnvOps::require_root().map_err(Error::from)?;
        AuthOps::publish_root_auth_material().await.map_err(|err| {
            log!(
                Topic::Auth,
                Warn,
                "root auth material publish failed: {err}"
            );
            Self::map_auth_error(err)
        })
    }

    /// Replace the verifier-local role-attestation key set.
    pub fn replace_attestation_key_set(key_set: AttestationKeySet) {
        AuthOps::replace_attestation_key_set(key_set);
    }

    /// Verify a role attestation, refreshing root keys once on unknown key.
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

    /// Verify a root-signed, method-scoped internal invocation proof for this endpoint.
    pub async fn verify_internal_invocation_proof(
        proof: &SignedInternalInvocationProofV1,
        target_method: &str,
        accepted_roles: &[CanisterRole],
    ) -> Result<(), Error> {
        let configured_min_accepted_epoch = ConfigOps::role_attestation_config()
            .map_err(Error::from)?
            .min_accepted_epoch_by_role
            .get(proof.payload.role.as_str())
            .copied();
        let min_accepted_epoch =
            verify_flow::resolve_min_accepted_epoch(0, configured_min_accepted_epoch);

        let caller = IcOps::msg_caller();
        let self_pid = IcOps::canister_self();
        let now_secs = IcOps::now_secs();
        let verifier_subnet = Some(EnvOps::subnet_pid().map_err(Error::from)?);
        let root_pid = EnvOps::root_pid().map_err(Error::from)?;

        let verify = || {
            AuthOps::verify_internal_invocation_proof_cached(
                proof,
                crate::ops::auth::InternalInvocationProofVerificationInput {
                    caller,
                    self_pid,
                    target_method,
                    accepted_roles,
                    verifier_subnet,
                    now_secs,
                    min_accepted_epoch,
                },
            )
            .map(|_| ())
        };
        let refresh = || async {
            let key_set = RootAuthMaterialClient::new(root_pid)
                .attestation_key_set()
                .await?;
            AuthOps::replace_attestation_key_set(key_set);
            Ok(())
        };

        match verify_flow::verify_role_attestation_with_single_refresh(verify, refresh).await {
            Ok(()) => Ok(()),
            Err(verify_flow::RoleAttestationVerifyFlowError::Initial(err)) => {
                verify_flow::record_attestation_verifier_rejection(&err);
                log!(
                    Topic::Auth,
                    Warn,
                    "internal invocation proof rejected phase=cached local={} caller={} subject={} role={} key_id={} audience={} method={} epoch={} error={}",
                    self_pid,
                    caller,
                    proof.payload.subject,
                    proof.payload.role,
                    proof.key_id,
                    proof.payload.audience,
                    proof.payload.audience_method,
                    proof.payload.epoch,
                    err
                );
                Err(Self::map_internal_invocation_verify_error(err))
            }
            Err(verify_flow::RoleAttestationVerifyFlowError::Refresh { trigger, source }) => {
                verify_flow::record_attestation_verifier_rejection(&trigger);
                record_attestation_refresh_failed();
                log!(
                    Topic::Auth,
                    Warn,
                    "internal invocation proof refresh failed local={} caller={} key_id={} error={}",
                    self_pid,
                    caller,
                    proof.key_id,
                    source
                );
                Err(Self::map_auth_error(source))
            }
            Err(verify_flow::RoleAttestationVerifyFlowError::PostRefresh(err)) => {
                verify_flow::record_attestation_verifier_rejection(&err);
                log!(
                    Topic::Auth,
                    Warn,
                    "internal invocation proof rejected phase=post_refresh local={} caller={} subject={} role={} key_id={} audience={} method={} epoch={} error={}",
                    self_pid,
                    caller,
                    proof.payload.subject,
                    proof.payload.role,
                    proof.key_id,
                    proof.payload.audience,
                    proof.payload.audience_method,
                    proof.payload.epoch,
                    err
                );
                Err(Self::map_internal_invocation_verify_error(err))
            }
        }
    }

    // Resolve the root-owned TTL ceiling from delegated-token config.
    fn delegated_token_max_ttl_secs() -> Result<u64, Error> {
        let cfg = ConfigOps::delegated_tokens_config().map_err(Error::from)?;
        if !cfg.enabled {
            return Err(Error::forbidden(Self::DELEGATED_TOKENS_DISABLED));
        }

        Ok(cfg
            .max_ttl_secs
            .unwrap_or(Self::MAX_DELEGATED_SESSION_TTL_SECS))
    }
}

impl AuthApi {
    // Route a self-contained delegation proof request over RPC to root.
    async fn request_delegation_remote(
        request: DelegationProofIssueRequest,
    ) -> Result<DelegationProof, Error> {
        let root_pid = EnvOps::root_pid().map_err(Error::from)?;
        RootAuthMaterialClient::new(root_pid)
            .request_delegation(request)
            .await
            .map_err(Self::map_auth_error)
    }

    // Execute one local root role-attestation request.
    pub async fn request_role_attestation_root(
        request: RoleAttestationRequest,
    ) -> Result<SignedRoleAttestation, Error> {
        let request = metadata::with_root_attestation_request_metadata(request);
        let response = RootResponseWorkflow::response(RootRequest::issue_role_attestation(request))
            .await
            .map_err(Self::map_auth_error)?;

        match response {
            RootCapabilityResponse::RoleAttestationIssued(response) => Ok(response),
            _ => Err(Error::internal(
                "invalid root response type for role attestation request",
            )),
        }
    }

    // Execute one local root internal-invocation proof request.
    pub async fn request_internal_invocation_proof_root(
        request: InternalInvocationProofRequest,
    ) -> Result<SignedInternalInvocationProofV1, Error> {
        let request = metadata::with_internal_invocation_proof_request_metadata(request);
        let response =
            RootResponseWorkflow::response(RootRequest::issue_internal_invocation_proof(request))
                .await
                .map_err(Self::map_auth_error)?;

        match response {
            RootCapabilityResponse::InternalInvocationProofIssued(response) => Ok(response),
            _ => Err(Error::internal(
                "invalid root response type for internal invocation proof request",
            )),
        }
    }

    // Route a canonical role-attestation request over RPC to root.
    async fn request_role_attestation_remote(
        request: RoleAttestationRequest,
    ) -> Result<SignedRoleAttestation, Error> {
        let root_pid = EnvOps::root_pid().map_err(Error::from)?;
        RootAuthMaterialClient::new(root_pid)
            .request_role_attestation(request)
            .await
            .map_err(Self::map_auth_error)
    }

    // Route a canonical internal-invocation proof request over RPC to root.
    async fn request_internal_invocation_proof_remote(
        request: InternalInvocationProofRequest,
    ) -> Result<SignedInternalInvocationProofV1, Error> {
        let root_pid = EnvOps::root_pid().map_err(Error::from)?;
        RootAuthMaterialClient::new(root_pid)
            .request_internal_invocation_proof(request)
            .await
            .map_err(Self::map_auth_error)
    }
}

#[cfg(test)]
mod tests {
    use super::AuthApi;
    use crate::{
        dto::error::ErrorCode,
        ops::auth::{AuthExpiryError, AuthOpsError},
    };

    #[test]
    fn internal_invocation_not_yet_valid_maps_to_non_retryable_proof_expiry() {
        let err = AuthApi::map_internal_invocation_verify_error(AuthOpsError::Expiry(
            AuthExpiryError::AttestationNotYetValid {
                issued_at: 20,
                now_secs: 10,
            },
        ));

        assert_eq!(err.code, ErrorCode::AuthProofExpired);
    }
}
