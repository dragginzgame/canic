//! Stable records for one Fleet Subnet Root's exclusive physical-Canister inventory.

use canic_core::{
    cdk::{
        structures::{
            DefaultMemoryImpl, btreemap::BTreeMap as StableBtreeMap, cell::Cell,
            memory::VirtualMemory,
        },
        types::{Cycles, Principal},
    },
    control_plane_support::model::replay::ReplayCostGuardSettlement,
    eager_static,
    ids::ComponentInstanceId,
    impl_storable_bounded, impl_storable_unbounded,
    role_contract::allocation::memory::control_plane::{
        ROOT_CANISTER_INVENTORY_ASSETS_ID, ROOT_CANISTER_POOL_HANDOFF_RECEIPTS_ID,
        ROOT_CANISTER_POOL_STATE_ID,
    },
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

eager_static! {
    static CANISTER_POOL: RefCell<
        StableBtreeMap<Principal, CanisterPoolAssetRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(StableBtreeMap::init(canic_core::ic_memory_key!(
        authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
        key = "canic.control_plane.root.canister_inventory.assets.v1",
        ty = CanisterPoolAssetRecord,
        id = ROOT_CANISTER_INVENTORY_ASSETS_ID
    )));
}

eager_static! {
    static CANISTER_POOL_HANDOFF_RECEIPTS: RefCell<
        StableBtreeMap<Principal, CanisterPoolHandoffReceiptRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(StableBtreeMap::init(canic_core::ic_memory_key!(
        authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
        key = "canic.control_plane.root.canister_pool.handoff_receipts.v1",
        ty = CanisterPoolHandoffReceiptRecord,
        id = ROOT_CANISTER_POOL_HANDOFF_RECEIPTS_ID
    )));
}

eager_static! {
    static CANISTER_POOL_STATE: RefCell<
        Cell<CanisterPoolStateRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(Cell::init(
        canic_core::ic_memory_key!(
            authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
            key = "canic.control_plane.root.canister_pool.state.v1",
            ty = CanisterPoolStateRecord,
            id = ROOT_CANISTER_POOL_STATE_ID
        ),
        CanisterPoolStateRecord::default(),
    ));
}

/// Why one autonomous pool refill stopped without a recoverable principal.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CanisterPoolCreationFailureRecord {
    UnresolvedAfterLedgerWindow,
    LedgerCreationFailed,
    LedgerRejected,
}

/// Durable progress of one exact Cycles Ledger pool-refill request.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CanisterPoolCreationProgressRecord {
    Intent {
        uncertain_result: bool,
    },
    Created {
        block_index: u64,
        canister_id: Principal,
    },
    Blocked {
        failure: CanisterPoolCreationFailureRecord,
    },
}

/// Exact authority frozen before one autonomous Cycles Ledger creation effect.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterPoolCreationRecord {
    pub operation_id: [u8; 32],
    pub cycles_ledger: Principal,
    pub placement_subnet: Principal,
    pub root: Principal,
    pub ledger_amount: Cycles,
    pub created_at_time_ns: u64,
    pub prepared_at_ns: u64,
    pub cost_guard_settlement: Option<ReplayCostGuardSettlement>,
    pub progress: CanisterPoolCreationProgressRecord,
}

/// Singleton state required to recover exact pool refill and draining handoff effects.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterPoolStateRecord {
    pub next_creation_sequence: u64,
    pub last_creation_timestamp_ns: u64,
    pub creation: Option<CanisterPoolCreationRecord>,
    pub handoff: Option<CanisterPoolHandoffRecord>,
}

impl CanisterPoolStateRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "CanisterPoolStateRecord";
}

impl_storable_bounded!(CanisterPoolStateRecord, 1_024, false);

/// Exact retry authority for one draining-root asset handoff.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterPoolHandoffRecord {
    pub canister_id: Principal,
    pub recipient: Principal,
    pub prepared_at_ns: u64,
}

/// Durable terminal receipt retaining exact replay after an asset leaves root inventory.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterPoolHandoffReceiptRecord {
    pub recipient: Principal,
    pub completed_at_ns: u64,
}

impl CanisterPoolHandoffReceiptRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "CanisterPoolHandoffReceiptRecord";
}

impl_storable_bounded!(CanisterPoolHandoffReceiptRecord, 64, false);

/// Canonical snapshot identity for terminal pool handoff receipts.
pub struct CanisterPoolHandoffReceiptData;

impl CanisterPoolHandoffReceiptData {
    pub const STATE_CONTRACT_NAME: &'static str = "CanisterPoolHandoffReceiptData";
}

/// Durable identity of one Component allocation claiming an empty Canister.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterPoolClaimRecord {
    pub component: ComponentInstanceId,
    pub operation_id: [u8; 32],
}

