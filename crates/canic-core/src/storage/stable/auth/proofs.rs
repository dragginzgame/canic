use super::{
    DelegationProofCacheStatsRecord, DelegationProofEntryRecord,
    DelegationProofEvictionClassRecord, DelegationProofKeyRecord, DelegationProofUpsertRecord,
    DelegationStateRecord,
};
use crate::storage::stable::auth::key_state;
use std::cmp::Ordering;

// Resolve one keyed proof entry from the verifier-local cache.
pub(super) fn get_proof_entry(
    entries: &[DelegationProofEntryRecord],
    key: &DelegationProofKeyRecord,
) -> Option<DelegationProofEntryRecord> {
    entries.iter().find(|entry| &entry.key == key).cloned()
}

// Upsert one proof entry, update shard key state, and compute the new cache stats.
pub(super) fn upsert_proof_entry_with_shard_public_key(
    data: &mut DelegationStateRecord,
    entry: DelegationProofEntryRecord,
    shard_public_key: Option<Vec<u8>>,
    now_secs: u64,
    capacity: usize,
    active_window_secs: u64,
) -> DelegationProofUpsertRecord {
    let mut evicted = None;

    let DelegationProofEntryRecord {
        key,
        proof,
        installed_at,
        ..
    } = entry;

    if let Some(public_key_sec1) = shard_public_key {
        key_state::set_shard_public_key(data, key.shard_pid, public_key_sec1);
    }

    if let Some(existing) = data.proofs.iter_mut().find(|current| current.key == key) {
        existing.proof = proof;
    } else {
        if data.proofs.len() >= capacity {
            evicted = evict_proof_entry(&mut data.proofs, now_secs, active_window_secs);
        }
        data.proofs.push(DelegationProofEntryRecord {
            key,
            proof,
            installed_at,
            last_verified_at: None,
        });
    }

    let stats =
        proof_cache_stats_from_entries(&data.proofs, now_secs, capacity, active_window_secs);
    DelegationProofUpsertRecord { stats, evicted }
}

// Mark one proof entry as recently verified.
pub(super) fn mark_proof_entry_verified(
    entries: &mut [DelegationProofEntryRecord],
    key: &DelegationProofKeyRecord,
    now_secs: u64,
) -> bool {
    let Some(entry) = entries.iter_mut().find(|current| &current.key == key) else {
        return false;
    };

    entry.last_verified_at = Some(now_secs);
    true
}

// Resolve the most recently installed proof entry.
pub(super) fn get_latest_proof_entry(
    entries: &[DelegationProofEntryRecord],
) -> Option<DelegationProofEntryRecord> {
    entries
        .iter()
        .max_by(|a, b| compare_proof_install_order(a, b))
        .cloned()
}

// Compute proof-cache stats from the current proof entries.
pub(super) fn proof_cache_stats_from_entries(
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

// Evict the lowest-priority proof entry under the current active window.
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

// Order two proof entries for cache eviction.
fn compare_proof_eviction_order(
    a: &DelegationProofEntryRecord,
    b: &DelegationProofEntryRecord,
    now_secs: u64,
    active_window_secs: u64,
) -> Ordering {
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

// Treat one proof entry as active if it was verified within the configured window.
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

// Order proof keys deterministically for tie-breaking.
fn compare_proof_keys(a: &DelegationProofKeyRecord, b: &DelegationProofKeyRecord) -> Ordering {
    a.shard_pid
        .as_slice()
        .cmp(b.shard_pid.as_slice())
        .then_with(|| a.cert_hash.cmp(&b.cert_hash))
}

// Order installed proofs by recency and then deterministically by proof contents.
fn compare_proof_install_order(
    a: &DelegationProofEntryRecord,
    b: &DelegationProofEntryRecord,
) -> Ordering {
    a.installed_at
        .cmp(&b.installed_at)
        .then_with(|| a.proof.cert.issued_at.cmp(&b.proof.cert.issued_at))
        .then_with(|| a.proof.cert.expires_at.cmp(&b.proof.cert.expires_at))
        .then_with(|| compare_proof_keys(&a.key, &b.key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cdk::types::Principal;
    use crate::storage::stable::auth::{DELEGATION_STATE, DelegationState};

    ///
    /// DelegationStateRestore
    ///

    struct DelegationStateRestore(DelegationStateRecord);

    impl Drop for DelegationStateRestore {
        fn drop(&mut self) {
            DELEGATION_STATE.with_borrow_mut(|cell| cell.set(self.0.clone()));
        }
    }

    // Build a deterministic dummy principal for proof-cache tests.
    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    // Build one keyed proof entry with predictable install and verify timestamps.
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
            proof: super::super::DelegationProofRecord {
                cert: super::super::DelegationCertRecord {
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

        let outcome = DelegationState::upsert_proof_entry_with_shard_public_key(
            entry(120, 2_000, None),
            None,
            1_000,
            96,
            600,
        );

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

        let outcome = DelegationState::upsert_proof_entry_with_shard_public_key(
            entry(121, 2_000, None),
            None,
            1_000,
            96,
            600,
        );

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
    fn latest_proof_entry_breaks_install_ties_by_proof_freshness_before_key() {
        let _restore =
            DelegationStateRestore(DELEGATION_STATE.with_borrow(|cell| cell.get().clone()));

        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            let mut older = entry(21, 400, None);
            older.proof.cert.expires_at = 450;
            let mut newer = entry(22, 400, None);
            newer.proof.cert.expires_at = 550;
            data.proofs = vec![older, newer];
            cell.set(data);
        });

        let latest = DelegationState::get_latest_proof_entry().expect("latest proof must exist");
        assert_eq!(latest.key.shard_pid, p(22));
        assert_eq!(latest.key.cert_hash, [22; 32]);
    }

    #[test]
    fn latest_proof_entry_breaks_full_ties_deterministically_by_key() {
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
