//! Module: workflow::runtime::auth::prepare
//!
//! Responsibility: prepare replay-protected auth proofs and delegated tokens.
//! Does not own: endpoint authorization, auth stable records, or crypto primitives.
//! Boundary: runtime auth workflow delegates proof creation to auth ops and replay ops.

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    domain::policy::auth::{AuthPolicyError, validate_public_delegated_token_prepare},
    dto::{
        auth::{
            AuthRequestMetadata, DelegatedRoleGrant, DelegatedTokenPrepareRequest,
            DelegatedTokenPrepareResponse, DelegationAudience, RoleAttestationPrepareResponse,
            RoleAttestationRequest,
        },
        error::Error,
        rpc::RootRequestMetadata,
    },
    ops::{
        auth::{AuthOps, PrepareDelegatedTokenIssuerProofInput, PrepareRootRoleAttestationInput},
        config::ConfigOps,
        ic::IcOps,
        replay::{
            self as replay_ops, DELEGATED_TOKEN_PREPARE_REPLAY_RESPONSE_SCHEMA_VERSION,
            ROLE_ATTESTATION_PREPARE_REPLAY_RESPONSE_SCHEMA_VERSION,
            guard::secs_to_ns,
            model::{
                CommandKind, OperationId, RecoveryReason, ReplayActor, ReplayPayloadHasher,
                ReplayReceipt,
            },
            receipt::{
                ReplayReceiptDecision, ReplayReceiptReserveInput, ReplayReceiptStoreError,
                abort_reserved_receipt, commit_receipt_response, mark_recovery_required,
                reserve_or_replay_receipt,
            },
        },
        runtime::env::EnvOps,
        storage::registry::subnet::SubnetRegistryOps,
    },
    workflow::runtime::auth::RuntimeAuthWorkflow,
};

const ROLE_ATTESTATION_REPLAY_COMMAND_KIND: &str = "auth.prepare_role_attestation.v1";
const MAX_ROLE_ATTESTATION_REPLAY_TTL_NS: u64 = 300_000_000_000;
const TOKEN_PREPARE_REPLAY_COMMAND_KIND: &str = "auth.prepare_delegated_token.v1";
const MAX_TOKEN_REPLAY_TTL_NS: u64 = 300_000_000_000;

impl RuntimeAuthWorkflow {
    /// Prepare a delegated token from issuer-local root-certified delegation material.
    pub fn prepare_delegated_token(
        request: DelegatedTokenPrepareRequest,
    ) -> Result<DelegatedTokenPrepareResponse, InternalError> {
        let label = "delegated token prepare";
        let metadata = token_replay_metadata(request.metadata, label)?;
        let caller = IcOps::msg_caller();
        validate_token_prepare_public_request(caller, &request)?;
        let command_kind = token_prepare_replay_command_kind();
        let actor = ReplayActor::direct_caller(caller);
        let payload_hash = token_prepare_replay_payload_hash(&command_kind, &actor, &request);
        let now_ns = IcOps::now_nanos();
        let replay_input = replay_reserve_input(
            command_kind,
            metadata.request_id,
            actor,
            payload_hash,
            now_ns,
            metadata.ttl_ns,
            "delegated token prepare replay metadata ttl_ns overflows nanoseconds",
        )?;

        let token = match reserve_or_replay_receipt(replay_input)
            .map_err(map_token_prepare_replay_store_error)?
        {
            ReplayReceiptDecision::Fresh(token) => token,
            decision => return map_token_prepare_replay_decision(decision),
        };

        let prepared = match AuthOps::prepare_delegated_token_issuer_proof(
            PrepareDelegatedTokenIssuerProofInput {
                subject: request.subject,
                audience: request.aud,
                grants: request.grants,
                ttl_ns: request.ttl_ns,
                ext: request.ext,
            },
            metadata.request_id,
            caller,
        ) {
            Ok(prepared) => prepared,
            Err(err) => {
                abort_reserved_receipt(&token);
                return Err(err);
            }
        };

        let response = DelegatedTokenPrepareResponse {
            claims: prepared.prepared.claims,
            claims_hash: prepared.claims_hash,
            retrieval_expires_at_ns: prepared.retrieval_expires_at_ns,
        };

        let response_bytes = match encode_token_prepare_response(&response) {
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
            DELEGATED_TOKEN_PREPARE_REPLAY_RESPONSE_SCHEMA_VERSION,
            response_bytes,
            secs_to_ns(IcOps::now_secs()),
        );
        Ok(response)
    }

