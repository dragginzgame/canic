use crate::{
    cdk::types::Principal,
    dto::{
        auth::{
            AttestationKeySet, AuthRequestMetadata, DelegatedRoleGrant, DelegatedToken,
            DelegatedTokenIssueRequest, DelegationAudience, DelegationCert, DelegationProof,
            DelegationProofGetRequest, DelegationProofIssueRequest, DelegationProofPrepareResponse,
            InternalInvocationProofRequest, RoleAttestationRequest, ShardKeyBinding,
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
    const DELEGATION_REPLAY_COMMAND_KIND: &str = "auth.prepare_delegation_proof.v1";
    const DELEGATION_REPLAY_RESPONSE_SCHEMA_VERSION: u32 = 1;
    const MAX_DELEGATION_REPLAY_TTL_NS: u64 = 300_000_000_000;
    const TOKEN_ISSUE_REPLAY_COMMAND_KIND: &str = "auth.issue_token.v1";
    const TOKEN_REPLAY_RESPONSE_SCHEMA_VERSION: u32 = 1;
    const MAX_TOKEN_REPLAY_TTL_NS: u64 = 300_000_000_000;
    const TOKEN_SIGNING_QUOTA_WINDOW_SECONDS: u64 = 60;
    const MAX_TOKEN_SIGNING_OPERATIONS_PER_WINDOW: u64 = 60;
    const TOKEN_SIGNING_CYCLE_RESERVATION_CYCLES: u128 = 1_000_000_000;
    const MIN_TOKEN_SIGNING_CYCLES_AFTER_RESERVATION: u128 = 1_000_000_000;
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
            max_cert_ttl_ns,
            max_token_ttl_ns,
            required_scopes,
            now_ns,
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
        let label = "delegated token issue";
        let metadata = Self::token_replay_metadata(request.metadata, "delegated token issue")?;
        let operation_id = OperationId::from_bytes(metadata.request_id);
        let command_kind = Self::token_issue_replay_command_kind();
        let caller = IcOps::msg_caller();
        let actor = ReplayActor::direct_caller(caller);
        let payload_hash = Self::token_issue_replay_payload_hash(&command_kind, &actor, &request);
        let token = match Self::reserve_token_replay_receipt(
            command_kind.clone(),
            metadata,
            actor,
            payload_hash,
        )? {
            ReplayReceiptDecision::Fresh(token) => {
                Self::log_token_replay_reserved(label, &command_kind, operation_id, caller);
                token
            }
            decision => {
                Self::log_token_replay_decision(
                    label,
                    &command_kind,
                    operation_id,
                    caller,
                    &decision,
                );
                return Self::map_token_replay_decision(decision, label);
            }
        };

        Self::issue_fresh_token_from_proof(
            token,
            command_kind,
            caller,
            operation_id,
            label,
            request,
        )
        .await
    }

    /// Prepare a root delegation proof from root over RPC.
    pub async fn prepare_delegation_proof(
        request: DelegationProofIssueRequest,
    ) -> Result<DelegationProofPrepareResponse, Error> {
        let request = metadata::with_delegation_request_metadata(request);
        Self::prepare_delegation_proof_remote(request).await
    }

    /// Prepare a root-certified delegation proof from the local root update path.
    pub async fn prepare_delegation_proof_root(
        request: DelegationProofIssueRequest,
    ) -> Result<DelegationProofPrepareResponse, Error> {
        EnvOps::require_root().map_err(Error::from)?;
        let caller = IcOps::msg_caller();
        Self::validate_delegation_request_caller(caller, request.shard_pid)?;
        let max_cert_ttl_ns = Self::delegated_token_max_ttl_ns()?;
        let metadata = Self::delegation_replay_metadata(request.metadata)?;
        let command_kind = Self::delegation_replay_command_kind();
        let actor = ReplayActor::direct_caller(caller);
        let payload_hash = Self::delegation_replay_payload_hash(&command_kind, &actor, &request);
        let now_ns = IcOps::now_nanos();
        let expires_at_ns = now_ns.checked_add(metadata.ttl_ns).ok_or_else(|| {
            Error::invalid("delegation proof replay metadata ttl_ns overflows nanoseconds")
        })?;
        let replay_input = ReplayReceiptReserveInput::new(
            command_kind.clone(),
            OperationId::from_bytes(metadata.request_id),
            actor,
            payload_hash,
            now_ns,
        )
        .with_expires_at_ns(expires_at_ns);

        let token = match reserve_or_replay_receipt(replay_input)
            .map_err(Self::map_delegation_replay_store_error)?
        {
            ReplayReceiptDecision::Fresh(token) => token,
            decision => return Self::map_delegation_replay_decision(decision),
        };

        Self::prepare_fresh_delegation_proof(token, caller, request, max_cert_ttl_ns).await
    }

    /// Retrieve a prepared self-contained delegation proof from the local root query path.
    pub fn get_delegation_proof_root(
        request: DelegationProofGetRequest,
    ) -> Result<DelegationProof, Error> {
        EnvOps::require_root().map_err(Error::from)?;
        let caller = IcOps::msg_caller();
        AuthOps::get_delegation_proof(caller, request.cert_hash).map_err(Self::map_auth_error)
    }

    async fn prepare_fresh_delegation_proof(
        token: ReplayReceiptToken,
        _caller: Principal,
        request: DelegationProofIssueRequest,
        max_cert_ttl_ns: u64,
    ) -> Result<DelegationProofPrepareResponse, Error> {
        let max_token_ttl_ns = request.cert_ttl_ns.min(max_cert_ttl_ns);
        let prepared = match AuthOps::prepare_delegation_proof(SignDelegationProofInput {
            operation_id: token.receipt().operation_id.into_bytes(),
            audience: request.aud,
            grants: request.grants,
            shard_pid: request.shard_pid,
            cert_ttl_ns: request.cert_ttl_ns,
            max_token_ttl_ns,
            max_cert_ttl_ns,
            issued_at_ns: IcOps::now_nanos(),
        })
        .await
        {
            Ok(prepared) => prepared,
            Err(err) => {
                abort_reserved_receipt(&token);
                return Err(Self::map_auth_error(err));
            }
        };

        let response = DelegationProofPrepareResponse {
            cert: prepared.cert,
            cert_hash: prepared.cert_hash,
            retrieval_expires_at_ns: prepared.retrieval_expires_at_ns,
        };

        let response_bytes = match Self::encode_delegation_prepare_response(&response) {
            Ok(response_bytes) => response_bytes,
            Err(err) => {
                mark_recovery_required(
                    &token,
                    RecoveryReason::ResponseCommitFailed,
                    secs_to_ns(IcOps::now_secs()),
                );
                return Err(err);
            }
        };

        commit_receipt_response(
            &token,
            Self::DELEGATION_REPLAY_RESPONSE_SCHEMA_VERSION,
            response_bytes,
            secs_to_ns(IcOps::now_secs()),
        );
        Ok(response)
    }

    async fn issue_fresh_token_from_proof(
        token: ReplayReceiptToken,
        command_kind: CommandKind,
        caller: Principal,
        operation_id: OperationId,
        label: &'static str,
        request: DelegatedTokenIssueRequest,
    ) -> Result<DelegatedToken, Error> {
        let prepared = match AuthOps::prepare_delegated_token_signature(SignDelegatedTokenInput {
            proof: request.proof,
            subject: request.subject,
            audience: request.aud,
            grants: request.grants,
            ttl_ns: request.ttl_ns,
            nonce: request.nonce,
        }) {
            Ok(prepared) => prepared,
            Err(err) => {
                abort_reserved_receipt(&token);
                return Err(Self::map_auth_error(err));
            }
        };

        let cost_permit = match CostGuardOps::reserve(Self::token_signing_cost_guard_request(
            command_kind.clone(),
            caller,
        )) {
            Ok(permit) => permit,
            Err(err) => {
                abort_reserved_receipt(&token);
                return Err(Self::map_auth_error(err));
            }
        };
        Self::log_token_signing_cost_guard_reserved(label, &command_kind, operation_id, caller);

        let effect = AuthOps::delegated_token_signing_effect(&prepared);
        mark_external_effect_in_flight(&token, effect.clone(), secs_to_ns(IcOps::now_secs()));
        Self::log_token_replay_effect_marked(label, &command_kind, operation_id, caller, &effect);

        let delegated_token =
            match AuthOps::sign_prepared_delegated_token(&cost_permit, prepared).await {
                Ok(delegated_token) => delegated_token,
                Err(err) => {
                    let _ = CostGuardOps::recover(&cost_permit, IcOps::now_secs());
                    mark_recovery_required(
                        &token,
                        RecoveryReason::ExternalEffectStatusUnknown,
                        secs_to_ns(IcOps::now_secs()),
                    );
                    Self::log_token_replay_recovery_required(
                        label,
                        &command_kind,
                        operation_id,
                        caller,
                        &err,
                    );
                    return Err(Self::map_auth_error(err));
                }
            };

        let response_bytes = match Self::encode_delegated_token_response(&delegated_token) {
            Ok(response_bytes) => response_bytes,
            Err(err) => {
                let _ = CostGuardOps::recover(&cost_permit, IcOps::now_secs());
                mark_recovery_required(
                    &token,
                    RecoveryReason::ResponseCommitFailed,
                    secs_to_ns(IcOps::now_secs()),
                );
                Self::log_token_replay_response_commit_failed(
                    label,
                    &command_kind,
                    operation_id,
                    caller,
                    &err,
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
            Self::log_token_replay_response_commit_failed_internal(
                label,
                &command_kind,
                operation_id,
                caller,
                &err,
            );
            return Err(Self::map_auth_error(err));
        }

        commit_receipt_response(
            &token,
            Self::TOKEN_REPLAY_RESPONSE_SCHEMA_VERSION,
            response_bytes,
            secs_to_ns(IcOps::now_secs()),
        );
        Self::log_token_replay_commit(label, &command_kind, operation_id, caller);
        Ok(delegated_token)
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
        let now_ns = IcOps::now_nanos();
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
                    now_ns,
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
        metadata: Option<AuthRequestMetadata>,
    ) -> Result<AuthRequestMetadata, Error> {
        let metadata = metadata.ok_or_else(Error::operation_id_required)?;
        if metadata.ttl_ns == 0 {
            return Err(Error::invalid(
                "delegation proof replay metadata ttl_ns must be greater than zero",
            ));
        }
        if metadata.ttl_ns > Self::MAX_DELEGATION_REPLAY_TTL_NS {
            return Err(Error::invalid(format!(
                "delegation proof replay metadata ttl_ns={} exceeds max {}",
                metadata.ttl_ns,
                Self::MAX_DELEGATION_REPLAY_TTL_NS
            )));
        }
        Ok(metadata)
    }

    fn token_replay_metadata(
        metadata: Option<AuthRequestMetadata>,
        label: &str,
    ) -> Result<AuthRequestMetadata, Error> {
        let metadata = metadata.ok_or_else(Error::operation_id_required)?;
        if metadata.ttl_ns == 0 {
            return Err(Error::invalid(format!(
                "{label} replay metadata ttl_ns must be greater than zero"
            )));
        }
        if metadata.ttl_ns > Self::MAX_TOKEN_REPLAY_TTL_NS {
            return Err(Error::invalid(format!(
                "{label} replay metadata ttl_ns={} exceeds max {}",
                metadata.ttl_ns,
                Self::MAX_TOKEN_REPLAY_TTL_NS
            )));
        }
        Ok(metadata)
    }

    fn delegation_replay_command_kind() -> CommandKind {
        CommandKind::new(Self::DELEGATION_REPLAY_COMMAND_KIND)
            .expect("delegation replay command kind is a valid static label")
    }

    fn token_issue_replay_command_kind() -> CommandKind {
        CommandKind::new(Self::TOKEN_ISSUE_REPLAY_COMMAND_KIND)
            .expect("delegated-token issue replay command kind is a valid static label")
    }

    fn delegation_replay_payload_hash(
        command_kind: &CommandKind,
        actor: &ReplayActor,
        request: &DelegationProofIssueRequest,
    ) -> [u8; 32] {
        let mut hasher = ReplayPayloadHasher::new(command_kind, actor);
        hasher.hash_principal(&request.shard_pid);
        Self::hash_delegation_audience(&mut hasher, &request.aud);
        Self::hash_delegated_role_grants(&mut hasher, &request.grants);
        hasher.hash_u64(request.cert_ttl_ns);
        hasher.finish()
    }

    fn token_issue_replay_payload_hash(
        command_kind: &CommandKind,
        actor: &ReplayActor,
        request: &DelegatedTokenIssueRequest,
    ) -> [u8; 32] {
        let mut hasher = ReplayPayloadHasher::new(command_kind, actor);
        Self::hash_delegation_proof(&mut hasher, &request.proof);
        hasher.hash_principal(&request.subject);
        Self::hash_delegation_audience(&mut hasher, &request.aud);
        Self::hash_delegated_role_grants(&mut hasher, &request.grants);
        hasher.hash_u64(request.ttl_ns);
        hasher.hash_bytes(&request.nonce);
        hasher.finish()
    }

    fn hash_delegation_audience(hasher: &mut ReplayPayloadHasher, aud: &DelegationAudience) {
        match aud {
            DelegationAudience::Canic => {
                hasher.hash_str("canic");
            }
            DelegationAudience::Project(project) => {
                hasher.hash_str("project");
                hasher.hash_str(project);
            }
        }
    }

    fn hash_delegated_role_grants(hasher: &mut ReplayPayloadHasher, grants: &[DelegatedRoleGrant]) {
        hasher.hash_u64(grants.len() as u64);
        for grant in grants {
            hasher.hash_role(&grant.target);
            Self::hash_string_vec(hasher, &grant.scopes);
        }
    }

    fn hash_delegation_proof(hasher: &mut ReplayPayloadHasher, proof: &DelegationProof) {
        Self::hash_delegation_cert(hasher, &proof.cert);
        Self::hash_root_proof(hasher, &proof.root_proof);
    }

    fn hash_delegation_cert(hasher: &mut ReplayPayloadHasher, cert: &DelegationCert) {
        hasher.hash_principal(&cert.root_pid);
        hasher.hash_principal(&cert.shard_pid);
        hasher.hash_str(&cert.shard_key_id);
        Self::hash_shard_signature_algorithm(hasher, cert.shard_sig_alg);
        hasher.hash_bytes(&cert.shard_public_key_sec1);
        hasher.hash_bytes(&cert.shard_key_hash);
        Self::hash_shard_key_binding(hasher, cert.shard_key_binding);
        hasher.hash_u64(cert.issued_at_ns);
        hasher.hash_u64(cert.not_before_ns);
        hasher.hash_u64(cert.expires_at_ns);
        hasher.hash_u64(cert.max_token_ttl_ns);
        Self::hash_delegation_audience(hasher, &cert.aud);
        Self::hash_delegated_role_grants(hasher, &cert.grants);
    }

    fn hash_root_proof(hasher: &mut ReplayPayloadHasher, proof: &crate::dto::auth::RootProof) {
        match proof {
            crate::dto::auth::RootProof::IcCanisterSignatureV1(proof) => {
                hasher.hash_str("IcCanisterSignatureV1");
                hasher.hash_bytes(&proof.signature_cbor);
                hasher.hash_bytes(&proof.public_key_der);
            }
        }
    }

    fn hash_shard_signature_algorithm(
        hasher: &mut ReplayPayloadHasher,
        alg: crate::dto::auth::ShardSignatureAlgorithm,
    ) {
        match alg {
            crate::dto::auth::ShardSignatureAlgorithm::IcThresholdEcdsaSecp256k1 => {
                hasher.hash_str("IcThresholdEcdsaSecp256k1");
            }
        }
    }

    fn hash_shard_key_binding(hasher: &mut ReplayPayloadHasher, binding: ShardKeyBinding) {
        match binding {
            ShardKeyBinding::IcThresholdEcdsaSecp256k1 {
                key_name_hash,
                derivation_path_hash,
            } => {
                hasher.hash_str("IcThresholdEcdsaSecp256k1");
                hasher.hash_bytes(&key_name_hash);
                hasher.hash_bytes(&derivation_path_hash);
            }
        }
    }

    fn hash_string_vec(hasher: &mut ReplayPayloadHasher, values: &[String]) {
        hasher.hash_u64(values.len() as u64);
        for value in values {
            hasher.hash_str(value);
        }
    }

    fn reserve_token_replay_receipt(
        command_kind: CommandKind,
        metadata: AuthRequestMetadata,
        actor: ReplayActor,
        payload_hash: [u8; 32],
    ) -> Result<ReplayReceiptDecision, Error> {
        let now_ns = IcOps::now_nanos();
        let expires_at_ns = now_ns.checked_add(metadata.ttl_ns).ok_or_else(|| {
            Error::invalid("delegated token issue replay metadata ttl_ns overflows nanoseconds")
        })?;
        let replay_input = ReplayReceiptReserveInput::new(
            command_kind,
            OperationId::from_bytes(metadata.request_id),
            actor,
            payload_hash,
            now_ns,
        )
        .with_expires_at_ns(expires_at_ns);

        reserve_or_replay_receipt(replay_input).map_err(Self::map_delegation_replay_store_error)
    }

    fn token_signing_cost_guard_request(
        command_kind: CommandKind,
        caller: Principal,
    ) -> CostGuardRequest {
        Self::token_signing_cost_guard_request_at(
            command_kind,
            caller,
            IcOps::canister_self(),
            IcOps::now_secs(),
            MgmtOps::canister_cycle_balance().to_u128(),
        )
    }

    const fn token_signing_cost_guard_request_at(
        command_kind: CommandKind,
        caller: Principal,
        payer: Principal,
        now_secs: u64,
        current_cycle_balance: u128,
    ) -> CostGuardRequest {
        CostGuardRequest {
            cost_class: crate::replay_policy::CostClass::ShardTokenSign,
            command_kind,
            quota_subject: caller,
            payer,
            now_secs,
            quota_window_secs: Self::TOKEN_SIGNING_QUOTA_WINDOW_SECONDS,
            max_operations_per_window: Self::MAX_TOKEN_SIGNING_OPERATIONS_PER_WINDOW,
            current_cycle_balance,
            cycle_reservation_cycles: Self::TOKEN_SIGNING_CYCLE_RESERVATION_CYCLES,
            min_cycles_after_reservation: Self::MIN_TOKEN_SIGNING_CYCLES_AFTER_RESERVATION,
        }
    }

    fn map_delegation_replay_decision(
        decision: ReplayReceiptDecision,
    ) -> Result<DelegationProofPrepareResponse, Error> {
        match decision {
            ReplayReceiptDecision::Fresh(_) => {
                Err(Error::invariant("fresh delegation replay decision escaped"))
            }
            ReplayReceiptDecision::ReturnCommitted(receipt) => {
                Self::decode_delegation_prepare_response(&receipt)
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
            ReplayReceiptDecision::PendingActorQuotaExceeded { max_pending, .. } => {
                Err(Error::exhausted(format!(
                    "delegation proof pending replay receipt quota exceeded for caller; max_pending={max_pending}"
                )))
            }
            ReplayReceiptDecision::PendingCommandQuotaExceeded { max_pending, .. } => {
                Err(Error::exhausted(format!(
                    "delegation proof pending replay receipt quota exceeded for command kind; max_pending={max_pending}"
                )))
            }
        }
    }

    fn map_token_replay_decision(
        decision: ReplayReceiptDecision,
        label: &str,
    ) -> Result<DelegatedToken, Error> {
        match decision {
            ReplayReceiptDecision::Fresh(_) => Err(Error::invariant(format!(
                "fresh {label} replay decision escaped"
            ))),
            ReplayReceiptDecision::ReturnCommitted(receipt) => {
                Self::decode_delegated_token_response(&receipt)
            }
            ReplayReceiptDecision::OperationInProgress => Err(Error::conflict(format!(
                "{label} request is already in progress; retry later with the same request id"
            ))),
            ReplayReceiptDecision::ActorMismatch => Err(Error::conflict(format!(
                "{label} request id was reused by a different caller"
            ))),
            ReplayReceiptDecision::PayloadMismatch => Err(Error::conflict(format!(
                "{label} request id was reused with a different payload"
            ))),
            ReplayReceiptDecision::Expired => Err(Error::conflict(format!(
                "{label} replay receipt expired; retry with a new request id"
            ))),
            ReplayReceiptDecision::RecoveryRequired(reason) => Err(Error::conflict(format!(
                "{label} request requires recovery before replay: {reason:?}"
            ))),
            ReplayReceiptDecision::TerminalFailed {
                error_code,
                error_bytes,
                error_bytes_truncated,
            } => Err(Error::conflict(format!(
                "{label} request previously failed: {error_code:?}; error_bytes_len={}; truncated={error_bytes_truncated}",
                error_bytes.len()
            ))),
            ReplayReceiptDecision::PendingActorQuotaExceeded { max_pending, .. } => {
                Err(Error::exhausted(format!(
                    "{label} pending replay receipt quota exceeded for caller; max_pending={max_pending}"
                )))
            }
            ReplayReceiptDecision::PendingCommandQuotaExceeded { max_pending, .. } => {
                Err(Error::exhausted(format!(
                    "{label} pending replay receipt quota exceeded for command kind; max_pending={max_pending}"
                )))
            }
        }
    }

    fn log_token_replay_reserved(
        label: &str,
        command_kind: &CommandKind,
        operation_id: OperationId,
        caller: Principal,
    ) {
        log!(
            Topic::Auth,
            Info,
            "{} replay receipt reserved command_kind={} operation_id={} caller={}",
            label,
            command_kind.as_str(),
            operation_id,
            caller
        );
    }

    fn log_token_replay_decision(
        label: &str,
        command_kind: &CommandKind,
        operation_id: OperationId,
        caller: Principal,
        decision: &ReplayReceiptDecision,
    ) {
        match decision {
            ReplayReceiptDecision::ReturnCommitted(_) => log!(
                Topic::Auth,
                Info,
                "{} committed replay returned command_kind={} operation_id={} caller={}",
                label,
                command_kind.as_str(),
                operation_id,
                caller
            ),
            _ => log!(
                Topic::Auth,
                Warn,
                "{} replay decision blocked command_kind={} operation_id={} caller={} decision={}",
                label,
                command_kind.as_str(),
                operation_id,
                caller,
                Self::token_replay_decision_name(decision)
            ),
        }
    }

    fn log_token_signing_cost_guard_reserved(
        label: &str,
        command_kind: &CommandKind,
        operation_id: OperationId,
        caller: Principal,
    ) {
        log!(
            Topic::Auth,
            Info,
            "{} signing cost guard reserved command_kind={} operation_id={} caller={}",
            label,
            command_kind.as_str(),
            operation_id,
            caller
        );
    }

    fn log_token_replay_effect_marked(
        label: &str,
        command_kind: &CommandKind,
        operation_id: OperationId,
        caller: Principal,
        effect: &ExternalEffectDescriptor,
    ) {
        log!(
            Topic::Auth,
            Info,
            "{} replay effect marked effect={} command_kind={} operation_id={} caller={}",
            label,
            Self::token_effect_name(effect),
            command_kind.as_str(),
            operation_id,
            caller
        );
    }

    fn log_token_replay_recovery_required(
        label: &str,
        command_kind: &CommandKind,
        operation_id: OperationId,
        caller: Principal,
        err: &crate::InternalError,
    ) {
        let (error_class, error_origin) = err.log_fields();
        log!(
            Topic::Auth,
            Error,
            "{} replay recovery required effect=threshold_ecdsa_sign_delegated_token command_kind={} operation_id={} caller={} error_class={} error_origin={}",
            label,
            command_kind.as_str(),
            operation_id,
            caller,
            error_class,
            error_origin
        );
    }

    fn log_token_replay_response_commit_failed(
        label: &str,
        command_kind: &CommandKind,
        operation_id: OperationId,
        caller: Principal,
        err: &Error,
    ) {
        log!(
            Topic::Auth,
            Error,
            "{} replay response commit failed command_kind={} operation_id={} caller={} error_code={:?}",
            label,
            command_kind.as_str(),
            operation_id,
            caller,
            err.code
        );
    }

    fn log_token_replay_response_commit_failed_internal(
        label: &str,
        command_kind: &CommandKind,
        operation_id: OperationId,
        caller: Principal,
        err: &crate::InternalError,
    ) {
        let (error_class, error_origin) = err.log_fields();
        log!(
            Topic::Auth,
            Error,
            "{} replay response commit failed command_kind={} operation_id={} caller={} error_class={} error_origin={}",
            label,
            command_kind.as_str(),
            operation_id,
            caller,
            error_class,
            error_origin
        );
    }

    fn log_token_replay_commit(
        label: &str,
        command_kind: &CommandKind,
        operation_id: OperationId,
        caller: Principal,
    ) {
        log!(
            Topic::Auth,
            Ok,
            "{} replay response committed command_kind={} operation_id={} caller={}",
            label,
            command_kind.as_str(),
            operation_id,
            caller
        );
    }

    const fn token_replay_decision_name(decision: &ReplayReceiptDecision) -> &'static str {
        match decision {
            ReplayReceiptDecision::Fresh(_) => "fresh",
            ReplayReceiptDecision::ReturnCommitted(_) => "return_committed",
            ReplayReceiptDecision::OperationInProgress => "operation_in_progress",
            ReplayReceiptDecision::ActorMismatch => "actor_mismatch",
            ReplayReceiptDecision::PayloadMismatch => "payload_mismatch",
            ReplayReceiptDecision::Expired => "expired",
            ReplayReceiptDecision::RecoveryRequired(_) => "recovery_required",
            ReplayReceiptDecision::TerminalFailed { .. } => "terminal_failed",
            ReplayReceiptDecision::PendingActorQuotaExceeded { .. } => {
                "pending_actor_quota_exceeded"
            }
            ReplayReceiptDecision::PendingCommandQuotaExceeded { .. } => {
                "pending_command_quota_exceeded"
            }
        }
    }

    const fn token_effect_name(effect: &ExternalEffectDescriptor) -> &'static str {
        match effect {
            ExternalEffectDescriptor::ThresholdEcdsaSign {
                purpose: EcdsaPurpose::DelegatedToken,
                ..
            } => "threshold_ecdsa_sign_delegated_token",
            ExternalEffectDescriptor::ThresholdEcdsaSign { .. } => "threshold_ecdsa_sign",
            ExternalEffectDescriptor::ManagementCreateCanister { .. } => {
                "management_create_canister"
            }
            ExternalEffectDescriptor::ManagementCall { .. } => "management_call",
            ExternalEffectDescriptor::IcpTransfer { .. } => "icp_transfer",
        }
    }

    fn map_delegation_replay_store_error(err: ReplayReceiptStoreError) -> Error {
        match err {
            ReplayReceiptStoreError::ReceiptDecodeFailed(message) => Error::internal(format!(
                "failed to decode delegation replay receipt: {message}"
            )),
        }
    }

    fn encode_delegation_prepare_response(
        response: &DelegationProofPrepareResponse,
    ) -> Result<Vec<u8>, Error> {
        encode_one(response).map_err(|err| {
            Error::internal(format!(
                "failed to encode delegation proof prepare replay response: {err}"
            ))
        })
    }

    fn encode_delegated_token_response(token: &DelegatedToken) -> Result<Vec<u8>, Error> {
        encode_one(token).map_err(|err| {
            Error::internal(format!(
                "failed to encode delegated token replay response: {err}"
            ))
        })
    }

    fn decode_delegation_prepare_response(
        receipt: &crate::ops::replay::model::ReplayReceipt,
    ) -> Result<DelegationProofPrepareResponse, Error> {
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
                "failed to decode delegation proof prepare replay response: {err}"
            ))
        })
    }

    fn decode_delegated_token_response(
        receipt: &crate::ops::replay::model::ReplayReceipt,
    ) -> Result<DelegatedToken, Error> {
        let response_schema_version = receipt.response_schema_version.ok_or_else(|| {
            Error::internal("delegated token replay receipt is missing response schema version")
        })?;
        if response_schema_version != Self::TOKEN_REPLAY_RESPONSE_SCHEMA_VERSION {
            return Err(Error::internal(format!(
                "unsupported delegated token replay response schema version {response_schema_version}"
            )));
        }
        let response_bytes = receipt.response_bytes.as_deref().ok_or_else(|| {
            Error::internal("delegated token replay receipt is missing response bytes")
        })?;
        decode_one(response_bytes).map_err(|err| {
            Error::internal(format!(
                "failed to decode delegated token replay response: {err}"
            ))
        })
    }
}

