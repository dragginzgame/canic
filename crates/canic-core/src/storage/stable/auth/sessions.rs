use super::{DelegatedSessionBootstrapBindingRecord, DelegatedSessionRecord};
use crate::storage::prelude::*;

///
/// DelegatedSessionUpsertResult
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelegatedSessionUpsertResult {
    Upserted,
    SessionCapacityReached {
        capacity: usize,
    },
    SessionSubjectCapacityReached {
        delegated_pid: Principal,
        capacity: usize,
    },
    BootstrapBindingCapacityReached {
        capacity: usize,
    },
    BootstrapBindingSubjectCapacityReached {
        delegated_pid: Principal,
        capacity: usize,
    },
}

///
/// DelegatedSessionCapacityLimits
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct DelegatedSessionCapacityLimits {
    pub(super) session: usize,
    pub(super) session_subject: usize,
    pub(super) binding: usize,
    pub(super) binding_subject: usize,
}

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
#[cfg(test)]
pub(super) fn upsert_delegated_session(
    sessions: &mut Vec<DelegatedSessionRecord>,
    session: DelegatedSessionRecord,
    now_secs: u64,
    capacity: usize,
    subject_capacity: usize,
) -> DelegatedSessionUpsertResult {
    prune_expired_sessions(sessions, now_secs);

    if let Some(result) =
        delegated_session_capacity_error(sessions, session, capacity, subject_capacity)
    {
        return result;
    }

    upsert_delegated_session_unchecked(sessions, session);
    DelegatedSessionUpsertResult::Upserted
}

