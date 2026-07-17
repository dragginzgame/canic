//! Module: ops::replay
//!
//! Responsibility: provide mechanical replay reservation and response helpers.
//! Does not own: authorization, command policy, or command execution.
//! Boundary: workflow calls replay ops after deciding which command is protected.

pub mod guard;
pub mod receipt;
pub mod ttl;

use crate::{
    cdk::types::Principal,
    dto::{
        auth::{DelegatedTokenPrepareResponse, RoleAttestationPrepareResponse},
        icp_refill::IcpRefillResponse,
        pool::PoolAdminResponse,
        rpc::{CyclesResponse, Response},
    },
    model::replay::{ExternalEffectDescriptor, RecoveryReason, ReplayActor, ReplayReceipt},
    ops::replay::{
        guard::ReplayPending,
        receipt::{
            ReplayReceiptStoreError, abort_reserved_receipt, commit_staged_receipt_response,
            mark_costed_external_effect_in_flight, mark_recovery_required,
            replay_cost_guard_settlement, reserve_receipt_token, stage_receipt_response,
        },
    },
    ops::storage::replay::ReplayReceiptOps,
};
use candid::{decode_one, encode_one};

pub const DELEGATED_TOKEN_PREPARE_REPLAY_RESPONSE_SCHEMA_VERSION: u32 = 1;
pub const ICP_REFILL_REPLAY_RESPONSE_SCHEMA_VERSION: u32 = 1;
pub const POOL_CREATE_EMPTY_REPLAY_RESPONSE_SCHEMA_VERSION: u32 = 1;
pub const ROLE_ATTESTATION_PREPARE_REPLAY_RESPONSE_SCHEMA_VERSION: u32 = 1;

const ROOT_REPLAY_COMPACT_TAG: &[u8] = b"RR2";
const ROOT_REPLAY_COMPACT_CYCLES_V1: u8 = 0;
const ROOT_REPLAY_RESPONSE_SCHEMA_VERSION: u32 = 1;

///
/// ReplayReserveError
///
/// Mechanical replay-reservation failures surfaced by ops replay reservation APIs.
/// Owned by replay ops and mapped by workflow callers into public errors.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayReserveError {
    CapacityReached {
        max_entries: usize,
    },
    CallerCapacityReached {
        caller: Principal,
        max_entries: usize,
    },
}

///
/// ReplayCommitError
///
/// Mechanical replay-commit failures surfaced by ops replay commit APIs.
/// Owned by replay ops and returned when response serialization fails.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayCommitError {
    EncodeFailed(String),
}

///
/// ReplayFinalizeError
///
/// Typed response-encoding and receipt-store failures for one replay commit.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayFinalizeError {
    Encode(ReplayCommitError),
    Store(ReplayReceiptStoreError),
}

///
/// ReplayDecodeError
///
/// Mechanical replay-decode failures surfaced by cached replay readers.
/// Owned by replay ops and mapped by workflow replay adapters.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayDecodeError {
    DecodeFailed(String),
}

/// reserve_root_replay
///
/// Persist a pending replay reservation marker before capability execution.
pub fn reserve_root_replay(
    pending: &ReplayPending,
    max_entries: usize,
    max_entries_per_caller: usize,
) -> Result<(), ReplayReserveError> {
    if ReplayReceiptOps::active_len_for_actor(
        ReplayActor::direct_caller(pending.caller),
        pending.issued_at_ns,
    ) >= max_entries_per_caller
    {
        return Err(ReplayReserveError::CallerCapacityReached {
            caller: pending.caller,
            max_entries: max_entries_per_caller,
        });
    }

    if ReplayReceiptOps::len() >= max_entries {
        return Err(ReplayReserveError::CapacityReached { max_entries });
    }

    reserve_receipt_token(&pending.receipt_token);
    Ok(())
}

/// Persist a root external-effect boundary together with its durable cost settlement identity.
pub fn mark_root_replay_costed_external_effect(
    pending: &ReplayPending,
    effect: ExternalEffectDescriptor,
    cost_permit: &crate::ops::cost_guard::CostGuardPermit,
    now_ns: u64,
) -> Result<(), ReplayReceiptStoreError> {
    mark_costed_external_effect_in_flight(
        &pending.receipt_token,
        effect,
        cost_permit.replay_settlement(),
        now_ns,
    )
}

