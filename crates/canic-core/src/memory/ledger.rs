//! Module: memory::ledger
//!
//! Responsibility: read and validate the native `ic-memory` allocation ledger.
//! Does not own: allocation policy, stable data schemas, or storage API snapshots.
//! Boundary: diagnostics call this to export memory-manager allocation state.

#[cfg(target_arch = "wasm32")]
use super::manager;
use super::{manager::MEMORY_MANAGER, policy, registry::MemoryRegistryError};
#[cfg(any(test, target_arch = "wasm32"))]
use ic_memory::stable_structures::Memory;
use ic_memory::{
    AllocationHistory, AllocationLedger, AllocationSlotDescriptor, DiagnosticExport,
    MemoryManagerAuthorityRecord, StableCellLedgerRecord,
    stable_structures::{
        DefaultMemoryImpl,
        cell::Cell,
        memory_manager::{MemoryId, VirtualMemory},
    },
};
#[cfg(any(test, target_arch = "wasm32"))]
use ic_memory::{decode_stable_cell_ledger_record, decode_stable_cell_payload};
use std::cell::RefCell;

pub const MEMORY_LAYOUT_LEDGER_ID: u8 = ic_memory::MEMORY_MANAGER_LEDGER_ID;
pub const MEMORY_LEDGER_SCHEMA_VERSION: u32 = 1;
pub const MEMORY_PHYSICAL_FORMAT_ID: u32 = 1;
thread_local! {
    static MEMORY_LAYOUT_LEDGER: RefCell<
        Cell<StableCellLedgerRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(Cell::init(
        open_memory(MEMORY_LAYOUT_LEDGER_ID),
        StableCellLedgerRecord::default(),
    ));
}

///
/// NativeMemoryLedgerSnapshot
///
/// Diagnostic snapshot of the native memory allocation ledger and authorities.
/// Owned by memory ledger and consumed by diagnostics/status surfaces.
///

pub struct NativeMemoryLedgerSnapshot {
    pub export: DiagnosticExport,
    pub authorities: Vec<MemoryManagerAuthorityRecord>,
}

/// Read the wasm stable-memory ledger after classifying raw stable memory.
#[cfg(target_arch = "wasm32")]
pub fn try_diagnostic_snapshot() -> Result<NativeMemoryLedgerSnapshot, MemoryRegistryError> {
    match manager::classify_raw_stable_memory() {
        manager::RawStableMemoryState::Empty => {
            snapshot_from_record(&StableCellLedgerRecord::default())
        }
        manager::RawStableMemoryState::ForeignOrCorrupt => {
            Err(MemoryRegistryError::LedgerCorrupt {
                reason: "foreign or corrupt raw stable memory state",
            })
        }
        manager::RawStableMemoryState::MemoryManager => {
            let memory = open_memory(MEMORY_LAYOUT_LEDGER_ID);
            validate_existing_ledger_memory(&memory)?;
            MEMORY_LAYOUT_LEDGER.with_borrow(|cell| snapshot_from_record(cell.get()))
        }
    }
}

/// Read the host memory ledger snapshot.
#[cfg(not(target_arch = "wasm32"))]
pub fn try_snapshot() -> Result<NativeMemoryLedgerSnapshot, MemoryRegistryError> {
    MEMORY_LAYOUT_LEDGER.with_borrow(|cell| snapshot_from_record(cell.get()))
}

fn open_memory(id: u8) -> VirtualMemory<DefaultMemoryImpl> {
    MEMORY_MANAGER.with_borrow_mut(|mgr| mgr.get(MemoryId::new(id)))
}

#[cfg(any(test, target_arch = "wasm32"))]
fn validate_existing_ledger_memory<M: Memory>(memory: &M) -> Result<(), MemoryRegistryError> {
    if memory.size() == 0 {
        return Ok(());
    }

    let bytes = decode_stable_cell_payload(memory).map_err(stable_cell_error)?;
    if decode_stable_cell_ledger_record(&bytes).is_ok() {
        return Ok(());
    }

    Err(MemoryRegistryError::LedgerCorrupt {
        reason: "foreign or corrupt native ic-memory ledger state",
    })
}

#[cfg(any(test, target_arch = "wasm32"))]
const fn stable_cell_error(_err: ic_memory::StableCellPayloadError) -> MemoryRegistryError {
    MemoryRegistryError::LedgerCorrupt {
        reason: "foreign or corrupt native ic-memory ledger state",
    }
}

fn snapshot_from_record(
    record: &StableCellLedgerRecord,
) -> Result<NativeMemoryLedgerSnapshot, MemoryRegistryError> {
    let store = record.store();
    let ledger = if store.physical().is_uninitialized() {
        genesis_ledger()
    } else {
        store
            .recover()
            .map_err(|_| MemoryRegistryError::LedgerCorrupt {
                reason: "native ic-memory ledger recovery failed",
            })?
            .ledger()
            .clone()
    };
    let commit_recovery = store.physical().diagnostic();
    Ok(NativeMemoryLedgerSnapshot {
        export: DiagnosticExport::from_ledger_with_commit_recovery(
            &ledger,
            AllocationSlotDescriptor::memory_manager(MEMORY_LAYOUT_LEDGER_ID)
                .expect("ledger ID is a usable MemoryManager ID"),
            Some(commit_recovery),
        ),
        authorities: policy::canonical_authority_records(),
    })
}

fn genesis_ledger() -> AllocationLedger {
    AllocationLedger::new_committed(0, AllocationHistory::default())
        .expect("empty ic-memory genesis ledger is structurally valid")
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use ic_memory::{STABLE_CELL_LAYOUT_VERSION, STABLE_CELL_MAGIC, STABLE_CELL_VALUE_OFFSET};

    #[test]
    fn validates_native_ledger_cell_payload() {
        let payload = crate::cdk::serialize::serialize(&StableCellLedgerRecord::default())
            .expect("native payload");
        let memory = DefaultMemoryImpl::default();
        memory.grow(1);
        write_stable_cell_payload(&memory, &payload);

        validate_existing_ledger_memory(&memory).expect("native payload should validate");
    }

    fn write_stable_cell_payload<M: Memory>(memory: &M, payload: &[u8]) {
        let len = u32::try_from(payload.len())
            .expect("test payload length")
            .to_le_bytes();
        memory.write(0, STABLE_CELL_MAGIC);
        memory.write(3, &[STABLE_CELL_LAYOUT_VERSION]);
        memory.write(4, &len);
        memory.write(STABLE_CELL_VALUE_OFFSET, payload);
    }
}