pub(super) fn upsert_delegated_session_with_bootstrap_binding(
    sessions: &mut Vec<DelegatedSessionRecord>,
    bindings: &mut Vec<DelegatedSessionBootstrapBindingRecord>,
    session: DelegatedSessionRecord,
    binding: DelegatedSessionBootstrapBindingRecord,
    now_secs: u64,
    limits: DelegatedSessionCapacityLimits,
) -> DelegatedSessionUpsertResult {
    prune_expired_sessions(sessions, now_secs);
    prune_expired_session_bindings(bindings, now_secs);

    if let Some(result) =
        delegated_session_capacity_error(sessions, session, limits.session, limits.session_subject)
    {
        return result;
    }

    if let Some(result) = delegated_session_bootstrap_binding_capacity_error(
        bindings,
        binding,
        limits.binding,
        limits.binding_subject,
    ) {
        return result;
    }

    upsert_delegated_session_unchecked(sessions, session);
    upsert_delegated_session_bootstrap_binding_unchecked(bindings, binding);
    DelegatedSessionUpsertResult::Upserted
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

// Prune expired bootstrap bindings and return the removal count.
pub(super) fn prune_expired_delegated_session_bootstrap_bindings(
    bindings: &mut Vec<DelegatedSessionBootstrapBindingRecord>,
    now_secs: u64,
) -> usize {
    let before = bindings.len();
    prune_expired_session_bindings(bindings, now_secs);
    before.saturating_sub(bindings.len())
}

// Treat delegated sessions as expired at the same exclusive boundary as tokens.
const fn session_expired(expires_at: u64, now_secs: u64) -> bool {
    now_secs >= expires_at
}

// Remove expired delegated sessions in-place.
fn prune_expired_sessions(sessions: &mut Vec<DelegatedSessionRecord>, now_secs: u64) {
    sessions.retain(|entry| !session_expired(entry.expires_at, now_secs));
}

// Treat bootstrap bindings as expired at the same exclusive boundary as tokens.
const fn session_binding_expired(expires_at: u64, now_secs: u64) -> bool {
    now_secs >= expires_at
}

// Remove expired delegated-session bootstrap bindings in-place.
fn prune_expired_session_bindings(
    bindings: &mut Vec<DelegatedSessionBootstrapBindingRecord>,
    now_secs: u64,
) {
    bindings.retain(|entry| !session_binding_expired(entry.expires_at, now_secs));
}

fn delegated_session_capacity_error(
    sessions: &[DelegatedSessionRecord],
    session: DelegatedSessionRecord,
    capacity: usize,
    subject_capacity: usize,
) -> Option<DelegatedSessionUpsertResult> {
    let existing = sessions
        .iter()
        .find(|entry| entry.wallet_pid == session.wallet_pid);
    let increases_subject =
        existing.is_none_or(|entry| entry.delegated_pid != session.delegated_pid);

    if increases_subject
        && sessions
            .iter()
            .filter(|entry| entry.delegated_pid == session.delegated_pid)
            .count()
            >= subject_capacity
    {
        return Some(
            DelegatedSessionUpsertResult::SessionSubjectCapacityReached {
                delegated_pid: session.delegated_pid,
                capacity: subject_capacity,
            },
        );
    }

    if existing.is_none() && sessions.len() >= capacity {
        return Some(DelegatedSessionUpsertResult::SessionCapacityReached { capacity });
    }

    None
}

fn delegated_session_bootstrap_binding_capacity_error(
    bindings: &[DelegatedSessionBootstrapBindingRecord],
    binding: DelegatedSessionBootstrapBindingRecord,
    capacity: usize,
    subject_capacity: usize,
) -> Option<DelegatedSessionUpsertResult> {
    let existing = bindings
        .iter()
        .find(|entry| entry.token_fingerprint == binding.token_fingerprint);
    let increases_subject =
        existing.is_none_or(|entry| entry.delegated_pid != binding.delegated_pid);

    if increases_subject
        && bindings
            .iter()
            .filter(|entry| entry.delegated_pid == binding.delegated_pid)
            .count()
            >= subject_capacity
    {
        return Some(
            DelegatedSessionUpsertResult::BootstrapBindingSubjectCapacityReached {
                delegated_pid: binding.delegated_pid,
                capacity: subject_capacity,
            },
        );
    }

    if existing.is_none() && bindings.len() >= capacity {
        return Some(DelegatedSessionUpsertResult::BootstrapBindingCapacityReached { capacity });
    }

    None
}

fn upsert_delegated_session_unchecked(
    sessions: &mut Vec<DelegatedSessionRecord>,
    session: DelegatedSessionRecord,
) {
    if let Some(entry) = sessions
        .iter_mut()
        .find(|entry| entry.wallet_pid == session.wallet_pid)
    {
        *entry = session;
    } else {
        sessions.push(session);
    }
}

fn upsert_delegated_session_bootstrap_binding_unchecked(
    bindings: &mut Vec<DelegatedSessionBootstrapBindingRecord>,
    binding: DelegatedSessionBootstrapBindingRecord,
) {
    if let Some(entry) = bindings
        .iter_mut()
        .find(|entry| entry.token_fingerprint == binding.token_fingerprint)
    {
        *entry = binding;
    } else {
        bindings.push(binding);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn session(wallet: u8, delegated: u8, issued_at: u64) -> DelegatedSessionRecord {
        DelegatedSessionRecord {
            wallet_pid: p(wallet),
            delegated_pid: p(delegated),
            issued_at,
            expires_at: issued_at + 100,
            bootstrap_token_fingerprint: None,
        }
    }

    fn binding(
        token: u8,
        wallet: u8,
        delegated: u8,
        bound_at: u64,
    ) -> DelegatedSessionBootstrapBindingRecord {
        DelegatedSessionBootstrapBindingRecord {
            wallet_pid: p(wallet),
            delegated_pid: p(delegated),
            token_fingerprint: [token; 32],
            bound_at,
            expires_at: bound_at + 100,
        }
    }

    fn limits(
        session: usize,
        session_subject: usize,
        binding: usize,
        binding_subject: usize,
    ) -> DelegatedSessionCapacityLimits {
        DelegatedSessionCapacityLimits {
            session,
            session_subject,
            binding,
            binding_subject,
        }
    }

    #[test]
    fn session_subject_capacity_rejects_without_global_eviction() {
        let mut sessions = vec![session(1, 9, 10), session(2, 8, 10)];

        let result = upsert_delegated_session(&mut sessions, session(3, 9, 11), 11, 10, 1);

        assert_eq!(
            result,
            DelegatedSessionUpsertResult::SessionSubjectCapacityReached {
                delegated_pid: p(9),
                capacity: 1,
            }
        );
        assert_eq!(sessions, vec![session(1, 9, 10), session(2, 8, 10)]);
    }

    #[test]
    fn session_global_capacity_rejects_without_oldest_eviction() {
        let mut sessions = vec![session(1, 9, 10), session(2, 8, 20)];

        let result = upsert_delegated_session(&mut sessions, session(3, 7, 30), 30, 2, 10);

        assert_eq!(
            result,
            DelegatedSessionUpsertResult::SessionCapacityReached { capacity: 2 }
        );
        assert_eq!(sessions, vec![session(1, 9, 10), session(2, 8, 20)]);
    }

    #[test]
    fn paired_upsert_is_atomic_when_binding_subject_capacity_rejects() {
        let mut sessions = Vec::new();
        let mut bindings = vec![binding(1, 1, 9, 10)];

        let result = upsert_delegated_session_with_bootstrap_binding(
            &mut sessions,
            &mut bindings,
            session(2, 9, 20),
            binding(2, 2, 9, 20),
            20,
            limits(10, 10, 10, 1),
        );

        assert_eq!(
            result,
            DelegatedSessionUpsertResult::BootstrapBindingSubjectCapacityReached {
                delegated_pid: p(9),
                capacity: 1,
            }
        );
        assert!(sessions.is_empty());
        assert_eq!(bindings, vec![binding(1, 1, 9, 10)]);
    }
}
