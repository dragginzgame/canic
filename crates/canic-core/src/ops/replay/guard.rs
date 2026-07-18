//! Module: ops::replay::guard
//!
//! Responsibility: evaluate root replay guard decisions without command logic.
//! Does not own: authorization, response encoding, or stable schemas.
//! Boundary: root request workflow calls this before executing replayed commands.

use crate::{
    cdk::types::Principal,
    model::replay::{CommandKind, OperationId, RecoveryReason, ReplayActor},
    ops::replay::{
        receipt::{
            ReplayReceiptDecision, ReplayReceiptReserveInput, ReplayReceiptStoreError,
            ReplayReceiptToken, prepare_replay_receipt,
        },
        ttl,
    },
    ops::storage::replay::ReplayReceiptOps,
};

///
/// RootReplayGuardInput
///
/// Mechanical replay input context used by the root replay guard.
///

#[derive(Clone, Debug)]
pub struct RootReplayGuardInput {
    pub caller: Principal,
    pub command_kind: CommandKind,
    pub operation_id: OperationId,
    pub ttl_ns: u64,
    pub payload_hash: [u8; 32],
    pub now_ns: u64,
    pub max_ttl_ns: u64,
    pub purge_scan_limit: usize,
}

///
/// ReplayPending
///
/// Validated replay reservation metadata for execution or recovery.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayPending {
    pub caller: Principal,
    pub receipt_token: Box<ReplayReceiptToken>,
    pub payload_hash: [u8; 32],
    pub issued_at_ns: u64,
    pub expires_at_ns: u64,
}

///
/// ReplayDecision
///
/// Pure replay outcome independent from auth/policy decisions.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayDecision {
    Fresh(ReplayPending),
    DuplicateSame(ReplayCached),
    InFlight,
    RecoveryRequired {
        pending: ReplayPending,
        reason: RecoveryReason,
    },
    DuplicateConflict,
    Expired,
    DecodeFailed(String),
}

///
/// ReplayCached
///
/// Canonical cached replay payload bytes for identical replay requests.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayCached {
    pub response_bytes: Vec<u8>,
}

///
/// ReplayGuardError
///
/// Mechanical guard failures emitted before decision classification.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayGuardError {
    InvalidTtl { ttl_ns: u64, max_ttl_ns: u64 },
    TtlOverflow { now_ns: u64, ttl_ns: u64 },
    ReceiptDecodeFailed(String),
}

/// evaluate_root_replay
///
/// Evaluate replay state and return a pure replay decision.
pub fn evaluate_root_replay(
    input: RootReplayGuardInput,
) -> Result<ReplayDecision, ReplayGuardError> {
    if let Some(decision) = evaluate_cross_command_replay(&input) {
        return Ok(decision);
    }

    validate_input_ttl(&input)?;

    let now_ns = input.now_ns;
    let expires_at_ns = now_ns
        .checked_add(input.ttl_ns)
        .ok_or(ReplayGuardError::TtlOverflow {
            now_ns,
            ttl_ns: input.ttl_ns,
        })?;
    let actor = ReplayActor::direct_caller(input.caller);
    let receipt_input = ReplayReceiptReserveInput::new(
        input.command_kind,
        input.operation_id,
        actor,
        input.payload_hash,
        now_ns,
    )
    .with_expires_at_ns(expires_at_ns);

    let decision = prepare_replay_receipt(receipt_input).map_err(|error| match error {
        ReplayReceiptStoreError::ReceiptMissing => {
            ReplayGuardError::ReceiptDecodeFailed("reserved replay receipt is missing".to_string())
        }
        ReplayReceiptStoreError::ReceiptDecodeFailed(message) => {
            ReplayGuardError::ReceiptDecodeFailed(message)
        }
        ReplayReceiptStoreError::ReceiptTokenMismatch => ReplayGuardError::ReceiptDecodeFailed(
            "replay receipt token no longer matches persisted receipt identity".to_string(),
        ),
        ReplayReceiptStoreError::StagedResponseMissing => ReplayGuardError::ReceiptDecodeFailed(
            "replay receipt is missing staged response data".to_string(),
        ),
        ReplayReceiptStoreError::CostGuardSettlementMissing => {
            ReplayGuardError::ReceiptDecodeFailed(
                "replay receipt is missing cost guard settlement identity".to_string(),
            )
        }
    })?;
    if matches!(decision, ReplayReceiptDecision::Fresh(_)) {
        let _ = ReplayReceiptOps::purge_expired(now_ns, input.purge_scan_limit);
    }
    Ok(map_receipt_decision(decision))
}

