//! Module: workflow::runtime::auth::prepare::replay
//!
//! Responsibility: own prepare-request replay identity, decisions, and response encoding.
//! Does not own: request admission, proof creation, or replay storage mutation.
//! Boundary: deterministic adapter between auth prepare workflows and replay ops.

use crate::{
    InternalError,
    dto::{
        auth::{RoleAttestationPrepareResponse, RoleAttestationRequest},
        rpc::RootRequestMetadata,
    },
    model::replay::{CommandKind, OperationId, ReplayActor, ReplayPayloadHasher, ReplayReceipt},
    ops::{
        replay as replay_ops,
        replay::receipt::{
            ReplayReceiptDecision, ReplayReceiptReserveInput, ReplayReceiptToken,
            commit_staged_receipt_response,
        },
    },
};

const ROLE_ATTESTATION_REPLAY_COMMAND_KIND: &str = "auth.prepare_role_attestation.v1";
const MAX_ROLE_ATTESTATION_REPLAY_TTL_NS: u64 = 300_000_000_000;

pub(super) fn replay_reserve_input(
    command_kind: CommandKind,
    request_id: [u8; 32],
    actor: ReplayActor,
    payload_hash: [u8; 32],
    now_ns: u64,
    ttl_ns: u64,
) -> Result<ReplayReceiptReserveInput, InternalError> {
    let expires_at_ns = now_ns
        .checked_add(ttl_ns)
        .ok_or_else(|| InternalError::public(crate::diagnostics::codes::TIME_CAPACITY))?;
    Ok(ReplayReceiptReserveInput::new(
        command_kind,
        OperationId::from_bytes(request_id),
        actor,
        payload_hash,
        now_ns,
    )
    .with_expires_at_ns(expires_at_ns))
}

pub(super) fn role_attestation_replay_metadata(
    metadata: Option<RootRequestMetadata>,
) -> Result<RootRequestMetadata, InternalError> {
    let metadata = metadata
        .ok_or_else(|| InternalError::public(crate::diagnostics::codes::AUTHORITY_UNAVAILABLE))?;
    if metadata.ttl_ns == 0 {
        return Err(InternalError::public(
            crate::diagnostics::codes::TIME_INVALID,
        ));
    }
    if metadata.ttl_ns > MAX_ROLE_ATTESTATION_REPLAY_TTL_NS {
        return Err(InternalError::public(
            crate::diagnostics::codes::TIME_CAPACITY,
        ));
    }
    Ok(metadata)
}

pub(super) fn role_attestation_replay_command_kind() -> CommandKind {
    CommandKind::new(ROLE_ATTESTATION_REPLAY_COMMAND_KIND)
        .expect("role attestation replay command kind is a valid static label")
}

