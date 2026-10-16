mod records;

use crate::{
    cdk::structures::{
        DefaultMemoryImpl, Vec as StableVec, btreemap::BTreeMap, cell::Cell, memory::RuntimeMemory,
    },
    impl_storable_bounded, impl_storable_unbounded,
    storage::prelude::*,
};
use std::cell::RefCell;

#[cfg(any(test, feature = "auth-local-application-authorization"))]
pub(crate) use records::LocalApplicationAuthorityBindingRecord;

pub use records::{
    ActiveDelegationProofRecord, ChainKeyAlgorithmRecord, ChainKeyBatchHeaderRecord,
    ChainKeyBatchWitnessRecord, ChainKeyBatchWitnessStepRecord, ChainKeyDelegationCertRecord,
    ChainKeyKeyIdRecord, ChainKeyRootDelegationBatchIssuerRecord,
    ChainKeyRootDelegationBatchRecord, ChainKeyRootDelegationBatchStatusRecord,
    ChainKeyRootSignatureRecord, DelegatedRoleGrantRecord, DelegatedTokenIssuerStateData,
    DelegatedTokenIssuerStateRecord, DelegationCertRecord, DelegationProofRecord,
    IcChainKeyBatchSignatureProofRecord, IssuerProofAlgorithmRecord, IssuerProofBindingRecord,
    LocalApplicationAuthorizationStateData, LocalApplicationAuthorizationStateRecord,
    LocalApplicationLoginRecord, LocalApplicationReplayRecord, LocalApplicationSessionRecord,
    RootDelegationStateData, RootDelegationStateRecord, RootIssuerRecord,
    RootIssuerRenewalStateRecord, RootIssuerRenewalTemplateRecord, RootProofRecord,
};

#[cfg(test)]
pub(crate) use records::LocalApplicationLoginTotalsRecord;

#[cfg(any(test, feature = "auth-local-application-authorization"))]
thread_local! {
    pub(super) static LOCAL_APPLICATION_AUTHORIZATION_STATE: RefCell<Cell<LocalApplicationAuthorizationStateRecord, RuntimeMemory<DefaultMemoryImpl>>> =
        RefCell::new(init_local_application_authorization_state());
}

thread_local! {
    static APPLICATION_LOGINS: RefCell<BTreeMap<Principal, LocalApplicationLoginRecord, RuntimeMemory<DefaultMemoryImpl>>> = RefCell::new(BTreeMap::init(
        crate::memory::open(crate::memory::keys::AUTH_LOCAL_APPLICATION_AUTHORIZATION_LOGINS_V1)
    ));
    static APPLICATION_SESSIONS: RefCell<StableVec<LocalApplicationSessionRecord, RuntimeMemory<DefaultMemoryImpl>>> = RefCell::new(StableVec::init(
        crate::memory::open(crate::memory::keys::AUTH_LOCAL_APPLICATION_AUTHORIZATION_SESSIONS_V1)
    ));
    static APPLICATION_REPLAYS: RefCell<StableVec<LocalApplicationReplayRecord, RuntimeMemory<DefaultMemoryImpl>>> = RefCell::new(StableVec::init(
        crate::memory::open(crate::memory::keys::AUTH_LOCAL_APPLICATION_AUTHORIZATION_REPLAYS_V1)
    ));
}

thread_local! {
    pub(super) static DELEGATED_TOKEN_ISSUER_STATE: RefCell<Cell<DelegatedTokenIssuerStateRecord, RuntimeMemory<DefaultMemoryImpl>>> =
        RefCell::new(init_delegated_token_issuer_state());
}

thread_local! {
    pub(super) static ROOT_DELEGATION_STATE: RefCell<Cell<RootDelegationStateRecord, RuntimeMemory<DefaultMemoryImpl>>> =
        RefCell::new(init_root_delegation_state());
}

