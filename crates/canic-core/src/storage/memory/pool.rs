use crate::{
    cdk::{
        structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
        types::{Cycles, Principal},
    },
    eager_static, ic_memory,
    ids::CanisterRole,
    memory::impl_storable_unbounded,
    storage::memory::id::pool::CANISTER_POOL_ID,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

eager_static! {
    static POOL_STORE: RefCell<
        BTreeMap<Principal, PoolRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        BTreeMap::init(ic_memory!(PoolStore, CANISTER_POOL_ID)),
    );
}

///
/// PoolData
/// Canonical storage-level export.
///

#[derive(Clone, Debug)]
pub struct PoolData {
    pub entries: Vec<(Principal, PoolRecord)>,
}

///
/// PoolStatus
/// Lifecycle status of a pooled canister.
///

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum PoolStatus {
    PendingReset,
    #[default]
    Ready,
    Failed {
        reason: String,
    },
}

impl PoolStatus {
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }
}

///
/// PoolRecord
/// Composite entry stored in stable memory.
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PoolRecord {
    pub header: PoolRecordHeader,
    pub state: PoolRecordState,
}

impl_storable_unbounded!(PoolRecord);

///
/// PoolRecordHeader
/// Immutable, ordering-relevant metadata.
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PoolRecordHeader {
    pub created_at: u64,
}

///
/// PoolRecordState
/// Mutable lifecycle state.
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PoolRecordState {
    pub cycles: Cycles,
    pub status: PoolStatus,
    pub role: Option<CanisterRole>,
    pub parent: Option<Principal>,
    pub module_hash: Option<Vec<u8>>,
}

///
/// PoolStore
/// Stable-memory backing store.
///

pub struct PoolStore;

impl PoolStore {
    /// Register a new canister into the pool.
    pub fn register(
        pid: Principal,
        cycles: Cycles,
        status: PoolStatus,
        role: Option<CanisterRole>,
        parent: Option<Principal>,
        module_hash: Option<Vec<u8>>,
        created_at: u64,
    ) {
        let record = PoolRecord {
            header: PoolRecordHeader { created_at },
            state: PoolRecordState {
                cycles,
                status,
                role,
                parent,
                module_hash,
            },
        };

        POOL_STORE.with_borrow_mut(|map| {
            map.insert(pid, record);
        });
    }

    /// Update mutable state while preserving header invariants.
    pub(crate) fn update_state_with<F>(pid: Principal, f: F) -> bool
    where
        F: FnOnce(PoolRecordState) -> PoolRecordState,
    {
        POOL_STORE.with_borrow_mut(|map| {
            let Some(old) = map.get(&pid) else {
                return false;
            };

            let new_state = f(old.state);
            let new_record = PoolRecord {
                header: old.header,
                state: new_state,
            };

            map.insert(pid, new_record);
            true
        })
    }

    pub(crate) fn remove(pid: &Principal) {
        let _ = POOL_STORE.with_borrow_mut(|map| map.remove(pid));
    }

    #[must_use]
    pub(crate) fn export() -> PoolData {
        PoolData {
            entries: POOL_STORE.with_borrow(BTreeMap::to_vec),
        }
    }

    #[must_use]
    pub(crate) fn has_status(status: PoolStatus) -> bool {
        POOL_STORE.with_borrow(|map| map.iter().any(|e| e.value().state.status == status))
    }

    #[must_use]
    pub(crate) fn contains(pid: &Principal) -> bool {
        POOL_STORE.with_borrow(|map| map.contains_key(pid))
    }
}
