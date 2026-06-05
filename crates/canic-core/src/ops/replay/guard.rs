use crate::{
    cdk::types::Principal,
    ops::replay::{
        model::{CommandKind, OperationId, ReplayActor},
        receipt::{
            ReplayReceiptDecision, ReplayReceiptReserveInput, ReplayReceiptStoreError,
            ReplayReceiptToken, prepare_replay_receipt,
        },
    },
    ops::storage::replay::ReplayReceiptOps,
};

use super::{slot, ttl};

/// RootReplayGuardInput
///
/// Mechanical replay input context used by the root replay guard.
#[derive(Clone, Debug)]
pub struct RootReplayGuardInput {
    pub caller: Principal,
    pub command_kind: CommandKind,
    pub operation_id: OperationId,
    pub ttl_seconds: u64,
    pub payload_hash: [u8; 32],
    pub now: u64,
    pub max_ttl_seconds: u64,
    pub purge_scan_limit: usize,
}

/// ReplayPending
///
/// Fresh replay reservation metadata for later commit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayPending {
    pub caller: Principal,
    pub receipt_token: Box<ReplayReceiptToken>,
    pub payload_hash: [u8; 32],
    pub issued_at: u64,
    pub expires_at: u64,
}

/// ReplayDecision
///
/// Pure replay outcome independent from auth/policy decisions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayDecision {
    Fresh(ReplayPending),
    DuplicateSame(ReplayCached),
    InFlight,
    DuplicateConflict,
    Expired,
    DecodeFailed(String),
}

///
/// ReplayCached
///
/// Canonical cached replay payload bytes for identical replay requests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayCached {
    pub response_bytes: Vec<u8>,
}

/// ReplayGuardError
///
/// Mechanical guard failures emitted before decision classification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayGuardError {
    InvalidTtl {
        ttl_seconds: u64,
        max_ttl_seconds: u64,
    },
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

    ttl::validate_replay_ttl(input.ttl_seconds, input.max_ttl_seconds).map_err(
        |ttl::ReplayTtlError::InvalidTtl {
             ttl_seconds,
             max_ttl_seconds,
         }| ReplayGuardError::InvalidTtl {
            ttl_seconds,
            max_ttl_seconds,
        },
    )?;

    let now_ns = secs_to_ns(input.now);
    let expires_at = input.now.saturating_add(input.ttl_seconds);
    let expires_at_ns = secs_to_ns(expires_at);
    let actor = ReplayActor::direct_caller(input.caller);
    let receipt_input = ReplayReceiptReserveInput::new(
        input.command_kind,
        input.operation_id,
        actor,
        input.payload_hash,
        now_ns,
    )
    .with_expires_at_ns(expires_at_ns);

    let decision = prepare_replay_receipt(receipt_input).map_err(
        |ReplayReceiptStoreError::ReceiptDecodeFailed(message)| {
            ReplayGuardError::ReceiptDecodeFailed(message)
        },
    )?;
    if matches!(decision, ReplayReceiptDecision::Fresh(_)) {
        let _ = slot::purge_root_expired(now_ns, input.purge_scan_limit);
    }
    Ok(map_receipt_decision(decision))
}

