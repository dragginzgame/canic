use crate::{
    cdk::types::Principal,
    dto::{
        auth::{
            AuthRequestMetadata, DelegatedRoleGrant, DelegatedToken, DelegatedTokenGetRequest,
            DelegatedTokenPrepareRequest, DelegatedTokenPrepareResponse, DelegationAudience,
            DelegationProof, DelegationProofGetRequest, DelegationProofIssueRequest,
            DelegationProofPrepareResponse, InstallActiveDelegationProofRequest,
            InstallActiveDelegationProofResponse, RoleAttestationGetRequest,
            RoleAttestationPrepareResponse, RoleAttestationRequest, SignedRoleAttestation,
        },
        error::Error,
    },
    error::InternalErrorClass,
    ops::{
        auth::{
            AuthOps, SignDelegatedTokenInput, SignDelegationProofInput, SignRoleAttestationInput,
            VerifyDelegatedTokenRuntimeInput,
        },
        config::ConfigOps,
        ic::IcOps,
        replay::{
            guard::secs_to_ns,
            model::{CommandKind, OperationId, RecoveryReason, ReplayActor, ReplayPayloadHasher},
            receipt::{
                ReplayReceiptDecision, ReplayReceiptReserveInput, ReplayReceiptStoreError,
                ReplayReceiptToken, abort_reserved_receipt, commit_receipt_response,
                mark_recovery_required, reserve_or_replay_receipt,
            },
        },
        runtime::env::EnvOps,
        storage::registry::subnet::SubnetRegistryOps,
    },
};
use candid::{decode_one, encode_one};
use root_client::RootAuthMaterialClient;