    /// Prepare a root-certified role attestation from the local root update path.
    pub fn prepare_role_attestation_root(
        request: RoleAttestationRequest,
    ) -> Result<RoleAttestationPrepareResponse, InternalError> {
        EnvOps::require_root()?;
        let caller = IcOps::msg_caller();
        validate_role_attestation_request(caller, &request)?;
        let metadata = role_attestation_replay_metadata(request.metadata)?;
        let command_kind = role_attestation_replay_command_kind();
        let actor = ReplayActor::direct_caller(caller);
        let payload_hash = role_attestation_replay_payload_hash(&command_kind, &actor, &request);
        let now_ns = IcOps::now_nanos();
        let replay_input = replay_reserve_input(
            command_kind,
            metadata.request_id,
            actor,
            payload_hash,
            now_ns,
            metadata.ttl_ns,
            "role attestation replay metadata ttl_ns overflows nanoseconds",
        )?;

        let token = match reserve_or_replay_receipt(replay_input)
            .map_err(map_role_attestation_replay_store_error)?
        {
            ReplayReceiptDecision::Fresh(token) => token,
            decision => return map_role_attestation_replay_decision(decision),
        };

        let prepared = match AuthOps::prepare_role_attestation(PrepareRootRoleAttestationInput {
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
                return Err(err);
            }
        };

        let response = RoleAttestationPrepareResponse {
            payload: prepared.payload,
            payload_hash: prepared.payload_hash,
            retrieval_expires_at_ns: prepared.retrieval_expires_at_ns,
        };

        let response_bytes = match encode_role_attestation_prepare_response(&response) {
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
            ROLE_ATTESTATION_PREPARE_REPLAY_RESPONSE_SCHEMA_VERSION,
            response_bytes,
            secs_to_ns(IcOps::now_secs()),
        );
        Ok(response)
    }
}

fn validate_role_attestation_request(
    caller: Principal,
    request: &RoleAttestationRequest,
) -> Result<(), InternalError> {
    if request.subject != caller {
        return Err(InternalError::public(Error::forbidden(format!(
            "role attestation subject {} must match caller {}",
            request.subject, caller
        ))));
    }

    let (registered_role, _) =
        SubnetRegistryOps::role_parent(request.subject).ok_or_else(|| {
            InternalError::public(Error::forbidden(format!(
                "role attestation subject {} is not registered",
                request.subject
            )))
        })?;
    if registered_role != request.role {
        return Err(InternalError::public(Error::forbidden(format!(
            "role attestation role mismatch for subject {}: requested {}, registered {}",
            request.subject, request.role, registered_role
        ))));
    }

    if let Some(requested_subnet) = request.subnet_id {
        let local_subnet = EnvOps::subnet_pid()?;
        if requested_subnet != local_subnet {
            return Err(InternalError::public(Error::forbidden(format!(
                "role attestation subnet mismatch for subject {}: requested {}, local {}",
                request.subject, requested_subnet, local_subnet
            ))));
        }
    }

    let max_ttl_ns = role_attestation_max_ttl_ns()?;
    if request.ttl_ns == 0 || request.ttl_ns > max_ttl_ns {
        return Err(InternalError::public(Error::invalid(format!(
            "role attestation ttl_ns must satisfy 0 < ttl_ns <= {max_ttl_ns} (got {})",
            request.ttl_ns
        ))));
    }

    Ok(())
}

fn role_attestation_max_ttl_ns() -> Result<u64, InternalError> {
    let cfg = ConfigOps::role_attestation_config()?;
    cfg.max_ttl_secs.checked_mul(1_000_000_000).ok_or_else(|| {
        InternalError::public(Error::invalid(
            "auth.role_attestation.max_ttl_secs overflows nanoseconds",
        ))
    })
}

fn replay_reserve_input(
    command_kind: CommandKind,
    request_id: [u8; 32],
    actor: ReplayActor,
    payload_hash: [u8; 32],
    now_ns: u64,
    ttl_ns: u64,
    overflow_message: &'static str,
) -> Result<ReplayReceiptReserveInput, InternalError> {
    let expires_at_ns = now_ns
        .checked_add(ttl_ns)
        .ok_or_else(|| InternalError::public(Error::invalid(overflow_message)))?;
    Ok(ReplayReceiptReserveInput::new(
        command_kind,
        OperationId::from_bytes(request_id),
        actor,
        payload_hash,
        now_ns,
    )
    .with_expires_at_ns(expires_at_ns))
}

