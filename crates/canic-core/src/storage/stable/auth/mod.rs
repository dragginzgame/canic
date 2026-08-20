use crate::impl_storable_unbounded;
use crate::{
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    eager_static,
    role_contract::allocation::memory::auth::AUTH_STATE_ID,
    storage::prelude::*,
};
use std::cell::RefCell;

mod records;

pub use records::{
    ActiveDelegationProofRecord, AuthStateData, AuthStateRecord, ChainKeyAlgorithmRecord,
    ChainKeyBatchHeaderRecord, ChainKeyBatchWitnessRecord, ChainKeyBatchWitnessStepRecord,
    ChainKeyDelegationCertRecord, ChainKeyKeyIdRecord, ChainKeyRootDelegationBatchIssuerRecord,
    ChainKeyRootDelegationBatchRecord, ChainKeyRootDelegationBatchStatusRecord,
    ChainKeyRootSignatureRecord, DelegatedRoleGrantRecord, DelegationCertRecord,
    DelegationProofRecord, IcChainKeyBatchSignatureProofRecord, IssuerProofAlgorithmRecord,
    IssuerProofBindingRecord, LocalApplicationAuthorityBindingRecord,
    LocalApplicationAuthorizationStateData, LocalApplicationReplayRecord,
    LocalApplicationSessionRecord, RootIssuerRecord, RootIssuerRenewalStateRecord,
    RootIssuerRenewalTemplateRecord, RootProofRecord,
};

eager_static! {
    pub(super) static AUTH_STATE: RefCell<Cell<AuthStateRecord, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            crate::ic_memory_key!(authority = CANIC_CORE_MEMORY_AUTHORITY, key = "canic.core.auth.state.v1", ty = AuthState, id = AUTH_STATE_ID),
            AuthStateRecord::default(),
        ));
}

impl_storable_unbounded!(AuthStateRecord);

///
/// AuthState
///

pub struct AuthState;

impl AuthState {
    #[cfg(test)]
    #[must_use]
    pub(crate) fn export() -> AuthStateData {
        AuthStateData {
            record: AUTH_STATE.with_borrow(|cell| cell.get().clone()),
        }
    }

    #[cfg(test)]
    pub(crate) fn import(data: AuthStateData) {
        AUTH_STATE.with_borrow_mut(|cell| cell.set(data.record));
    }

    // Return one atomic snapshot of current local application authorization state.
    #[must_use]
    pub(crate) fn application_authorization_state() -> LocalApplicationAuthorizationStateData {
        AUTH_STATE.with_borrow(|cell| {
            let data = cell.get();
            LocalApplicationAuthorizationStateData {
                sessions: data.application_sessions.clone(),
                replays: data.application_replays.clone(),
                authority_generation: data.application_authority_generation,
                authority_binding: data.application_authority_binding.clone(),
            }
        })
    }

    // Resolve one canonical application session record by its derived index.
    #[must_use]
    pub(crate) fn application_session_record(
        index: usize,
    ) -> Option<LocalApplicationSessionRecord> {
        AUTH_STATE.with_borrow(|cell| cell.get().application_sessions.get(index).cloned())
    }

    // Resolve one canonical replay record by its derived index.
    #[must_use]
    pub(crate) fn application_replay_record(index: usize) -> Option<LocalApplicationReplayRecord> {
        AUTH_STATE.with_borrow(|cell| cell.get().application_replays.get(index).copied())
    }