#[cfg(any(test, feature = "auth-local-application-authorization"))]
fn init_local_application_authorization_state()
-> Cell<LocalApplicationAuthorizationStateRecord, RuntimeMemory<DefaultMemoryImpl>> {
    #[cfg(any(test, feature = "auth-local-application-authorization"))]
    {
        Cell::init(
            crate::memory::open(crate::memory::keys::AUTH_LOCAL_APPLICATION_AUTHORIZATION_STATE_V1),
            LocalApplicationAuthorizationStateRecord::default(),
        )
    }

    #[cfg(not(any(test, feature = "auth-local-application-authorization")))]
    panic!("local application authorization state requires its compile-time capability");
}

fn init_delegated_token_issuer_state()
-> Cell<DelegatedTokenIssuerStateRecord, RuntimeMemory<DefaultMemoryImpl>> {
    #[cfg(any(test, feature = "auth-delegated-token-issuer-state"))]
    {
        Cell::init(
            crate::memory::open(crate::memory::keys::AUTH_DELEGATED_TOKEN_ISSUER_STATE_V1),
            DelegatedTokenIssuerStateRecord::default(),
        )
    }

    #[cfg(not(any(test, feature = "auth-delegated-token-issuer-state")))]
    panic!("delegated-token issuer state requires its compile-time capability");
}

fn init_root_delegation_state() -> Cell<RootDelegationStateRecord, RuntimeMemory<DefaultMemoryImpl>>
{
    #[cfg(any(test, feature = "auth-root-delegation-state"))]
    {
        Cell::init(
            crate::memory::open(crate::memory::keys::AUTH_ROOT_DELEGATION_STATE_V1),
            RootDelegationStateRecord::default(),
        )
    }

    #[cfg(not(any(test, feature = "auth-root-delegation-state")))]
    panic!("Root delegation state requires its compile-time capability");
}

impl_storable_unbounded!(LocalApplicationAuthorizationStateRecord);
impl_storable_bounded!(LocalApplicationSessionRecord, 4096, false);
impl_storable_bounded!(LocalApplicationReplayRecord, 512, false);
impl_storable_bounded!(LocalApplicationLoginRecord, 1_152, false);
impl_storable_unbounded!(DelegatedTokenIssuerStateRecord);
impl_storable_unbounded!(RootDelegationStateRecord);

///
/// LocalApplicationAuthorizationState
///

#[cfg(any(test, feature = "auth-local-application-authorization"))]
pub struct LocalApplicationAuthorizationState;

#[cfg(any(test, feature = "auth-local-application-authorization"))]
impl LocalApplicationAuthorizationState {
    #[cfg(test)]
    #[must_use]
    pub(crate) fn export() -> LocalApplicationAuthorizationStateData {
        Self::application_authorization_state()
    }

    #[cfg(test)]
    pub(crate) fn import(data: LocalApplicationAuthorizationStateData) {
        Self::replace_application_authorization_state(data);
    }

    // Return one atomic snapshot of current local application authorization state.
    #[must_use]
    #[cfg(any(test, feature = "auth-local-application-authorization"))]
    pub(crate) fn application_authorization_state() -> LocalApplicationAuthorizationStateData {
        LOCAL_APPLICATION_AUTHORIZATION_STATE.with_borrow(|cell| {
            let data = cell.get();
            LocalApplicationAuthorizationStateData {
                sessions: APPLICATION_SESSIONS.with_borrow(|rows| rows.iter().collect()),
                replays: APPLICATION_REPLAYS.with_borrow(|rows| rows.iter().collect()),
                authority_generation: data.application_authority_generation,
                authority_binding: data.application_authority_binding.clone(),
                logins: APPLICATION_LOGINS.with_borrow(|rows| {
                    let mut logins = Vec::new();
                    for entry in rows.iter() {
                        logins.push(entry.into_pair());
                    }
                    logins
                }),
                login_totals: data.login_totals,
            }
        })
    }

