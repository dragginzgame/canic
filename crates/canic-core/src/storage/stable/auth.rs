use crate::{
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    eager_static, ic_memory,
    memory::impl_storable_unbounded,
    storage::{prelude::*, stable::memory::auth::DELEGATION_STATE_ID},
};
use std::cell::RefCell;

const DELEGATED_SESSION_CAPACITY: usize = 2_048;

eager_static! {
    static DELEGATION_STATE: RefCell<Cell<DelegationStateRecord, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            ic_memory!(DelegationState, DELEGATION_STATE_ID),
            DelegationStateRecord::default(),
        ));
}

///
/// DelegationState
///

pub struct DelegationState;

impl DelegationState {
    #[must_use]
    pub(crate) fn get_proof() -> Option<DelegationProofRecord> {
        DELEGATION_STATE.with_borrow(|cell| cell.get().proof.clone())
    }

    pub(crate) fn set_proof(proof: DelegationProofRecord) {
        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            data.proof = Some(proof);
            cell.set(data);
        });
    }

    #[must_use]
    pub(crate) fn get_root_public_key() -> Option<Vec<u8>> {
        DELEGATION_STATE.with_borrow(|cell| cell.get().root_public_key.clone())
    }

    pub(crate) fn set_root_public_key(public_key_sec1: Vec<u8>) {
        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            data.root_public_key = Some(public_key_sec1);
            cell.set(data);
        });
    }

    #[must_use]
    pub(crate) fn get_shard_public_key(shard_pid: Principal) -> Option<Vec<u8>> {
        DELEGATION_STATE.with_borrow(|cell| {
            cell.get()
                .shard_public_keys
                .iter()
                .find(|entry| entry.shard_pid == shard_pid)
                .map(|entry| entry.public_key_sec1.clone())
        })
    }

    pub(crate) fn set_shard_public_key(shard_pid: Principal, public_key_sec1: Vec<u8>) {
        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();

            if let Some(entry) = data
                .shard_public_keys
                .iter_mut()
                .find(|entry| entry.shard_pid == shard_pid)
            {
                entry.public_key_sec1 = public_key_sec1;
            } else {
                data.shard_public_keys.push(ShardPublicKeyRecord {
                    shard_pid,
                    public_key_sec1,
                });
            }

            cell.set(data);
        });
    }

    // Resolve an active delegated session record for a wallet caller.
    #[must_use]
    pub(crate) fn get_active_delegated_session(
        wallet_pid: Principal,
        now_secs: u64,
    ) -> Option<DelegatedSessionRecord> {
        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            let delegated = data
                .delegated_sessions
                .iter()
                .find(|entry| entry.wallet_pid == wallet_pid)
                .copied();

            let active = delegated.filter(|entry| !session_expired(entry.expires_at, now_secs));
            if active.is_none() {
                let original_len = data.delegated_sessions.len();
                data.delegated_sessions
                    .retain(|entry| entry.wallet_pid != wallet_pid);
                if data.delegated_sessions.len() != original_len {
                    cell.set(data);
                }
            }

            active
        })
    }

    // Upsert a delegated session for a wallet caller.
    pub(crate) fn upsert_delegated_session(session: DelegatedSessionRecord, now_secs: u64) {
        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            prune_expired_sessions(&mut data.delegated_sessions, now_secs);

            if let Some(entry) = data
                .delegated_sessions
                .iter_mut()
                .find(|entry| entry.wallet_pid == session.wallet_pid)
            {
                *entry = session;
            } else {
                if data.delegated_sessions.len() >= DELEGATED_SESSION_CAPACITY {
                    evict_oldest_session(&mut data.delegated_sessions);
                }
                data.delegated_sessions.push(session);
            }

            cell.set(data);
        });
    }

    // Remove the delegated session for a wallet caller.
    pub(crate) fn clear_delegated_session(wallet_pid: Principal) {
        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            data.delegated_sessions
                .retain(|entry| entry.wallet_pid != wallet_pid);
            cell.set(data);
        });
    }

    // Prune expired delegated sessions and return how many were removed.
    pub(crate) fn prune_expired_delegated_sessions(now_secs: u64) -> usize {
        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            let before = data.delegated_sessions.len();
            prune_expired_sessions(&mut data.delegated_sessions, now_secs);
            let removed = before.saturating_sub(data.delegated_sessions.len());
            if removed > 0 {
                cell.set(data);
            }
            removed
        })
    }

    #[must_use]
    pub(crate) fn get_attestation_public_key(key_id: u32) -> Option<AttestationPublicKeyRecord> {
        DELEGATION_STATE.with_borrow(|cell| {
            cell.get()
                .attestation_public_keys
                .iter()
                .find(|entry| entry.key_id == key_id)
                .cloned()
        })
    }

    #[must_use]
    pub(crate) fn get_attestation_public_keys() -> Vec<AttestationPublicKeyRecord> {
        DELEGATION_STATE.with_borrow(|cell| cell.get().attestation_public_keys.clone())
    }

    pub(crate) fn set_attestation_public_keys(keys: Vec<AttestationPublicKeyRecord>) {
        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            data.attestation_public_keys = keys;
            cell.set(data);
        });
    }

    pub(crate) fn upsert_attestation_public_key(key: AttestationPublicKeyRecord) {
        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();

            if let Some(existing) = data
                .attestation_public_keys
                .iter_mut()
                .find(|entry| entry.key_id == key.key_id)
            {
                *existing = key;
            } else {
                data.attestation_public_keys.push(key);
            }

            cell.set(data);
        });
    }
}

