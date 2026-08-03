//! Module: storage::stable::authority_restore
//!
//! Responsibility: persist the authority snapshot seal.
//! Does not own: canister-history observation, endpoint policy, or timer suspension.
//! Boundary: ops validates complete transitions before this single-record store mutates.

use crate::{
    cdk::{
        structures::{
            DefaultMemoryImpl, btreemap::BTreeMap as StableBtreeMap, memory::VirtualMemory,
        },
        types::Principal,
    },
    role_contract::allocation::memory::authority_restore::AUTHORITY_RESTORE_FENCE_ID,
    storage::prelude::*,
};
use std::cell::RefCell;

/// Maximum encoded bytes admitted for the complete authority restore fence.
pub const MAX_AUTHORITY_RESTORE_FENCE_RECORD_BYTES: u32 = 256;

const AUTHORITY_RESTORE_FENCE_RECORD_KEY: u8 = 0;

eager_static! {
    static AUTHORITY_RESTORE_FENCE: RefCell<
        StableBtreeMap<u8, AuthorityRestoreFenceRecord, VirtualMemory<DefaultMemoryImpl>>,
    > = RefCell::new(StableBtreeMap::init(crate::ic_memory_key!(
        authority = CANIC_CORE_MEMORY_AUTHORITY,
        key = "canic.core.authority_restore.fence.v1",
        ty = AuthorityRestoreFenceStore,
        id = AUTHORITY_RESTORE_FENCE_ID,
    )));
}

/// Exact terminal receipt retained after one live authority snapshot resumes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthorityRestoreResumeReceiptRecord {
    pub operation_id: [u8; 32],
    pub history_total_num_changes: u64,
    pub resumed_at_ns: u64,
}

/// Durable open or snapshot-sealed authority state.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AuthorityRestoreFenceStateRecord {
    Open {
        last_resume: Option<AuthorityRestoreResumeReceiptRecord>,
    },
    Sealed {
        operation_id: [u8; 32],
        history_total_num_changes: u64,
        sealed_at_ns: u64,
    },
}

/// Complete authority identity and restore-fence state.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthorityRestoreFenceRecord {
    pub authority_canister: Principal,
    pub state: AuthorityRestoreFenceStateRecord,
}

impl AuthorityRestoreFenceRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "AuthorityRestoreFenceRecord";
}

impl_storable_bounded!(
    AuthorityRestoreFenceRecord,
    MAX_AUTHORITY_RESTORE_FENCE_RECORD_BYTES,
    false
);

/// Test/audit snapshot of the optional authority restore fence.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AuthorityRestoreFenceData {
    pub record: Option<AuthorityRestoreFenceRecord>,
}

impl AuthorityRestoreFenceData {
    pub const STATE_CONTRACT_NAME: &'static str = "AuthorityRestoreFenceData";
}

/// Single-record stable authority restore-fence store.
pub struct AuthorityRestoreFenceStore;

impl AuthorityRestoreFenceStore {
    #[must_use]
    pub(crate) fn get() -> Option<AuthorityRestoreFenceRecord> {
        AUTHORITY_RESTORE_FENCE.with_borrow(|store| store.get(&AUTHORITY_RESTORE_FENCE_RECORD_KEY))
    }

    pub(crate) fn initialize(record: AuthorityRestoreFenceRecord) -> bool {
        AUTHORITY_RESTORE_FENCE.with_borrow_mut(|store| {
            if store.get(&AUTHORITY_RESTORE_FENCE_RECORD_KEY).is_some() {
                return false;
            }
            let previous = store.insert(AUTHORITY_RESTORE_FENCE_RECORD_KEY, record);
            debug_assert!(previous.is_none());
            true
        })
    }

    pub(crate) fn replace(record: AuthorityRestoreFenceRecord) -> bool {
        AUTHORITY_RESTORE_FENCE.with_borrow_mut(|store| {
            if store.get(&AUTHORITY_RESTORE_FENCE_RECORD_KEY).is_none() {
                return false;
            }
            store.insert(AUTHORITY_RESTORE_FENCE_RECORD_KEY, record);
            true
        })
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn export() -> AuthorityRestoreFenceData {
        AuthorityRestoreFenceData {
            record: Self::get(),
        }
    }

    #[cfg(test)]
    pub(crate) fn import(data: AuthorityRestoreFenceData) {
        AUTHORITY_RESTORE_FENCE.with_borrow_mut(|store| {
            store.clear_new();
            if let Some(record) = data.record {
                store.insert(AUTHORITY_RESTORE_FENCE_RECORD_KEY, record);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cdk::structures::storable::Storable;

    #[test]
    fn authority_restore_fence_roundtrips_with_a_bounded_encoding() {
        let record = AuthorityRestoreFenceRecord {
            authority_canister: Principal::from_slice(&[1]),
            state: AuthorityRestoreFenceStateRecord::Sealed {
                operation_id: [7; 32],
                history_total_num_changes: 11,
                sealed_at_ns: 13,
            },
        };

        let bytes = record.to_bytes();
        assert!(bytes.len() <= MAX_AUTHORITY_RESTORE_FENCE_RECORD_BYTES as usize);
        assert_eq!(AuthorityRestoreFenceRecord::from_bytes(bytes), record);
    }

    #[test]
    fn authority_restore_fence_initializes_once_and_replaces_exactly() {
        AuthorityRestoreFenceStore::import(AuthorityRestoreFenceData::default());
        let authority = Principal::from_slice(&[2]);
        let open = AuthorityRestoreFenceRecord {
            authority_canister: authority,
            state: AuthorityRestoreFenceStateRecord::Open { last_resume: None },
        };
        assert!(AuthorityRestoreFenceStore::initialize(open.clone()));
        assert!(!AuthorityRestoreFenceStore::initialize(open));

        let sealed = AuthorityRestoreFenceRecord {
            authority_canister: authority,
            state: AuthorityRestoreFenceStateRecord::Sealed {
                operation_id: [17; 32],
                history_total_num_changes: 19,
                sealed_at_ns: 23,
            },
        };
        assert!(AuthorityRestoreFenceStore::replace(sealed.clone()));
        assert_eq!(
            AuthorityRestoreFenceStore::export(),
            AuthorityRestoreFenceData {
                record: Some(sealed),
            }
        );
    }
}