/// evaluate_existing_root_replay
///
/// Evaluate only existing replay state, leaving fresh requests unreserved and
/// otherwise unclassified for callers that must authorize before reserving.
pub fn evaluate_existing_root_replay(
    input: RootReplayGuardInput,
) -> Result<Option<ReplayDecision>, ReplayGuardError> {
    if let Some(decision) = evaluate_cross_command_replay(&input) {
        return Ok(Some(decision));
    }

    let now_ns = secs_to_ns(input.now);
    let expires_at = input.now.saturating_add(input.ttl_seconds);
    let expires_at_ns = secs_to_ns(expires_at);
    let actor = ReplayActor::direct_caller(input.caller);
    let receipt_input = ReplayReceiptReserveInput::new(
        input.command_kind,
        input.operation_id,
        actor,
        input.payload_hash,
        now_ns,
    )
    .with_expires_at_ns(expires_at_ns);

    let decision = prepare_replay_receipt(receipt_input).map_err(
        |ReplayReceiptStoreError::ReceiptDecodeFailed(message)| {
            ReplayGuardError::ReceiptDecodeFailed(message)
        },
    )?;
    match decision {
        ReplayReceiptDecision::Fresh(_) => Ok(None),
        other => Ok(Some(map_receipt_decision(other))),
    }
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

    let now_ns = secs_to_ns(input.now);
    if matches.iter().any(|record| {
        record
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
            let issued_at = receipt.created_at_ns / 1_000_000_000;
            let expires_at = receipt
                .expires_at_ns
                .map_or(u64::MAX, |expires_at_ns| expires_at_ns / 1_000_000_000);
            ReplayDecision::Fresh(ReplayPending {
                caller,
                receipt_token: Box::new(receipt_token),
                payload_hash,
                issued_at,
                expires_at,
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
        ReplayReceiptDecision::OperationInProgress
        | ReplayReceiptDecision::RecoveryRequired(_)
        | ReplayReceiptDecision::TerminalFailed { .. } => ReplayDecision::InFlight,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::types::Principal,
        ops::{
            replay::model::{REPLAY_RECEIPT_SCHEMA_VERSION, ReplayReceiptStatus},
            storage::replay::ReplayReceiptOps,
        },
        storage::stable::replay::ReplayReceiptRecord,
    };

    /// p
    ///
    /// Build deterministic principals for replay tests.
    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    /// base_input
    ///
    /// Build a baseline replay guard input for decision tests.
    fn base_input() -> RootReplayGuardInput {
        RootReplayGuardInput {
            caller: p(1),
            command_kind: CommandKind::new("root.test.v1").expect("command kind"),
            operation_id: OperationId::from_bytes([9u8; 32]),
            ttl_seconds: 60,
            payload_hash: [7u8; 32],
            now: 1_000,
            max_ttl_seconds: 300,
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
                    crate::ops::replay::model::REPLAY_PAYLOAD_HASH_SCHEMA_VERSION,
                payload_hash: input.payload_hash,
                status,
                created_at_ns: secs_to_ns(900),
                updated_at_ns: secs_to_ns(900),
                expires_at_ns: Some(secs_to_ns(1_200)),
                response_schema_version: None,
                response_bytes: None,
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
                    crate::ops::replay::model::REPLAY_PAYLOAD_HASH_SCHEMA_VERSION,
                payload_hash: input.payload_hash,
                status: ReplayReceiptStatus::Committed,
                created_at_ns: secs_to_ns(900),
                updated_at_ns: secs_to_ns(950),
                expires_at_ns: Some(secs_to_ns(1_200)),
                response_schema_version: Some(1),
                response_bytes: Some(expected.clone()),
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
        input.now = 1_500;
        seed_receipt(&input, ReplayReceiptStatus::Reserved);

        let decision = evaluate_root_replay(input).expect("decision");
        assert_eq!(decision, ReplayDecision::Expired);
    }

    #[test]
    fn evaluate_root_replay_returns_expired_at_expiry_boundary() {
        ReplayReceiptOps::reset_for_tests();

        let mut input = base_input();
        input.now = 1_200;
        seed_receipt(&input, ReplayReceiptStatus::Committed);

        let decision = evaluate_root_replay(input).expect("decision");
        assert_eq!(decision, ReplayDecision::Expired);
    }

    #[test]
    fn evaluate_root_replay_rejects_zero_ttl() {
        ReplayReceiptOps::reset_for_tests();

        let mut input = base_input();
        input.ttl_seconds = 0;
        let max_ttl_seconds = input.max_ttl_seconds;

        let err = evaluate_root_replay(input).expect_err("zero ttl must fail");
        assert_eq!(
            err,
            ReplayGuardError::InvalidTtl {
                ttl_seconds: 0,
                max_ttl_seconds,
            }
        );
    }

    #[test]
    fn evaluate_root_replay_rejects_ttl_above_max() {
        ReplayReceiptOps::reset_for_tests();

        let mut input = base_input();
        input.ttl_seconds = input.max_ttl_seconds + 1;
        let ttl_seconds = input.ttl_seconds;
        let max_ttl_seconds = input.max_ttl_seconds;

        let err = evaluate_root_replay(input).expect_err("ttl above max must fail");
        assert_eq!(
            err,
            ReplayGuardError::InvalidTtl {
                ttl_seconds,
                max_ttl_seconds,
            }
        );
    }
}
