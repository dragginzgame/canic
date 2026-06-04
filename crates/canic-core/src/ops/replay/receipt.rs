#![allow(dead_code)]
// Slice B extracts the shared receipt API before root and domain commands are
// migrated onto it.

use crate::{
    ops::{
        replay::model::{
            ExternalEffectDescriptor, OperationId, REPLAY_PAYLOAD_HASH_SCHEMA_VERSION,
            REPLAY_RECEIPT_SCHEMA_VERSION, ReplayActor, ReplayReceipt, ReplayReceiptStatus,
            ReplayTerminalErrorCode, bounded_terminal_error_bytes,
        },
        storage::replay::ReplayReceiptOps,
    },
    storage::stable::replay::{ReplayReceiptRecord, ReplayReceiptSlotKey},
};

use super::model::{CommandKind, RecoveryReason};

///
/// ReplayReceiptReserveInput
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayReceiptReserveInput {
    pub command_kind: CommandKind,
    pub operation_id: OperationId,
    pub actor: ReplayActor,
    pub payload_hash_schema_version: u32,
    pub payload_hash: [u8; 32],
    pub now_ns: u64,
    pub expires_at_ns: Option<u64>,
}

impl ReplayReceiptReserveInput {
    #[must_use]
    pub const fn new(
        command_kind: CommandKind,
        operation_id: OperationId,
        actor: ReplayActor,
        payload_hash: [u8; 32],
        now_ns: u64,
    ) -> Self {
        Self {
            command_kind,
            operation_id,
            actor,
            payload_hash_schema_version: REPLAY_PAYLOAD_HASH_SCHEMA_VERSION,
            payload_hash,
            now_ns,
            expires_at_ns: None,
        }
    }

    #[must_use]
    pub const fn with_expires_at_ns(mut self, expires_at_ns: u64) -> Self {
        self.expires_at_ns = Some(expires_at_ns);
        self
    }
}

///
/// ReplayReceiptToken
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayReceiptToken {
    key: ReplayReceiptSlotKey,
    receipt: ReplayReceipt,
}

impl ReplayReceiptToken {
    #[must_use]
    pub const fn key(&self) -> ReplayReceiptSlotKey {
        self.key
    }

    #[must_use]
    pub const fn receipt(&self) -> &ReplayReceipt {
        &self.receipt
    }
}

///
/// ReplayReceiptDecision
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayReceiptDecision {
    Fresh(ReplayReceiptToken),
    ReturnCommitted(ReplayReceipt),
    OperationInProgress,
    ActorMismatch,
    PayloadMismatch,
    Expired,
    RecoveryRequired(RecoveryReason),
    TerminalFailed {
        error_code: ReplayTerminalErrorCode,
        error_bytes: Vec<u8>,
        error_bytes_truncated: bool,
    },
}

///
/// ReplayReceiptStoreError
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayReceiptStoreError {
    ReceiptDecodeFailed(String),
}

pub fn reserve_or_replay_receipt(
    input: ReplayReceiptReserveInput,
) -> Result<ReplayReceiptDecision, ReplayReceiptStoreError> {
    let decision = prepare_replay_receipt(input)?;
    if let ReplayReceiptDecision::Fresh(token) = &decision {
        reserve_receipt_token(token);
    }
    Ok(decision)
}

pub fn prepare_replay_receipt(
    input: ReplayReceiptReserveInput,
) -> Result<ReplayReceiptDecision, ReplayReceiptStoreError> {
    let key = ReplayReceiptOps::slot_key(&input.command_kind, input.operation_id);
    let Some(existing) = ReplayReceiptOps::get(key) else {
        let receipt = ReplayReceipt {
            schema_version: REPLAY_RECEIPT_SCHEMA_VERSION,
            command_kind: input.command_kind,
            operation_id: input.operation_id,
            actor: input.actor,
            payload_hash_schema_version: input.payload_hash_schema_version,
            payload_hash: input.payload_hash,
            status: ReplayReceiptStatus::Reserved,
            created_at_ns: input.now_ns,
            updated_at_ns: input.now_ns,
            expires_at_ns: input.expires_at_ns,
            response_schema_version: None,
            response_bytes: None,
            effect: None,
        };
        return Ok(ReplayReceiptDecision::Fresh(ReplayReceiptToken {
            key,
            receipt,
        }));
    };

    let existing = existing
        .into_receipt()
        .map_err(ReplayReceiptStoreError::ReceiptDecodeFailed)?;
    Ok(classify_existing_receipt(&input, existing))
}

