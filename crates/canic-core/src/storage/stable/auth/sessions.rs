use super::{DelegatedSessionBootstrapBindingRecord, DelegatedSessionRecord};
use crate::storage::prelude::*;

// Resolve an active delegated session and prune a stale wallet entry in-place.
pub(super) fn get_active_delegated_session(
    sessions: &mut Vec<DelegatedSessionRecord>,
    wallet_pid: Principal,
    now_secs: u64,
) -> Option<DelegatedSessionRecord> {
    let delegated = sessions
        .iter()
        .find(|entry| entry.wallet_pid == wallet_pid)
        .copied();

    let active = delegated.filter(|entry| !session_expired(entry.expires_at, now_secs));
    if active.is_none() {
        sessions.retain(|entry| entry.wallet_pid != wallet_pid);
    }

    active
}

// Upsert a delegated session after pruning expired entries and enforcing capacity.
pub(super) fn upsert_delegated_session(
    sessions: &mut Vec<DelegatedSessionRecord>,
    session: DelegatedSessionRecord,
    now_secs: u64,
    capacity: usize,
) {
    prune_expired_sessions(sessions, now_secs);

    if let Some(entry) = sessions
        .iter_mut()
        .find(|entry| entry.wallet_pid == session.wallet_pid)
    {
        *entry = session;
    } else {
        if sessions.len() >= capacity {
            evict_oldest_session(sessions);
        }
        sessions.push(session);
    }
}

// Remove the delegated session for the wallet caller.
pub(super) fn clear_delegated_session(
    sessions: &mut Vec<DelegatedSessionRecord>,
    wallet_pid: Principal,
) {
    sessions.retain(|entry| entry.wallet_pid != wallet_pid);
}

// Prune expired delegated sessions and return the removal count.
pub(super) fn prune_expired_delegated_sessions(
    sessions: &mut Vec<DelegatedSessionRecord>,
    now_secs: u64,
) -> usize {
    let before = sessions.len();
    prune_expired_sessions(sessions, now_secs);
    before.saturating_sub(sessions.len())
}

// Resolve an active bootstrap binding and prune a stale token entry in-place.
pub(super) fn get_active_delegated_session_bootstrap_binding(
    bindings: &mut Vec<DelegatedSessionBootstrapBindingRecord>,
    token_fingerprint: [u8; 32],
    now_secs: u64,
) -> Option<DelegatedSessionBootstrapBindingRecord> {
    let binding = bindings
        .iter()
        .find(|entry| entry.token_fingerprint == token_fingerprint)
        .copied();

    let active = binding.filter(|entry| !session_binding_expired(entry.expires_at, now_secs));
    if active.is_none() {
        bindings.retain(|entry| entry.token_fingerprint != token_fingerprint);
    }

    active
}

// Upsert a bootstrap binding after pruning expired entries and enforcing capacity.
pub(super) fn upsert_delegated_session_bootstrap_binding(
    bindings: &mut Vec<DelegatedSessionBootstrapBindingRecord>,
    binding: DelegatedSessionBootstrapBindingRecord,
    now_secs: u64,
    capacity: usize,
) {
    prune_expired_session_bindings(bindings, now_secs);

    if let Some(entry) = bindings
        .iter_mut()
        .find(|entry| entry.token_fingerprint == binding.token_fingerprint)
    {
        *entry = binding;
    } else {
        if bindings.len() >= capacity {
            evict_oldest_session_binding(bindings);
        }
        bindings.push(binding);
    }
}

// Prune expired bootstrap bindings and return the removal count.
pub(super) fn prune_expired_delegated_session_bootstrap_bindings(
    bindings: &mut Vec<DelegatedSessionBootstrapBindingRecord>,
    now_secs: u64,
) -> usize {
    let before = bindings.len();
    prune_expired_session_bindings(bindings, now_secs);
    before.saturating_sub(bindings.len())
}

// Treat delegated sessions as expired once `now_secs` passes `expires_at`.
const fn session_expired(expires_at: u64, now_secs: u64) -> bool {
    now_secs > expires_at
}

// Remove expired delegated sessions in-place.
fn prune_expired_sessions(sessions: &mut Vec<DelegatedSessionRecord>, now_secs: u64) {
    sessions.retain(|entry| !session_expired(entry.expires_at, now_secs));
}

// Treat bootstrap bindings as expired once `now_secs` passes `expires_at`.
const fn session_binding_expired(expires_at: u64, now_secs: u64) -> bool {
    now_secs > expires_at
}

// Remove expired delegated-session bootstrap bindings in-place.
fn prune_expired_session_bindings(
    bindings: &mut Vec<DelegatedSessionBootstrapBindingRecord>,
    now_secs: u64,
) {
    bindings.retain(|entry| !session_binding_expired(entry.expires_at, now_secs));
}

// Evict the oldest delegated session by expiry and then issue time.
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

// Evict the oldest bootstrap binding by expiry and then bind time.
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
