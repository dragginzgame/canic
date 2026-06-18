#![expect(dead_code)]
//! Module: ops::replay::receipt
//!
//! Responsibility: reserve, classify, and mutate shared replay receipts.
//! Does not own: command authorization, response encoding, or stable schemas.
//! Boundary: workflow and replay guards call this API for receipt lifecycle.

use crate::{
    ops::{
        replay::model::{
            CommandKind, ExternalEffectDescriptor, OperationId, REPLAY_PAYLOAD_HASH_SCHEMA_VERSION,
            REPLAY_RECEIPT_SCHEMA_VERSION, RecoveryReason, ReplayActor, ReplayReceipt,
            ReplayReceiptStatus, ReplayTerminalErrorCode, bounded_terminal_error_bytes,
        },
        storage::replay::ReplayReceiptOps,
    },
    storage::stable::replay::{ReplayReceiptRecord, ReplayReceiptSlotKey},
};

pub const MAX_PENDING_REPLAY_RECEIPTS_PER_ACTOR: usize = 64;
pub const MAX_PENDING_REPLAY_RECEIPTS_PER_COMMAND_KIND: usize = 512;

///
/// ReplayReceiptReserveInput
///
/// Input used to reserve or replay a shared receipt.
/// Owned by replay ops and supplied by workflow replay adapters.
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
/// Capability proving a fresh replay receipt reservation.
/// Owned by replay ops and passed to commit/abort/effect helpers.
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
/// Mechanical replay decision for one shared receipt lookup.
/// Owned by replay ops and mapped by workflow into command-specific outcomes.
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
    PendingActorQuotaExceeded {
        actor: ReplayActor,
        max_pending: usize,
    },
    PendingCommandQuotaExceeded {
        command_kind: CommandKind,
        max_pending: usize,
    },
}

///
/// ReplayReceiptStoreError
///
/// Storage adapter failure while decoding shared replay receipts.
/// Owned by replay ops and mapped by callers into workflow errors.
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayReceiptStoreError {
    ReceiptDecodeFailed(String),
}

pub fn reserve_or_replay_receipt(
    input: ReplayReceiptReserveInput,
) -> Result<ReplayReceiptDecision, ReplayReceiptStoreError> {
    reserve_or_replay_receipt_with_limits(
        input,
        MAX_PENDING_REPLAY_RECEIPTS_PER_ACTOR,
        MAX_PENDING_REPLAY_RECEIPTS_PER_COMMAND_KIND,
    )
}

