//! Module: workflow::runtime::auth::prepare
//!
//! Responsibility: prepare replay-protected role attestations.
//! Does not own: endpoint authorization, auth stable records, or crypto primitives.
//! Boundary: runtime auth workflow delegates proof creation to auth ops and replay ops.

mod admission;
mod replay;

use crate::{
    InternalError,
    dto::auth::{
        RoleAttestationGetRequest, RoleAttestationPrepareResponse, RoleAttestationRequest,
        SignedRoleAttestation,
    },
    model::replay::RecoveryReason,
    model::replay::ReplayActor,
    ops::{
        auth::{AuthOps, PrepareRootRoleAttestationInput},
        ic::IcOps,
        replay::{
            ROLE_ATTESTATION_PREPARE_REPLAY_RESPONSE_SCHEMA_VERSION,
            receipt::{
                ReplayReceiptDecision, ReplayReceiptToken, commit_staged_receipt_response,
                mark_recovery_required, reserve_or_replay_receipt, stage_receipt_response,
            },
        },
        runtime::env::EnvOps,
    },
    workflow::{replay::abort_reserved_receipt_after_failure, runtime::auth::RuntimeAuthWorkflow},
};
use admission::validate_role_attestation_request;
use replay::{
    encode_role_attestation_prepare_response, map_role_attestation_replay_decision,
    replay_reserve_input, role_attestation_replay_command_kind, role_attestation_replay_metadata,
    role_attestation_replay_payload_hash,
};

impl RuntimeAuthWorkflow {
    /// Prepare a root-certified role attestation from the local root update path.
    pub fn prepare_component_role_attestation_root(
        request: RoleAttestationRequest,
        member: &crate::ids::ManagedCanisterBinding,
    ) -> Result<RoleAttestationPrepareResponse, InternalError> {
        EnvOps::require_root()?;
        let caller = IcOps::msg_caller();
        validate_role_attestation_request(caller, &request, member)?;
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
        )?;

        let token = match reserve_or_replay_receipt(replay_input).map_err(InternalError::from)? {
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
                return Err(abort_reserved_receipt_after_failure(
                    &token,
                    err,
                    "role attestation replay reservation cleanup failed",
                ));
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
                return Err(preserve_auth_response_failure(&token, err));
            }
        };

        commit_auth_prepare_response(
            &token,
            ROLE_ATTESTATION_PREPARE_REPLAY_RESPONSE_SCHEMA_VERSION,
            response_bytes,
        )?;
        Ok(response)
    }

    /// Retrieve a prepared root-certified attestation for its calling Component.
    pub fn get_role_attestation_root(
        request: RoleAttestationGetRequest,
    ) -> Result<SignedRoleAttestation, InternalError> {
        EnvOps::require_root()?;
        AuthOps::get_role_attestation(IcOps::msg_caller(), request.payload_hash)
    }
}

fn commit_auth_prepare_response(
    token: &ReplayReceiptToken,
    response_schema_version: u32,
    response_bytes: Vec<u8>,
) -> Result<(), InternalError> {
    if let Err(err) = stage_receipt_response(
        token,
        response_schema_version,
        response_bytes,
        IcOps::now_nanos(),
    ) {
        return Err(preserve_auth_response_failure(
            token,
            InternalError::from(err),
        ));
    }
    if let Err(err) = commit_staged_receipt_response(token, IcOps::now_nanos()) {
        return Err(preserve_auth_response_failure(
            token,
            InternalError::from(err),
        ));
    }
    Ok(())
}

fn preserve_auth_response_failure(token: &ReplayReceiptToken, err: InternalError) -> InternalError {
    let _ = mark_recovery_required(
        token,
        RecoveryReason::ResponseCommitFailed,
        IcOps::now_nanos(),
    )
    .map_err(InternalError::from);
    err
}
