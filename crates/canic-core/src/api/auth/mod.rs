use crate::{
    cdk::types::Principal,
    dto::{
        auth::{
            AttestationKeySet, DelegatedToken, DelegatedTokenIssueRequest,
            DelegatedTokenMintRequest, DelegationAudience, DelegationProof,
            DelegationProofIssueRequest, InternalInvocationProofRequest, RoleAttestationRequest,
            SignedInternalInvocationProofV1, SignedRoleAttestation,
        },
        error::{Error, ErrorCode},
        rpc::{Request as RootRequest, Response as RootCapabilityResponse, RootRequestMetadata},
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
        cost_guard::{CostGuardOps, CostGuardRequest},
        ic::{IcOps, mgmt::MgmtOps},
        replay::{
            guard::secs_to_ns,
            model::{
                CommandKind, EcdsaPurpose, ExternalEffectDescriptor, OperationId, RecoveryReason,
                ReplayActor, ReplayPayloadHasher,
            },
            receipt::{
                ReplayReceiptDecision, ReplayReceiptReserveInput, ReplayReceiptStoreError,
                ReplayReceiptToken, abort_reserved_receipt, commit_receipt_response,
                mark_external_effect_in_flight, mark_recovery_required, reserve_or_replay_receipt,
            },
        },
        runtime::env::EnvOps,
        runtime::metrics::auth::record_attestation_refresh_failed,
    },
    workflow::rpc::request::handler::RootResponseWorkflow,
};
use candid::{decode_one, encode_one};
use root_client::RootAuthMaterialClient;
use sha2::{Digest, Sha256};

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
    const DELEGATION_REPLAY_COMMAND_KIND: &str = "auth.issue_delegation_proof.v1";
    const DELEGATION_REPLAY_RESPONSE_SCHEMA_VERSION: u32 = 1;
    const MAX_DELEGATION_REPLAY_TTL_SECONDS: u64 = 300;
    const DELEGATION_SIGNING_QUOTA_WINDOW_SECONDS: u64 = 60;
    const MAX_DELEGATION_SIGNING_OPERATIONS_PER_WINDOW: u64 = 60;
    const DELEGATION_SIGNING_CYCLE_RESERVATION_CYCLES: u128 = 1_000_000_000;
    const MIN_DELEGATION_SIGNING_CYCLES_AFTER_RESERVATION: u128 = 1_000_000_000;
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
            metadata: None,
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
        let request = metadata::with_delegation_request_metadata(request);
        Self::request_delegation_remote(request).await
    }

    /// Issue a self-contained delegation proof from the local root.
    pub async fn issue_delegation_proof(
        request: DelegationProofIssueRequest,
    ) -> Result<DelegationProof, Error> {
        EnvOps::require_root().map_err(Error::from)?;
        let caller = IcOps::msg_caller();
        Self::validate_delegation_request_caller(caller, request.shard_pid)?;
        let max_cert_ttl_secs = Self::delegated_token_max_ttl_secs()?;
        let metadata = Self::delegation_replay_metadata(request.metadata)?;
        let command_kind = Self::delegation_replay_command_kind();
        let actor = ReplayActor::direct_caller(caller);
        let payload_hash = Self::delegation_replay_payload_hash(&command_kind, &actor, &request);
        let now_secs = IcOps::now_secs();
        let replay_input = ReplayReceiptReserveInput::new(
            command_kind.clone(),
            OperationId::from_bytes(metadata.request_id),
            actor,
            payload_hash,
            secs_to_ns(now_secs),
        )
        .with_expires_at_ns(secs_to_ns(now_secs.saturating_add(metadata.ttl_seconds)));

        let token = match reserve_or_replay_receipt(replay_input)
            .map_err(Self::map_delegation_replay_store_error)?
        {
            ReplayReceiptDecision::Fresh(token) => token,
            decision => return Self::map_delegation_replay_decision(decision),
        };

        Self::issue_fresh_delegation_proof(token, command_kind, caller, request, max_cert_ttl_secs)
            .await
    }

    async fn issue_fresh_delegation_proof(
        token: ReplayReceiptToken,
        command_kind: CommandKind,
        caller: Principal,
        request: DelegationProofIssueRequest,
        max_cert_ttl_secs: u64,
    ) -> Result<DelegationProof, Error> {
        let max_token_ttl_secs = request.cert_ttl_secs.min(max_cert_ttl_secs);
        let prepared = match AuthOps::prepare_delegation_proof(SignDelegationProofInput {
            audience: request.aud,
            scopes: request.scopes,
            shard_pid: request.shard_pid,
            cert_ttl_secs: request.cert_ttl_secs,
            max_token_ttl_secs,
            max_cert_ttl_secs,
            issued_at: IcOps::now_secs(),
        })
        .await
        {
            Ok(prepared) => prepared,
            Err(err) => {
                abort_reserved_receipt(&token);
                return Err(Self::map_auth_error(err));
            }
        };

        let cost_permit = match CostGuardOps::reserve(CostGuardRequest {
            cost_class: crate::replay_policy::CostClass::ThresholdEcdsaSign,
            command_kind,
            quota_subject: caller,
            payer: IcOps::canister_self(),
            now_secs: IcOps::now_secs(),
            quota_window_secs: Self::DELEGATION_SIGNING_QUOTA_WINDOW_SECONDS,
            max_operations_per_window: Self::MAX_DELEGATION_SIGNING_OPERATIONS_PER_WINDOW,
            current_cycle_balance: MgmtOps::canister_cycle_balance().to_u128(),
            cycle_reservation_cycles: Self::DELEGATION_SIGNING_CYCLE_RESERVATION_CYCLES,
            min_cycles_after_reservation: Self::MIN_DELEGATION_SIGNING_CYCLES_AFTER_RESERVATION,
        }) {
            Ok(permit) => permit,
            Err(err) => {
                abort_reserved_receipt(&token);
                return Err(Self::map_auth_error(err));
            }
        };

        mark_external_effect_in_flight(
            &token,
            ExternalEffectDescriptor::ThresholdEcdsaSign {
                key_id_hash: Self::hash_delegation_effect_key(&prepared.key_name),
                purpose: EcdsaPurpose::DelegationProof,
                message_hash: prepared.cert_hash,
            },
            secs_to_ns(IcOps::now_secs()),
        );

        let proof = match AuthOps::sign_prepared_delegation_proof(&cost_permit, prepared).await {
            Ok(proof) => proof,
            Err(err) => {
                let _ = CostGuardOps::recover(&cost_permit, IcOps::now_secs());
                mark_recovery_required(
                    &token,
                    RecoveryReason::ExternalEffectStatusUnknown,
                    secs_to_ns(IcOps::now_secs()),
                );
                return Err(Self::map_auth_error(err));
            }
        };

        let response_bytes = match Self::encode_delegation_proof_response(&proof) {
            Ok(response_bytes) => response_bytes,
            Err(err) => {
                let _ = CostGuardOps::recover(&cost_permit, IcOps::now_secs());
                mark_recovery_required(
                    &token,
                    RecoveryReason::ResponseCommitFailed,
                    secs_to_ns(IcOps::now_secs()),
                );
                return Err(err);
            }
        };

        if let Err(err) = CostGuardOps::complete(&cost_permit, IcOps::now_secs()) {
            mark_recovery_required(
                &token,
                RecoveryReason::ResponseCommitFailed,
                secs_to_ns(IcOps::now_secs()),
            );
            return Err(Self::map_auth_error(err));
        }

        commit_receipt_response(
            &token,
            Self::DELEGATION_REPLAY_RESPONSE_SCHEMA_VERSION,
            response_bytes,
            secs_to_ns(IcOps::now_secs()),
        );
        Ok(proof)
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

    fn validate_delegation_request_caller(
        caller: Principal,
        shard_pid: Principal,
    ) -> Result<(), Error> {
        if caller == shard_pid {
            return Ok(());
        }

        Err(Error::forbidden(format!(
            "delegation request caller {caller} must match shard_pid {shard_pid}"
        )))
    }

    fn delegation_replay_metadata(
        metadata: Option<RootRequestMetadata>,
    ) -> Result<RootRequestMetadata, Error> {
        let metadata = metadata
            .ok_or_else(|| Error::invalid("delegation proof request requires replay metadata"))?;
        if metadata.ttl_seconds == 0 {
            return Err(Error::invalid(
                "delegation proof replay metadata ttl_seconds must be greater than zero",
            ));
        }
        if metadata.ttl_seconds > Self::MAX_DELEGATION_REPLAY_TTL_SECONDS {
            return Err(Error::invalid(format!(
                "delegation proof replay metadata ttl_seconds={} exceeds max {}",
                metadata.ttl_seconds,
                Self::MAX_DELEGATION_REPLAY_TTL_SECONDS
            )));
        }
        Ok(metadata)
    }

    fn delegation_replay_command_kind() -> CommandKind {
        CommandKind::new(Self::DELEGATION_REPLAY_COMMAND_KIND)
            .expect("delegation replay command kind is a valid static label")
    }

    fn delegation_replay_payload_hash(
        command_kind: &CommandKind,
        actor: &ReplayActor,
        request: &DelegationProofIssueRequest,
    ) -> [u8; 32] {
        let mut hasher = ReplayPayloadHasher::new(command_kind, actor);
        hasher.hash_principal(&request.shard_pid);
        hasher.hash_u64(request.scopes.len() as u64);
        for scope in &request.scopes {
            hasher.hash_str(scope);
        }
        Self::hash_delegation_audience(&mut hasher, &request.aud);
        hasher.hash_u64(request.cert_ttl_secs);
        hasher.finish()
    }

    fn hash_delegation_audience(hasher: &mut ReplayPayloadHasher, aud: &DelegationAudience) {
        match aud {
            DelegationAudience::Role(role) => {
                hasher.hash_str("role");
                hasher.hash_role(role);
            }
            DelegationAudience::Principal(principal) => {
                hasher.hash_str("principal");
                hasher.hash_principal(principal);
            }
        }
    }

    fn map_delegation_replay_decision(
        decision: ReplayReceiptDecision,
    ) -> Result<DelegationProof, Error> {
        match decision {
            ReplayReceiptDecision::Fresh(_) => {
                Err(Error::invariant("fresh delegation replay decision escaped"))
            }
            ReplayReceiptDecision::ReturnCommitted(receipt) => {
                Self::decode_delegation_proof_response(&receipt)
            }
            ReplayReceiptDecision::OperationInProgress => Err(Error::conflict(
                "delegation proof request is already in progress; retry later with the same request id",
            )),
            ReplayReceiptDecision::ActorMismatch => Err(Error::conflict(
                "delegation proof request id was reused by a different caller",
            )),
            ReplayReceiptDecision::PayloadMismatch => Err(Error::conflict(
                "delegation proof request id was reused with a different payload",
            )),
            ReplayReceiptDecision::Expired => Err(Error::conflict(
                "delegation proof replay receipt expired; retry with a new request id",
            )),
            ReplayReceiptDecision::RecoveryRequired(reason) => Err(Error::conflict(format!(
                "delegation proof request requires recovery before replay: {reason:?}"
            ))),
            ReplayReceiptDecision::TerminalFailed {
                error_code,
                error_bytes,
                error_bytes_truncated,
            } => Err(Error::conflict(format!(
                "delegation proof request previously failed: {error_code:?}; error_bytes_len={}; truncated={error_bytes_truncated}",
                error_bytes.len()
            ))),
        }
    }

    fn map_delegation_replay_store_error(err: ReplayReceiptStoreError) -> Error {
        match err {
            ReplayReceiptStoreError::ReceiptDecodeFailed(message) => Error::internal(format!(
                "failed to decode delegation replay receipt: {message}"
            )),
        }
    }

    fn encode_delegation_proof_response(proof: &DelegationProof) -> Result<Vec<u8>, Error> {
        encode_one(proof).map_err(|err| {
            Error::internal(format!(
                "failed to encode delegation proof replay response: {err}"
            ))
        })
    }

    fn decode_delegation_proof_response(
        receipt: &crate::ops::replay::model::ReplayReceipt,
    ) -> Result<DelegationProof, Error> {
        let response_schema_version = receipt.response_schema_version.ok_or_else(|| {
            Error::internal("delegation replay receipt is missing response schema version")
        })?;
        if response_schema_version != Self::DELEGATION_REPLAY_RESPONSE_SCHEMA_VERSION {
            return Err(Error::internal(format!(
                "unsupported delegation replay response schema version {response_schema_version}"
            )));
        }
        let response_bytes = receipt.response_bytes.as_deref().ok_or_else(|| {
            Error::internal("delegation replay receipt is missing response bytes")
        })?;
        decode_one(response_bytes).map_err(|err| {
            Error::internal(format!(
                "failed to decode delegation proof replay response: {err}"
            ))
        })
    }

    fn hash_delegation_effect_key(key_name: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"canic-delegation-proof-effect-key:v1");
        hasher.update(key_name.as_bytes());
        hasher.finalize().into()
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
        cdk::types::Principal,
        dto::{
            auth::{DelegationAudience, DelegationProofIssueRequest},
            error::ErrorCode,
            rpc::RootRequestMetadata,
        },
        ops::auth::{AuthExpiryError, AuthOpsError},
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn delegation_request(metadata_id: u8) -> DelegationProofIssueRequest {
        DelegationProofIssueRequest {
            metadata: Some(RootRequestMetadata {
                request_id: [metadata_id; 32],
                ttl_seconds: 60,
            }),
            shard_pid: p(2),
            scopes: vec!["canic.verify".to_string()],
            aud: DelegationAudience::Principal(p(3)),
            cert_ttl_secs: 60,
        }
    }

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

    #[test]
    fn delegation_request_caller_must_match_requested_shard() {
        AuthApi::validate_delegation_request_caller(p(2), p(2)).expect("matching shard");

        let err = AuthApi::validate_delegation_request_caller(p(1), p(2))
            .expect_err("mismatched caller must fail");

        assert_eq!(err.code, ErrorCode::Forbidden);
        assert!(err.message.contains("must match shard_pid"));
    }

    #[test]
    fn delegation_replay_metadata_rejects_missing_or_invalid_ttl() {
        let missing = AuthApi::delegation_replay_metadata(None).expect_err("metadata is required");
        assert_eq!(missing.code, ErrorCode::InvalidInput);

        let zero = AuthApi::delegation_replay_metadata(Some(RootRequestMetadata {
            request_id: [1; 32],
            ttl_seconds: 0,
        }))
        .expect_err("zero ttl is invalid");
        assert_eq!(zero.code, ErrorCode::InvalidInput);

        let too_large = AuthApi::delegation_replay_metadata(Some(RootRequestMetadata {
            request_id: [1; 32],
            ttl_seconds: AuthApi::MAX_DELEGATION_REPLAY_TTL_SECONDS + 1,
        }))
        .expect_err("oversized ttl is invalid");
        assert_eq!(too_large.code, ErrorCode::InvalidInput);
    }

    #[test]
    fn delegation_replay_payload_hash_ignores_metadata() {
        let command_kind = AuthApi::delegation_replay_command_kind();
        let actor = crate::ops::replay::model::ReplayActor::direct_caller(p(2));
        let a = delegation_request(1);
        let b = delegation_request(9);

        assert_eq!(
            AuthApi::delegation_replay_payload_hash(&command_kind, &actor, &a),
            AuthApi::delegation_replay_payload_hash(&command_kind, &actor, &b)
        );
    }

    #[test]
    fn delegation_replay_payload_hash_binds_authoritative_payload() {
        let command_kind = AuthApi::delegation_replay_command_kind();
        let actor = crate::ops::replay::model::ReplayActor::direct_caller(p(2));
        let a = delegation_request(1);
        let mut b = a.clone();
        b.cert_ttl_secs += 1;

        assert_ne!(
            AuthApi::delegation_replay_payload_hash(&command_kind, &actor, &a),
            AuthApi::delegation_replay_payload_hash(&command_kind, &actor, &b)
        );
    }
}