fn reserve_or_replay_receipt_with_limits(
    input: ReplayReceiptReserveInput,
    max_pending_per_actor: usize,
    max_pending_per_command_kind: usize,
) -> Result<ReplayReceiptDecision, ReplayReceiptStoreError> {
    let decision = prepare_replay_receipt(input)?;
    if let ReplayReceiptDecision::Fresh(token) = &decision {
        if let Some(quota_decision) = pending_receipt_quota_decision(
            token.receipt(),
            max_pending_per_actor,
            max_pending_per_command_kind,
        ) {
            return Ok(quota_decision);
        }
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

fn pending_receipt_quota_decision(
    receipt: &ReplayReceipt,
    max_pending_per_actor: usize,
    max_pending_per_command_kind: usize,
) -> Option<ReplayReceiptDecision> {
    if ReplayReceiptOps::pending_len_for_actor(receipt.actor, receipt.created_at_ns)
        >= max_pending_per_actor
    {
        return Some(ReplayReceiptDecision::PendingActorQuotaExceeded {
            actor: receipt.actor,
            max_pending: max_pending_per_actor,
        });
    }

    if ReplayReceiptOps::pending_len_for_command_kind(&receipt.command_kind, receipt.created_at_ns)
        >= max_pending_per_command_kind
    {
        return Some(ReplayReceiptDecision::PendingCommandQuotaExceeded {
            command_kind: receipt.command_kind.clone(),
            max_pending: max_pending_per_command_kind,
        });
    }

    None
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
    let mut receipt = latest_receipt_for_token(token);
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
    let mut receipt = latest_receipt_for_token(token);
    receipt.status = ReplayReceiptStatus::TerminalFailed {
        error_code,
        error_bytes: bounded.bytes,
        error_bytes_truncated: bounded.truncated,
    };
    receipt.updated_at_ns = now_ns;
    ReplayReceiptOps::upsert(token.key, ReplayReceiptRecord::from_receipt(receipt));
}

pub fn mark_recovery_required(token: &ReplayReceiptToken, reason: RecoveryReason, now_ns: u64) {
    let mut receipt = latest_receipt_for_token(token);
    receipt.status = ReplayReceiptStatus::RecoveryRequired { reason };
    receipt.updated_at_ns = now_ns;
    ReplayReceiptOps::upsert(token.key, ReplayReceiptRecord::from_receipt(receipt));
}

pub fn abort_reserved_receipt(token: &ReplayReceiptToken) {
    let Some(receipt) =
        ReplayReceiptOps::get(token.key).and_then(|record| record.into_receipt().ok())
    else {
        return;
    };
    if receipt.status == ReplayReceiptStatus::Reserved {
        let _ = ReplayReceiptOps::remove(token.key);
    }
}

pub fn abort_uncommitted_receipt(token: &ReplayReceiptToken) {
    let Some(receipt) =
        ReplayReceiptOps::get(token.key).and_then(|record| record.into_receipt().ok())
    else {
        return;
    };
    if matches!(
        receipt.status,
        ReplayReceiptStatus::Reserved | ReplayReceiptStatus::ExternalEffectInFlight
    ) {
        let _ = ReplayReceiptOps::remove(token.key);
    }
}

fn latest_receipt_for_token(token: &ReplayReceiptToken) -> ReplayReceipt {
    ReplayReceiptOps::get(token.key)
        .and_then(|record| record.into_receipt().ok())
        .unwrap_or_else(|| token.receipt.clone())
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
        input_with("test.command.v1", p(1), [7; 32], 100)
    }

    fn input_with(
        command_kind: &str,
        actor: Principal,
        operation_id: [u8; 32],
        now_ns: u64,
    ) -> ReplayReceiptReserveInput {
        ReplayReceiptReserveInput::new(
            CommandKind::new(command_kind).expect("command"),
            OperationId::from_bytes(operation_id),
            ReplayActor::direct_caller(actor),
            [9; 32],
            now_ns,
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
    fn reserve_or_replay_receipt_enforces_pending_actor_quota_for_fresh_receipts() {
        ReplayReceiptOps::reset_for_tests();

        let first = reserve_or_replay_receipt_with_limits(input(), 1, 10).expect("first reserve");
        assert!(matches!(first, ReplayReceiptDecision::Fresh(_)));

        let second = reserve_or_replay_receipt_with_limits(
            input_with("test.command.v1", p(1), [8; 32], 101),
            1,
            10,
        )
        .expect("second decision");

        assert_eq!(
            second,
            ReplayReceiptDecision::PendingActorQuotaExceeded {
                actor: ReplayActor::direct_caller(p(1)),
                max_pending: 1,
            }
        );
        assert_eq!(
            ReplayReceiptOps::len(),
            1,
            "quota rejection must not write a fresh receipt"
        );
    }

    #[test]
    fn reserve_or_replay_receipt_enforces_pending_command_quota_for_fresh_receipts() {
        ReplayReceiptOps::reset_for_tests();

        let first = reserve_or_replay_receipt_with_limits(input(), 10, 1).expect("first reserve");
        assert!(matches!(first, ReplayReceiptDecision::Fresh(_)));

        let second = reserve_or_replay_receipt_with_limits(
            input_with("test.command.v1", p(2), [8; 32], 101),
            10,
            1,
        )
        .expect("second decision");

        assert_eq!(
            second,
            ReplayReceiptDecision::PendingCommandQuotaExceeded {
                command_kind: CommandKind::new("test.command.v1").expect("command"),
                max_pending: 1,
            }
        );
        assert_eq!(
            ReplayReceiptOps::len(),
            1,
            "quota rejection must not write a fresh receipt"
        );
    }

    #[test]
    fn pending_receipt_quota_ignores_expired_pending_receipts() {
        ReplayReceiptOps::reset_for_tests();

        let first = reserve_or_replay_receipt_with_limits(input().with_expires_at_ns(100), 1, 10)
            .expect("first reserve");
        assert!(matches!(first, ReplayReceiptDecision::Fresh(_)));

        let second = reserve_or_replay_receipt_with_limits(
            input_with("test.command.v1", p(1), [8; 32], 101),
            1,
            10,
        )
        .expect("second reserve");

        assert!(
            matches!(second, ReplayReceiptDecision::Fresh(_)),
            "expired pending receipts must not consume pending quota"
        );
        assert_eq!(ReplayReceiptOps::len(), 2);
    }

    #[test]
    fn pending_receipt_quota_ignores_pending_receipts_at_expiry_boundary() {
        ReplayReceiptOps::reset_for_tests();

        let first = reserve_or_replay_receipt_with_limits(
            input_with("test.command.v1", p(1), [7; 32], 90).with_expires_at_ns(100),
            1,
            10,
        )
        .expect("first reserve");
        assert!(matches!(first, ReplayReceiptDecision::Fresh(_)));

        let second = reserve_or_replay_receipt_with_limits(
            input_with("test.command.v1", p(1), [8; 32], 100),
            1,
            10,
        )
        .expect("second reserve");

        assert!(
            matches!(second, ReplayReceiptDecision::Fresh(_)),
            "receipts at their expiry boundary must not consume pending quota"
        );
        assert_eq!(ReplayReceiptOps::len(), 2);
    }

    #[test]
    fn pending_receipt_quota_does_not_block_committed_replay() {
        ReplayReceiptOps::reset_for_tests();

        let committed_token = match reserve_or_replay_receipt_with_limits(input(), 1, 10)
            .expect("reserve committed target")
        {
            ReplayReceiptDecision::Fresh(token) => token,
            other => panic!("expected fresh, got {other:?}"),
        };
        commit_receipt_response(&committed_token, 1, vec![1, 2, 3], 150);

        let pending = reserve_or_replay_receipt_with_limits(
            input_with("test.command.v1", p(1), [8; 32], 160),
            1,
            10,
        )
        .expect("reserve pending filler");
        assert!(matches!(pending, ReplayReceiptDecision::Fresh(_)));

        let duplicate =
            reserve_or_replay_receipt_with_limits(input(), 1, 10).expect("committed duplicate");
        assert!(matches!(
            duplicate,
            ReplayReceiptDecision::ReturnCommitted(_)
        ));
    }

    #[test]
    fn terminal_receipt_transitions_preserve_recorded_external_effect() {
        ReplayReceiptOps::reset_for_tests();

        let token = match reserve_or_replay_receipt(input()).expect("reserve") {
            ReplayReceiptDecision::Fresh(token) => token,
            other => panic!("expected fresh, got {other:?}"),
        };
        let effect = ExternalEffectDescriptor::ManagementCall {
            canister: Principal::from_slice(&[8; 29]),
            method: "deposit_cycles".to_string(),
        };
        mark_external_effect_in_flight(&token, effect.clone(), 150);
        commit_receipt_response(&token, 1, vec![1, 2, 3], 200);

        let receipt = ReplayReceiptOps::get(token.key())
            .expect("receipt stored")
            .into_receipt()
            .expect("receipt decodes");
        assert_eq!(receipt.status, ReplayReceiptStatus::Committed);
        assert_eq!(receipt.effect, Some(effect));
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
