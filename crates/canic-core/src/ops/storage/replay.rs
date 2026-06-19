//! Module: ops::storage::replay
//!
//! Responsibility: provide deterministic access to shared replay receipt storage.
//! Does not own: replay decisions, command payload hashing, or response encoding.
//! Boundary: replay ops call this storage facade instead of stable records directly.

use crate::{
    model::replay::{CommandKind, OperationId, ReplayActor},
    storage::stable::replay::{ReplayReceiptRecord, ReplayReceiptSlotKey, ReplayReceiptStore},
};
use sha2::{Digest, Sha256};

const REPLAY_RECEIPT_SLOT_KEY_DOMAIN: &[u8] = b"canic-replay-receipt-slot-key:v1";

///
/// ReplayReceiptOps
///
/// Mechanical shared replay receipt store access (no policy).
///

pub struct ReplayReceiptOps;

impl ReplayReceiptOps {
    /// Build a shared receipt slot key from domain, command kind, and operation id.
    #[must_use]
    pub fn slot_key(command_kind: &CommandKind, operation_id: OperationId) -> ReplayReceiptSlotKey {
        let mut hasher = Sha256::new();
        hasher.update((REPLAY_RECEIPT_SLOT_KEY_DOMAIN.len() as u64).to_be_bytes());
        hasher.update(REPLAY_RECEIPT_SLOT_KEY_DOMAIN);
        let command_kind = command_kind.as_str().as_bytes();
        hasher.update((command_kind.len() as u64).to_be_bytes());
        hasher.update(command_kind);
        hasher.update(operation_id.as_bytes());
        ReplayReceiptSlotKey(hasher.finalize().into())
    }

    #[must_use]
    pub fn get(key: ReplayReceiptSlotKey) -> Option<ReplayReceiptRecord> {
        ReplayReceiptStore::get(key)
    }

    #[must_use]
    pub fn list_by_actor_operation_excluding_command(
        actor: ReplayActor,
        operation_id: OperationId,
        command_kind: &CommandKind,
    ) -> Vec<ReplayReceiptRecord> {
        ReplayReceiptStore::list_by_actor_operation_excluding_command(
            actor,
            operation_id.into_bytes(),
            command_kind.as_str(),
        )
    }

    pub fn upsert(key: ReplayReceiptSlotKey, record: ReplayReceiptRecord) {
        ReplayReceiptStore::upsert(key, record);
    }

    pub fn remove(key: ReplayReceiptSlotKey) -> Option<ReplayReceiptRecord> {
        ReplayReceiptStore::remove(key)
    }

    #[must_use]
    pub fn len() -> usize {
        ReplayReceiptStore::len()
    }

    #[must_use]
    pub fn active_len_for_actor(actor: ReplayActor, now_ns: u64) -> usize {
        ReplayReceiptStore::active_len_for_actor(actor, now_ns)
    }

    #[must_use]
    pub fn pending_len_for_actor(actor: ReplayActor, now_ns: u64) -> usize {
        ReplayReceiptStore::pending_len_for_actor(actor, now_ns)
    }

    #[must_use]
    pub fn pending_len_for_command_kind(command_kind: &CommandKind, now_ns: u64) -> usize {
        ReplayReceiptStore::pending_len_for_command_kind(command_kind.as_str(), now_ns)
    }

    pub fn purge_expired(now_ns: u64, limit: usize) -> usize {
        let expired = ReplayReceiptStore::collect_expired(now_ns, limit);
        for key in &expired {
            let _ = ReplayReceiptStore::remove(*key);
        }
        expired.len()
    }
}

#[cfg(test)]
impl ReplayReceiptOps {
    pub fn reset_for_tests() {
        ReplayReceiptStore::reset_for_tests();
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_slot_key_binds_command_kind_and_operation_id() {
        let command = CommandKind::new("test.command.v1").expect("command kind");
        let other_command = CommandKind::new("test.command.v2").expect("command kind");
        let operation_id = OperationId::from_bytes([7; 32]);
        let key = ReplayReceiptOps::slot_key(&command, operation_id);

        assert_ne!(
            key,
            ReplayReceiptOps::slot_key(&other_command, operation_id),
            "command kind must affect receipt slot key"
        );
        assert_ne!(
            key,
            ReplayReceiptOps::slot_key(&command, OperationId::from_bytes([8; 32])),
            "operation id must affect receipt slot key"
        );
    }
}
