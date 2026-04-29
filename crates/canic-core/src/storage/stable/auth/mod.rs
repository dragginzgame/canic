use crate::{
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    eager_static, ic_memory,
    memory::impl_storable_unbounded,
    storage::{prelude::*, stable::memory::auth::DELEGATION_STATE_ID},
};
use std::cell::RefCell;

mod key_state;
mod records;
mod sessions;

pub use records::{
    AttestationKeyStatusRecord, AttestationPublicKeyRecord, DelegatedSessionBootstrapBindingRecord,
    DelegatedSessionRecord, DelegationStateRecord, ShardPublicKeyRecord,
};

const DELEGATED_SESSION_CAPACITY: usize = 2_048;
const DELEGATED_SESSION_BOOTSTRAP_BINDING_CAPACITY: usize = 4_096;

eager_static! {
    pub(super) static DELEGATION_STATE: RefCell<Cell<DelegationStateRecord, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            ic_memory!(DelegationState, DELEGATION_STATE_ID),
            DelegationStateRecord::default(),
        ));
}

impl_storable_unbounded!(DelegationStateRecord);

///
/// DelegationState
///

pub struct DelegationState;

impl DelegationState {
    // Resolve the root verifier key, if present.
    #[must_use]
    pub(crate) fn get_root_public_key() -> Option<Vec<u8>> {
        DELEGATION_STATE.with_borrow(|cell| key_state::get_root_public_key(cell.get()))
    }

    // Persist the current root verifier key.
    pub(crate) fn set_root_public_key(public_key_sec1: Vec<u8>) {
        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            key_state::set_root_public_key(&mut data, public_key_sec1);
            cell.set(data);
        });
    }

    // Resolve a shard public key by shard principal.
    #[must_use]
    pub(crate) fn get_shard_public_key(shard_pid: Principal) -> Option<Vec<u8>> {
        DELEGATION_STATE.with_borrow(|cell| key_state::get_shard_public_key(cell.get(), shard_pid))
    }

    // Persist or replace a shard public key.
    pub(crate) fn set_shard_public_key(shard_pid: Principal, public_key_sec1: Vec<u8>) {
        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            key_state::set_shard_public_key(&mut data, shard_pid, public_key_sec1);
            cell.set(data);
        });
    }

    // Resolve an active delegated session for the wallet caller.
    #[must_use]
    pub(crate) fn get_active_delegated_session(
        wallet_pid: Principal,
        now_secs: u64,
    ) -> Option<DelegatedSessionRecord> {
        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            let session = sessions::get_active_delegated_session(
                &mut data.delegated_sessions,
                wallet_pid,
                now_secs,
            );
            if session.is_none() {
                cell.set(data);
            }
            session
        })
    }

    // Upsert a delegated session for a wallet caller.
    pub(crate) fn upsert_delegated_session(session: DelegatedSessionRecord, now_secs: u64) {
        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            sessions::upsert_delegated_session(
                &mut data.delegated_sessions,
                session,
                now_secs,
                DELEGATED_SESSION_CAPACITY,
            );
            cell.set(data);
        });
    }

    // Clear the delegated session for a wallet caller.
    pub(crate) fn clear_delegated_session(wallet_pid: Principal) {
        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            sessions::clear_delegated_session(&mut data.delegated_sessions, wallet_pid);
            cell.set(data);
        });
    }

    // Prune expired delegated sessions and report the removal count.
    pub(crate) fn prune_expired_delegated_sessions(now_secs: u64) -> usize {
        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            let removed =
                sessions::prune_expired_delegated_sessions(&mut data.delegated_sessions, now_secs);
            if removed > 0 {
                cell.set(data);
            }
            removed
        })
    }

    // Resolve an active delegated-session bootstrap binding by token fingerprint.
    #[must_use]
    pub(crate) fn get_active_delegated_session_bootstrap_binding(
        token_fingerprint: [u8; 32],
        now_secs: u64,
    ) -> Option<DelegatedSessionBootstrapBindingRecord> {
        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            let binding = sessions::get_active_delegated_session_bootstrap_binding(
                &mut data.delegated_session_bootstrap_bindings,
                token_fingerprint,
                now_secs,
            );
            if binding.is_none() {
                cell.set(data);
            }
            binding
        })
    }

    // Upsert a delegated-session bootstrap binding by token fingerprint.
    pub(crate) fn upsert_delegated_session_bootstrap_binding(
        binding: DelegatedSessionBootstrapBindingRecord,
        now_secs: u64,
    ) {
        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            sessions::upsert_delegated_session_bootstrap_binding(
                &mut data.delegated_session_bootstrap_bindings,
                binding,
                now_secs,
                DELEGATED_SESSION_BOOTSTRAP_BINDING_CAPACITY,
            );
            cell.set(data);
        });
    }

    // Prune expired delegated-session bootstrap bindings and report the removal count.
    pub(crate) fn prune_expired_delegated_session_bootstrap_bindings(now_secs: u64) -> usize {
        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            let removed = sessions::prune_expired_delegated_session_bootstrap_bindings(
                &mut data.delegated_session_bootstrap_bindings,
                now_secs,
            );
            if removed > 0 {
                cell.set(data);
            }
            removed
        })
    }

    // Resolve one attestation public key by key id.
    #[must_use]
    pub(crate) fn get_attestation_public_key(key_id: u32) -> Option<AttestationPublicKeyRecord> {
        DELEGATION_STATE
            .with_borrow(|cell| key_state::get_attestation_public_key(cell.get(), key_id))
    }

    // Resolve the full attestation public key set.
    #[must_use]
    pub(crate) fn get_attestation_public_keys() -> Vec<AttestationPublicKeyRecord> {
        DELEGATION_STATE.with_borrow(|cell| key_state::get_attestation_public_keys(cell.get()))
    }

    // Replace the attestation public key set.
    pub(crate) fn set_attestation_public_keys(keys: Vec<AttestationPublicKeyRecord>) {
        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            key_state::set_attestation_public_keys(&mut data, keys);
            cell.set(data);
        });
    }

    // Upsert one attestation public key by key id.
    pub(crate) fn upsert_attestation_public_key(key: AttestationPublicKeyRecord) {
        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            key_state::upsert_attestation_public_key(&mut data, key);
            cell.set(data);
        });
    }
}