// Internal auth pipeline:
// - `session` owns delegated-session ingress and replay/session state handling.
// - `metadata` owns root request metadata construction.
mod metadata;
mod root_client;
mod session;

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
    const ROLE_ATTESTATION_REPLAY_COMMAND_KIND: &str = "auth.prepare_role_attestation.v1";
    const ROLE_ATTESTATION_REPLAY_RESPONSE_SCHEMA_VERSION: u32 = 1;
    const MAX_ROLE_ATTESTATION_REPLAY_TTL_NS: u64 = 300_000_000_000;
    const TOKEN_PREPARE_REPLAY_COMMAND_KIND: &str = "auth.prepare_delegated_token.v1";
    const TOKEN_PREPARE_REPLAY_RESPONSE_SCHEMA_VERSION: u32 = 1;
    const MAX_TOKEN_REPLAY_TTL_NS: u64 = 300_000_000_000;
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
        let label = "delegated token prepare";
        let metadata = Self::token_replay_metadata(request.metadata, label)?;
        let caller = IcOps::msg_caller();
        let command_kind = Self::token_prepare_replay_command_kind();
        let actor = ReplayActor::direct_caller(caller);
        let payload_hash = Self::token_prepare_replay_payload_hash(&command_kind, &actor, &request);
        let now_ns = IcOps::now_nanos();
        let expires_at_ns = now_ns.checked_add(metadata.ttl_ns).ok_or_else(|| {
            Error::invalid("delegated token prepare replay metadata ttl_ns overflows nanoseconds")
        })?;
        let replay_input = ReplayReceiptReserveInput::new(
            command_kind,
            OperationId::from_bytes(metadata.request_id),
            actor,
            payload_hash,
            now_ns,
        )
        .with_expires_at_ns(expires_at_ns);

        let token = match reserve_or_replay_receipt(replay_input)
            .map_err(Self::map_token_prepare_replay_store_error)?
        {
            ReplayReceiptDecision::Fresh(token) => token,
            decision => return Self::map_token_prepare_replay_decision(decision),
        };

        let prepared = AuthOps::prepare_delegated_token_issuer_proof(
            SignDelegatedTokenInput {
                subject: request.subject,
                audience: request.aud,
                grants: request.grants,
                ttl_ns: request.ttl_ns,
                ext: request.ext,
            },
            metadata.request_id,
            caller,
        )
        .map_err(|err| {
            abort_reserved_receipt(&token);
            Self::map_auth_error(err)
        })?;

        let response = DelegatedTokenPrepareResponse {
            claims: prepared.prepared.claims,
            claims_hash: prepared.claims_hash,
            retrieval_expires_at_ns: prepared.retrieval_expires_at_ns,
        };

        let response_bytes = match Self::encode_token_prepare_response(&response) {
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
            Self::TOKEN_PREPARE_REPLAY_RESPONSE_SCHEMA_VERSION,
            response_bytes,
            secs_to_ns(IcOps::now_secs()),
        );
        Ok(response)
    }

    /// Retrieve a prepared delegated token with its issuer canister-signature proof.
    pub fn get_delegated_token(request: DelegatedTokenGetRequest) -> Result<DelegatedToken, Error> {
        AuthOps::get_delegated_token_issuer_proof(request.claims_hash, IcOps::msg_caller())
            .map_err(Self::map_auth_error)
    }

    /// Install validated root-certified delegation material for issuer-local token issuance.
    pub fn install_active_delegation_proof(
        request: InstallActiveDelegationProofRequest,
    ) -> Result<InstallActiveDelegationProofResponse, Error> {
        let active_proof =
            AuthOps::install_active_delegation_proof(request.proof, IcOps::msg_caller())
                .map_err(Self::map_auth_error)?;

        Ok(InstallActiveDelegationProofResponse { active_proof })
    }

    /// Prepare a root delegation proof from root over RPC.
    pub async fn prepare_delegation_proof(
        request: DelegationProofIssueRequest,
    ) -> Result<DelegationProofPrepareResponse, Error> {
        let request = metadata::with_delegation_request_metadata(request);
        Self::prepare_delegation_proof_remote(request).await
    }

    /// Prepare a root-certified delegation proof from the local root update path.
    pub fn prepare_delegation_proof_root(
        request: DelegationProofIssueRequest,
    ) -> Result<DelegationProofPrepareResponse, Error> {
        EnvOps::require_root().map_err(Error::from)?;
        let caller = IcOps::msg_caller();
        Self::validate_delegation_request_caller(caller, request.issuer_pid)?;
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
            command_kind,
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

        Self::prepare_fresh_delegation_proof(token, caller, request, max_cert_ttl_ns)
    }

    /// Retrieve a prepared self-contained delegation proof from the local root query path.
    pub fn get_delegation_proof_root(
        request: DelegationProofGetRequest,
    ) -> Result<DelegationProof, Error> {
        EnvOps::require_root().map_err(Error::from)?;
        let caller = IcOps::msg_caller();
        AuthOps::get_delegation_proof(caller, request.cert_hash).map_err(Self::map_auth_error)
    }

    /// Prepare a root-certified role attestation from the local root update path.
    pub fn prepare_role_attestation_root(
        request: RoleAttestationRequest,
    ) -> Result<RoleAttestationPrepareResponse, Error> {
        EnvOps::require_root().map_err(Error::from)?;
        let caller = IcOps::msg_caller();
        Self::validate_role_attestation_request(caller, &request)?;
        let metadata = Self::role_attestation_replay_metadata(request.metadata)?;
        let command_kind = Self::role_attestation_replay_command_kind();
        let actor = ReplayActor::direct_caller(caller);
        let payload_hash =
            Self::role_attestation_replay_payload_hash(&command_kind, &actor, &request);
        let now_ns = IcOps::now_nanos();
        let expires_at_ns = now_ns.checked_add(metadata.ttl_ns).ok_or_else(|| {
            Error::invalid("role attestation replay metadata ttl_ns overflows nanoseconds")
        })?;
        let replay_input = ReplayReceiptReserveInput::new(
            command_kind,
            OperationId::from_bytes(metadata.request_id),
            actor,
            payload_hash,
            now_ns,
        )
        .with_expires_at_ns(expires_at_ns);

        let token = match reserve_or_replay_receipt(replay_input)
            .map_err(Self::map_role_attestation_replay_store_error)?
        {
            ReplayReceiptDecision::Fresh(token) => token,
            decision => return Self::map_role_attestation_replay_decision(decision),
        };

        let prepared = match AuthOps::prepare_role_attestation(SignRoleAttestationInput {
            operation_id: token.receipt().operation_id.into_bytes(),
            subject: request.subject,
            role: request.role,
            subnet_id: request.subnet_id,
            audience: request.audience,
            ttl_ns: request.ttl_ns,
            epoch: request.epoch,
            issued_at_ns: now_ns,
        }) {
            Ok(prepared) => prepared,
            Err(err) => {
                abort_reserved_receipt(&token);
                return Err(Self::map_auth_error(err));
            }
        };

        let response = RoleAttestationPrepareResponse {
            payload: prepared.payload,
            payload_hash: prepared.payload_hash,
            retrieval_expires_at_ns: prepared.retrieval_expires_at_ns,
        };

        let response_bytes = match Self::encode_role_attestation_prepare_response(&response) {
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
            Self::ROLE_ATTESTATION_REPLAY_RESPONSE_SCHEMA_VERSION,
            response_bytes,
            secs_to_ns(IcOps::now_secs()),
        );
        Ok(response)
    }

    /// Retrieve a prepared role attestation with its root canister-signature proof.
    pub fn get_role_attestation_root(
        request: RoleAttestationGetRequest,
    ) -> Result<SignedRoleAttestation, Error> {
        EnvOps::require_root().map_err(Error::from)?;
        AuthOps::get_role_attestation(IcOps::msg_caller(), request.payload_hash)
            .map_err(Self::map_auth_error)
    }

    fn prepare_fresh_delegation_proof(
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
            issuer_pid: request.issuer_pid,
            cert_ttl_ns: request.cert_ttl_ns,
            max_token_ttl_ns,
            max_cert_ttl_ns,
            issued_at_ns: IcOps::now_nanos(),
        }) {
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
        issuer_pid: Principal,
    ) -> Result<(), Error> {
        if caller == issuer_pid {
            return Ok(());
        }

        Err(Error::forbidden(format!(
            "delegation request caller {caller} must match issuer_pid {issuer_pid}"
        )))
    }

    fn validate_role_attestation_request(
        caller: Principal,
        request: &RoleAttestationRequest,
    ) -> Result<(), Error> {
        if request.subject != caller {
            return Err(Error::forbidden(format!(
                "role attestation subject {} must match caller {}",
                request.subject, caller
            )));
        }

        let registered = SubnetRegistryOps::get(request.subject).ok_or_else(|| {
            Error::forbidden(format!(
                "role attestation subject {} is not registered",
                request.subject
            ))
        })?;
        if registered.role != request.role {
            return Err(Error::forbidden(format!(
                "role attestation role mismatch for subject {}: requested {}, registered {}",
                request.subject, request.role, registered.role
            )));
        }

        if let Some(requested_subnet) = request.subnet_id {
            let local_subnet = EnvOps::subnet_pid().map_err(Error::from)?;
            if requested_subnet != local_subnet {
                return Err(Error::forbidden(format!(
                    "role attestation subnet mismatch for subject {}: requested {}, local {}",
                    request.subject, requested_subnet, local_subnet
                )));
            }
        }

        let max_ttl_ns = Self::role_attestation_max_ttl_ns()?;
        if request.ttl_ns == 0 || request.ttl_ns > max_ttl_ns {
            return Err(Error::invalid(format!(
                "role attestation ttl_ns must satisfy 0 < ttl_ns <= {max_ttl_ns} (got {})",
                request.ttl_ns
            )));
        }

        Ok(())
    }

    fn role_attestation_max_ttl_ns() -> Result<u64, Error> {
        let cfg = ConfigOps::role_attestation_config().map_err(Error::from)?;
        cfg.max_ttl_secs.checked_mul(1_000_000_000).ok_or_else(|| {
            Error::invalid("auth.role_attestation.max_ttl_secs overflows nanoseconds")
        })
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

    fn role_attestation_replay_metadata(
        metadata: Option<crate::dto::rpc::RootRequestMetadata>,
    ) -> Result<crate::dto::rpc::RootRequestMetadata, Error> {
        let metadata = metadata.ok_or_else(Error::operation_id_required)?;
        if metadata.ttl_ns == 0 {
            return Err(Error::invalid(
                "role attestation replay metadata ttl_ns must be greater than zero",
            ));
        }
        if metadata.ttl_ns > Self::MAX_ROLE_ATTESTATION_REPLAY_TTL_NS {
            return Err(Error::invalid(format!(
                "role attestation replay metadata ttl_ns={} exceeds max {}",
                metadata.ttl_ns,
                Self::MAX_ROLE_ATTESTATION_REPLAY_TTL_NS
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

    fn role_attestation_replay_command_kind() -> CommandKind {
        CommandKind::new(Self::ROLE_ATTESTATION_REPLAY_COMMAND_KIND)
            .expect("role attestation replay command kind is a valid static label")
    }

    fn token_prepare_replay_command_kind() -> CommandKind {
        CommandKind::new(Self::TOKEN_PREPARE_REPLAY_COMMAND_KIND)
            .expect("delegated-token prepare replay command kind is a valid static label")
    }

    fn delegation_replay_payload_hash(
        command_kind: &CommandKind,
        actor: &ReplayActor,
        request: &DelegationProofIssueRequest,
    ) -> [u8; 32] {
        let mut hasher = ReplayPayloadHasher::new(command_kind, actor);
        hasher.hash_principal(&request.issuer_pid);
        Self::hash_delegation_audience(&mut hasher, &request.aud);
        Self::hash_delegated_role_grants(&mut hasher, &request.grants);
        hasher.hash_u64(request.cert_ttl_ns);
        hasher.finish()
    }

    fn role_attestation_replay_payload_hash(
        command_kind: &CommandKind,
        actor: &ReplayActor,
        request: &RoleAttestationRequest,
    ) -> [u8; 32] {
        let mut hasher = ReplayPayloadHasher::new(command_kind, actor);
        hasher.hash_principal(&request.subject);
        hasher.hash_role(&request.role);
        hasher.hash_bool(request.subnet_id.is_some());
        if let Some(subnet_id) = request.subnet_id {
            hasher.hash_principal(&subnet_id);
        }
        hasher.hash_principal(&request.audience);
        hasher.hash_u64(request.ttl_ns);
        hasher.hash_u64(request.epoch);
        hasher.finish()
    }

    fn token_prepare_replay_payload_hash(
        command_kind: &CommandKind,
        actor: &ReplayActor,
        request: &DelegatedTokenPrepareRequest,
    ) -> [u8; 32] {
        let mut hasher = ReplayPayloadHasher::new(command_kind, actor);
        hasher.hash_principal(&request.subject);
        Self::hash_delegation_audience(&mut hasher, &request.aud);
        Self::hash_delegated_role_grants(&mut hasher, &request.grants);
        hasher.hash_u64(request.ttl_ns);
        Self::hash_optional_bytes(&mut hasher, request.ext.as_deref());
        hasher.finish()
    }

    fn hash_delegation_audience(hasher: &mut ReplayPayloadHasher, aud: &DelegationAudience) {
        match aud {
            DelegationAudience::Canister(canister) => {
                hasher.hash_str("canister");
                hasher.hash_principal(canister);
            }
            DelegationAudience::CanicSubnet(subnet) => {
                hasher.hash_str("canic_subnet");
                hasher.hash_principal(subnet);
            }
            DelegationAudience::Project(project) => {
                hasher.hash_str("project");
                hasher.hash_str(project);
            }
        }
    }

    fn hash_optional_bytes(hasher: &mut ReplayPayloadHasher, bytes: Option<&[u8]>) {
        hasher.hash_bool(bytes.is_some());
        if let Some(bytes) = bytes {
            hasher.hash_bytes(bytes);
        }
    }

    fn hash_delegated_role_grants(hasher: &mut ReplayPayloadHasher, grants: &[DelegatedRoleGrant]) {
        hasher.hash_u64(grants.len() as u64);
        for grant in grants {
            hasher.hash_role(&grant.target);
            Self::hash_string_vec(hasher, &grant.scopes);
        }
    }

    fn hash_string_vec(hasher: &mut ReplayPayloadHasher, values: &[String]) {
        hasher.hash_u64(values.len() as u64);
        for value in values {
            hasher.hash_str(value);
        }
    }

    fn map_token_prepare_replay_decision(
        decision: ReplayReceiptDecision,
    ) -> Result<DelegatedTokenPrepareResponse, Error> {
        match decision {
            ReplayReceiptDecision::Fresh(_) => Err(Error::invariant(
                "fresh delegated token replay decision escaped",
            )),
            ReplayReceiptDecision::ReturnCommitted(receipt) => {
                Self::decode_token_prepare_response(&receipt)
            }
            ReplayReceiptDecision::OperationInProgress => Err(Error::conflict(
                "delegated token prepare request is already in progress; retry later with the same request id",
            )),
            ReplayReceiptDecision::ActorMismatch => Err(Error::conflict(
                "delegated token prepare request id was reused by a different caller",
            )),
            ReplayReceiptDecision::PayloadMismatch => Err(Error::conflict(
                "delegated token prepare request id was reused with a different payload",
            )),
            ReplayReceiptDecision::Expired => Err(Error::conflict(
                "delegated token prepare replay receipt expired; retry with a new request id",
            )),
            ReplayReceiptDecision::RecoveryRequired(reason) => Err(Error::conflict(format!(
                "delegated token prepare request requires recovery before replay: {reason:?}"
            ))),
            ReplayReceiptDecision::TerminalFailed {
                error_code,
                error_bytes,
                error_bytes_truncated,
            } => Err(Error::conflict(format!(
                "delegated token prepare request previously failed: {error_code:?}; error_bytes_len={}; truncated={error_bytes_truncated}",
                error_bytes.len()
            ))),
            ReplayReceiptDecision::PendingActorQuotaExceeded { max_pending, .. } => {
                Err(Error::exhausted(format!(
                    "delegated token prepare pending replay receipt quota exceeded for caller; max_pending={max_pending}"
                )))
            }
            ReplayReceiptDecision::PendingCommandQuotaExceeded { max_pending, .. } => {
                Err(Error::exhausted(format!(
                    "delegated token prepare pending replay receipt quota exceeded for command kind; max_pending={max_pending}"
                )))
            }
        }
    }

    fn map_token_prepare_replay_store_error(err: ReplayReceiptStoreError) -> Error {
        match err {
            ReplayReceiptStoreError::ReceiptDecodeFailed(message) => Error::internal(format!(
                "failed to decode delegated token prepare replay receipt: {message}"
            )),
        }
    }

    fn encode_token_prepare_response(
        response: &DelegatedTokenPrepareResponse,
    ) -> Result<Vec<u8>, Error> {
        encode_one(response).map_err(|err| {
            Error::internal(format!(
                "failed to encode delegated token prepare replay response: {err}"
            ))
        })
    }

    fn decode_token_prepare_response(
        receipt: &crate::ops::replay::model::ReplayReceipt,
    ) -> Result<DelegatedTokenPrepareResponse, Error> {
        let response_schema_version = receipt.response_schema_version.ok_or_else(|| {
            Error::internal(
                "delegated token prepare replay receipt is missing response schema version",
            )
        })?;
        if response_schema_version != Self::TOKEN_PREPARE_REPLAY_RESPONSE_SCHEMA_VERSION {
            return Err(Error::internal(format!(
                "unsupported delegated token prepare replay response schema version {response_schema_version}"
            )));
        }
        let response_bytes = receipt.response_bytes.as_deref().ok_or_else(|| {
            Error::internal("delegated token prepare replay receipt is missing response bytes")
        })?;
        decode_one(response_bytes).map_err(|err| {
            Error::internal(format!(
                "failed to decode delegated token prepare replay response: {err}"
            ))
        })
    }

    fn map_role_attestation_replay_decision(
        decision: ReplayReceiptDecision,
    ) -> Result<RoleAttestationPrepareResponse, Error> {
        match decision {
            ReplayReceiptDecision::Fresh(_) => Err(Error::invariant(
                "fresh role attestation replay decision escaped",
            )),
            ReplayReceiptDecision::ReturnCommitted(receipt) => {
                Self::decode_role_attestation_prepare_response(&receipt)
            }
            ReplayReceiptDecision::OperationInProgress => Err(Error::conflict(
                "role attestation prepare request is already in progress; retry later with the same request id",
            )),
            ReplayReceiptDecision::ActorMismatch => Err(Error::conflict(
                "role attestation prepare request id was reused by a different caller",
            )),
            ReplayReceiptDecision::PayloadMismatch => Err(Error::conflict(
                "role attestation prepare request id was reused with a different payload",
            )),
            ReplayReceiptDecision::Expired => Err(Error::conflict(
                "role attestation prepare replay receipt expired; retry with a new request id",
            )),
            ReplayReceiptDecision::RecoveryRequired(reason) => Err(Error::conflict(format!(
                "role attestation prepare request requires recovery before replay: {reason:?}"
            ))),
            ReplayReceiptDecision::TerminalFailed {
                error_code,
                error_bytes,
                error_bytes_truncated,
            } => Err(Error::conflict(format!(
                "role attestation prepare request previously failed: {error_code:?}; error_bytes_len={}; truncated={error_bytes_truncated}",
                error_bytes.len()
            ))),
            ReplayReceiptDecision::PendingActorQuotaExceeded { max_pending, .. } => {
                Err(Error::exhausted(format!(
                    "role attestation prepare pending replay receipt quota exceeded for caller; max_pending={max_pending}"
                )))
            }
            ReplayReceiptDecision::PendingCommandQuotaExceeded { max_pending, .. } => {
                Err(Error::exhausted(format!(
                    "role attestation prepare pending replay receipt quota exceeded for command kind; max_pending={max_pending}"
                )))
            }
        }
    }

    fn map_role_attestation_replay_store_error(err: ReplayReceiptStoreError) -> Error {
        match err {
            ReplayReceiptStoreError::ReceiptDecodeFailed(message) => Error::internal(format!(
                "failed to decode role attestation replay receipt: {message}"
            )),
        }
    }

    fn encode_role_attestation_prepare_response(
        response: &RoleAttestationPrepareResponse,
    ) -> Result<Vec<u8>, Error> {
        encode_one(response).map_err(|err| {
            Error::internal(format!(
                "failed to encode role attestation prepare replay response: {err}"
            ))
        })
    }

    fn decode_role_attestation_prepare_response(
        receipt: &crate::ops::replay::model::ReplayReceipt,
    ) -> Result<RoleAttestationPrepareResponse, Error> {
        let response_schema_version = receipt.response_schema_version.ok_or_else(|| {
            Error::internal(
                "role attestation prepare replay receipt is missing response schema version",
            )
        })?;
        if response_schema_version != Self::ROLE_ATTESTATION_REPLAY_RESPONSE_SCHEMA_VERSION {
            return Err(Error::internal(format!(
                "unsupported role attestation prepare replay response schema version {response_schema_version}"
            )));
        }
        let response_bytes = receipt.response_bytes.as_deref().ok_or_else(|| {
            Error::internal("role attestation prepare replay receipt is missing response bytes")
        })?;
        decode_one(response_bytes).map_err(|err| {
            Error::internal(format!(
                "failed to decode role attestation prepare replay response: {err}"
            ))
        })
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
}

#[cfg(test)]
mod tests {
    use super::AuthApi;
    use crate::{
        cdk::types::Principal,
        dto::{
            auth::{
                AuthRequestMetadata, DelegatedRoleGrant, DelegatedTokenPrepareRequest,
                DelegationAudience, DelegationProofIssueRequest,
            },
            error::ErrorCode,
        },
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn delegation_request(metadata_id: u8) -> DelegationProofIssueRequest {
        DelegationProofIssueRequest {
            metadata: Some(meta(metadata_id, 60_000_000_000)),
            issuer_pid: p(2),
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

    fn token_prepare_request(metadata_id: u8) -> DelegatedTokenPrepareRequest {
        DelegatedTokenPrepareRequest {
            metadata: Some(meta(metadata_id, 60_000_000_000)),
            subject: p(8),
            aud: DelegationAudience::Project("test".to_string()),
            grants: vec![grant("project_instance", &["canic.verify"])],
            ttl_ns: 30_000_000_000,
            ext: None,
        }
    }

    #[test]
    fn delegation_request_caller_must_match_requested_issuer() {
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
    fn delegated_token_prepare_payload_hash_ignores_metadata() {
        let command_kind = AuthApi::token_prepare_replay_command_kind();
        let actor = crate::ops::replay::model::ReplayActor::direct_caller(p(2));
        let a = token_prepare_request(1);
        let b = token_prepare_request(9);

        assert_eq!(
            AuthApi::token_prepare_replay_payload_hash(&command_kind, &actor, &a),
            AuthApi::token_prepare_replay_payload_hash(&command_kind, &actor, &b)
        );
    }

    #[test]
    fn delegated_token_prepare_payload_hash_binds_authoritative_payload() {
        let command_kind = AuthApi::token_prepare_replay_command_kind();
        let actor = crate::ops::replay::model::ReplayActor::direct_caller(p(2));
        let a = token_prepare_request(1);
        let mut b = a.clone();
        b.ttl_ns += 1;

        assert_ne!(
            AuthApi::token_prepare_replay_payload_hash(&command_kind, &actor, &a),
            AuthApi::token_prepare_replay_payload_hash(&command_kind, &actor, &b)
        );
    }

    #[test]
    fn delegated_token_prepare_payload_hash_binds_ext() {
        let command_kind = AuthApi::token_prepare_replay_command_kind();
        let actor = crate::ops::replay::model::ReplayActor::direct_caller(p(2));
        let a = token_prepare_request(1);
        let mut b = a.clone();
        b.ext = Some(b"app-context".to_vec());

        assert_ne!(
            AuthApi::token_prepare_replay_payload_hash(&command_kind, &actor, &a),
            AuthApi::token_prepare_replay_payload_hash(&command_kind, &actor, &b)
        );
    }
}
