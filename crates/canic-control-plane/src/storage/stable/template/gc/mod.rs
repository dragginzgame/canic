use crate::ids::WasmStoreGcMode;
use canic_cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory};
use canic_memory::{eager_static, ic_memory, impl_storable_bounded};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

const WASM_STORE_GC_STATE_ID: u8 = 62;

const _: () = {
    #[canic_memory::__reexports::ctor::ctor(
        anonymous,
        crate_path = canic_memory::__reexports::ctor
    )]
    fn __canic_reserve_wasm_store_gc_memory_range() {
        canic_memory::ic_memory_range!(62, 62);
    }
};

eager_static! {
    static WASM_STORE_GC_STATE: RefCell<
        Cell<WasmStoreGcStateRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(Cell::init(
        ic_memory!(WasmStoreGcStateRecord, WASM_STORE_GC_STATE_ID),
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

impl_storable_bounded!(WasmStoreGcStateRecord, 64, true);

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
    pub fn clear_for_test() {
        Self::set(WasmStoreGcStateRecord::default());
    }
}