///
/// DelegationCertRecord
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DelegationCertRecord {
    pub root_pid: Principal,
    pub shard_pid: Principal,
    pub issued_at: u64,
    pub expires_at: u64,
    pub scopes: Vec<String>,
    pub aud: Vec<Principal>,
}

///
/// DelegationProofRecord
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DelegationProofRecord {
    pub cert: DelegationCertRecord,
    pub cert_sig: Vec<u8>,
}

///
/// ShardPublicKeyRecord
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ShardPublicKeyRecord {
    pub shard_pid: Principal,
    pub public_key_sec1: Vec<u8>,
}

///
/// DelegatedSessionRecord
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedSessionRecord {
    pub wallet_pid: Principal,
    pub delegated_pid: Principal,
    #[serde(default)]
    pub issued_at: u64,
    #[serde(default)]
    pub expires_at: u64,
}

///
/// AttestationKeyStatusRecord
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AttestationKeyStatusRecord {
    Current,
    Previous,
}

///
/// AttestationPublicKeyRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AttestationPublicKeyRecord {
    pub key_id: u32,
    pub public_key_sec1: Vec<u8>,
    pub status: AttestationKeyStatusRecord,
    #[serde(default)]
    pub valid_from: Option<u64>,
    #[serde(default)]
    pub valid_until: Option<u64>,
}

///
/// DelegationStateRecord
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct DelegationStateRecord {
    #[serde(default)]
    pub proof: Option<DelegationProofRecord>,

    #[serde(default)]
    pub root_public_key: Option<Vec<u8>>,

    #[serde(default)]
    pub shard_public_keys: Vec<ShardPublicKeyRecord>,

    #[serde(default, alias = "delegated_callers")]
    pub delegated_sessions: Vec<DelegatedSessionRecord>,

    #[serde(default)]
    pub attestation_public_keys: Vec<AttestationPublicKeyRecord>,
}

impl_storable_unbounded!(DelegationStateRecord);

fn session_expired(expires_at: u64, now_secs: u64) -> bool {
    now_secs > expires_at
}

fn prune_expired_sessions(sessions: &mut Vec<DelegatedSessionRecord>, now_secs: u64) {
    sessions.retain(|entry| !session_expired(entry.expires_at, now_secs));
}

fn evict_oldest_session(sessions: &mut Vec<DelegatedSessionRecord>) {
    if sessions.is_empty() {
        return;
    }

    let mut oldest_index = 0usize;
    for (idx, entry) in sessions.iter().enumerate().skip(1) {
        let oldest = &sessions[oldest_index];
        if (entry.expires_at, entry.issued_at) < (oldest.expires_at, oldest.issued_at) {
            oldest_index = idx;
        }
    }

    sessions.swap_remove(oldest_index);
}
