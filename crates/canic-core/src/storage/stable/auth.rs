use crate::{
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    eager_static, ic_memory,
    memory::impl_storable_unbounded,
    storage::{prelude::*, stable::memory::auth::DELEGATION_STATE_ID},
};
use std::cell::RefCell;

const DELEGATED_SESSION_CAPACITY: usize = 2_048;
const DELEGATED_SESSION_BOOTSTRAP_BINDING_CAPACITY: usize = 4_096;

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
    pub(crate) fn get_proof_entry(
        key: &DelegationProofKeyRecord,
    ) -> Option<DelegationProofEntryRecord> {
        DELEGATION_STATE.with_borrow(|cell| {
            cell.get()
                .proofs
                .iter()
                .find(|entry| &entry.key == key)
                .cloned()
        })
    }

    pub(crate) fn upsert_proof_entry(
        entry: DelegationProofEntryRecord,
        now_secs: u64,
        capacity: usize,
        active_window_secs: u64,
    ) -> DelegationProofUpsertRecord {
        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            let mut evicted = None;

            if let Some(existing) = data
                .proofs
                .iter_mut()
                .find(|current| current.key == entry.key)
            {
                existing.proof = entry.proof.clone();
            } else {
                if data.proofs.len() >= capacity {
                    evicted = evict_proof_entry(&mut data.proofs, now_secs, active_window_secs);
                }
                data.proofs.push(entry.clone());
            }

            let stats = proof_cache_stats_from_entries(
                &data.proofs,
                now_secs,
                capacity,
                active_window_secs,
            );
            cell.set(data);
            DelegationProofUpsertRecord { stats, evicted }
        })
    }

    pub(crate) fn mark_proof_entry_verified(key: &DelegationProofKeyRecord, now_secs: u64) -> bool {
        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            let Some(entry) = data.proofs.iter_mut().find(|current| &current.key == key) else {
                return false;
            };

            entry.last_verified_at = Some(now_secs);
            cell.set(data);
            true
        })
    }

    #[must_use]
    pub(crate) fn get_latest_proof_entry() -> Option<DelegationProofEntryRecord> {
        DELEGATION_STATE.with_borrow(|cell| {
            cell.get()
                .proofs
                .iter()
                .max_by(|a, b| compare_proof_install_order(a, b))
                .cloned()
        })
    }

    #[must_use]
    pub(crate) fn proof_cache_stats(
        now_secs: u64,
        capacity: usize,
        active_window_secs: u64,
    ) -> DelegationProofCacheStatsRecord {
        DELEGATION_STATE.with_borrow(|cell| {
            proof_cache_stats_from_entries(
                &cell.get().proofs,
                now_secs,
                capacity,
                active_window_secs,
            )
        })
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

    // Resolve an active delegated-session bootstrap binding by token fingerprint.
    #[must_use]
    pub(crate) fn get_active_delegated_session_bootstrap_binding(
        token_fingerprint: [u8; 32],
        now_secs: u64,
    ) -> Option<DelegatedSessionBootstrapBindingRecord> {
        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            let binding = data
                .delegated_session_bootstrap_bindings
                .iter()
                .find(|entry| entry.token_fingerprint == token_fingerprint)
                .copied();

            let active =
                binding.filter(|entry| !session_binding_expired(entry.expires_at, now_secs));
            if active.is_none() {
                let before = data.delegated_session_bootstrap_bindings.len();
                data.delegated_session_bootstrap_bindings
                    .retain(|entry| entry.token_fingerprint != token_fingerprint);
                if data.delegated_session_bootstrap_bindings.len() != before {
                    cell.set(data);
                }
            }

            active
        })
    }

    // Upsert a delegated-session bootstrap binding by token fingerprint.
    pub(crate) fn upsert_delegated_session_bootstrap_binding(
        binding: DelegatedSessionBootstrapBindingRecord,
        now_secs: u64,
    ) {
        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            prune_expired_session_bindings(
                &mut data.delegated_session_bootstrap_bindings,
                now_secs,
            );

            if let Some(entry) = data
                .delegated_session_bootstrap_bindings
                .iter_mut()
                .find(|entry| entry.token_fingerprint == binding.token_fingerprint)
            {
                *entry = binding;
            } else {
                if data.delegated_session_bootstrap_bindings.len()
                    >= DELEGATED_SESSION_BOOTSTRAP_BINDING_CAPACITY
                {
                    evict_oldest_session_binding(&mut data.delegated_session_bootstrap_bindings);
                }
                data.delegated_session_bootstrap_bindings.push(binding);
            }

            cell.set(data);
        });
    }

    // Prune expired delegated-session bootstrap bindings and return removed count.
    pub(crate) fn prune_expired_delegated_session_bootstrap_bindings(now_secs: u64) -> usize {
        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            let before = data.delegated_session_bootstrap_bindings.len();
            prune_expired_session_bindings(
                &mut data.delegated_session_bootstrap_bindings,
                now_secs,
            );
            let removed = before.saturating_sub(data.delegated_session_bootstrap_bindings.len());
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegationProofRecord {
    pub cert: DelegationCertRecord,
    pub cert_sig: Vec<u8>,
}

///
/// DelegationProofKeyRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegationProofKeyRecord {
    pub shard_pid: Principal,
    pub cert_hash: [u8; 32],
}