/// Encode and stage a root response before cost settlement is attempted.
pub fn stage_root_replay_response(
    pending: &ReplayPending,
    response: &Response,
    now_ns: u64,
) -> Result<(), ReplayFinalizeError> {
    let response_bytes =
        encode_root_replay_response(response).map_err(ReplayFinalizeError::Encode)?;
    stage_receipt_response(
        &pending.receipt_token,
        ROOT_REPLAY_RESPONSE_SCHEMA_VERSION,
        response_bytes,
        now_ns,
    )
    .map_err(ReplayFinalizeError::Store)
}

/// Return the durable cost settlement identity for one root replay receipt.
pub fn root_replay_cost_guard_settlement(
    pending: &ReplayPending,
) -> Result<crate::model::replay::ReplayCostGuardSettlement, ReplayReceiptStoreError> {
    replay_cost_guard_settlement(&pending.receipt_token)
}

/// Promote a staged root response after durable cost settlement succeeds.
pub fn commit_staged_root_replay_response(
    pending: &ReplayPending,
    now_ns: u64,
) -> Result<ReplayReceipt, ReplayReceiptStoreError> {
    commit_staged_receipt_response(&pending.receipt_token, now_ns)
}

/// mark_root_replay_recovery_required
///
/// Preserve a replay receipt after an expensive external-effect boundary became uncertain.
pub fn mark_root_replay_recovery_required(
    pending: &ReplayPending,
    reason: RecoveryReason,
    now_ns: u64,
) -> Result<(), ReplayReceiptStoreError> {
    mark_recovery_required(&pending.receipt_token, reason, now_ns)
}

/// decode_root_replay_response
///
/// Decode cached replay bytes back into the canonical root response payload.
pub fn decode_root_replay_response(bytes: &[u8]) -> Result<Response, ReplayDecodeError> {
    if let Some(response) = try_decode_compact_root_replay_response(bytes)? {
        return Ok(response);
    }

    decode_one(bytes).map_err(|err| ReplayDecodeError::DecodeFailed(err.to_string()))
}

/// encode_delegated_token_prepare_replay_response
///
/// Encode the delegated-token prepare response payload stored in shared replay receipts.
pub fn encode_delegated_token_prepare_replay_response(
    response: &DelegatedTokenPrepareResponse,
) -> Result<Vec<u8>, ReplayCommitError> {
    encode_one(response).map_err(|err| ReplayCommitError::EncodeFailed(err.to_string()))
}

/// decode_delegated_token_prepare_replay_response
///
/// Decode a committed delegated-token prepare response from shared replay receipts.
pub fn decode_delegated_token_prepare_replay_response(
    receipt: &ReplayReceipt,
) -> Result<DelegatedTokenPrepareResponse, ReplayDecodeError> {
    let response_bytes = committed_response_bytes(
        receipt,
        DELEGATED_TOKEN_PREPARE_REPLAY_RESPONSE_SCHEMA_VERSION,
        "delegated token prepare",
    )?;
    decode_one(response_bytes).map_err(|err| {
        ReplayDecodeError::DecodeFailed(format!(
            "failed to decode delegated token prepare replay response: {err}"
        ))
    })
}

/// encode_icp_refill_replay_response
///
/// Encode the ICP refill response payload stored in shared replay receipts.
pub fn encode_icp_refill_replay_response(
    response: &IcpRefillResponse,
) -> Result<Vec<u8>, ReplayCommitError> {
    encode_one(response).map_err(|err| ReplayCommitError::EncodeFailed(err.to_string()))
}

/// decode_icp_refill_replay_response
///
/// Decode a committed ICP refill response payload from shared replay receipts.
pub fn decode_icp_refill_replay_response(
    receipt: &ReplayReceipt,
) -> Result<IcpRefillResponse, ReplayDecodeError> {
    let response_bytes = committed_response_bytes(
        receipt,
        ICP_REFILL_REPLAY_RESPONSE_SCHEMA_VERSION,
        "ICP refill",
    )?;
    decode_one(response_bytes).map_err(|err| {
        ReplayDecodeError::DecodeFailed(format!(
            "failed to decode ICP refill replay response: {err}"
        ))
    })
}

/// encode_pool_create_empty_replay_response
///
/// Encode the pool create-empty response payload stored in shared replay receipts.
pub fn encode_pool_create_empty_replay_response(
    response: &PoolAdminResponse,
) -> Result<Vec<u8>, ReplayCommitError> {
    encode_one(response).map_err(|err| ReplayCommitError::EncodeFailed(err.to_string()))
}