/// Durable reset outcome while a stopped workload is still Registry-owned.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CanisterPoolRecycleResetRecord {
    Pending,
    Ready,
    Failed(String),
}

/// Durable provenance for one physical Canister asset.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CanisterPoolAssetOriginRecord {
    InfrastructureStore,
    Created,
    Imported,
    Recycled,
}

/// Durable lifecycle state for one physical Canister asset.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CanisterPoolAssetStatusRecord {
    Store,
    StoreDeletionPending {
        operation_id: [u8; 32],
    },
    PendingReset,
    Ready,
    Claimed(CanisterPoolClaimRecord),
    Workload(CanisterPoolClaimRecord),
    Recycling {
        claim: CanisterPoolClaimRecord,
        reset: CanisterPoolRecycleResetRecord,
    },
    HandingOff {
        recipient: Principal,
    },
    Failed(String),
}

/// Complete persisted row for one physical Canister asset.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterPoolAssetRecord {
    pub cycles: Cycles,
    pub origin: CanisterPoolAssetOriginRecord,
    pub status: CanisterPoolAssetStatusRecord,
    pub last_recycle: Option<CanisterPoolClaimRecord>,
    pub added_at_ns: u64,
    pub updated_at_ns: u64,
}

impl CanisterPoolAssetRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "CanisterPoolAssetRecord";
}

impl_storable_unbounded!(CanisterPoolAssetRecord);

/// One canonical export row including the stable-map key.
#[derive(Clone, Debug)]
pub struct CanisterPoolEntryRecord {
    pub canister_id: Principal,
    pub asset: CanisterPoolAssetRecord,
}

/// Canonical stable snapshot of the root-owned physical inventory.
#[derive(Clone, Debug)]
pub struct CanisterPoolData {
    pub entries: Vec<CanisterPoolEntryRecord>,
    pub state: CanisterPoolStateRecord,
}

impl CanisterPoolData {
    pub const STATE_CONTRACT_NAME: &'static str = "CanisterPoolData";
}

/// Stable-memory owner for the root-owned physical Canister inventory.
pub struct CanisterPoolStore;

impl CanisterPoolStore {
    #[must_use]
    pub fn get(canister_id: &Principal) -> Option<CanisterPoolAssetRecord> {
        CANISTER_POOL.with_borrow(|pool| pool.get(canister_id))
    }

    pub fn insert(
        canister_id: Principal,
        asset: CanisterPoolAssetRecord,
    ) -> Option<CanisterPoolAssetRecord> {
        CANISTER_POOL.with_borrow_mut(|pool| pool.insert(canister_id, asset))
    }

    pub fn remove(canister_id: &Principal) -> Option<CanisterPoolAssetRecord> {
        CANISTER_POOL.with_borrow_mut(|pool| pool.remove(canister_id))
    }

    #[must_use]
    pub fn export() -> CanisterPoolData {
        CanisterPoolData {
            entries: CANISTER_POOL.with_borrow(|pool| {
                pool.iter()
                    .map(|entry| CanisterPoolEntryRecord {
                        canister_id: *entry.key(),
                        asset: entry.value(),
                    })
                    .collect()
            }),
            state: CANISTER_POOL_STATE.with_borrow(|state| state.get().clone()),
        }
    }

    #[must_use]
    pub fn state() -> CanisterPoolStateRecord {
        CANISTER_POOL_STATE.with_borrow(|state| state.get().clone())
    }

    pub fn set_state(state: CanisterPoolStateRecord) {
        CANISTER_POOL_STATE.with_borrow_mut(|current| {
            current.set(state);
        });
    }

    #[must_use]
    pub fn handoff_receipt(canister_id: &Principal) -> Option<CanisterPoolHandoffReceiptRecord> {
        CANISTER_POOL_HANDOFF_RECEIPTS.with_borrow(|receipts| receipts.get(canister_id))
    }

    pub fn insert_handoff_receipt(
        canister_id: Principal,
        receipt: CanisterPoolHandoffReceiptRecord,
    ) -> Option<CanisterPoolHandoffReceiptRecord> {
        CANISTER_POOL_HANDOFF_RECEIPTS
            .with_borrow_mut(|receipts| receipts.insert(canister_id, receipt))
    }

    #[must_use]
    pub fn handoff_receipt_count() -> u64 {
        CANISTER_POOL_HANDOFF_RECEIPTS.with_borrow(StableBtreeMap::len)
    }

    #[cfg(test)]
    pub fn clear() {
        CANISTER_POOL.with_borrow_mut(StableBtreeMap::clear_new);
        CANISTER_POOL_HANDOFF_RECEIPTS.with_borrow_mut(StableBtreeMap::clear_new);
        Self::set_state(CanisterPoolStateRecord::default());
    }
}