///
/// DelegationProofEntryRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegationProofEntryRecord {
    pub key: DelegationProofKeyRecord,
    pub proof: DelegationProofRecord,
    #[serde(default)]
    pub installed_at: u64,
    #[serde(default)]
    pub last_verified_at: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelegationProofEvictionClassRecord {
    Cold,
    Active,
}

///
/// DelegationProofCacheStatsRecord
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DelegationProofCacheStatsRecord {
    pub size: usize,
    pub active_count: usize,
    pub capacity: usize,
}

///
/// DelegationProofUpsertRecord
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DelegationProofUpsertRecord {
    pub stats: DelegationProofCacheStatsRecord,
    pub evicted: Option<DelegationProofEvictionClassRecord>,
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
    #[serde(default)]
    pub bootstrap_token_fingerprint: Option<[u8; 32]>,
}

///
/// DelegatedSessionBootstrapBindingRecord
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedSessionBootstrapBindingRecord {
    pub wallet_pid: Principal,
    pub delegated_pid: Principal,
    pub token_fingerprint: [u8; 32],
    #[serde(default)]
    pub bound_at: u64,
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
    pub proofs: Vec<DelegationProofEntryRecord>,

    #[serde(default)]
    pub root_public_key: Option<Vec<u8>>,

    #[serde(default)]
    pub shard_public_keys: Vec<ShardPublicKeyRecord>,

    #[serde(default)]
    pub delegated_sessions: Vec<DelegatedSessionRecord>,

    #[serde(default)]
    pub delegated_session_bootstrap_bindings: Vec<DelegatedSessionBootstrapBindingRecord>,

    #[serde(default)]
    pub attestation_public_keys: Vec<AttestationPublicKeyRecord>,
}

impl_storable_unbounded!(DelegationStateRecord);

const fn session_expired(expires_at: u64, now_secs: u64) -> bool {
    now_secs > expires_at
}

fn prune_expired_sessions(sessions: &mut Vec<DelegatedSessionRecord>, now_secs: u64) {
    sessions.retain(|entry| !session_expired(entry.expires_at, now_secs));
}

const fn session_binding_expired(expires_at: u64, now_secs: u64) -> bool {
    now_secs > expires_at
}

