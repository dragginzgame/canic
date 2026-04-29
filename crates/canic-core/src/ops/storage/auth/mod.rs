pub mod mapper;

use crate::{
    cdk::types::Principal,
    dto::auth::{AttestationKey, AttestationKeySet},
    storage::stable::auth::{
        DelegatedSessionBootstrapBindingRecord, DelegatedSessionRecord, DelegationState,
    },
};
use mapper::AttestationPublicKeyRecordMapper;

///
/// DelegatedSession
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DelegatedSession {
    pub wallet_pid: Principal,
    pub delegated_pid: Principal,
    pub issued_at: u64,
    pub expires_at: u64,
    pub bootstrap_token_fingerprint: Option<[u8; 32]>,
}

///
/// DelegatedSessionBootstrapBinding
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DelegatedSessionBootstrapBinding {
    pub wallet_pid: Principal,
    pub delegated_pid: Principal,
    pub token_fingerprint: [u8; 32],
    pub bound_at: u64,
    pub expires_at: u64,
}

///
/// DelegationStateOps
///
/// WHY THIS FILE EXISTS
/// --------------------
/// This module defines the **only authorized access path** to persisted
/// delegation state stored in stable memory.
///
/// It intentionally sits between:
///   - access / auth logic
///   - stable storage implementation details
///
/// Responsibilities:
/// - Provide a narrow, explicit API for delegation state access
/// - Prevent access-layer code from depending on storage internals
/// - Serve as the choke point for future changes (migration, versioning)
///
/// This is a **security-sensitive boundary**:
/// delegation state determines which signer authorities are trusted.
///

pub struct DelegationStateOps;

impl DelegationStateOps {
    #[must_use]
    pub fn root_public_key() -> Option<Vec<u8>> {
        DelegationState::get_root_public_key()
    }

    pub fn set_root_public_key(public_key_sec1: Vec<u8>) {
        DelegationState::set_root_public_key(public_key_sec1);
    }

    #[must_use]
    pub fn shard_public_key(shard_pid: Principal) -> Option<Vec<u8>> {
        DelegationState::get_shard_public_key(shard_pid)
    }

    pub fn set_shard_public_key(shard_pid: Principal, public_key_sec1: Vec<u8>) {
        DelegationState::set_shard_public_key(shard_pid, public_key_sec1);
    }

    /// Return an active delegated session for the provided wallet caller.
    #[must_use]
    pub fn delegated_session(wallet_pid: Principal, now_secs: u64) -> Option<DelegatedSession> {
        DelegationState::get_active_delegated_session(wallet_pid, now_secs)
            .map(delegated_session_record_to_view)
    }

    /// Return the active delegated subject for the provided wallet caller.
    #[must_use]
    pub fn delegated_session_subject(wallet_pid: Principal, now_secs: u64) -> Option<Principal> {
        Self::delegated_session(wallet_pid, now_secs).map(|session| session.delegated_pid)
    }

    /// Upsert the delegated session for the provided wallet caller.
    pub fn upsert_delegated_session(session: DelegatedSession, now_secs: u64) {
        DelegationState::upsert_delegated_session(
            delegated_session_view_to_record(session),
            now_secs,
        );
    }

    /// Remove the delegated session for the provided wallet caller.
    pub fn clear_delegated_session(wallet_pid: Principal) {
        DelegationState::clear_delegated_session(wallet_pid);
    }

    /// Remove all expired delegated sessions and return removed count.
    #[must_use]
    pub fn prune_expired_delegated_sessions(now_secs: u64) -> usize {
        DelegationState::prune_expired_delegated_sessions(now_secs)
    }

    /// Resolve an active delegated-session bootstrap binding by token fingerprint.
    #[must_use]
    pub fn delegated_session_bootstrap_binding(
        token_fingerprint: [u8; 32],
        now_secs: u64,
    ) -> Option<DelegatedSessionBootstrapBinding> {
        DelegationState::get_active_delegated_session_bootstrap_binding(token_fingerprint, now_secs)
            .map(delegated_session_bootstrap_binding_record_to_view)
    }

    /// Upsert delegated-session bootstrap binding metadata by token fingerprint.
    pub fn upsert_delegated_session_bootstrap_binding(
        binding: DelegatedSessionBootstrapBinding,
        now_secs: u64,
    ) {
        DelegationState::upsert_delegated_session_bootstrap_binding(
            delegated_session_bootstrap_binding_view_to_record(binding),
            now_secs,
        );
    }

    /// Remove expired delegated-session bootstrap bindings and return removed count.
    #[must_use]
    pub fn prune_expired_delegated_session_bootstrap_bindings(now_secs: u64) -> usize {
        DelegationState::prune_expired_delegated_session_bootstrap_bindings(now_secs)
    }

    #[must_use]
    pub fn attestation_public_key(key_id: u32) -> Option<AttestationKey> {
        DelegationState::get_attestation_public_key(key_id)
            .map(AttestationPublicKeyRecordMapper::record_to_dto)
    }

    #[must_use]
    pub fn attestation_public_key_sec1(key_id: u32) -> Option<Vec<u8>> {
        Self::attestation_public_key(key_id).map(|entry| entry.public_key)
    }

    #[must_use]
    pub fn attestation_keys() -> Vec<AttestationKey> {
        DelegationState::get_attestation_public_keys()
            .into_iter()
            .map(AttestationPublicKeyRecordMapper::record_to_dto)
            .collect()
    }

    pub fn set_attestation_key_set(key_set: AttestationKeySet) {
        let keys = key_set
            .keys
            .into_iter()
            .map(AttestationPublicKeyRecordMapper::dto_to_record)
            .collect();
        DelegationState::set_attestation_public_keys(keys);
    }

    pub fn upsert_attestation_key(key: AttestationKey) {
        DelegationState::upsert_attestation_public_key(
            AttestationPublicKeyRecordMapper::dto_to_record(key),
        );
    }
}

const fn delegated_session_record_to_view(record: DelegatedSessionRecord) -> DelegatedSession {
    DelegatedSession {
        wallet_pid: record.wallet_pid,
        delegated_pid: record.delegated_pid,
        issued_at: record.issued_at,
        expires_at: record.expires_at,
        bootstrap_token_fingerprint: record.bootstrap_token_fingerprint,
    }
}

const fn delegated_session_view_to_record(view: DelegatedSession) -> DelegatedSessionRecord {
    DelegatedSessionRecord {
        wallet_pid: view.wallet_pid,
        delegated_pid: view.delegated_pid,
        issued_at: view.issued_at,
        expires_at: view.expires_at,
        bootstrap_token_fingerprint: view.bootstrap_token_fingerprint,
    }
}

const fn delegated_session_bootstrap_binding_record_to_view(
    record: DelegatedSessionBootstrapBindingRecord,
) -> DelegatedSessionBootstrapBinding {
    DelegatedSessionBootstrapBinding {
        wallet_pid: record.wallet_pid,
        delegated_pid: record.delegated_pid,
        token_fingerprint: record.token_fingerprint,
        bound_at: record.bound_at,
        expires_at: record.expires_at,
    }
}

const fn delegated_session_bootstrap_binding_view_to_record(
    view: DelegatedSessionBootstrapBinding,
) -> DelegatedSessionBootstrapBindingRecord {
    DelegatedSessionBootstrapBindingRecord {
        wallet_pid: view.wallet_pid,
        delegated_pid: view.delegated_pid,
        token_fingerprint: view.token_fingerprint,
        bound_at: view.bound_at,
        expires_at: view.expires_at,
    }
}