pub(super) fn role_attestation_replay_payload_hash(
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

pub(super) fn map_role_attestation_replay_decision(
    decision: ReplayReceiptDecision,
) -> Result<RoleAttestationPrepareResponse, InternalError> {
    match decision {
        ReplayReceiptDecision::Fresh(_) => Err(InternalError::public(
            crate::diagnostics::codes::REQUEST_INVALID_STATE,
        )),
        ReplayReceiptDecision::ReturnCommitted(receipt) => {
            decode_role_attestation_prepare_response(&receipt)
        }
        ReplayReceiptDecision::RecoveryRequired {
            token,
            reason: crate::model::replay::RecoveryReason::ResponseCommitFailed,
        } => recover_role_attestation_prepare_response(&token),
        ReplayReceiptDecision::OperationInProgress => Err(InternalError::public(
            crate::diagnostics::codes::REQUEST_INCOMPLETE,
        )),
        ReplayReceiptDecision::ActorMismatch => Err(InternalError::public(
            crate::diagnostics::codes::AUTHORITY_CONFLICT,
        )),
        ReplayReceiptDecision::PayloadMismatch => Err(InternalError::public(
            crate::diagnostics::codes::CODEC_CONFLICT,
        )),
        ReplayReceiptDecision::Expired => Err(InternalError::public(
            crate::diagnostics::codes::EVIDENCE_EXPIRED,
        )),
        ReplayReceiptDecision::RecoveryRequired { .. } => Err(InternalError::public(
            crate::diagnostics::codes::REQUEST_INVALID,
        )),
        ReplayReceiptDecision::PendingActorQuotaExceeded { .. } => Err(InternalError::public(
            crate::diagnostics::codes::AUTHORITY_CAPACITY,
        )),
        ReplayReceiptDecision::PendingCommandQuotaExceeded { .. } => Err(InternalError::public(
            crate::diagnostics::codes::REQUEST_CAPACITY,
        )),
    }
}

pub(super) fn encode_role_attestation_prepare_response(
    response: &RoleAttestationPrepareResponse,
) -> Result<Vec<u8>, InternalError> {
    replay_ops::encode_replay_response(response).map_err(|err| match err {
        replay_ops::ReplayCommitError::EncodeFailed(_) => {
            InternalError::public(crate::diagnostics::codes::CODEC_FAILED)
        }
    })
}

fn decode_role_attestation_prepare_response(
    receipt: &ReplayReceipt,
) -> Result<RoleAttestationPrepareResponse, InternalError> {
    replay_ops::decode_role_attestation_prepare_replay_response(receipt).map_err(|err| match err {
        replay_ops::ReplayDecodeError::DecodeFailed(_) => {
            InternalError::public(crate::diagnostics::codes::CODEC_FAILED)
        }
    })
}

fn recover_role_attestation_prepare_response(
    token: &ReplayReceiptToken,
) -> Result<RoleAttestationPrepareResponse, InternalError> {
    let receipt = commit_staged_receipt_response(token, crate::ops::ic::IcOps::now_nanos())
        .map_err(InternalError::from)?;
    decode_role_attestation_prepare_response(&receipt)
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::auth::RoleAttestation,
        model::replay::{RecoveryReason, ReplayReceiptStatus},
        ops::{
            replay::receipt::{
                mark_recovery_required, reserve_or_replay_receipt, stage_receipt_response,
            },
            storage::replay::ReplayReceiptOps,
        },
    };
    use canic_contracts::ids::CanisterRole;

    fn p(id: u8) -> crate::cdk::types::Principal {
        crate::cdk::types::Principal::from_slice(&[id; 29])
    }

    fn replay_input(
        command_kind: CommandKind,
        operation_id: [u8; 32],
    ) -> ReplayReceiptReserveInput {
        ReplayReceiptReserveInput::new(
            command_kind,
            OperationId::from_bytes(operation_id),
            ReplayActor::direct_caller(p(1)),
            [9; 32],
            100,
        )
        .with_expires_at_ns(1_000)
    }

    #[test]
    fn role_attestation_response_commit_retry_promotes_staged_response() {
        ReplayReceiptOps::reset_for_tests();
        let input = replay_input(role_attestation_replay_command_kind(), [42; 32]);
        let token = match reserve_or_replay_receipt(input.clone()).expect("reserve") {
            ReplayReceiptDecision::Fresh(token) => token,
            other => panic!("expected fresh receipt, got {other:?}"),
        };
        let response = RoleAttestationPrepareResponse {
            payload: RoleAttestation {
                subject: p(2),
                role: CanisterRole::new("project_hub"),
                subnet_id: Some(p(3)),
                audience: p(4),
                issued_at_ns: 100,
                expires_at_ns: 200,
                epoch: 1,
            },
            payload_hash: [5; 32],
            retrieval_expires_at_ns: 200,
        };
        stage_receipt_response(
            &token,
            replay_ops::ROLE_ATTESTATION_PREPARE_REPLAY_RESPONSE_SCHEMA_VERSION,
            encode_role_attestation_prepare_response(&response).expect("encode response"),
            110,
        )
        .expect("stage response");
        mark_recovery_required(&token, RecoveryReason::ResponseCommitFailed, 120)
            .expect("mark recovery");

        let decision = reserve_or_replay_receipt(input).expect("retry decision");
        let recovered = map_role_attestation_replay_decision(decision).expect("recover response");
        assert_eq!(recovered, response);
        let receipt = ReplayReceiptOps::get(token.key())
            .expect("receipt")
            .into_receipt()
            .expect("receipt decodes");
        assert_eq!(receipt.status, ReplayReceiptStatus::Committed);

        ReplayReceiptOps::reset_for_tests();
    }
}
