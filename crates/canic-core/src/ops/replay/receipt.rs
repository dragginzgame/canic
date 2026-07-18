//! Module: ops::replay::receipt
//!
//! Responsibility: reserve, classify, and mutate shared replay receipts.
//! Does not own: command authorization, response encoding, or stable schemas.
//! Boundary: workflow and replay guards call this API for receipt lifecycle.

#[cfg(test)]
use crate::model::replay::ROOT_PROVISION_REPLAY_COMMAND_KIND;
use crate::{
    model::replay::{
        CommandKind, ExternalEffectDescriptor, OperationId, PLACEMENT_CHILD_REPLAY_COMMAND_KIND,
        REPLAY_PAYLOAD_HASH_SCHEMA_VERSION, REPLAY_RECEIPT_SCHEMA_VERSION, RecoveryReason,
        ReplayActor, ReplayCostGuardSettlement, ReplayReceipt, ReplayReceiptStatus,
        placement_receipt_requires_acknowledgement,
    },
    ops::storage::replay::ReplayReceiptOps,
    storage::stable::replay::{ReplayReceiptRecord, ReplayReceiptSlotKey},
};
use thiserror::Error as ThisError;

pub const MAX_PENDING_REPLAY_RECEIPTS_PER_ACTOR: usize = 64;
pub const MAX_PENDING_REPLAY_RECEIPTS_PER_COMMAND_KIND: usize = 512;

/// Active replay-receipt retention limits for one bounded command family.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReplayReceiptRetentionLimits {
    pub max_active_per_actor: usize,
    pub max_active_per_command_kind: usize,
    pub purge_scan_limit: usize,
}

/// Rejection from command-specific retained-response admission.
#[derive(Clone, Debug, Eq, PartialEq, ThisError)]
pub enum ReplayReceiptRetentionError {
    #[error("actor retained replay response quota exceeded")]
    ActorQuotaExceeded {
        actor: ReplayActor,
        max_retained: usize,
    },

    #[error("command retained replay response quota exceeded")]
    CommandQuotaExceeded {
        command_kind: CommandKind,
        max_retained: usize,
    },

    #[error(transparent)]
    Store(#[from] ReplayReceiptStoreError),
}

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
/// Capability identifying a validated replay receipt reservation or recovery.
/// Owned by replay ops and passed to lifecycle mutation helpers.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayReceiptToken {
    key: ReplayReceiptSlotKey,
    receipt: ReplayReceipt,
}

impl ReplayReceiptToken {
    #[must_use]
    #[cfg(test)]
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
    RecoveryRequired {
        token: ReplayReceiptToken,
        reason: RecoveryReason,
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
/// PlacementReceiptAcknowledgementDecision
///
/// Mechanical result of releasing one caller-owned committed placement receipt.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlacementReceiptAcknowledgementDecision {
    Acknowledged,
    AlreadyAbsent,
    ActorMismatch,
    NotCommitted,
    NotPlacementEffect,
}

///
/// ReplayReceiptStoreError
///
/// Storage adapter failure while decoding shared replay receipts.
/// Owned by replay ops and mapped by callers into workflow errors.
///

#[derive(Clone, Debug, Eq, PartialEq, ThisError)]
pub enum ReplayReceiptStoreError {
    #[error("reserved replay receipt is missing")]
    ReceiptMissing,

    #[error("failed to decode replay receipt: {0}")]
    ReceiptDecodeFailed(String),

    #[error("replay receipt token no longer matches persisted receipt identity")]
    ReceiptTokenMismatch,

    #[error("replay receipt is missing staged response data")]
    StagedResponseMissing,

