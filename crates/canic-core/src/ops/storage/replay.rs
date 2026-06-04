use crate::{
    cdk::types::Principal,
    ops::replay::model::{CommandKind, OperationId},
    storage::stable::replay::{
        ReplayReceiptRecord, ReplayReceiptSlotKey, ReplayReceiptStore, ReplaySlotKey,
        RootReplayRecord, RootReplayStore,
    },
};
use sha2::{Digest, Sha256};

const REPLAY_SLOT_KEY_DOMAIN: &[u8] = b"canic-replay-slot-key:v1";
const REPLAY_RECEIPT_SLOT_KEY_DOMAIN: &[u8] = b"canic-replay-receipt-slot-key:v1";

///
/// ReplayService
/// Shared replay service discriminator for replay key derivation.
///

#[derive(Clone, Copy, Debug)]
pub enum ReplayService {
    Root,
}

impl ReplayService {
    const fn as_bytes(self) -> &'static [u8] {
        match self {
            Self::Root => b"Root",
        }
    }
}

///
/// RootReplayOps
/// Mechanical stable replay store access (no policy).
///

pub struct RootReplayOps;

impl RootReplayOps {
    /// Build a shared replay slot key:
    /// H(domain || caller || target || service || request_id || nonce)
    #[must_use]
    pub fn slot_key(
        caller: Principal,
        target_canister: Principal,
        service: ReplayService,
        request_id: &[u8],
        nonce: [u8; 16],
    ) -> ReplaySlotKey {
        let mut hasher = Sha256::new();
        hasher.update((REPLAY_SLOT_KEY_DOMAIN.len() as u64).to_be_bytes());
        hasher.update(REPLAY_SLOT_KEY_DOMAIN);
        hasher.update(caller.as_slice());
        hasher.update(target_canister.as_slice());
        let service = service.as_bytes();
        hasher.update((service.len() as u64).to_be_bytes());
        hasher.update(service);
        hasher.update((request_id.len() as u64).to_be_bytes());
        hasher.update(request_id);
        hasher.update(nonce);
        ReplaySlotKey(hasher.finalize().into())
    }

    #[must_use]
    pub fn get(key: ReplaySlotKey) -> Option<RootReplayRecord> {
        RootReplayStore::get(key)
    }

    pub fn upsert(key: ReplaySlotKey, record: RootReplayRecord) {
        RootReplayStore::upsert(key, record);
    }

    pub fn remove(key: ReplaySlotKey) -> Option<RootReplayRecord> {
        RootReplayStore::remove(key)
    }

    #[must_use]
    pub fn len() -> usize {
        RootReplayStore::len()
    }

    #[must_use]
    pub fn active_len_for_caller(caller: Principal, now: u64) -> usize {
        RootReplayStore::active_len_for_caller(caller, now)
    }

    pub fn purge_expired(now: u64, limit: usize) -> usize {
        let expired = RootReplayStore::collect_expired(now, limit);
        for key in &expired {
            let _ = RootReplayStore::remove(*key);
        }
        expired.len()
    }
}

///
/// ReplayReceiptOps
/// Mechanical shared replay receipt store access (no policy).
///

pub struct ReplayReceiptOps;

impl ReplayReceiptOps {
    /// Build a shared receipt slot key:
    /// H(domain || command_kind || operation_id)
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

    pub fn upsert(key: ReplayReceiptSlotKey, record: ReplayReceiptRecord) {
        ReplayReceiptStore::upsert(key, record);
    }
}

#[cfg(test)]
impl ReplayReceiptOps {
    #[must_use]
    pub fn len() -> usize {
        ReplayReceiptStore::len()
    }

    pub fn reset_for_tests() {
        ReplayReceiptStore::reset_for_tests();
    }
}

#[cfg(test)]
impl RootReplayOps {
    pub fn reset_for_tests() {
        RootReplayStore::reset_for_tests();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    #[test]
    fn slot_key_binds_all_shared_identity_fields() {
        let request_id = [7u8; 32];
        let nonce = [1u8; 16];
        let key = RootReplayOps::slot_key(p(1), p(2), ReplayService::Root, &request_id, nonce);

        assert_ne!(
            key,
            RootReplayOps::slot_key(p(3), p(2), ReplayService::Root, &request_id, nonce),
            "caller must affect replay slot key"
        );
        assert_ne!(
            key,
            RootReplayOps::slot_key(p(1), p(4), ReplayService::Root, &request_id, nonce),
            "target canister must affect replay slot key"
        );
        assert_ne!(
            key,
            RootReplayOps::slot_key(p(1), p(2), ReplayService::Root, &[8u8; 32], nonce),
            "request_id must affect replay slot key"
        );
        assert_ne!(
            key,
            RootReplayOps::slot_key(p(1), p(2), ReplayService::Root, &request_id, [2u8; 16]),
            "nonce must affect replay slot key"
        );
    }

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
