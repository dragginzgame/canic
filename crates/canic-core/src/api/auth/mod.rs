use crate::{
    dto::{
        auth::{
            AttestationKeySet, DelegatedToken, DelegatedTokenIssueRequest,
            DelegatedTokenMintRequest, DelegationProof, DelegationProofIssueRequest,
            RoleAttestationRequest, SignedRoleAttestation,
        },
        error::Error,
        rpc::{Request as RootRequest, Response as RootCapabilityResponse},
    },
    error::InternalErrorClass,
    log,
    log::Topic,
    ops::{
        auth::{
            DelegatedTokenOps, SignDelegatedTokenInput, SignDelegationProofInput,
            VerifyDelegatedTokenRuntimeInput,
        },
        config::ConfigOps,
        ic::IcOps,
        rpc::RpcOps,
        runtime::env::EnvOps,
        runtime::metrics::auth::record_attestation_refresh_failed,
    },
    protocol,
    workflow::rpc::request::handler::RootResponseWorkflow,
};

// Internal auth pipeline:
// - `session` owns delegated-session ingress and replay/session state handling.
// - `metadata` owns root request metadata construction.
// - `verify_flow` owns verifier-side attestation refresh behavior.
mod metadata;
mod session;
mod verify_flow;

///
/// DelegationApi
///
/// Requires auth.delegated_tokens.enabled = true in config.
///

pub struct DelegationApi;

impl DelegationApi {
    const DELEGATED_TOKENS_DISABLED: &str =
        "delegated token auth disabled; set auth.delegated_tokens.enabled=true in canic.toml";
    const MAX_DELEGATED_SESSION_TTL_SECS: u64 = 24 * 60 * 60;
    const SESSION_BOOTSTRAP_TOKEN_FINGERPRINT_DOMAIN: &[u8] =
        b"canic-session-bootstrap-token-fingerprint";

    // Map internal auth failures onto public endpoint errors.
    fn map_delegation_error(err: crate::InternalError) -> Error {
        match err.class() {
            InternalErrorClass::Infra | InternalErrorClass::Ops | InternalErrorClass::Workflow => {
                Error::internal(err.to_string())
            }
            _ => Error::from(err),
        }
    }

    /// Resolve the local shard public key in SEC1 encoding.
    pub async fn local_shard_public_key_sec1() -> Result<Vec<u8>, Error> {
        DelegatedTokenOps::local_shard_public_key_sec1(IcOps::canister_self())
            .await
            .map_err(Self::map_delegation_error)
    }