    #[error("replay receipt is missing cost guard settlement identity")]
    CostGuardSettlementMissing,
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

/// Reserve or replay one receipt while bounding all unexpired responses for its command family.
pub fn reserve_or_replay_receipt_with_retention(
    input: ReplayReceiptReserveInput,
    limits: ReplayReceiptRetentionLimits,
) -> Result<ReplayReceiptDecision, ReplayReceiptRetentionError> {
    let decision = prepare_replay_receipt(input)?;
    if let ReplayReceiptDecision::Fresh(token) = &decision {
        let receipt = token.receipt();
        let _ = ReplayReceiptOps::purge_expired_for_command_kind(
            &receipt.command_kind,
            receipt.created_at_ns,
            limits.purge_scan_limit,
        );
        if let Some(quota_error) = retained_receipt_quota_error(receipt, limits) {
            return Err(quota_error);
        }
        reserve_receipt_token(token);
    }
    Ok(decision)
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
            staged_response_schema_version: None,
            staged_response_bytes: None,
            cost_guard_settlement: None,
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
    Ok(classify_existing_receipt(&input, key, existing))
}

/// Remove one committed placement-child receipt after its caller durably consumed the result.
pub fn acknowledge_placement_receipt(
    operation_id: OperationId,
    actor: ReplayActor,
) -> Result<PlacementReceiptAcknowledgementDecision, ReplayReceiptStoreError> {
    let command_kind = CommandKind::new(PLACEMENT_CHILD_REPLAY_COMMAND_KIND)
        .expect("placement child command kind is a valid static label");
    let key = ReplayReceiptOps::slot_key(&command_kind, operation_id);
    let Some(record) = ReplayReceiptOps::get(key) else {
        return Ok(PlacementReceiptAcknowledgementDecision::AlreadyAbsent);
    };
    let receipt = record
        .into_receipt()
        .map_err(ReplayReceiptStoreError::ReceiptDecodeFailed)?;

    if receipt.actor != actor {
        return Ok(PlacementReceiptAcknowledgementDecision::ActorMismatch);
    }
    if receipt.status != ReplayReceiptStatus::Committed {
        return Ok(PlacementReceiptAcknowledgementDecision::NotCommitted);
    }
    if !placement_receipt_requires_acknowledgement(&receipt.status, receipt.effect.as_ref()) {
        return Ok(PlacementReceiptAcknowledgementDecision::NotPlacementEffect);
    }

    let _ = ReplayReceiptOps::remove(key);
    Ok(PlacementReceiptAcknowledgementDecision::Acknowledged)
}

pub fn reserve_receipt_token(token: &ReplayReceiptToken) {
    ReplayReceiptOps::upsert(
        token.key,
        ReplayReceiptRecord::from_receipt(token.receipt.clone()),
    );
}

/// Confirm that a token still owns the persisted receipt identity.
pub fn validate_receipt_token(token: &ReplayReceiptToken) -> Result<(), ReplayReceiptStoreError> {
    latest_receipt_for_token(token).map(|_| ())
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

fn retained_receipt_quota_error(
    receipt: &ReplayReceipt,
    limits: ReplayReceiptRetentionLimits,
) -> Option<ReplayReceiptRetentionError> {
    if ReplayReceiptOps::active_len_for_actor_command_kind(
        receipt.actor,
        &receipt.command_kind,
        receipt.created_at_ns,
    ) >= limits.max_active_per_actor
    {
        return Some(ReplayReceiptRetentionError::ActorQuotaExceeded {
            actor: receipt.actor,
            max_retained: limits.max_active_per_actor,
        });
    }

    if ReplayReceiptOps::active_len_for_command_kind(&receipt.command_kind, receipt.created_at_ns)
        >= limits.max_active_per_command_kind
    {
        return Some(ReplayReceiptRetentionError::CommandQuotaExceeded {
            command_kind: receipt.command_kind.clone(),
            max_retained: limits.max_active_per_command_kind,
        });
    }

    None
}

pub fn mark_external_effect_in_flight(
    token: &ReplayReceiptToken,
    effect: ExternalEffectDescriptor,
    now_ns: u64,
) -> Result<(), ReplayReceiptStoreError> {
    let mut receipt = latest_receipt_for_token(token)?;
    receipt.status = ReplayReceiptStatus::ExternalEffectInFlight;
    receipt.effect = Some(effect);
    receipt.updated_at_ns = now_ns;
    ReplayReceiptOps::upsert(token.key, ReplayReceiptRecord::from_receipt(receipt));
    Ok(())
}

/// Atomically persist an external-effect boundary and its cost intent identity.
pub fn mark_costed_external_effect_in_flight(
    token: &ReplayReceiptToken,
    effect: ExternalEffectDescriptor,
    settlement: ReplayCostGuardSettlement,
    now_ns: u64,
) -> Result<(), ReplayReceiptStoreError> {
    let mut receipt = latest_receipt_for_token(token)?;
    receipt.status = ReplayReceiptStatus::ExternalEffectInFlight;
    receipt.effect = Some(effect);
    receipt.cost_guard_settlement = Some(settlement);
    receipt.updated_at_ns = now_ns;
    ReplayReceiptOps::upsert(token.key, ReplayReceiptRecord::from_receipt(receipt));
    Ok(())
}

/// Persist the cost intent identity before a workflow crosses an external-effect boundary.
pub fn record_cost_guard_settlement(
    token: &ReplayReceiptToken,
    settlement: ReplayCostGuardSettlement,
    now_ns: u64,
) -> Result<(), ReplayReceiptStoreError> {
    let mut receipt = latest_receipt_for_token(token)?;
    receipt.cost_guard_settlement = Some(settlement);
    receipt.updated_at_ns = now_ns;
    ReplayReceiptOps::upsert(token.key, ReplayReceiptRecord::from_receipt(receipt));
    Ok(())
}

/// Persist a terminal response before its cost intents are settled.
pub fn stage_receipt_response(
    token: &ReplayReceiptToken,
    response_schema_version: u32,
    response_bytes: Vec<u8>,
    now_ns: u64,
) -> Result<(), ReplayReceiptStoreError> {
    let mut receipt = latest_receipt_for_token(token)?;
    receipt.staged_response_schema_version = Some(response_schema_version);
    receipt.staged_response_bytes = Some(response_bytes);
    receipt.updated_at_ns = now_ns;
    ReplayReceiptOps::upsert(token.key, ReplayReceiptRecord::from_receipt(receipt));
    Ok(())
}

/// Promote a staged terminal response after cost settlement succeeds.
pub fn commit_staged_receipt_response(
    token: &ReplayReceiptToken,
    now_ns: u64,
) -> Result<ReplayReceipt, ReplayReceiptStoreError> {
    let mut receipt = latest_receipt_for_token(token)?;
    let response_schema_version = receipt
        .staged_response_schema_version
        .take()
        .ok_or(ReplayReceiptStoreError::StagedResponseMissing)?;
    let response_bytes = receipt
        .staged_response_bytes
        .take()
        .ok_or(ReplayReceiptStoreError::StagedResponseMissing)?;
    receipt.status = ReplayReceiptStatus::Committed;
    receipt.response_schema_version = Some(response_schema_version);
    receipt.response_bytes = Some(response_bytes);
    receipt.updated_at_ns = now_ns;
    ReplayReceiptOps::upsert(
        token.key,
        ReplayReceiptRecord::from_receipt(receipt.clone()),
    );
    Ok(receipt)
}

/// Read the durable cost intent identity for an accounting-only retry.
pub fn replay_cost_guard_settlement(
    token: &ReplayReceiptToken,
) -> Result<ReplayCostGuardSettlement, ReplayReceiptStoreError> {
    latest_receipt_for_token(token)?
        .cost_guard_settlement
        .ok_or(ReplayReceiptStoreError::CostGuardSettlementMissing)
}

pub fn mark_recovery_required(
    token: &ReplayReceiptToken,
    reason: RecoveryReason,
    now_ns: u64,
) -> Result<(), ReplayReceiptStoreError> {
    let mut receipt = latest_receipt_for_token(token)?;
    receipt.status = ReplayReceiptStatus::RecoveryRequired { reason };
    receipt.updated_at_ns = now_ns;
    ReplayReceiptOps::upsert(token.key, ReplayReceiptRecord::from_receipt(receipt));
    Ok(())
}

pub fn abort_reserved_receipt(token: &ReplayReceiptToken) -> Result<(), ReplayReceiptStoreError> {
    let receipt = latest_receipt_for_token(token)?;
    if receipt.status == ReplayReceiptStatus::Reserved {
        let _ = ReplayReceiptOps::remove(token.key);
    }
    Ok(())
}

pub fn abort_uncommitted_receipt(
    token: &ReplayReceiptToken,
) -> Result<(), ReplayReceiptStoreError> {
    let receipt = latest_receipt_for_token(token)?;
    if matches!(
        receipt.status,
        ReplayReceiptStatus::Reserved | ReplayReceiptStatus::ExternalEffectInFlight
    ) {
        let _ = ReplayReceiptOps::remove(token.key);
    }
    Ok(())
}

fn latest_receipt_for_token(
    token: &ReplayReceiptToken,
) -> Result<ReplayReceipt, ReplayReceiptStoreError> {
    let Some(record) = ReplayReceiptOps::get(token.key) else {
        return Err(ReplayReceiptStoreError::ReceiptMissing);
    };
    let receipt = record
        .into_receipt()
        .map_err(ReplayReceiptStoreError::ReceiptDecodeFailed)?;
    if !same_receipt_identity(&receipt, &token.receipt) {
        return Err(ReplayReceiptStoreError::ReceiptTokenMismatch);
    }
    Ok(receipt)
}

fn same_receipt_identity(current: &ReplayReceipt, token: &ReplayReceipt) -> bool {
    current.schema_version == token.schema_version
        && current.command_kind == token.command_kind
        && current.operation_id == token.operation_id
        && current.actor == token.actor
        && current.payload_hash_schema_version == token.payload_hash_schema_version
        && current.payload_hash == token.payload_hash
        && current.created_at_ns == token.created_at_ns
        && current.expires_at_ns == token.expires_at_ns
}

fn classify_existing_receipt(
    input: &ReplayReceiptReserveInput,
    key: ReplayReceiptSlotKey,
    existing: ReplayReceipt,
) -> ReplayReceiptDecision {
    if existing.actor != input.actor {
        return ReplayReceiptDecision::ActorMismatch;
    }
    if existing.payload_hash_schema_version != input.payload_hash_schema_version
        || existing.payload_hash != input.payload_hash
    {
        return ReplayReceiptDecision::PayloadMismatch;
    }

    if let ReplayReceiptStatus::RecoveryRequired { reason } = &existing.status {
        let reason = reason.clone();
        return ReplayReceiptDecision::RecoveryRequired {
            token: ReplayReceiptToken {
                key,
                receipt: existing,
            },
            reason,
        };
    }

    if existing.status == ReplayReceiptStatus::ExternalEffectInFlight {
        return ReplayReceiptDecision::OperationInProgress;
    }

    if !placement_receipt_requires_acknowledgement(&existing.status, existing.effect.as_ref())
        && existing
            .expires_at_ns
            .is_some_and(|expires_at_ns| input.now_ns >= expires_at_ns)
    {
        return ReplayReceiptDecision::Expired;
    }

    match existing.status {
        ReplayReceiptStatus::Reserved => ReplayReceiptDecision::OperationInProgress,
        ReplayReceiptStatus::ExternalEffectInFlight => {
            unreachable!("external-effect receipts are classified before expiry")
        }
        ReplayReceiptStatus::Committed => ReplayReceiptDecision::ReturnCommitted(existing),
        ReplayReceiptStatus::RecoveryRequired { .. } => {
            unreachable!("recovery receipts are classified before expiry")
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

    fn commit_response(
        token: &ReplayReceiptToken,
        response_schema_version: u32,
        response_bytes: Vec<u8>,
        now_ns: u64,
    ) -> Result<(), ReplayReceiptStoreError> {
        stage_receipt_response(token, response_schema_version, response_bytes, now_ns)?;
        commit_staged_receipt_response(token, now_ns).map(|_| ())
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
        commit_response(&token, 1, vec![1, 2, 3], 200).expect("commit receipt");

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

        let original_input = input().with_expires_at_ns(175);
        let token = match reserve_or_replay_receipt(original_input.clone()).expect("reserve") {
            ReplayReceiptDecision::Fresh(token) => token,
            other => panic!("expected fresh, got {other:?}"),
        };
        assert_eq!(
            reserve_or_replay_receipt(original_input.clone()).expect("reserved duplicate"),
            ReplayReceiptDecision::OperationInProgress
        );

        mark_external_effect_in_flight(
            &token,
            ExternalEffectDescriptor::ManagementCreateCanister {
                command_kind: CommandKind::new("test.command.v1").expect("command"),
            },
            150,
        )
        .expect("mark external effect");
        let mut expired_retry = original_input;
        expired_retry.now_ns = 200;
        assert_eq!(
            reserve_or_replay_receipt(expired_retry).expect("in-flight duplicate"),
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
        commit_response(&committed_token, 1, vec![1, 2, 3], 150).expect("commit receipt");

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
    fn retained_receipt_quota_counts_committed_responses() {
        ReplayReceiptOps::reset_for_tests();
        let limits = ReplayReceiptRetentionLimits {
            max_active_per_actor: 1,
            max_active_per_command_kind: 10,
            purge_scan_limit: 10,
        };
        let first_input = input().with_expires_at_ns(200);
        let token = match reserve_or_replay_receipt_with_retention(first_input, limits)
            .expect("reserve first")
        {
            ReplayReceiptDecision::Fresh(token) => token,
            other => panic!("expected fresh, got {other:?}"),
        };
        commit_response(&token, 1, vec![1], 110).expect("commit first");

        let second = reserve_or_replay_receipt_with_retention(
            input_with("test.command.v1", p(1), [8; 32], 120).with_expires_at_ns(220),
            limits,
        )
        .expect_err("retention quota must reject");

        assert_eq!(
            second,
            ReplayReceiptRetentionError::ActorQuotaExceeded {
                actor: ReplayActor::direct_caller(p(1)),
                max_retained: 1,
            }
        );
        assert_eq!(ReplayReceiptOps::len(), 1);
    }

    #[test]
    fn retained_receipt_admission_purges_expired_same_command_at_boundary() {
        ReplayReceiptOps::reset_for_tests();
        let limits = ReplayReceiptRetentionLimits {
            max_active_per_actor: 1,
            max_active_per_command_kind: 1,
            purge_scan_limit: 10,
        };
        let expired = match reserve_or_replay_receipt_with_retention(
            input_with("test.command.v1", p(1), [7; 32], 90).with_expires_at_ns(100),
            limits,
        )
        .expect("reserve expiring receipt")
        {
            ReplayReceiptDecision::Fresh(token) => token,
            other => panic!("expected fresh, got {other:?}"),
        };
        commit_response(&expired, 1, vec![1], 95).expect("commit expiring receipt");
        let unrelated = match reserve_or_replay_receipt(
            input_with("other.command.v1", p(2), [8; 32], 90).with_expires_at_ns(100),
        )
        .expect("reserve unrelated")
        {
            ReplayReceiptDecision::Fresh(token) => token,
            other => panic!("expected fresh, got {other:?}"),
        };
        commit_response(&unrelated, 1, vec![2], 95).expect("commit unrelated");

        let admitted = reserve_or_replay_receipt_with_retention(
            input_with("test.command.v1", p(1), [9; 32], 100).with_expires_at_ns(200),
            limits,
        )
        .expect("admit after expiry");

        assert!(matches!(admitted, ReplayReceiptDecision::Fresh(_)));
        assert_eq!(
            ReplayReceiptOps::len(),
            2,
            "same-command expiry is purged without deleting another command"
        );
    }

    #[test]
    fn retained_receipt_quota_does_not_block_exact_committed_replay() {
        ReplayReceiptOps::reset_for_tests();
        let limits = ReplayReceiptRetentionLimits {
            max_active_per_actor: 1,
            max_active_per_command_kind: 1,
            purge_scan_limit: 10,
        };
        let original = input().with_expires_at_ns(200);
        let token = match reserve_or_replay_receipt_with_retention(original.clone(), limits)
            .expect("reserve")
        {
            ReplayReceiptDecision::Fresh(token) => token,
            other => panic!("expected fresh, got {other:?}"),
        };
        commit_response(&token, 1, vec![1], 110).expect("commit");

        let duplicate = reserve_or_replay_receipt_with_retention(original, limits).expect("replay");

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
        mark_external_effect_in_flight(&token, effect.clone(), 150).expect("mark external effect");
        commit_response(&token, 1, vec![1, 2, 3], 200).expect("commit receipt");

        let receipt = ReplayReceiptOps::get(token.key())
            .expect("receipt stored")
            .into_receipt()
            .expect("receipt decodes");
        assert_eq!(receipt.status, ReplayReceiptStatus::Committed);
        assert_eq!(receipt.effect, Some(effect));
    }

    #[test]
    fn committed_placement_receipt_replays_past_expiry_until_caller_acknowledges() {
        ReplayReceiptOps::reset_for_tests();

        let original_input = input_with(PLACEMENT_CHILD_REPLAY_COMMAND_KIND, p(1), [31; 32], 100)
            .with_expires_at_ns(120);
        let token = match reserve_or_replay_receipt(original_input.clone()).expect("reserve") {
            ReplayReceiptDecision::Fresh(token) => token,
            other => panic!("expected fresh, got {other:?}"),
        };
        mark_external_effect_in_flight(
            &token,
            ExternalEffectDescriptor::ManagementCreateCanister {
                command_kind: CommandKind::new(PLACEMENT_CHILD_REPLAY_COMMAND_KIND)
                    .expect("command"),
            },
            105,
        )
        .expect("mark provision effect");
        commit_response(&token, 1, vec![1, 2, 3], 110).expect("commit response");

        let mut expired_retry = original_input;
        expired_retry.now_ns = 200;
        assert!(matches!(
            reserve_or_replay_receipt(expired_retry).expect("replay retained receipt"),
            ReplayReceiptDecision::ReturnCommitted(_)
        ));
        assert_eq!(
            acknowledge_placement_receipt(
                OperationId::from_bytes([31; 32]),
                ReplayActor::direct_caller(p(2)),
            )
            .expect("wrong caller decision"),
            PlacementReceiptAcknowledgementDecision::ActorMismatch
        );
        assert_eq!(ReplayReceiptOps::len(), 1);
        assert_eq!(
            acknowledge_placement_receipt(
                OperationId::from_bytes([31; 32]),
                ReplayActor::direct_caller(p(1)),
            )
            .expect("acknowledge receipt"),
            PlacementReceiptAcknowledgementDecision::Acknowledged
        );
        assert_eq!(ReplayReceiptOps::len(), 0);
        assert_eq!(
            acknowledge_placement_receipt(
                OperationId::from_bytes([31; 32]),
                ReplayActor::direct_caller(p(1)),
            )
            .expect("repeat acknowledgement"),
            PlacementReceiptAcknowledgementDecision::AlreadyAbsent
        );
    }

    #[test]
    fn placement_receipt_acknowledgement_rejects_uncommitted_or_wrong_effect() {
        ReplayReceiptOps::reset_for_tests();

        let pending = match reserve_or_replay_receipt(input_with(
            PLACEMENT_CHILD_REPLAY_COMMAND_KIND,
            p(1),
            [32; 32],
            100,
        ))
        .expect("reserve pending")
        {
            ReplayReceiptDecision::Fresh(token) => token,
            other => panic!("expected fresh, got {other:?}"),
        };
        assert_eq!(
            acknowledge_placement_receipt(
                pending.receipt.operation_id,
                ReplayActor::direct_caller(p(1)),
            )
            .expect("pending decision"),
            PlacementReceiptAcknowledgementDecision::NotCommitted
        );

        let wrong_effect = match reserve_or_replay_receipt(input_with(
            PLACEMENT_CHILD_REPLAY_COMMAND_KIND,
            p(1),
            [33; 32],
            100,
        ))
        .expect("reserve wrong effect")
        {
            ReplayReceiptDecision::Fresh(token) => token,
            other => panic!("expected fresh, got {other:?}"),
        };
        commit_response(&wrong_effect, 1, vec![4], 110).expect("commit without effect");
        assert_eq!(
            acknowledge_placement_receipt(
                wrong_effect.receipt.operation_id,
                ReplayActor::direct_caller(p(1)),
            )
            .expect("wrong effect decision"),
            PlacementReceiptAcknowledgementDecision::NotPlacementEffect
        );
        assert_eq!(ReplayReceiptOps::len(), 2);
    }

    #[test]
    fn committed_generic_provision_receipt_expires_without_acknowledgement() {
        ReplayReceiptOps::reset_for_tests();

        let original_input = input_with(ROOT_PROVISION_REPLAY_COMMAND_KIND, p(1), [34; 32], 100)
            .with_expires_at_ns(120);
        let token = match reserve_or_replay_receipt(original_input.clone()).expect("reserve") {
            ReplayReceiptDecision::Fresh(token) => token,
            other => panic!("expected fresh, got {other:?}"),
        };
        mark_external_effect_in_flight(
            &token,
            ExternalEffectDescriptor::ManagementCreateCanister {
                command_kind: CommandKind::new(ROOT_PROVISION_REPLAY_COMMAND_KIND)
                    .expect("command"),
            },
            105,
        )
        .expect("mark generic provision effect");
        commit_response(&token, 1, vec![1, 2, 3], 110).expect("commit response");

        let mut expired_retry = original_input;
        expired_retry.now_ns = 200;
        assert!(matches!(
            reserve_or_replay_receipt(expired_retry).expect("classify expired receipt"),
            ReplayReceiptDecision::Expired
        ));
    }

    #[test]
    fn cost_settlement_recovery_preserves_staged_response_past_request_expiry() {
        ReplayReceiptOps::reset_for_tests();

        let original_input = input().with_expires_at_ns(120);
        let token = match reserve_or_replay_receipt(original_input.clone()).expect("reserve") {
            ReplayReceiptDecision::Fresh(token) => token,
            other => panic!("expected fresh, got {other:?}"),
        };
        let settlement = ReplayCostGuardSettlement {
            quota_intent_id: crate::ids::IntentId(41),
            reservation_intent_id: crate::ids::IntentId(42),
        };
        mark_costed_external_effect_in_flight(
            &token,
            ExternalEffectDescriptor::ManagementCall {
                canister: p(8),
                method: "install_code".to_string(),
            },
            settlement,
            105,
        )
        .expect("mark costed effect");
        stage_receipt_response(&token, 1, vec![1, 2, 3], 110).expect("stage response");
        mark_recovery_required(&token, RecoveryReason::CostSettlementFailed, 115)
            .expect("mark recovery");

        let mut retry_input = original_input;
        retry_input.now_ns = 200;
        let ReplayReceiptDecision::RecoveryRequired {
            token: recovery_token,
            reason,
        } = reserve_or_replay_receipt(retry_input).expect("recovery decision")
        else {
            panic!("expected recovery decision");
        };
        assert_eq!(reason, RecoveryReason::CostSettlementFailed);
        assert_eq!(
            replay_cost_guard_settlement(&recovery_token).expect("settlement identity"),
            settlement
        );

        let committed =
            commit_staged_receipt_response(&recovery_token, 205).expect("commit staged response");
        assert_eq!(committed.status, ReplayReceiptStatus::Committed);
        assert_eq!(committed.response_bytes.as_deref(), Some(&[1, 2, 3][..]));
        assert!(committed.staged_response_bytes.is_none());
    }

    #[test]
    fn stale_token_cannot_mutate_or_abort_a_reused_receipt_slot() {
        ReplayReceiptOps::reset_for_tests();

        let stale_token = match reserve_or_replay_receipt(input().with_expires_at_ns(120))
            .expect("reserve original receipt")
        {
            ReplayReceiptDecision::Fresh(token) => token,
            other => panic!("expected fresh original receipt, got {other:?}"),
        };
        ReplayReceiptOps::remove(stale_token.key()).expect("purge original receipt");

        let mut replacement_input =
            input_with("test.command.v1", p(2), [7; 32], 200).with_expires_at_ns(220);
        replacement_input.payload_hash = [8; 32];
        let replacement_token = match reserve_or_replay_receipt(replacement_input)
            .expect("reserve replacement receipt")
        {
            ReplayReceiptDecision::Fresh(token) => token,
            other => panic!("expected fresh replacement receipt, got {other:?}"),
        };
        let replacement =
            ReplayReceiptOps::get(replacement_token.key()).expect("replacement receipt is stored");
        let effect = ExternalEffectDescriptor::ManagementCall {
            canister: p(8),
            method: "deposit_cycles".to_string(),
        };
        let settlement = ReplayCostGuardSettlement {
            quota_intent_id: crate::ids::IntentId(41),
            reservation_intent_id: crate::ids::IntentId(42),
        };

        for result in [
            validate_receipt_token(&stale_token),
            mark_external_effect_in_flight(&stale_token, effect.clone(), 205),
            mark_costed_external_effect_in_flight(&stale_token, effect, settlement, 205),
            record_cost_guard_settlement(&stale_token, settlement, 205),
            stage_receipt_response(&stale_token, 1, vec![1, 2, 3], 205),
            mark_recovery_required(&stale_token, RecoveryReason::ResponseCommitFailed, 205),
            abort_reserved_receipt(&stale_token),
            abort_uncommitted_receipt(&stale_token),
        ] {
            assert_eq!(result, Err(ReplayReceiptStoreError::ReceiptTokenMismatch));
            assert_eq!(
                ReplayReceiptOps::get(replacement_token.key()),
                Some(replacement.clone())
            );
        }
        assert_eq!(
            commit_staged_receipt_response(&stale_token, 205),
            Err(ReplayReceiptStoreError::ReceiptTokenMismatch)
        );
        assert_eq!(
            replay_cost_guard_settlement(&stale_token),
            Err(ReplayReceiptStoreError::ReceiptTokenMismatch)
        );
        assert_eq!(
            ReplayReceiptOps::get(replacement_token.key()),
            Some(replacement)
        );
    }

    #[test]
    fn terminal_receipt_transitions_reject_corrupt_records_without_mutation() {
        ReplayReceiptOps::reset_for_tests();

        let token = match reserve_or_replay_receipt(input()).expect("reserve") {
            ReplayReceiptDecision::Fresh(token) => token,
            other => panic!("expected fresh, got {other:?}"),
        };
        let mut corrupt = ReplayReceiptOps::get(token.key()).expect("receipt stored");
        corrupt.schema_version = REPLAY_RECEIPT_SCHEMA_VERSION + 1;
        ReplayReceiptOps::upsert(token.key(), corrupt.clone());

        for result in [
            commit_response(&token, 1, vec![1, 2, 3], 200),
            mark_recovery_required(&token, RecoveryReason::ResponseCommitFailed, 200),
            abort_reserved_receipt(&token),
            abort_uncommitted_receipt(&token),
        ] {
            assert!(matches!(
                result,
                Err(ReplayReceiptStoreError::ReceiptDecodeFailed(_))
            ));
            assert_eq!(ReplayReceiptOps::get(token.key()), Some(corrupt.clone()));
        }
    }

    #[test]
    fn terminal_receipt_transitions_reject_missing_records() {
        ReplayReceiptOps::reset_for_tests();

        let token = match reserve_or_replay_receipt(input()).expect("reserve") {
            ReplayReceiptDecision::Fresh(token) => token,
            other => panic!("expected fresh, got {other:?}"),
        };
        ReplayReceiptOps::remove(token.key()).expect("remove reserved receipt");

        for result in [
            commit_response(&token, 1, vec![1, 2, 3], 200),
            mark_recovery_required(&token, RecoveryReason::ResponseCommitFailed, 200),
            abort_reserved_receipt(&token),
            abort_uncommitted_receipt(&token),
        ] {
            assert_eq!(result, Err(ReplayReceiptStoreError::ReceiptMissing));
            assert_eq!(ReplayReceiptOps::get(token.key()), None);
        }
    }
}