pub fn reserve_receipt_token(token: &ReplayReceiptToken) {
    ReplayReceiptOps::upsert(
        token.key,
        ReplayReceiptRecord::from_receipt(token.receipt.clone()),
    );
}

pub fn mark_external_effect_in_flight(
    token: &ReplayReceiptToken,
    effect: ExternalEffectDescriptor,
    now_ns: u64,
) {
    let mut receipt = token.receipt.clone();
    receipt.status = ReplayReceiptStatus::ExternalEffectInFlight;
    receipt.effect = Some(effect);
    receipt.updated_at_ns = now_ns;
    ReplayReceiptOps::upsert(token.key, ReplayReceiptRecord::from_receipt(receipt));
}

pub fn commit_receipt_response(
    token: &ReplayReceiptToken,
    response_schema_version: u32,
    response_bytes: Vec<u8>,
    now_ns: u64,
) {
    let mut receipt = token.receipt.clone();
    receipt.status = ReplayReceiptStatus::Committed;
    receipt.response_schema_version = Some(response_schema_version);
    receipt.response_bytes = Some(response_bytes);
    receipt.updated_at_ns = now_ns;
    ReplayReceiptOps::upsert(token.key, ReplayReceiptRecord::from_receipt(receipt));
}

pub fn commit_terminal_failure(
    token: &ReplayReceiptToken,
    error_code: ReplayTerminalErrorCode,
    error_bytes: &[u8],
    now_ns: u64,
) {
    let bounded = bounded_terminal_error_bytes(error_bytes);
    let mut receipt = token.receipt.clone();
    receipt.status = ReplayReceiptStatus::TerminalFailed {
        error_code,
        error_bytes: bounded.bytes,
        error_bytes_truncated: bounded.truncated,
    };
    receipt.updated_at_ns = now_ns;
    ReplayReceiptOps::upsert(token.key, ReplayReceiptRecord::from_receipt(receipt));
}

pub fn mark_recovery_required(token: &ReplayReceiptToken, reason: RecoveryReason, now_ns: u64) {
    let mut receipt = token.receipt.clone();
    receipt.status = ReplayReceiptStatus::RecoveryRequired { reason };
    receipt.updated_at_ns = now_ns;
    ReplayReceiptOps::upsert(token.key, ReplayReceiptRecord::from_receipt(receipt));
}

pub fn abort_reserved_receipt(token: &ReplayReceiptToken) {
    let _ = ReplayReceiptOps::remove(token.key);
}