    // Replace local application session and replay state in one stable-cell commit.
    pub(crate) fn replace_application_authorization_state(
        state: LocalApplicationAuthorizationStateData,
    ) {
        AUTH_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            data.application_sessions = state.sessions;
            data.application_replays = state.replays;
            data.application_authority_generation = state.authority_generation;
            data.application_authority_binding = state.authority_binding;
            cell.set(data);
        });
    }

    // Return the current target-local application authority generation.
    #[must_use]
    pub(crate) fn application_authority_generation() -> u64 {
        AUTH_STATE.with_borrow(|cell| cell.get().application_authority_generation)
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
    #[cfg(test)]
    pub(crate) fn clear_active_delegation_proof() {
        AUTH_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            data.active_delegation_proof = None;
            cell.set(data);
        });
    }

    // Resolve a root delegation-proof issuer policy record by issuer principal.
    #[must_use]
    pub(crate) fn get_root_issuer(issuer_pid: Principal) -> Option<RootIssuerRecord> {
        AUTH_STATE.with_borrow(|cell| {
            cell.get()
                .root_issuers
                .iter()
                .find(|record| record.issuer_pid == issuer_pid)
                .cloned()
        })
    }

    // List root delegation-proof issuer policy records.
    #[must_use]
    pub(crate) fn list_root_issuers() -> Vec<RootIssuerRecord> {
        AUTH_STATE.with_borrow(|cell| cell.get().root_issuers.clone())
    }

    // Upsert a root delegation-proof issuer policy record.
    pub(crate) fn upsert_root_issuer(record: RootIssuerRecord) {
        AUTH_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            if let Some(existing) = data
                .root_issuers
                .iter_mut()
                .find(|existing| existing.issuer_pid == record.issuer_pid)
            {
                *existing = record;
            } else {
                data.root_issuers.push(record);
            }
            cell.set(data);
        });
    }

    // Return the current delegated-auth registry epoch.
    #[must_use]
    pub(crate) fn delegated_auth_registry_epoch() -> u64 {
        AUTH_STATE.with_borrow(|cell| cell.get().delegated_auth_registry_epoch)
    }

    // Advance the delegated-auth registry epoch after an authority-shaping mutation.
    pub(crate) fn advance_delegated_auth_registry_epoch() -> u64 {
        AUTH_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            data.delegated_auth_registry_epoch =
                data.delegated_auth_registry_epoch.saturating_add(1);
            let epoch = data.delegated_auth_registry_epoch;
            cell.set(data);
            epoch
        })
    }

    // Raise the delegated-auth registry epoch to a configured revocation floor.
    pub(crate) fn advance_delegated_auth_registry_epoch_at_least(min_epoch: u64) -> u64 {
        AUTH_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            data.delegated_auth_registry_epoch = data.delegated_auth_registry_epoch.max(min_epoch);
            let epoch = data.delegated_auth_registry_epoch;
            cell.set(data);
            epoch
        })
    }

    // Return the current delegated-auth proof epoch.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn delegated_auth_proof_epoch() -> u64 {
        AUTH_STATE.with_borrow(|cell| cell.get().delegated_auth_proof_epoch)
    }

    // Advance the delegated-auth proof epoch for a newly persisted root batch.
    pub(crate) fn advance_delegated_auth_proof_epoch_at_least(min_epoch: u64) -> u64 {
        AUTH_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            data.delegated_auth_proof_epoch = data
                .delegated_auth_proof_epoch
                .saturating_add(1)
                .max(min_epoch);
            let epoch = data.delegated_auth_proof_epoch;
            cell.set(data);
            epoch
        })
    }

    // Resolve a root-managed renewal template by issuer principal.
    #[must_use]
    pub(crate) fn get_root_issuer_renewal_template(
        issuer_pid: Principal,
    ) -> Option<RootIssuerRenewalTemplateRecord> {
        AUTH_STATE.with_borrow(|cell| {
            cell.get()
                .root_issuer_renewal_templates
                .iter()
                .find(|record| record.issuer_pid == issuer_pid)
                .cloned()
        })
    }

    // List all root-managed renewal templates.
    #[must_use]
    pub(crate) fn list_root_issuer_renewal_templates() -> Vec<RootIssuerRenewalTemplateRecord> {
        AUTH_STATE.with_borrow(|cell| cell.get().root_issuer_renewal_templates.clone())
    }

    // Upsert a root-managed renewal template.
    pub(crate) fn upsert_root_issuer_renewal_template(record: RootIssuerRenewalTemplateRecord) {
        AUTH_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            if let Some(existing) = data
                .root_issuer_renewal_templates
                .iter_mut()
                .find(|existing| existing.issuer_pid == record.issuer_pid)
            {
                *existing = record;
            } else {
                data.root_issuer_renewal_templates.push(record);
            }
            cell.set(data);
        });
    }

    // Resolve root-managed renewal state by issuer principal.
    #[must_use]
    pub(crate) fn get_root_issuer_renewal_state(
        issuer_pid: Principal,
    ) -> Option<RootIssuerRenewalStateRecord> {
        AUTH_STATE.with_borrow(|cell| {
            cell.get()
                .root_issuer_renewal_states
                .iter()
                .find(|record| record.issuer_pid == issuer_pid)
                .cloned()
        })
    }

    // Upsert root-managed renewal state.
    pub(crate) fn upsert_root_issuer_renewal_state(record: RootIssuerRenewalStateRecord) {
        AUTH_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            if let Some(existing) = data
                .root_issuer_renewal_states
                .iter_mut()
                .find(|existing| existing.issuer_pid == record.issuer_pid)
            {
                *existing = record;
            } else {
                data.root_issuer_renewal_states.push(record);
            }
            cell.set(data);
        });
    }

    // Resolve a chain-key root delegation batch by batch id.
    #[must_use]
    pub(crate) fn get_chain_key_root_delegation_batch(
        batch_id: [u8; 32],
    ) -> Option<ChainKeyRootDelegationBatchRecord> {
        AUTH_STATE.with_borrow(|cell| {
            cell.get()
                .chain_key_root_delegation_batches
                .iter()
                .find(|record| record.batch_id == batch_id)
                .cloned()
        })
    }

    // List chain-key root delegation batches.
    #[must_use]
    pub(crate) fn list_chain_key_root_delegation_batches() -> Vec<ChainKeyRootDelegationBatchRecord>
    {
        AUTH_STATE.with_borrow(|cell| cell.get().chain_key_root_delegation_batches.clone())
    }

    // Upsert a chain-key root delegation batch.
    pub(crate) fn upsert_chain_key_root_delegation_batch(
        record: ChainKeyRootDelegationBatchRecord,
    ) {
        AUTH_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            if let Some(existing) = data
                .chain_key_root_delegation_batches
                .iter_mut()
                .find(|existing| existing.batch_id == record.batch_id)
            {
                *existing = record;
            } else {
                data.chain_key_root_delegation_batches.push(record);
            }
            cell.set(data);
        });
    }

    // Remove expired chain-key root delegation batches.
    pub(crate) fn prune_chain_key_root_delegation_batches(now_ns: u64) -> usize {
        AUTH_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            let before = data.chain_key_root_delegation_batches.len();
            data.chain_key_root_delegation_batches
                .retain(|record| now_ns < record.header.expires_at_ns);
            let removed = before.saturating_sub(data.chain_key_root_delegation_batches.len());
            if removed > 0 {
                cell.set(data);
            }
            removed
        })
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::seams;

    struct AuthStateRestore(AuthStateData);

    impl Drop for AuthStateRestore {
        fn drop(&mut self) {
            AuthState::import(self.0.clone());
        }
    }

    #[test]
    fn auth_state_round_trips_through_canonical_data_snapshot() {
        let _guard = seams::lock();
        let original = AuthState::export();
        let original_epoch = original.record.delegated_auth_registry_epoch;
        let _restore = AuthStateRestore(original.clone());
        let next_epoch = AuthState::advance_delegated_auth_registry_epoch();

        assert_eq!(
            AuthState::export().record.delegated_auth_registry_epoch,
            next_epoch
        );

        AuthState::import(original);
        assert_eq!(
            AuthState::export().record.delegated_auth_registry_epoch,
            original_epoch
        );
    }
}