    /// Issue a delegated token from an explicit self-contained proof.
    pub async fn issue_token(request: DelegatedTokenIssueRequest) -> Result<DelegatedToken, Error> {
        DelegatedTokenOps::sign_token(SignDelegatedTokenInput {
            proof: request.proof,
            subject: request.subject,
            audience: request.aud,
            scopes: request.scopes,
            ttl_secs: request.ttl_secs,
            nonce: request.nonce,
        })
        .await
        .map_err(Self::map_delegation_error)
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

    /// Full delegated token verification without verifier-local proof lookup.
    pub fn verify_token(
        token: &DelegatedToken,
        max_cert_ttl_secs: u64,
        max_token_ttl_secs: u64,
        required_scopes: &[String],
        now_secs: u64,
    ) -> Result<(), Error> {
        DelegatedTokenOps::verify_token(VerifyDelegatedTokenRuntimeInput {
            token,
            max_cert_ttl_secs,
            max_token_ttl_secs,
            required_scopes,
            now_secs,
        })
        .map(|_| ())
        .map_err(Self::map_delegation_error)
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
        let max_cert_ttl_secs = Self::delegated_auth_max_ttl_secs()?;
        let max_token_ttl_secs = request.cert_ttl_secs.min(max_cert_ttl_secs);
        DelegatedTokenOps::sign_delegation_proof(SignDelegationProofInput {
            audience: request.aud,
            scopes: request.scopes,
            shard_pid: request.shard_pid,
            cert_ttl_secs: request.cert_ttl_secs,
            max_token_ttl_secs,
            max_cert_ttl_secs,
            issued_at: IcOps::now_secs(),
        })
        .await
        .map_err(Self::map_delegation_error)
    }

    /// Request a signed role attestation from root over RPC.
    pub async fn request_role_attestation(
        request: RoleAttestationRequest,
    ) -> Result<SignedRoleAttestation, Error> {
        let request = metadata::with_root_attestation_request_metadata(request);
        let response = Self::request_role_attestation_remote(request).await?;

        match response {
            RootCapabilityResponse::RoleAttestationIssued(response) => Ok(response),
            _ => Err(Error::internal(
                "invalid root response type for role attestation request",
            )),
        }
    }

    /// Return the current root role-attestation key set.
    pub async fn attestation_key_set() -> Result<AttestationKeySet, Error> {
        DelegatedTokenOps::attestation_key_set()
            .await
            .map_err(Self::map_delegation_error)
    }

    /// Publish root auth material into subnet state and warm root-owned keys once.
    pub async fn publish_root_auth_material() -> Result<(), Error> {
        EnvOps::require_root().map_err(Error::from)?;
        DelegatedTokenOps::publish_root_auth_material()
            .await
            .map_err(|err| {
                log!(
                    Topic::Auth,
                    Warn,
                    "root auth material publish failed: {err}"
                );
                Self::map_delegation_error(err)
            })
    }

    /// Replace the verifier-local role-attestation key set.
    pub fn replace_attestation_key_set(key_set: AttestationKeySet) {
        DelegatedTokenOps::replace_attestation_key_set(key_set);
    }

    /// Verify a role attestation, refreshing root keys once on unknown key.
    pub async fn verify_role_attestation(
        attestation: &SignedRoleAttestation,
        min_accepted_epoch: u64,
    ) -> Result<(), Error> {
        let configured_min_accepted_epoch = ConfigOps::role_attestation_config()
            .map_err(Error::from)?
            .min_accepted_epoch_by_role
            .get(attestation.payload.role.as_str())
            .copied();
        let min_accepted_epoch = verify_flow::resolve_min_accepted_epoch(
            min_accepted_epoch,
            configured_min_accepted_epoch,
        );

        let caller = IcOps::msg_caller();
        let self_pid = IcOps::canister_self();
        let now_secs = IcOps::now_secs();
        let verifier_subnet = Some(EnvOps::subnet_pid().map_err(Error::from)?);
        let root_pid = EnvOps::root_pid().map_err(Error::from)?;

        let verify = || {
            DelegatedTokenOps::verify_role_attestation_cached(
                attestation,
                caller,
                self_pid,
                verifier_subnet,
                now_secs,
                min_accepted_epoch,
            )
            .map(|_| ())
        };
        let refresh = || async {
            let key_set: AttestationKeySet =
                RpcOps::call_rpc_result(root_pid, protocol::CANIC_ATTESTATION_KEY_SET, ()).await?;
            DelegatedTokenOps::replace_attestation_key_set(key_set);
            Ok(())
        };

        match verify_flow::verify_role_attestation_with_single_refresh(verify, refresh).await {
            Ok(()) => Ok(()),
            Err(verify_flow::RoleAttestationVerifyFlowError::Initial(err)) => {
                verify_flow::record_attestation_verifier_rejection(&err);
                verify_flow::log_attestation_verifier_rejection(
                    &err,
                    attestation,
                    caller,
                    self_pid,
                    "cached",
                );
                Err(Self::map_delegation_error(err.into()))
            }
            Err(verify_flow::RoleAttestationVerifyFlowError::Refresh { trigger, source }) => {
                verify_flow::record_attestation_verifier_rejection(&trigger);
                verify_flow::log_attestation_verifier_rejection(
                    &trigger,
                    attestation,
                    caller,
                    self_pid,
                    "cache_miss_refresh",
                );
                record_attestation_refresh_failed();
                log!(
                    Topic::Auth,
                    Warn,
                    "role attestation refresh failed local={} caller={} key_id={} error={}",
                    self_pid,
                    caller,
                    attestation.key_id,
                    source
                );
                Err(Self::map_delegation_error(source))
            }
            Err(verify_flow::RoleAttestationVerifyFlowError::PostRefresh(err)) => {
                verify_flow::record_attestation_verifier_rejection(&err);
                verify_flow::log_attestation_verifier_rejection(
                    &err,
                    attestation,
                    caller,
                    self_pid,
                    "post_refresh",
                );
                Err(Self::map_delegation_error(err.into()))
            }
        }
    }

    // Resolve the root-owned TTL ceiling from delegated-token config.
    fn delegated_auth_max_ttl_secs() -> Result<u64, Error> {
        let cfg = ConfigOps::delegated_tokens_config().map_err(Error::from)?;
        if !cfg.enabled {
            return Err(Error::forbidden(Self::DELEGATED_TOKENS_DISABLED));
        }

        Ok(cfg
            .max_ttl_secs
            .unwrap_or(Self::MAX_DELEGATED_SESSION_TTL_SECS))
    }
}

impl DelegationApi {
    // Route a self-contained delegation proof request over RPC to root.
    async fn request_delegation_remote(
        request: DelegationProofIssueRequest,
    ) -> Result<DelegationProof, Error> {
        let root_pid = EnvOps::root_pid().map_err(Error::from)?;
        RpcOps::call_rpc_result(root_pid, protocol::CANIC_REQUEST_DELEGATION, request)
            .await
            .map_err(Self::map_delegation_error)
    }

    // Execute one local root role-attestation request.
    pub async fn request_role_attestation_root(
        request: RoleAttestationRequest,
    ) -> Result<SignedRoleAttestation, Error> {
        let request = metadata::with_root_attestation_request_metadata(request);
        let response = RootResponseWorkflow::response(RootRequest::issue_role_attestation(request))
            .await
            .map_err(Self::map_delegation_error)?;

        match response {
            RootCapabilityResponse::RoleAttestationIssued(response) => Ok(response),
            _ => Err(Error::internal(
                "invalid root response type for role attestation request",
            )),
        }
    }

    // Route a canonical role-attestation request over RPC to root.
    async fn request_role_attestation_remote(
        request: RoleAttestationRequest,
    ) -> Result<RootCapabilityResponse, Error> {
        let root_pid = EnvOps::root_pid().map_err(Error::from)?;
        RpcOps::call_rpc_result(root_pid, protocol::CANIC_REQUEST_ROLE_ATTESTATION, request)
            .await
            .map_err(Self::map_delegation_error)
    }
}