fn classify_existing_receipt(
    input: &ReplayReceiptReserveInput,
    existing: ReplayReceipt,
) -> ReplayReceiptDecision {
    if existing
        .expires_at_ns
        .is_some_and(|expires_at_ns| input.now_ns >= expires_at_ns)
    {
        return ReplayReceiptDecision::Expired;
    }

    if existing.actor != input.actor {
        return ReplayReceiptDecision::ActorMismatch;
    }
    if existing.payload_hash_schema_version != input.payload_hash_schema_version
        || existing.payload_hash != input.payload_hash
    {
        return ReplayReceiptDecision::PayloadMismatch;
    }

    match existing.status {
        ReplayReceiptStatus::Reserved | ReplayReceiptStatus::ExternalEffectInFlight => {
            ReplayReceiptDecision::OperationInProgress
        }
        ReplayReceiptStatus::Committed => ReplayReceiptDecision::ReturnCommitted(existing),
        ReplayReceiptStatus::TerminalFailed {
            error_code,
            error_bytes,
            error_bytes_truncated,
        } => ReplayReceiptDecision::TerminalFailed {
            error_code,
            error_bytes,
            error_bytes_truncated,
        },
        ReplayReceiptStatus::RecoveryRequired { reason } => {
            ReplayReceiptDecision::RecoveryRequired(reason)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cdk::types::Principal, ops::storage::replay::ReplayReceiptOps};

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn input() -> ReplayReceiptReserveInput {
        ReplayReceiptReserveInput::new(
            CommandKind::new("test.command.v1").expect("command"),
            OperationId::from_bytes([7; 32]),
            ReplayActor::direct_caller(p(1)),
            [9; 32],
            100,
        )
    }

    #[test]
    fn reserve_or_replay_receipt_reserves_new_receipt() {
        ReplayReceiptOps::reset_for_tests();

        let decision = reserve_or_replay_receipt(input()).expect("decision");
        let ReplayReceiptDecision::Fresh(token) = decision else {
            panic!("expected fresh reservation");
        };

        assert_eq!(ReplayReceiptOps::len(), 1);
        assert_eq!(token.receipt.status, ReplayReceiptStatus::Reserved);
    }

    #[test]
    fn reserve_or_replay_receipt_returns_committed_response_for_duplicate() {
        ReplayReceiptOps::reset_for_tests();

        let token = match reserve_or_replay_receipt(input()).expect("reserve") {
            ReplayReceiptDecision::Fresh(token) => token,
            other => panic!("expected fresh, got {other:?}"),
        };
        commit_receipt_response(&token, 1, vec![1, 2, 3], 200);

        let duplicate = reserve_or_replay_receipt(input()).expect("duplicate");
        let ReplayReceiptDecision::ReturnCommitted(receipt) = duplicate else {
            panic!("expected committed replay");
        };

        assert_eq!(receipt.response_schema_version, Some(1));
        assert_eq!(receipt.response_bytes.as_deref(), Some(&[1, 2, 3][..]));
    }

    #[test]
    fn reserve_or_replay_receipt_rejects_actor_or_payload_mismatch() {
        ReplayReceiptOps::reset_for_tests();

        let _ = reserve_or_replay_receipt(input()).expect("reserve");

        let mut actor_mismatch = input();
        actor_mismatch.actor = ReplayActor::direct_caller(p(2));
        assert_eq!(
            reserve_or_replay_receipt(actor_mismatch).expect("actor mismatch"),
            ReplayReceiptDecision::ActorMismatch
        );

        let mut payload_mismatch = input();
        payload_mismatch.payload_hash = [8; 32];
        assert_eq!(
            reserve_or_replay_receipt(payload_mismatch).expect("payload mismatch"),
            ReplayReceiptDecision::PayloadMismatch
        );
    }

    #[test]
    fn reserve_or_replay_receipt_reports_in_progress_for_reserved_or_effect() {
        ReplayReceiptOps::reset_for_tests();

        let token = match reserve_or_replay_receipt(input()).expect("reserve") {
            ReplayReceiptDecision::Fresh(token) => token,
            other => panic!("expected fresh, got {other:?}"),
        };
        assert_eq!(
            reserve_or_replay_receipt(input()).expect("reserved duplicate"),
            ReplayReceiptDecision::OperationInProgress
        );

        mark_external_effect_in_flight(
            &token,
            ExternalEffectDescriptor::ManagementCreateCanister {
                command_kind: CommandKind::new("test.command.v1").expect("command"),
            },
            150,
        );
        assert_eq!(
            reserve_or_replay_receipt(input()).expect("in-flight duplicate"),
            ReplayReceiptDecision::OperationInProgress
        );
    }

    #[test]
    fn terminal_failure_is_bounded_before_storage() {
        ReplayReceiptOps::reset_for_tests();

        let token = match reserve_or_replay_receipt(input()).expect("reserve") {
            ReplayReceiptDecision::Fresh(token) => token,
            other => panic!("expected fresh, got {other:?}"),
        };
        commit_terminal_failure(
            &token,
            ReplayTerminalErrorCode::ExecutionFailed,
            &vec![7; super::super::model::MAX_REPLAY_TERMINAL_ERROR_BYTES + 1],
            300,
        );

        let duplicate = reserve_or_replay_receipt(input()).expect("duplicate");
        let ReplayReceiptDecision::TerminalFailed {
            error_code,
            error_bytes,
            error_bytes_truncated,
        } = duplicate
        else {
            panic!("expected terminal failure replay");
        };

        assert_eq!(error_code, ReplayTerminalErrorCode::ExecutionFailed);
        assert_eq!(
            error_bytes.len(),
            super::super::model::MAX_REPLAY_TERMINAL_ERROR_BYTES
        );
        assert!(error_bytes_truncated);
    }
}
