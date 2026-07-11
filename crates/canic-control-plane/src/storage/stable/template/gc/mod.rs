use crate::ids::WasmStoreGcMode;
use canic_core::cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory};
use canic_core::eager_static;
use canic_core::{
    impl_storable_bounded, role_contract::allocation::memory::template::WASM_STORE_GC_STATE_ID,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

eager_static! {
    static WASM_STORE_GC_STATE: RefCell<
        Cell<WasmStoreGcStateRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(Cell::init(
        canic_core::ic_memory_key!("canic.control_plane.wasm_store_gc_state.v1", WasmStoreGcStateRecord, WASM_STORE_GC_STATE_ID),
        WasmStoreGcStateRecord::default(),
    ));
}

///
/// WasmStoreGcStateRecord
///

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct WasmStoreGcStateRecord {
    pub mode: WasmStoreGcMode,
    pub changed_at: u64,
    pub prepared_at: Option<u64>,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub runs_completed: u32,
}

impl WasmStoreGcStateRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "WasmStoreGcStateRecord";
}

impl_storable_bounded!(WasmStoreGcStateRecord, 64, true);

///
/// WasmStoreGcStateData
///
/// Canonical local wasm-store GC-state allocation snapshot.
///

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WasmStoreGcStateData {
    pub record: WasmStoreGcStateRecord,
}

impl WasmStoreGcStateData {
    pub const STATE_CONTRACT_NAME: &'static str = "WasmStoreGcStateData";
}

///
/// WasmStoreGcStateStore
///

pub struct WasmStoreGcStateStore;

impl WasmStoreGcStateStore {
    // Return the current local wasm-store GC state record.
    #[must_use]
    pub fn get() -> WasmStoreGcStateRecord {
        WASM_STORE_GC_STATE.with_borrow(|cell| cell.get().clone())
    }

    // Replace the current local wasm-store GC state record.
    pub fn set(record: WasmStoreGcStateRecord) {
        WASM_STORE_GC_STATE.with_borrow_mut(|cell| {
            cell.set(record);
        });
    }

    #[cfg(test)]
    pub fn export() -> WasmStoreGcStateData {
        WasmStoreGcStateData {
            record: Self::get(),
        }
    }

    #[cfg(test)]
    pub fn import(data: WasmStoreGcStateData) {
        Self::set(data.record);
    }

    #[cfg(test)]
    pub fn clear_for_test() {
        Self::set(WasmStoreGcStateRecord::default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gc_state_round_trips_through_canonical_data_snapshot() {
        WasmStoreGcStateStore::clear_for_test();
        WasmStoreGcStateStore::set(WasmStoreGcStateRecord {
            mode: WasmStoreGcMode::Complete,
            changed_at: 11,
            prepared_at: Some(12),
            started_at: Some(13),
            completed_at: Some(14),
            runs_completed: 15,
        });

        let data = WasmStoreGcStateStore::export();
        WasmStoreGcStateStore::clear_for_test();
        WasmStoreGcStateStore::import(data.clone());

        assert_eq!(WasmStoreGcStateStore::export(), data);
        WasmStoreGcStateStore::clear_for_test();
    }
}
