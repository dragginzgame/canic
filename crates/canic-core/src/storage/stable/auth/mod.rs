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
    ActiveDelegationProofRecord, AuthStateRecord, BuildNetworkRecord, ChainKeyAlgorithmRecord,
    ChainKeyBatchHeaderRecord, ChainKeyBatchWitnessRecord, ChainKeyBatchWitnessStepRecord,
    ChainKeyDelegationCertRecord, ChainKeyKeyIdRecord, ChainKeyRootDelegationBatchIssuerRecord,
    ChainKeyRootDelegationBatchRecord, ChainKeyRootDelegationBatchStatusRecord,
    ChainKeyRootSignatureRecord, DelegatedAuthIssuerPolicySnapshotRecord,
    DelegatedAuthRegistrySnapshotRecord, DelegatedRoleGrantRecord,
    DelegatedSessionBootstrapBindingRecord, DelegatedSessionRecord, DelegationAudienceRecord,
    DelegationCertRecord, DelegationProofRecord, IcCanisterSignatureProofRecord,
    IcChainKeyBatchSignatureProofRecord, IssuerProofAlgorithmRecord, IssuerProofBindingRecord,
    RootDelegationRenewalBatchRecord, RootIssuerRecord, RootIssuerRenewalAttemptRecord,
    RootIssuerRenewalAttemptStatusRecord, RootIssuerRenewalOutcomeRecord,
    RootIssuerRenewalProofRefRecord, RootIssuerRenewalStateRecord, RootIssuerRenewalTemplateRecord,
    RootKeyPolicyRecord, RootProofModeRecord, RootProofRecord, RootProvisionerRecord,
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

    // Resolve a scheduled root-managed renewal attempt by attempt id.
    #[must_use]
    pub(crate) fn get_root_issuer_renewal_attempt(
        attempt_id: [u8; 32],
    ) -> Option<RootIssuerRenewalAttemptRecord> {
        AUTH_STATE.with_borrow(|cell| {
            cell.get()
                .root_issuer_renewal_attempts
                .iter()
                .find(|record| record.attempt_id == attempt_id)
                .cloned()
        })
    }

    // Upsert a scheduled root-managed renewal attempt.
    pub(crate) fn upsert_root_issuer_renewal_attempt(record: RootIssuerRenewalAttemptRecord) {
        AUTH_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            if let Some(existing) = data
                .root_issuer_renewal_attempts
                .iter_mut()
                .find(|existing| existing.attempt_id == record.attempt_id)
            {
                *existing = record;
            } else {
                data.root_issuer_renewal_attempts.push(record);
            }
            cell.set(data);
        });
    }

    // Resolve a scheduled root-managed renewal batch by batch id.
    #[must_use]
    pub(crate) fn get_root_delegation_renewal_batch(
        batch_id: [u8; 32],
    ) -> Option<RootDelegationRenewalBatchRecord> {
        AUTH_STATE.with_borrow(|cell| {
            cell.get()
                .root_delegation_renewal_batches
                .iter()
                .find(|record| record.batch_id == batch_id)
                .cloned()
        })
    }

    // List scheduled root-managed renewal batches.
    #[must_use]
    pub(crate) fn list_root_delegation_renewal_batches() -> Vec<RootDelegationRenewalBatchRecord> {
        AUTH_STATE.with_borrow(|cell| cell.get().root_delegation_renewal_batches.clone())
    }

    // Upsert a scheduled root-managed renewal batch.
    #[allow(
        dead_code,
        reason = "pre-0.76 bridge-backed renewal batch writer is retained for historical scheduler code during the hard-cut migration"
    )]
    pub(crate) fn upsert_root_delegation_renewal_batch(record: RootDelegationRenewalBatchRecord) {
        AUTH_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            if let Some(existing) = data
                .root_delegation_renewal_batches
                .iter_mut()
                .find(|existing| existing.batch_id == record.batch_id)
            {
                *existing = record;
            } else {
                data.root_delegation_renewal_batches.push(record);
            }
            cell.set(data);
        });
    }

    // Remove expired root-managed renewal batch transport records.
    pub(crate) fn prune_root_delegation_renewal_batches(now_ns: u64) -> usize {
        AUTH_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            let before = data.root_delegation_renewal_batches.len();
            data.root_delegation_renewal_batches
                .retain(|record| now_ns < record.retrieval_expires_at_ns);
            let removed = before.saturating_sub(data.root_delegation_renewal_batches.len());
            if removed > 0 {
                cell.set(data);
            }
            removed
        })
    }

    // Resolve a chain-key root delegation batch by batch id.
    #[must_use]
    #[allow(
        dead_code,
        reason = "0.76 chain-key install and lazy-repair wiring will use direct batch lookup"
    )]
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

    // Resolve a root-managed delegation renewal provisioner by principal.
    #[must_use]
    pub(crate) fn get_root_provisioner(principal: Principal) -> Option<RootProvisionerRecord> {
        AUTH_STATE.with_borrow(|cell| {
            cell.get()
                .root_provisioners
                .iter()
                .find(|record| record.principal == principal)
                .copied()
        })
    }

    // List root-managed delegation renewal provisioners.
    #[must_use]
    pub(crate) fn list_root_provisioners() -> Vec<RootProvisionerRecord> {
        AUTH_STATE.with_borrow(|cell| cell.get().root_provisioners.clone())
    }

    // Upsert a root-managed delegation renewal provisioner.
    pub(crate) fn upsert_root_provisioner(record: RootProvisionerRecord) {
        AUTH_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            if let Some(existing) = data
                .root_provisioners
                .iter_mut()
                .find(|existing| existing.principal == record.principal)
            {
                *existing = record;
            } else {
                data.root_provisioners.push(record);
            }
            cell.set(data);
        });
    }
}