fn evaluate_cross_command_replay(input: &RootReplayGuardInput) -> Option<ReplayDecision> {
    let actor = ReplayActor::direct_caller(input.caller);
    let matches = ReplayReceiptOps::list_by_actor_operation_excluding_command(
        actor,
        input.operation_id,
        &input.command_kind,
    );
    if matches.is_empty() {
        return None;
    }

    let now_ns = input.now_ns;
    if matches.iter().any(|record| {
        matches!(
            record.status,
            crate::model::replay::ReplayReceiptStatus::ExternalEffectInFlight
                | crate::model::replay::ReplayReceiptStatus::RecoveryRequired { .. }
        ) || record
            .expires_at_ns
            .is_none_or(|expires_at_ns| now_ns < expires_at_ns)
    }) {
        return Some(ReplayDecision::DuplicateConflict);
    }
    Some(ReplayDecision::Expired)
}

fn map_receipt_decision(decision: ReplayReceiptDecision) -> ReplayDecision {
    match decision {
        ReplayReceiptDecision::Fresh(receipt_token) => {
            let receipt = receipt_token.receipt();
            let caller = receipt.actor.effective_principal;
            let payload_hash = receipt.payload_hash;
            let issued_at_ns = receipt.created_at_ns;
            let expires_at_ns = receipt.expires_at_ns.unwrap_or(u64::MAX);
            ReplayDecision::Fresh(ReplayPending {
                caller,
                receipt_token: Box::new(receipt_token),
                payload_hash,
                issued_at_ns,
                expires_at_ns,
            })
        }
        ReplayReceiptDecision::ReturnCommitted(receipt) => {
            let Some(response_bytes) = receipt.response_bytes else {
                return ReplayDecision::DecodeFailed(
                    "committed replay receipt missing response bytes".to_string(),
                );
            };
            ReplayDecision::DuplicateSame(ReplayCached { response_bytes })
        }
        ReplayReceiptDecision::RecoveryRequired {
            token: receipt_token,
            reason,
        } => {
            let receipt = receipt_token.receipt();
            ReplayDecision::RecoveryRequired {
                pending: ReplayPending {
                    caller: receipt.actor.effective_principal,
                    payload_hash: receipt.payload_hash,
                    issued_at_ns: receipt.created_at_ns,
                    expires_at_ns: receipt.expires_at_ns.unwrap_or(u64::MAX),
                    receipt_token: Box::new(receipt_token),
                },
                reason,
            }
        }
        ReplayReceiptDecision::OperationInProgress
        | ReplayReceiptDecision::PendingActorQuotaExceeded { .. }
        | ReplayReceiptDecision::PendingCommandQuotaExceeded { .. } => ReplayDecision::InFlight,
        ReplayReceiptDecision::ActorMismatch | ReplayReceiptDecision::PayloadMismatch => {
            ReplayDecision::DuplicateConflict
        }
        ReplayReceiptDecision::Expired => ReplayDecision::Expired,
    }
}

#[must_use]
pub const fn secs_to_ns(secs: u64) -> u64 {
    secs.saturating_mul(1_000_000_000)
}

