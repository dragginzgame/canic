use crate::impl_storable_unbounded;
use crate::{
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    eager_static,
    storage::{prelude::*, stable::memory::auth::AUTH_STATE_ID},
};
use std::cell::RefCell;

mod records;
mod sessions;

pub use records::{
    ActiveDelegationProofRecord, AuthStateRecord, DelegatedRoleGrantRecord,
    DelegatedSessionBootstrapBindingRecord, DelegatedSessionRecord, DelegationAudienceRecord,
    DelegationCertRecord, DelegationProofRecord, IcCanisterSignatureProofRecord,
    IssuerProofAlgorithmRecord, IssuerProofBindingRecord, RootProofRecord,
};
pub use sessions::DelegatedSessionUpsertResult;

const DELEGATED_SESSION_CAPACITY: usize = 2_048;
const DELEGATED_SESSION_SUBJECT_CAPACITY: usize = 128;
const DELEGATED_SESSION_BOOTSTRAP_BINDING_CAPACITY: usize = 4_096;
const DELEGATED_SESSION_BOOTSTRAP_BINDING_SUBJECT_CAPACITY: usize = 256;

eager_static! {
    pub(super) static AUTH_STATE: RefCell<Cell<AuthStateRecord, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            crate::ic_memory_key!("canic.core.auth_state.v1", AuthState, AUTH_STATE_ID),
            AuthStateRecord::default(),
        ));
}

impl_storable_unbounded!(AuthStateRecord);

///
/// AuthState
///

pub struct AuthState;

impl AuthState {
    // Resolve an active delegated session for the wallet caller.
    #[must_use]
    pub(crate) fn get_active_delegated_session(
        wallet_pid: Principal,
        now_secs: u64,
    ) -> Option<DelegatedSessionRecord> {
        AUTH_STATE.with_borrow_mut(|cell| {
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
    #[cfg(test)]
    pub(crate) fn upsert_delegated_session(
        session: DelegatedSessionRecord,
        now_secs: u64,
    ) -> DelegatedSessionUpsertResult {
        AUTH_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            let result = sessions::upsert_delegated_session(
                &mut data.delegated_sessions,
                session,
                now_secs,
                DELEGATED_SESSION_CAPACITY,
                DELEGATED_SESSION_SUBJECT_CAPACITY,
            );
            if matches!(result, DelegatedSessionUpsertResult::Upserted) {
                cell.set(data);
            }
            result
        })
    }

    pub(crate) fn upsert_delegated_session_with_bootstrap_binding(
        session: DelegatedSessionRecord,
        binding: DelegatedSessionBootstrapBindingRecord,
        now_secs: u64,
    ) -> DelegatedSessionUpsertResult {
        AUTH_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            let result = sessions::upsert_delegated_session_with_bootstrap_binding(
                &mut data.delegated_sessions,
                &mut data.delegated_session_bootstrap_bindings,
                session,
                binding,
                now_secs,
                sessions::DelegatedSessionCapacityLimits {
                    session: DELEGATED_SESSION_CAPACITY,
                    session_subject: DELEGATED_SESSION_SUBJECT_CAPACITY,
                    binding: DELEGATED_SESSION_BOOTSTRAP_BINDING_CAPACITY,
                    binding_subject: DELEGATED_SESSION_BOOTSTRAP_BINDING_SUBJECT_CAPACITY,
                },
            );
            if matches!(result, DelegatedSessionUpsertResult::Upserted) {
                cell.set(data);
            }
            result
        })
    }

    // Clear the delegated session for a wallet caller.
    pub(crate) fn clear_delegated_session(wallet_pid: Principal) {
        AUTH_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            sessions::clear_delegated_session(&mut data.delegated_sessions, wallet_pid);
            cell.set(data);
        });
    }

    // Prune expired delegated sessions and report the removal count.
    pub(crate) fn prune_expired_delegated_sessions(now_secs: u64) -> usize {
        AUTH_STATE.with_borrow_mut(|cell| {
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
        AUTH_STATE.with_borrow_mut(|cell| {
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

    // Prune expired delegated-session bootstrap bindings and report the removal count.
    pub(crate) fn prune_expired_delegated_session_bootstrap_bindings(now_secs: u64) -> usize {
        AUTH_STATE.with_borrow_mut(|cell| {
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

    // Resolve the issuer's installed active delegation proof.
    #[must_use]
    pub(crate) fn get_active_delegation_proof() -> Option<ActiveDelegationProofRecord> {
        AUTH_STATE.with_borrow(|cell| cell.get().active_delegation_proof.clone())
    }

    // Replace the issuer's installed active delegation proof.
    pub(crate) fn set_active_delegation_proof(proof: ActiveDelegationProofRecord) {
        AUTH_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            data.active_delegation_proof = Some(proof);
            cell.set(data);
        });
    }

    // Clear the issuer's installed active delegation proof.
    pub(crate) fn clear_active_delegation_proof() {
        AUTH_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            data.active_delegation_proof = None;
            cell.set(data);
        });
    }
}