    // Resolve one canonical application session record by its derived index.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn application_session_record(
        index: usize,
    ) -> Option<LocalApplicationSessionRecord> {
        APPLICATION_SESSIONS
            .with_borrow(|map| map.get(u64::try_from(index).expect("bounded session index")))
    }

    // Replace the header and both vectors in one synchronous IC message.
    #[cfg(any(test, feature = "auth-local-application-authorization"))]
    pub(crate) fn replace_application_authorization_state(
        state: LocalApplicationAuthorizationStateData,
    ) {
        LOCAL_APPLICATION_AUTHORIZATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            data.application_authority_generation = state.authority_generation;
            data.application_authority_binding = state.authority_binding;
            data.login_totals = state.login_totals;
            cell.set(data);
        });
        APPLICATION_LOGINS.with_borrow_mut(|rows| {
            rows.clear_new();
            for (caller, record) in state.logins {
                rows.insert(caller, record);
            }
        });
        APPLICATION_SESSIONS.with_borrow(|rows| {
            for (index, record) in state.sessions.iter().enumerate() {
                let index = u64::try_from(index).expect("bounded index");
                if index < rows.len() {
                    rows.set(index, record);
                } else {
                    rows.push(record);
                }
            }
            while rows.len() > state.sessions.len() as u64 {
                rows.pop();
            }
        });
        APPLICATION_REPLAYS.with_borrow(|rows| {
            for (index, record) in state.replays.iter().enumerate() {
                let index = u64::try_from(index).expect("bounded index");
                if index < rows.len() {
                    rows.set(index, record);
                } else {
                    rows.push(record);
                }
            }
            while rows.len() > state.replays.len() as u64 {
                rows.pop();
            }
        });
    }

    /// Commit one fully validated pair in the current synchronous IC message.
    /// No fallible work or asynchronous boundary may separate these writes.
    #[cfg(test)]
    pub(crate) fn commit_shared_session_pair(
        session_index: usize,
        session: LocalApplicationSessionRecord,
        replay_index: usize,
        replay: LocalApplicationReplayRecord,
        login: LocalApplicationLoginRecord,
        totals: LocalApplicationLoginTotalsRecord,
    ) {
        Self::commit_application_login(session.transport_caller, login, totals);
        APPLICATION_SESSIONS.with_borrow(|rows| {
            let index = u64::try_from(session_index).expect("bounded index");
            if index < rows.len() {
                rows.set(index, &session);
            } else {
                rows.push(&session);
            }
        });
        APPLICATION_REPLAYS.with_borrow(|rows| {
            assert_eq!(rows.len(), replay_index as u64, "admitted replay append");
            rows.push(&replay);
        });
    }

    /// Read one retained canonical login without loading the ledger census.
    #[cfg(test)]
    pub(crate) fn application_login_record(
        caller: Principal,
    ) -> Option<LocalApplicationLoginRecord> {
        APPLICATION_LOGINS.with_borrow(|rows| rows.get(&caller))
    }

    /// Read protected occupancy from the same synchronous commit as login rows.
    #[cfg(test)]
    pub(crate) fn application_login_totals() -> LocalApplicationLoginTotalsRecord {
        LOCAL_APPLICATION_AUTHORIZATION_STATE.with_borrow(|cell| cell.get().login_totals)
    }

    #[cfg(test)]
    fn commit_application_login(
        caller: Principal,
        login: LocalApplicationLoginRecord,
        totals: LocalApplicationLoginTotalsRecord,
    ) {
        APPLICATION_LOGINS.with_borrow_mut(|rows| {
            rows.insert(caller, login);
        });
        LOCAL_APPLICATION_AUTHORIZATION_STATE.with_borrow_mut(|cell| {
            let mut header = cell.get().clone();
            header.login_totals = totals;
            cell.set(header);
        });
    }

    /// Commit a logout tombstone and swap-remove only its session. Replay survives.
    #[cfg(test)]
    pub(crate) fn commit_shared_logout(
        caller: Principal,
        session_index: Option<usize>,
        login: LocalApplicationLoginRecord,
        totals: LocalApplicationLoginTotalsRecord,
    ) {
        Self::commit_application_login(caller, login, totals);
        if let Some(index) = session_index {
            APPLICATION_SESSIONS.with_borrow(|rows| {
                let last = rows.pop().expect("validated nonempty session store");
                if (index as u64) < rows.len() {
                    rows.set(index as u64, &last);
                }
            });
        }
    }

    // Return the current target-local application authority generation.
    #[must_use]
    #[cfg(any(test, feature = "auth-local-application-authorization"))]
    pub(crate) fn application_authority_generation() -> u64 {
        LOCAL_APPLICATION_AUTHORIZATION_STATE
            .with_borrow(|cell| cell.get().application_authority_generation)
    }
}