fn role_attestation_replay_metadata(
    metadata: Option<RootRequestMetadata>,
) -> Result<RootRequestMetadata, InternalError> {
    let metadata = metadata.ok_or_else(|| InternalError::public(Error::operation_id_required()))?;
    if metadata.ttl_ns == 0 {
        return Err(InternalError::public(Error::invalid(
            "role attestation replay metadata ttl_ns must be greater than zero",
        )));
    }
    if metadata.ttl_ns > MAX_ROLE_ATTESTATION_REPLAY_TTL_NS {
        return Err(InternalError::public(Error::invalid(format!(
            "role attestation replay metadata ttl_ns={} exceeds max {}",
            metadata.ttl_ns, MAX_ROLE_ATTESTATION_REPLAY_TTL_NS
        ))));
    }
    Ok(metadata)
}

fn token_replay_metadata(
    metadata: Option<AuthRequestMetadata>,
    label: &str,
) -> Result<AuthRequestMetadata, InternalError> {
    let metadata = metadata.ok_or_else(|| InternalError::public(Error::operation_id_required()))?;
    if metadata.ttl_ns == 0 {
        return Err(InternalError::public(Error::invalid(format!(
            "{label} replay metadata ttl_ns must be greater than zero"
        ))));
    }
    if metadata.ttl_ns > MAX_TOKEN_REPLAY_TTL_NS {
        return Err(InternalError::public(Error::invalid(format!(
            "{label} replay metadata ttl_ns={} exceeds max {}",
            metadata.ttl_ns, MAX_TOKEN_REPLAY_TTL_NS
        ))));
    }
    Ok(metadata)
}

fn validate_token_prepare_public_request(
    caller: Principal,
    request: &DelegatedTokenPrepareRequest,
) -> Result<(), InternalError> {
    validate_public_delegated_token_prepare(caller, request.subject, &request.grants)
        .map_err(map_token_prepare_policy_error)
}

fn map_token_prepare_policy_error(err: AuthPolicyError) -> InternalError {
    InternalError::public(Error::forbidden(err.to_string()))
}

fn role_attestation_replay_command_kind() -> CommandKind {
    CommandKind::new(ROLE_ATTESTATION_REPLAY_COMMAND_KIND)
        .expect("role attestation replay command kind is a valid static label")
}

fn token_prepare_replay_command_kind() -> CommandKind {
    CommandKind::new(TOKEN_PREPARE_REPLAY_COMMAND_KIND)
        .expect("delegated-token prepare replay command kind is a valid static label")
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
    hash_delegation_audience(&mut hasher, &request.aud);
    hash_delegated_role_grants(&mut hasher, &request.grants);
    hasher.hash_u64(request.ttl_ns);
    hash_optional_bytes(&mut hasher, request.ext.as_deref());
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
        hash_string_vec(hasher, &grant.scopes);
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
) -> Result<DelegatedTokenPrepareResponse, InternalError> {
    match decision {
        ReplayReceiptDecision::Fresh(_) => Err(InternalError::invariant(
            InternalErrorOrigin::Workflow,
            "fresh delegated token replay decision escaped",
        )),
        ReplayReceiptDecision::ReturnCommitted(receipt) => decode_token_prepare_response(&receipt),
        ReplayReceiptDecision::OperationInProgress => Err(InternalError::public(Error::conflict(
            "delegated token prepare request is already in progress; retry later with the same request id",
        ))),
        ReplayReceiptDecision::ActorMismatch => Err(InternalError::public(Error::conflict(
            "delegated token prepare request id was reused by a different caller",
        ))),
        ReplayReceiptDecision::PayloadMismatch => Err(InternalError::public(Error::conflict(
            "delegated token prepare request id was reused with a different payload",
        ))),
        ReplayReceiptDecision::Expired => Err(InternalError::public(Error::conflict(
            "delegated token prepare replay receipt expired; retry with a new request id",
        ))),
        ReplayReceiptDecision::RecoveryRequired(reason) => {
            Err(InternalError::public(Error::conflict(format!(
                "delegated token prepare request requires recovery before replay: {reason:?}"
            ))))
        }
        ReplayReceiptDecision::TerminalFailed {
            error_code,
            error_bytes,
            error_bytes_truncated,
        } => Err(InternalError::public(Error::conflict(format!(
            "delegated token prepare request previously failed: {error_code:?}; error_bytes_len={}; truncated={error_bytes_truncated}",
            error_bytes.len()
        )))),
        ReplayReceiptDecision::PendingActorQuotaExceeded { max_pending, .. } => {
            Err(InternalError::public(Error::exhausted(format!(
                "delegated token prepare pending replay receipt quota exceeded for caller; max_pending={max_pending}"
            ))))
        }
        ReplayReceiptDecision::PendingCommandQuotaExceeded { max_pending, .. } => {
            Err(InternalError::public(Error::exhausted(format!(
                "delegated token prepare pending replay receipt quota exceeded for command kind; max_pending={max_pending}"
            ))))
        }
    }
}

