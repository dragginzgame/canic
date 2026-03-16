use crate::{cdk::types::Principal, storage::stable::replay::ReplaySlotKey};

use super::{key, slot, ttl};

/// RootReplayGuardInput
///
/// Mechanical replay input context used by the root replay guard.
#[derive(Clone, Copy, Debug)]
pub struct RootReplayGuardInput {
    pub caller: Principal,
    pub target_canister: Principal,
    pub request_id: [u8; 32],
    pub ttl_seconds: u64,
    pub payload_hash: [u8; 32],
    pub now: u64,
    pub max_ttl_seconds: u64,
    pub purge_scan_limit: usize,
}

/// ReplayPending
///
/// Fresh replay reservation metadata for later commit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReplayPending {
    pub slot_key: ReplaySlotKey,
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
}

///
/// ReplayCached
///
/// Canonical cached replay payload bytes for identical replay requests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayCached {
    pub response_candid: Vec<u8>,
}

/// ReplayGuardError
///
/// Mechanical guard failures emitted before decision classification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplayGuardError {
    InvalidTtl {
        ttl_seconds: u64,
        max_ttl_seconds: u64,
    },
}

/// evaluate_root_replay
///
/// Evaluate replay state and return a pure replay decision.
pub fn evaluate_root_replay(
    input: RootReplayGuardInput,
) -> Result<ReplayDecision, ReplayGuardError> {
    ttl::validate_replay_ttl(input.ttl_seconds, input.max_ttl_seconds).map_err(
        |ttl::ReplayTtlError::InvalidTtl {
             ttl_seconds,
             max_ttl_seconds,
         }| ReplayGuardError::InvalidTtl {
            ttl_seconds,
            max_ttl_seconds,
        },
    )?;

    let slot_key = key::root_slot_key(input.caller, input.target_canister, input.request_id);
    if let Some(existing) = slot::get_root_slot(slot_key) {
        return Ok(resolve_existing(input.now, input.payload_hash, existing));
    }

    let _ = slot::purge_root_expired(input.now, input.purge_scan_limit);

    let issued_at = input.now;
    let expires_at = issued_at.saturating_add(input.ttl_seconds);
    Ok(ReplayDecision::Fresh(ReplayPending {
        slot_key,
        payload_hash: input.payload_hash,
        issued_at,
        expires_at,
    }))
}

/// resolve_existing
///
/// Classify an existing replay record against the new request payload.
fn resolve_existing(
    now: u64,
    payload_hash: [u8; 32],
    existing: crate::storage::stable::replay::RootReplayRecord,
) -> ReplayDecision {
    if now > existing.expires_at {
        return ReplayDecision::Expired;
    }

    if existing.payload_hash != payload_hash {
        return ReplayDecision::DuplicateConflict;
    }

    if existing.response_candid.is_empty() {
        return ReplayDecision::InFlight;
    }

    ReplayDecision::DuplicateSame(ReplayCached {
        response_candid: existing.response_candid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::types::Principal, ops::storage::replay::RootReplayOps,
        storage::stable::replay::RootReplayRecord,
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
            target_canister: p(2),
            request_id: [9u8; 32],
            ttl_seconds: 60,
            payload_hash: [7u8; 32],
            now: 1_000,
            max_ttl_seconds: 300,
            purge_scan_limit: 16,
        }
    }

    #[test]
    fn evaluate_root_replay_returns_fresh_when_slot_missing() {
        RootReplayOps::reset_for_tests();

        let decision = evaluate_root_replay(base_input()).expect("fresh decision");
        assert!(matches!(decision, ReplayDecision::Fresh(_)));
    }

    #[test]
    fn evaluate_root_replay_returns_duplicate_same_on_identical_payload() {
        RootReplayOps::reset_for_tests();

        let input = base_input();
        let slot_key = key::root_slot_key(input.caller, input.target_canister, input.request_id);
        let expected = vec![1, 2, 3];
        slot::upsert_root_slot(
            slot_key,
            RootReplayRecord {
                payload_hash: input.payload_hash,
                issued_at: 900,
                expires_at: 1_200,
                response_candid: expected.clone(),
            },
        );

        let decision = evaluate_root_replay(input).expect("decision");
        assert_eq!(
            decision,
            ReplayDecision::DuplicateSame(ReplayCached {
                response_candid: expected
            })
        );
    }

    #[test]
    fn evaluate_root_replay_returns_in_flight_for_reserved_entry_without_response() {
        RootReplayOps::reset_for_tests();

        let input = base_input();
        let slot_key = key::root_slot_key(input.caller, input.target_canister, input.request_id);
        slot::upsert_root_slot(
            slot_key,
            RootReplayRecord {
                payload_hash: input.payload_hash,
                issued_at: 900,
                expires_at: 1_200,
                response_candid: vec![],
            },
        );

        let decision = evaluate_root_replay(input).expect("decision");
        assert_eq!(decision, ReplayDecision::InFlight);
    }

    #[test]
    fn evaluate_root_replay_returns_duplicate_conflict_on_payload_mismatch() {
        RootReplayOps::reset_for_tests();

        let input = base_input();
        let slot_key = key::root_slot_key(input.caller, input.target_canister, input.request_id);
        slot::upsert_root_slot(
            slot_key,
            RootReplayRecord {
                payload_hash: [8u8; 32],
                issued_at: 900,
                expires_at: 1_200,
                response_candid: vec![],
            },
        );

        let decision = evaluate_root_replay(input).expect("decision");
        assert_eq!(decision, ReplayDecision::DuplicateConflict);
    }

    #[test]
    fn evaluate_root_replay_returns_expired_for_expired_record() {
        RootReplayOps::reset_for_tests();

        let mut input = base_input();
        input.now = 1_500;
        let slot_key = key::root_slot_key(input.caller, input.target_canister, input.request_id);
        slot::upsert_root_slot(
            slot_key,
            RootReplayRecord {
                payload_hash: input.payload_hash,
                issued_at: 900,
                expires_at: 1_200,
                response_candid: vec![],
            },
        );

        let decision = evaluate_root_replay(input).expect("decision");
        assert_eq!(decision, ReplayDecision::Expired);
    }

    #[test]
    fn evaluate_root_replay_rejects_zero_ttl() {
        RootReplayOps::reset_for_tests();

        let mut input = base_input();
        input.ttl_seconds = 0;

        let err = evaluate_root_replay(input).expect_err("zero ttl must fail");
        assert_eq!(
            err,
            ReplayGuardError::InvalidTtl {
                ttl_seconds: 0,
                max_ttl_seconds: input.max_ttl_seconds,
            }
        );
    }

    #[test]
    fn evaluate_root_replay_rejects_ttl_above_max() {
        RootReplayOps::reset_for_tests();

        let mut input = base_input();
        input.ttl_seconds = input.max_ttl_seconds + 1;

        let err = evaluate_root_replay(input).expect_err("ttl above max must fail");
        assert_eq!(
            err,
            ReplayGuardError::InvalidTtl {
                ttl_seconds: input.ttl_seconds,
                max_ttl_seconds: input.max_ttl_seconds,
            }
        );
    }
}
