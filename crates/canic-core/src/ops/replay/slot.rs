use crate::ops::{
    replay::{
        ROOT_REPLAY_RESPONSE_SCHEMA_VERSION,
        guard::{ReplayPending, secs_to_ns},
        receipt::{commit_receipt_response, reserve_receipt_token},
    },
    storage::replay::ReplayReceiptOps,
};

/// reserve_root_slot
///
/// Persist a reserved replay receipt before capability execution.
pub fn reserve_root_slot(pending: &ReplayPending) {
    reserve_receipt_token(&pending.receipt_token);
}

/// commit_root_slot
///
/// Persist canonical replay response bytes for an already-reserved replay receipt.
pub fn commit_root_slot(pending: &ReplayPending, response_bytes: Vec<u8>) {
    commit_receipt_response(
        &pending.receipt_token,
        ROOT_REPLAY_RESPONSE_SCHEMA_VERSION,
        response_bytes,
        secs_to_ns(pending.issued_at),
    );
}

/// root_slot_len
///
/// Return the number of replay receipts currently stored.
#[must_use]
pub fn root_slot_len() -> usize {
    ReplayReceiptOps::len()
}

/// active_root_slot_len_for_caller
///
/// Return the number of non-expired replay receipts currently stored for a caller.
#[must_use]
pub fn active_root_slot_len_for_caller(
    caller: crate::cdk::types::Principal,
    now_secs: u64,
) -> usize {
    ReplayReceiptOps::active_len_for_actor(
        crate::ops::replay::model::ReplayActor::direct_caller(caller),
        secs_to_ns(now_secs),
    )
}

/// purge_root_expired
///
/// Purge expired replay receipts up to the provided scan limit.
pub fn purge_root_expired(now_ns: u64, limit: usize) -> usize {
    ReplayReceiptOps::purge_expired(now_ns, limit)
}