/// Capability-owned stable state for one delegated-token issuer.
pub struct DelegatedTokenIssuerState;

impl DelegatedTokenIssuerState {
    /// Open the selected store synchronously so invalid stable bytes fail restoration.
    #[cfg(feature = "auth-delegated-token-issuer-state")]
    pub(crate) fn restore() {
        DELEGATED_TOKEN_ISSUER_STATE.with_borrow(|cell| {
            let _ = cell.get();
        });
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn export() -> DelegatedTokenIssuerStateData {
        DelegatedTokenIssuerStateData {
            record: DELEGATED_TOKEN_ISSUER_STATE.with_borrow(|cell| cell.get().clone()),
        }
    }

    #[cfg(test)]
    pub(crate) fn import(data: DelegatedTokenIssuerStateData) {
        DELEGATED_TOKEN_ISSUER_STATE.with_borrow_mut(|cell| cell.set(data.record));
    }

    // Resolve the issuer's installed active delegation proof.
    #[must_use]
    pub(crate) fn get_active_delegation_proof() -> Option<ActiveDelegationProofRecord> {
        DELEGATED_TOKEN_ISSUER_STATE.with_borrow(|cell| cell.get().active_delegation_proof.clone())
    }

    // Replace the issuer's installed active delegation proof.
    pub(crate) fn set_active_delegation_proof(proof: ActiveDelegationProofRecord) {
        DELEGATED_TOKEN_ISSUER_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            data.active_delegation_proof = Some(proof);
            cell.set(data);
        });
    }

    // Clear the issuer's installed active delegation proof.
    #[cfg(test)]
    pub(crate) fn clear_active_delegation_proof() {
        DELEGATED_TOKEN_ISSUER_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            data.active_delegation_proof = None;
            cell.set(data);
        });
    }
}

/// Capability-owned stable state for Root delegation policy and renewal.
pub struct RootDelegationState;

impl RootDelegationState {
    #[cfg(test)]
    #[must_use]
    pub(crate) fn export() -> RootDelegationStateData {
        RootDelegationStateData {
            record: ROOT_DELEGATION_STATE.with_borrow(|cell| cell.get().clone()),
        }
    }

    #[cfg(test)]
    pub(crate) fn import(data: RootDelegationStateData) {
        ROOT_DELEGATION_STATE.with_borrow_mut(|cell| cell.set(data.record));
    }