impl AuthApi {
    // Route a delegation proof prepare request over RPC to root.
    async fn prepare_delegation_proof_remote(
        request: DelegationProofIssueRequest,
    ) -> Result<DelegationProofPrepareResponse, Error> {
        let root_pid = EnvOps::root_pid().map_err(Error::from)?;
        RootAuthMaterialClient::new(root_pid)
            .prepare_delegation_proof(request)
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
            auth::{
                AuthRequestMetadata, DelegatedRoleGrant, DelegatedTokenIssueRequest,
                DelegationAudience, DelegationCert, DelegationProof, DelegationProofIssueRequest,
                IcCanisterSignatureProofV1, RootProof, ShardKeyBinding, ShardSignatureAlgorithm,
            },
            error::ErrorCode,
        },
        ops::auth::{AuthExpiryError, AuthOpsError},
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn delegation_request(metadata_id: u8) -> DelegationProofIssueRequest {
        DelegationProofIssueRequest {
            metadata: Some(meta(metadata_id, 60_000_000_000)),
            shard_pid: p(2),
            aud: DelegationAudience::Project("test".to_string()),
            grants: vec![grant("project_instance", &["canic.verify"])],
            cert_ttl_ns: 60_000_000_000,
        }
    }

    fn grant(role: &str, scopes: &[&str]) -> DelegatedRoleGrant {
        DelegatedRoleGrant {
            target: crate::ids::CanisterRole::owned(role.to_string()),
            scopes: scopes.iter().map(|scope| (*scope).to_string()).collect(),
        }
    }