fn map_token_prepare_replay_store_error(err: ReplayReceiptStoreError) -> InternalError {
    match err {
        ReplayReceiptStoreError::ReceiptDecodeFailed(message) => InternalError::workflow(
            InternalErrorOrigin::Workflow,
            format!("failed to decode delegated token prepare replay receipt: {message}"),
        ),
    }
}

fn encode_token_prepare_response(
    response: &DelegatedTokenPrepareResponse,
) -> Result<Vec<u8>, InternalError> {
    replay_ops::encode_delegated_token_prepare_replay_response(response).map_err(|err| match err {
        replay_ops::ReplayCommitError::EncodeFailed(message) => InternalError::workflow(
            InternalErrorOrigin::Workflow,
            format!("failed to encode delegated token prepare replay response: {message}"),
        ),
    })
}

fn decode_token_prepare_response(
    receipt: &ReplayReceipt,
) -> Result<DelegatedTokenPrepareResponse, InternalError> {
    replay_ops::decode_delegated_token_prepare_replay_response(receipt).map_err(|err| match err {
        replay_ops::ReplayDecodeError::DecodeFailed(message) => {
            InternalError::workflow(InternalErrorOrigin::Workflow, message)
        }
    })
}

fn map_role_attestation_replay_decision(
    decision: ReplayReceiptDecision,
) -> Result<RoleAttestationPrepareResponse, InternalError> {
    match decision {
        ReplayReceiptDecision::Fresh(_) => Err(InternalError::invariant(
            InternalErrorOrigin::Workflow,
            "fresh role attestation replay decision escaped",
        )),
        ReplayReceiptDecision::ReturnCommitted(receipt) => {
            decode_role_attestation_prepare_response(&receipt)
        }
        ReplayReceiptDecision::OperationInProgress => Err(InternalError::public(Error::conflict(
            "role attestation prepare request is already in progress; retry later with the same request id",
        ))),
        ReplayReceiptDecision::ActorMismatch => Err(InternalError::public(Error::conflict(
            "role attestation prepare request id was reused by a different caller",
        ))),
        ReplayReceiptDecision::PayloadMismatch => Err(InternalError::public(Error::conflict(
            "role attestation prepare request id was reused with a different payload",
        ))),
        ReplayReceiptDecision::Expired => Err(InternalError::public(Error::conflict(
            "role attestation prepare replay receipt expired; retry with a new request id",
        ))),
        ReplayReceiptDecision::RecoveryRequired(reason) => {
            Err(InternalError::public(Error::conflict(format!(
                "role attestation prepare request requires recovery before replay: {reason:?}"
            ))))
        }
        ReplayReceiptDecision::TerminalFailed {
            error_code,
            error_bytes,
            error_bytes_truncated,
        } => Err(InternalError::public(Error::conflict(format!(
            "role attestation prepare request previously failed: {error_code:?}; error_bytes_len={}; truncated={error_bytes_truncated}",
            error_bytes.len()
        )))),
        ReplayReceiptDecision::PendingActorQuotaExceeded { max_pending, .. } => {
            Err(InternalError::public(Error::exhausted(format!(
                "role attestation prepare pending replay receipt quota exceeded for caller; max_pending={max_pending}"
            ))))
        }
        ReplayReceiptDecision::PendingCommandQuotaExceeded { max_pending, .. } => {
            Err(InternalError::public(Error::exhausted(format!(
                "role attestation prepare pending replay receipt quota exceeded for command kind; max_pending={max_pending}"
            ))))
        }
    }
}

fn map_role_attestation_replay_store_error(err: ReplayReceiptStoreError) -> InternalError {
    match err {
        ReplayReceiptStoreError::ReceiptDecodeFailed(message) => InternalError::workflow(
            InternalErrorOrigin::Workflow,
            format!("failed to decode role attestation replay receipt: {message}"),
        ),
    }
}