    // Resolve a root delegation-proof issuer policy record by issuer principal.
    #[must_use]
    pub(crate) fn get_root_issuer(issuer_pid: Principal) -> Option<RootIssuerRecord> {
        ROOT_DELEGATION_STATE.with_borrow(|cell| {
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
        ROOT_DELEGATION_STATE.with_borrow(|cell| cell.get().root_issuers.clone())
    }

    // Upsert a root delegation-proof issuer policy record.
    pub(crate) fn upsert_root_issuer(record: RootIssuerRecord) {
        ROOT_DELEGATION_STATE.with_borrow_mut(|cell| {
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
        ROOT_DELEGATION_STATE.with_borrow(|cell| cell.get().delegated_auth_registry_epoch)
    }

    // Advance the delegated-auth registry epoch after an authority-shaping mutation.
    pub(crate) fn advance_delegated_auth_registry_epoch() -> u64 {
        ROOT_DELEGATION_STATE.with_borrow_mut(|cell| {
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
        ROOT_DELEGATION_STATE.with_borrow_mut(|cell| {
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
        ROOT_DELEGATION_STATE.with_borrow(|cell| cell.get().delegated_auth_proof_epoch)
    }

    // Advance the delegated-auth proof epoch for a newly persisted root batch.
    pub(crate) fn advance_delegated_auth_proof_epoch_at_least(min_epoch: u64) -> u64 {
        ROOT_DELEGATION_STATE.with_borrow_mut(|cell| {
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
        ROOT_DELEGATION_STATE.with_borrow(|cell| {
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
        ROOT_DELEGATION_STATE.with_borrow(|cell| cell.get().root_issuer_renewal_templates.clone())
    }

    // Upsert a root-managed renewal template.
    pub(crate) fn upsert_root_issuer_renewal_template(record: RootIssuerRenewalTemplateRecord) {
        ROOT_DELEGATION_STATE.with_borrow_mut(|cell| {
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
        ROOT_DELEGATION_STATE.with_borrow(|cell| {
            cell.get()
                .root_issuer_renewal_states
                .iter()
                .find(|record| record.issuer_pid == issuer_pid)
                .cloned()
        })
    }

    // Upsert root-managed renewal state.
    pub(crate) fn upsert_root_issuer_renewal_state(record: RootIssuerRenewalStateRecord) {
        ROOT_DELEGATION_STATE.with_borrow_mut(|cell| {
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
        ROOT_DELEGATION_STATE.with_borrow(|cell| {
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
        ROOT_DELEGATION_STATE
            .with_borrow(|cell| cell.get().chain_key_root_delegation_batches.clone())
    }

    // Upsert a chain-key root delegation batch.
    pub(crate) fn upsert_chain_key_root_delegation_batch(
        record: ChainKeyRootDelegationBatchRecord,
    ) {
        ROOT_DELEGATION_STATE.with_borrow_mut(|cell| {
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
        ROOT_DELEGATION_STATE.with_borrow_mut(|cell| {
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

    struct DelegatedTokenIssuerStateRestore(DelegatedTokenIssuerStateData);

    impl Drop for DelegatedTokenIssuerStateRestore {
        fn drop(&mut self) {
            DelegatedTokenIssuerState::import(self.0.clone());
        }
    }

    struct RootDelegationStateRestore(RootDelegationStateData);

    impl Drop for RootDelegationStateRestore {
        fn drop(&mut self) {
            RootDelegationState::import(self.0.clone());
        }
    }

    #[test]
    fn delegated_token_issuer_state_round_trips_through_canonical_data_snapshot() {
        let _guard = seams::lock();
        let original = DelegatedTokenIssuerState::export();
        let _restore = DelegatedTokenIssuerStateRestore(original.clone());

        DelegatedTokenIssuerState::import(DelegatedTokenIssuerStateData {
            record: original.record.clone(),
        });

        assert_eq!(DelegatedTokenIssuerState::export().record, original.record);
    }

    #[test]
    fn root_delegation_state_round_trips_through_canonical_data_snapshot() {
        let _guard = seams::lock();
        let original = RootDelegationState::export();
        let original_epoch = original.record.delegated_auth_registry_epoch;
        let _restore = RootDelegationStateRestore(original.clone());
        let next_epoch = RootDelegationState::advance_delegated_auth_registry_epoch();

        assert_eq!(
            RootDelegationState::export()
                .record
                .delegated_auth_registry_epoch,
            next_epoch
        );

        RootDelegationState::import(original);
        assert_eq!(
            RootDelegationState::export()
                .record
                .delegated_auth_registry_epoch,
            original_epoch
        );
    }
}