    fn meta(id: u8, ttl_ns: u64) -> AuthRequestMetadata {
        AuthRequestMetadata {
            request_id: [id; 32],
            ttl_ns,
        }
    }

    fn delegation_proof() -> DelegationProof {
        DelegationProof {
            cert: DelegationCert {
                root_pid: p(1),
                shard_pid: p(2),
                shard_key_id: "shard-key".to_string(),
                shard_sig_alg: ShardSignatureAlgorithm::IcThresholdEcdsaSecp256k1,
                shard_public_key_sec1: vec![3; 33],
                shard_key_hash: [4; 32],
                shard_key_binding: ShardKeyBinding::IcThresholdEcdsaSecp256k1 {
                    key_name_hash: [5; 32],
                    derivation_path_hash: [6; 32],
                },
                issued_at_ns: 10,
                not_before_ns: 10,
                expires_at_ns: 100,
                max_token_ttl_ns: 60,
                aud: DelegationAudience::Project("test".to_string()),
                grants: vec![grant("project_instance", &["canic.verify"])],
            },
            root_proof: RootProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1 {
                signature_cbor: vec![7; 64],
                public_key_der: vec![8; 32],
            }),
        }
    }

    fn issue_request(metadata_id: u8) -> DelegatedTokenIssueRequest {
        DelegatedTokenIssueRequest {
            metadata: Some(meta(metadata_id, 60_000_000_000)),
            proof: delegation_proof(),
            subject: p(8),
            aud: DelegationAudience::Project("test".to_string()),
            grants: vec![grant("project_instance", &["canic.verify"])],
            ttl_ns: 30_000_000_000,
            nonce: [9; 16],
        }
    }

    #[test]
    fn internal_invocation_not_yet_valid_maps_to_non_retryable_proof_expiry() {
        let err = AuthApi::map_internal_invocation_verify_error(AuthOpsError::Expiry(
            AuthExpiryError::AttestationNotYetValid {
                issued_at_ns: 20,
                now_ns: 10,
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
    }

    #[test]
    fn delegation_replay_metadata_rejects_missing_or_invalid_ttl() {
        let missing = AuthApi::delegation_replay_metadata(None).expect_err("metadata is required");
        assert_eq!(missing.code, ErrorCode::OperationIdRequired);

        let zero = AuthApi::delegation_replay_metadata(Some(AuthRequestMetadata {
            request_id: [1; 32],
            ttl_ns: 0,
        }))
        .expect_err("zero ttl is invalid");
        assert_eq!(zero.code, ErrorCode::InvalidInput);

        let too_large = AuthApi::delegation_replay_metadata(Some(AuthRequestMetadata {
            request_id: [1; 32],
            ttl_ns: AuthApi::MAX_DELEGATION_REPLAY_TTL_NS + 1,
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
    fn delegated_token_replay_metadata_rejects_missing_or_invalid_ttl() {
        let missing =
            AuthApi::token_replay_metadata(None, "delegated token mint").expect_err("required");
        assert_eq!(missing.code, ErrorCode::OperationIdRequired);

        let zero = AuthApi::token_replay_metadata(Some(meta(1, 0)), "delegated token mint")
            .expect_err("zero ttl is invalid");
        assert_eq!(zero.code, ErrorCode::InvalidInput);

        let too_large = AuthApi::token_replay_metadata(
            Some(meta(1, AuthApi::MAX_TOKEN_REPLAY_TTL_NS + 1)),
            "delegated token mint",
        )
        .expect_err("oversized ttl is invalid");
        assert_eq!(too_large.code, ErrorCode::InvalidInput);
    }

    #[test]
    fn delegation_replay_payload_hash_binds_authoritative_payload() {
        let command_kind = AuthApi::delegation_replay_command_kind();
        let actor = crate::ops::replay::model::ReplayActor::direct_caller(p(2));
        let a = delegation_request(1);
        let mut b = a.clone();
        b.cert_ttl_ns += 1;

        assert_ne!(
            AuthApi::delegation_replay_payload_hash(&command_kind, &actor, &a),
            AuthApi::delegation_replay_payload_hash(&command_kind, &actor, &b)
        );
    }

    #[test]
    fn delegated_token_issue_payload_hash_ignores_metadata() {
        let command_kind = AuthApi::token_issue_replay_command_kind();
        let actor = crate::ops::replay::model::ReplayActor::direct_caller(p(2));
        let a = issue_request(1);
        let b = issue_request(9);

        assert_eq!(
            AuthApi::token_issue_replay_payload_hash(&command_kind, &actor, &a),
            AuthApi::token_issue_replay_payload_hash(&command_kind, &actor, &b)
        );
    }

    #[test]
    fn delegated_token_issue_payload_hash_binds_authoritative_payload() {
        let command_kind = AuthApi::token_issue_replay_command_kind();
        let actor = crate::ops::replay::model::ReplayActor::direct_caller(p(2));
        let a = issue_request(1);
        let mut b = a.clone();
        b.nonce = [10; 16];

        assert_ne!(
            AuthApi::token_issue_replay_payload_hash(&command_kind, &actor, &a),
            AuthApi::token_issue_replay_payload_hash(&command_kind, &actor, &b)
        );
    }
}