fn prune_expired_session_bindings(
    bindings: &mut Vec<DelegatedSessionBootstrapBindingRecord>,
    now_secs: u64,
) {
    bindings.retain(|entry| !session_binding_expired(entry.expires_at, now_secs));
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

fn evict_oldest_session_binding(bindings: &mut Vec<DelegatedSessionBootstrapBindingRecord>) {
    if bindings.is_empty() {
        return;
    }

    let mut oldest_index = 0usize;
    for (idx, entry) in bindings.iter().enumerate().skip(1) {
        let oldest = &bindings[oldest_index];
        if (entry.expires_at, entry.bound_at) < (oldest.expires_at, oldest.bound_at) {
            oldest_index = idx;
        }
    }

    bindings.swap_remove(oldest_index);
}

fn evict_proof_entry(
    entries: &mut Vec<DelegationProofEntryRecord>,
    now_secs: u64,
    active_window_secs: u64,
) -> Option<DelegationProofEvictionClassRecord> {
    if entries.is_empty() {
        return None;
    }

    let mut eviction_index = 0usize;
    for (idx, entry) in entries.iter().enumerate().skip(1) {
        let current = &entries[eviction_index];
        if compare_proof_eviction_order(entry, current, now_secs, active_window_secs).is_lt() {
            eviction_index = idx;
        }
    }

    let evicted = entries.swap_remove(eviction_index);
    Some(
        if proof_entry_is_active(&evicted, now_secs, active_window_secs) {
            DelegationProofEvictionClassRecord::Active
        } else {
            DelegationProofEvictionClassRecord::Cold
        },
    )
}

fn compare_proof_eviction_order(
    a: &DelegationProofEntryRecord,
    b: &DelegationProofEntryRecord,
    now_secs: u64,
    active_window_secs: u64,
) -> std::cmp::Ordering {
    let a_active = proof_entry_is_active(a, now_secs, active_window_secs);
    let b_active = proof_entry_is_active(b, now_secs, active_window_secs);

    a_active
        .cmp(&b_active)
        .then_with(|| {
            a.last_verified_at
                .unwrap_or(0)
                .cmp(&b.last_verified_at.unwrap_or(0))
        })
        .then_with(|| a.installed_at.cmp(&b.installed_at))
        .then_with(|| compare_proof_keys(&a.key, &b.key))
}

const fn proof_entry_is_active(
    entry: &DelegationProofEntryRecord,
    now_secs: u64,
    active_window_secs: u64,
) -> bool {
    match entry.last_verified_at {
        Some(last_verified_at) => now_secs.saturating_sub(last_verified_at) <= active_window_secs,
        None => false,
    }
}

fn proof_cache_stats_from_entries(
    entries: &[DelegationProofEntryRecord],
    now_secs: u64,
    capacity: usize,
    active_window_secs: u64,
) -> DelegationProofCacheStatsRecord {
    let active_count = entries
        .iter()
        .filter(|entry| proof_entry_is_active(entry, now_secs, active_window_secs))
        .count();

    DelegationProofCacheStatsRecord {
        size: entries.len(),
        active_count,
        capacity,
    }
}

fn compare_proof_keys(
    a: &DelegationProofKeyRecord,
    b: &DelegationProofKeyRecord,
) -> std::cmp::Ordering {
    a.shard_pid
        .as_slice()
        .cmp(b.shard_pid.as_slice())
        .then_with(|| a.cert_hash.cmp(&b.cert_hash))
}

fn compare_proof_install_order(
    a: &DelegationProofEntryRecord,
    b: &DelegationProofEntryRecord,
) -> std::cmp::Ordering {
    a.installed_at
        .cmp(&b.installed_at)
        .then_with(|| compare_proof_keys(&a.key, &b.key))
}

#[cfg(test)]
mod tests {
    use super::*;

    ///
    /// DelegationStateRestore
    ///

    struct DelegationStateRestore(DelegationStateRecord);

    impl Drop for DelegationStateRestore {
        fn drop(&mut self) {
            DELEGATION_STATE.with_borrow_mut(|cell| cell.set(self.0.clone()));
        }
    }

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn entry(
        id: u8,
        installed_at: u64,
        last_verified_at: Option<u64>,
    ) -> DelegationProofEntryRecord {
        DelegationProofEntryRecord {
            key: DelegationProofKeyRecord {
                shard_pid: p(id),
                cert_hash: [id; 32],
            },
            proof: DelegationProofRecord {
                cert: DelegationCertRecord {
                    root_pid: p(1),
                    shard_pid: p(id),
                    issued_at: installed_at,
                    expires_at: installed_at + 100,
                    scopes: vec!["verify".to_string()],
                    aud: vec![p(2)],
                },
                cert_sig: vec![id],
            },
            installed_at,
            last_verified_at,
        }
    }

    #[test]
    fn proof_cache_stats_count_active_entries_within_window() {
        let _restore =
            DelegationStateRestore(DELEGATION_STATE.with_borrow(|cell| cell.get().clone()));

        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            data.proofs = vec![entry(11, 100, Some(50)), entry(12, 110, Some(500))];
            cell.set(data);
        });

        let stats = DelegationState::proof_cache_stats(700, 96, 600);
        assert_eq!(stats.size, 2);
        assert_eq!(stats.active_count, 1);
        assert_eq!(stats.capacity, 96);
    }

    #[test]
    fn eviction_prefers_cold_entries_before_active_entries() {
        let _restore =
            DelegationStateRestore(DELEGATION_STATE.with_borrow(|cell| cell.get().clone()));

        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            data.proofs = (0..96)
                .map(|idx| {
                    let id = u8::try_from(idx + 1).expect("capacity fits in u8");
                    let last_verified_at = if idx == 0 { Some(1_000) } else { None };
                    entry(id, u64::try_from(idx).expect("idx fits"), last_verified_at)
                })
                .collect();
            cell.set(data);
        });

        let outcome = DelegationState::upsert_proof_entry(entry(120, 2_000, None), 1_000, 96, 600);

        assert_eq!(
            outcome.evicted,
            Some(DelegationProofEvictionClassRecord::Cold)
        );
        assert_eq!(outcome.stats.size, 96);
        assert_eq!(outcome.stats.active_count, 1);
        assert!(
            DelegationState::get_proof_entry(&DelegationProofKeyRecord {
                shard_pid: p(1),
                cert_hash: [1; 32],
            })
            .is_some()
        );
    }

    #[test]
    fn eviction_reports_active_when_all_entries_are_active() {
        let _restore =
            DelegationStateRestore(DELEGATION_STATE.with_borrow(|cell| cell.get().clone()));

        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            data.proofs = (0..96)
                .map(|idx| {
                    let id = u8::try_from(idx + 1).expect("capacity fits in u8");
                    entry(id, u64::try_from(idx).expect("idx fits"), Some(1_000))
                })
                .collect();
            cell.set(data);
        });

        let outcome = DelegationState::upsert_proof_entry(entry(121, 2_000, None), 1_000, 96, 600);

        assert_eq!(
            outcome.evicted,
            Some(DelegationProofEvictionClassRecord::Active)
        );
        assert_eq!(outcome.stats.size, 96);
        assert_eq!(outcome.stats.active_count, 95);
    }

    #[test]
    fn latest_proof_entry_prefers_most_recent_install() {
        let _restore =
            DelegationStateRestore(DELEGATION_STATE.with_borrow(|cell| cell.get().clone()));

        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            data.proofs = vec![
                entry(11, 100, None),
                entry(12, 300, None),
                entry(13, 200, None),
            ];
            cell.set(data);
        });

        let latest = DelegationState::get_latest_proof_entry().expect("latest proof must exist");
        assert_eq!(latest.key.shard_pid, p(12));
        assert_eq!(latest.installed_at, 300);
    }

    #[test]
    fn latest_proof_entry_breaks_install_ties_deterministically_by_key() {
        let _restore =
            DelegationStateRestore(DELEGATION_STATE.with_borrow(|cell| cell.get().clone()));

        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            data.proofs = vec![entry(21, 400, None), entry(22, 400, None)];
            cell.set(data);
        });

        let latest = DelegationState::get_latest_proof_entry().expect("latest proof must exist");
        assert_eq!(latest.key.shard_pid, p(22));
        assert_eq!(latest.key.cert_hash, [22; 32]);
    }
}