/// decode_pool_create_empty_replay_response
///
/// Decode a committed pool create-empty response from shared replay receipts.
pub fn decode_pool_create_empty_replay_response(
    receipt: &ReplayReceipt,
) -> Result<Principal, ReplayDecodeError> {
    let response_bytes = committed_response_bytes(
        receipt,
        POOL_CREATE_EMPTY_REPLAY_RESPONSE_SCHEMA_VERSION,
        "pool create-empty",
    )?;
    let response: PoolAdminResponse = decode_one(response_bytes).map_err(|err| {
        ReplayDecodeError::DecodeFailed(format!(
            "failed to decode pool create-empty replay response: {err}"
        ))
    })?;
    match response {
        PoolAdminResponse::Created { pid } => Ok(pid),
        _ => Err(ReplayDecodeError::DecodeFailed(
            "pool create-empty replay receipt contains the wrong response variant".to_string(),
        )),
    }
}

/// encode_role_attestation_prepare_replay_response
///
/// Encode the role-attestation prepare response payload stored in shared replay receipts.
pub fn encode_role_attestation_prepare_replay_response(
    response: &RoleAttestationPrepareResponse,
) -> Result<Vec<u8>, ReplayCommitError> {
    encode_one(response).map_err(|err| ReplayCommitError::EncodeFailed(err.to_string()))
}

/// decode_role_attestation_prepare_replay_response
///
/// Decode a committed role-attestation prepare response from shared replay receipts.
pub fn decode_role_attestation_prepare_replay_response(
    receipt: &ReplayReceipt,
) -> Result<RoleAttestationPrepareResponse, ReplayDecodeError> {
    let response_bytes = committed_response_bytes(
        receipt,
        ROLE_ATTESTATION_PREPARE_REPLAY_RESPONSE_SCHEMA_VERSION,
        "role attestation prepare",
    )?;
    decode_one(response_bytes).map_err(|err| {
        ReplayDecodeError::DecodeFailed(format!(
            "failed to decode role attestation prepare replay response: {err}"
        ))
    })
}

/// abort_root_replay
///
/// Remove an in-flight replay reservation after failed capability execution.
pub fn abort_root_replay(pending: ReplayPending) -> Result<(), ReplayReceiptStoreError> {
    abort_reserved_receipt(&pending.receipt_token)
}

fn encode_root_replay_response(response: &Response) -> Result<Vec<u8>, ReplayCommitError> {
    if let Some(bytes) = try_encode_compact_root_replay_response(response) {
        return Ok(bytes);
    }

    encode_one(response).map_err(|err| ReplayCommitError::EncodeFailed(err.to_string()))
}

fn try_encode_compact_root_replay_response(response: &Response) -> Option<Vec<u8>> {
    let Response::Cycles(CyclesResponse { cycles_transferred }) = response else {
        return None;
    };

    let payload = cycles_transferred.to_be_bytes();
    let mut bytes = Vec::with_capacity(ROOT_REPLAY_COMPACT_TAG.len() + 1 + payload.len());
    bytes.extend_from_slice(ROOT_REPLAY_COMPACT_TAG);
    bytes.push(ROOT_REPLAY_COMPACT_CYCLES_V1);
    bytes.extend_from_slice(&payload);
    Some(bytes)
}

fn committed_response_bytes<'a>(
    receipt: &'a ReplayReceipt,
    expected_schema_version: u32,
    response_label: &'static str,
) -> Result<&'a [u8], ReplayDecodeError> {
    let response_schema_version = receipt.response_schema_version.ok_or_else(|| {
        ReplayDecodeError::DecodeFailed(format!(
            "{response_label} replay receipt is missing response schema version"
        ))
    })?;
    if response_schema_version != expected_schema_version {
        return Err(ReplayDecodeError::DecodeFailed(format!(
            "unsupported {response_label} replay response schema version {response_schema_version}"
        )));
    }
    receipt.response_bytes.as_deref().ok_or_else(|| {
        ReplayDecodeError::DecodeFailed(format!(
            "{response_label} replay receipt is missing response bytes"
        ))
    })
}