fn encode_role_attestation_prepare_response(
    response: &RoleAttestationPrepareResponse,
) -> Result<Vec<u8>, InternalError> {
    replay_ops::encode_role_attestation_prepare_replay_response(response).map_err(|err| match err {
        replay_ops::ReplayCommitError::EncodeFailed(message) => InternalError::workflow(
            InternalErrorOrigin::Workflow,
            format!("failed to encode role attestation prepare replay response: {message}"),
        ),
    })
}

fn decode_role_attestation_prepare_response(
    receipt: &ReplayReceipt,
) -> Result<RoleAttestationPrepareResponse, InternalError> {
    replay_ops::decode_role_attestation_prepare_replay_response(receipt).map_err(|err| match err {
        replay_ops::ReplayDecodeError::DecodeFailed(message) => {
            InternalError::workflow(InternalErrorOrigin::Workflow, message)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::error::ErrorCode,
        ids::{CanisterRole, cap},
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn grant(role: &str, scopes: &[&str]) -> DelegatedRoleGrant {
        DelegatedRoleGrant {
            target: CanisterRole::owned(role.to_string()),
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
    fn delegated_token_replay_metadata_rejects_missing_or_invalid_ttl() {
        let missing = token_replay_metadata(None, "delegated token prepare").expect_err("required");
        assert_eq!(
            missing.public_error().expect("public error").code,
            ErrorCode::OperationIdRequired
        );

        let zero = token_replay_metadata(Some(meta(1, 0)), "delegated token prepare")
            .expect_err("zero ttl is invalid");
        assert_eq!(
            zero.public_error().expect("public error").code,
            ErrorCode::InvalidInput
        );

        let too_large = token_replay_metadata(
            Some(meta(1, MAX_TOKEN_REPLAY_TTL_NS + 1)),
            "delegated token prepare",
        )
        .expect_err("oversized ttl is invalid");
        assert_eq!(
            too_large.public_error().expect("public error").code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn delegated_token_public_prepare_rejects_subject_mismatch_before_replay() {
        let mut request = token_prepare_request(1);
        request.subject = p(9);
        request.grants = vec![grant("project_instance", &[cap::SESSION])];

        let err = validate_token_prepare_public_request(p(8), &request)
            .expect_err("subject mismatch must fail");

        assert_eq!(
            err.public_error().expect("public error").code,
            ErrorCode::Forbidden
        );
    }

    #[test]
    fn delegated_token_public_prepare_rejects_privileged_self_grants_before_replay() {
        let mut request = token_prepare_request(1);
        request.grants = vec![grant("project_instance", &[cap::WRITE])];

        let err = validate_token_prepare_public_request(p(8), &request)
            .expect_err("privileged self-grant must fail");

        assert_eq!(
            err.public_error().expect("public error").code,
            ErrorCode::Forbidden
        );
    }

    #[test]
    fn delegated_token_public_prepare_accepts_login_scopes_before_replay() {
        let mut request = token_prepare_request(1);
        request.grants = vec![
            grant("project_hub", &[cap::SESSION]),
            grant("project_instance", &[cap::VERIFY]),
        ];

        validate_token_prepare_public_request(p(8), &request).expect("login scopes");
    }

    #[test]
    fn delegated_token_prepare_payload_hash_ignores_metadata() {
        let command_kind = token_prepare_replay_command_kind();
        let actor = ReplayActor::direct_caller(p(2));
        let a = token_prepare_request(1);
        let b = token_prepare_request(9);

        assert_eq!(
            token_prepare_replay_payload_hash(&command_kind, &actor, &a),
            token_prepare_replay_payload_hash(&command_kind, &actor, &b)
        );
    }

    #[test]
    fn delegated_token_prepare_payload_hash_binds_authoritative_payload() {
        let command_kind = token_prepare_replay_command_kind();
        let actor = ReplayActor::direct_caller(p(2));
        let a = token_prepare_request(1);
        let mut b = a.clone();
        b.ttl_ns += 1;

        assert_ne!(
            token_prepare_replay_payload_hash(&command_kind, &actor, &a),
            token_prepare_replay_payload_hash(&command_kind, &actor, &b)
        );
    }

    #[test]
    fn delegated_token_prepare_payload_hash_binds_ext() {
        let command_kind = token_prepare_replay_command_kind();
        let actor = ReplayActor::direct_caller(p(2));
        let a = token_prepare_request(1);
        let mut b = a.clone();
        b.ext = Some(b"app-context".to_vec());

        assert_ne!(
            token_prepare_replay_payload_hash(&command_kind, &actor, &a),
            token_prepare_replay_payload_hash(&command_kind, &actor, &b)
        );
    }
}