fn validate_input_ttl(input: &RootReplayGuardInput) -> Result<(), ReplayGuardError> {
    ttl::validate_replay_ttl(input.ttl_ns, input.max_ttl_ns).map_err(|err| match err {
        ttl::ReplayTtlError::InvalidTtl { ttl_ns, max_ttl_ns } => {
            ReplayGuardError::InvalidTtl { ttl_ns, max_ttl_ns }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::types::Principal,
        model::replay::{REPLAY_RECEIPT_SCHEMA_VERSION, ReplayReceiptStatus},
        ops::storage::replay::ReplayReceiptOps,
        storage::stable::replay::ReplayReceiptRecord,
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn base_input() -> RootReplayGuardInput {
        RootReplayGuardInput {
            caller: p(1),
            command_kind: CommandKind::new("root.test.v1").expect("command kind"),
            operation_id: OperationId::from_bytes([9u8; 32]),
            ttl_ns: secs_to_ns(60),
            payload_hash: [7u8; 32],
            now_ns: secs_to_ns(1_000),
            max_ttl_ns: secs_to_ns(300),
            purge_scan_limit: 16,
        }
    }

    fn seed_receipt(input: &RootReplayGuardInput, status: ReplayReceiptStatus) {
        let actor = ReplayActor::direct_caller(input.caller);
        let key = ReplayReceiptOps::slot_key(&input.command_kind, input.operation_id);
        ReplayReceiptOps::upsert(
            key,
            ReplayReceiptRecord {
                schema_version: REPLAY_RECEIPT_SCHEMA_VERSION,
                command_kind: input.command_kind.as_str().to_string(),
                operation_id: input.operation_id.into_bytes(),
                actor,
                payload_hash_schema_version:
                    crate::model::replay::REPLAY_PAYLOAD_HASH_SCHEMA_VERSION,
                payload_hash: input.payload_hash,
                status,
                created_at_ns: secs_to_ns(900),
                updated_at_ns: secs_to_ns(900),
                expires_at_ns: Some(secs_to_ns(1_200)),
                response_schema_version: None,
                response_bytes: None,
                staged_response_schema_version: None,
                staged_response_bytes: None,
                cost_guard_settlement: None,
                effect: None,
            },
        );
    }

    #[test]
    fn evaluate_root_replay_returns_fresh_when_slot_missing() {
        ReplayReceiptOps::reset_for_tests();

        let decision = evaluate_root_replay(base_input()).expect("fresh decision");
        std::assert_matches!(decision, ReplayDecision::Fresh(_));
    }

    #[test]
    fn evaluate_root_replay_returns_duplicate_same_on_identical_payload() {
        ReplayReceiptOps::reset_for_tests();

        let input = base_input();
        let expected = vec![1, 2, 3];
        let actor = ReplayActor::direct_caller(input.caller);
        let key = ReplayReceiptOps::slot_key(&input.command_kind, input.operation_id);
        ReplayReceiptOps::upsert(
            key,
            ReplayReceiptRecord {
                schema_version: REPLAY_RECEIPT_SCHEMA_VERSION,
                command_kind: input.command_kind.as_str().to_string(),
                operation_id: input.operation_id.into_bytes(),
                actor,
                payload_hash_schema_version:
                    crate::model::replay::REPLAY_PAYLOAD_HASH_SCHEMA_VERSION,
                payload_hash: input.payload_hash,
                status: ReplayReceiptStatus::Committed,
                created_at_ns: secs_to_ns(900),
                updated_at_ns: secs_to_ns(950),
                expires_at_ns: Some(secs_to_ns(1_200)),
                response_schema_version: Some(1),
                response_bytes: Some(expected.clone()),
                staged_response_schema_version: None,
                staged_response_bytes: None,
                cost_guard_settlement: None,
                effect: None,
            },
        );

        let decision = evaluate_root_replay(input).expect("decision");
        assert_eq!(
            decision,
            ReplayDecision::DuplicateSame(ReplayCached {
                response_bytes: expected
            })
        );
    }

    #[test]
    fn evaluate_root_replay_returns_in_flight_for_reserved_entry_without_response() {
        ReplayReceiptOps::reset_for_tests();

        let input = base_input();
        seed_receipt(&input, ReplayReceiptStatus::Reserved);

        let decision = evaluate_root_replay(input).expect("decision");
        assert_eq!(decision, ReplayDecision::InFlight);
    }

    #[test]
    fn evaluate_root_replay_returns_duplicate_conflict_on_payload_mismatch() {
        ReplayReceiptOps::reset_for_tests();

        let input = base_input();
        let mut seeded = input.clone();
        seeded.payload_hash = [8u8; 32];
        seed_receipt(&seeded, ReplayReceiptStatus::Reserved);

        let decision = evaluate_root_replay(input).expect("decision");
        assert_eq!(decision, ReplayDecision::DuplicateConflict);
    }

    #[test]
    fn evaluate_root_replay_returns_expired_for_expired_record() {
        ReplayReceiptOps::reset_for_tests();

        let mut input = base_input();
        input.now_ns = secs_to_ns(1_500);
        seed_receipt(&input, ReplayReceiptStatus::Reserved);

        let decision = evaluate_root_replay(input).expect("decision");
        assert_eq!(decision, ReplayDecision::Expired);
    }

    #[test]
    fn evaluate_root_replay_returns_expired_at_expiry_boundary() {
        ReplayReceiptOps::reset_for_tests();

        let mut input = base_input();
        input.now_ns = secs_to_ns(1_200);
        seed_receipt(&input, ReplayReceiptStatus::Committed);

        let decision = evaluate_root_replay(input).expect("decision");
        assert_eq!(decision, ReplayDecision::Expired);
    }

    #[test]
    fn evaluate_root_replay_rejects_zero_ttl() {
        ReplayReceiptOps::reset_for_tests();

        let mut input = base_input();
        input.ttl_ns = 0;
        let max_ttl_ns = input.max_ttl_ns;

        let err = evaluate_root_replay(input).expect_err("zero ttl must fail");
        assert_eq!(
            err,
            ReplayGuardError::InvalidTtl {
                ttl_ns: 0,
                max_ttl_ns,
            }
        );
    }

    #[test]
    fn evaluate_root_replay_rejects_ttl_above_max() {
        ReplayReceiptOps::reset_for_tests();

        let mut input = base_input();
        input.ttl_ns = input.max_ttl_ns + 1;
        let ttl_ns = input.ttl_ns;
        let max_ttl_ns = input.max_ttl_ns;

        let err = evaluate_root_replay(input).expect_err("ttl above max must fail");
        assert_eq!(err, ReplayGuardError::InvalidTtl { ttl_ns, max_ttl_ns });
    }
}