fn try_decode_compact_root_replay_response(
    bytes: &[u8],
) -> Result<Option<Response>, ReplayDecodeError> {
    if !bytes.starts_with(ROOT_REPLAY_COMPACT_TAG) {
        return Ok(None);
    }

    let Some((&kind, mut payload)) = bytes[ROOT_REPLAY_COMPACT_TAG.len()..].split_first() else {
        return Err(ReplayDecodeError::DecodeFailed(
            "root replay compact payload missing variant tag".to_string(),
        ));
    };

    match kind {
        ROOT_REPLAY_COMPACT_CYCLES_V1 => {
            let cycles_transferred = decode_u128(&mut payload)?;
            if !payload.is_empty() {
                return Err(ReplayDecodeError::DecodeFailed(
                    "root replay compact cycles payload had trailing bytes".to_string(),
                ));
            }
            Ok(Some(Response::Cycles(CyclesResponse {
                cycles_transferred,
            })))
        }
        other => Err(ReplayDecodeError::DecodeFailed(format!(
            "unknown root replay compact variant tag: {other}"
        ))),
    }
}

fn decode_u128(payload: &mut &[u8]) -> Result<u128, ReplayDecodeError> {
    let raw = take_exact(payload, 16, "u128 field")?;
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(raw);
    Ok(u128::from_be_bytes(bytes))
}

fn take_exact<'a>(
    payload: &mut &'a [u8],
    len: usize,
    context: &'static str,
) -> Result<&'a [u8], ReplayDecodeError> {
    if payload.len() < len {
        return Err(ReplayDecodeError::DecodeFailed(format!(
            "root replay compact payload truncated while reading {context}"
        )));
    }
    let (value, rest) = payload.split_at(len);
    *payload = rest;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::types::Principal,
        model::replay::{CommandKind, OperationId, ReplayActor},
        ops::{
            replay::{
                guard::secs_to_ns,
                receipt::{
                    ReplayReceiptDecision, ReplayReceiptReserveInput, prepare_replay_receipt,
                },
            },
            storage::replay::ReplayReceiptOps,
        },
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn pending(caller: Principal, request_id: [u8; 32]) -> ReplayPending {
        let command_kind = CommandKind::new("root.test.v1").expect("command kind");
        let operation_id = OperationId::from_bytes(request_id);
        let receipt_input = ReplayReceiptReserveInput::new(
            command_kind,
            operation_id,
            ReplayActor::direct_caller(caller),
            [7u8; 32],
            secs_to_ns(1_000),
        )
        .with_expires_at_ns(secs_to_ns(1_300));
        let receipt_token = match prepare_replay_receipt(receipt_input).expect("prepare") {
            ReplayReceiptDecision::Fresh(token) => token,
            other => panic!("expected fresh receipt token, got {other:?}"),
        };
        ReplayPending {
            caller,
            receipt_token: Box::new(receipt_token),
            payload_hash: [7u8; 32],
            issued_at_ns: secs_to_ns(1_000),
            expires_at_ns: secs_to_ns(1_300),
        }
    }

    #[test]
    fn compact_root_replay_round_trips_cycles_response() {
        let response = Response::Cycles(CyclesResponse {
            cycles_transferred: 123_456_789_012_345_678_901_234_567_890u128,
        });
        let encoded = encode_root_replay_response(&response).expect("encode");

        assert!(
            encoded.starts_with(ROOT_REPLAY_COMPACT_TAG),
            "cycles replay should use compact encoding"
        );

        let decoded = decode_root_replay_response(&encoded).expect("decode");
        match (decoded, response) {
            (Response::Cycles(decoded), Response::Cycles(expected)) => {
                assert_eq!(decoded.cycles_transferred, expected.cycles_transferred);
            }
            _ => panic!("expected cycles replay response"),
        }
    }

    #[test]
    fn reserve_root_replay_rejects_caller_capacity_before_global_capacity() {
        ReplayReceiptOps::reset_for_tests();

        let caller = p(240);
        reserve_root_replay(&pending(caller, [1u8; 32]), 10, 1).expect("first reservation");

        let err = reserve_root_replay(&pending(caller, [2u8; 32]), 10, 1)
            .expect_err("same caller should hit caller cap");
        assert_eq!(
            err,
            ReplayReserveError::CallerCapacityReached {
                caller,
                max_entries: 1,
            }
        );

        reserve_root_replay(&pending(p(241), [3u8; 32]), 10, 1)
            .expect("other caller should still reserve");
    }
}
