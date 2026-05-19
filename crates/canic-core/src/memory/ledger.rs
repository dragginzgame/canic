#[cfg(target_arch = "wasm32")]
use super::manager;
use super::{
    manager::{MEMORY_MANAGER, RawStableMemoryState},
    policy,
    registry::MemoryRegistryError,
};
use ic_memory::{
    AllocationBootstrap, AllocationHistory, AllocationLedger, AllocationPolicy,
    AllocationSlotDescriptor, BootstrapCommit, BootstrapError, CURRENT_LEDGER_SCHEMA_VERSION,
    CURRENT_PHYSICAL_FORMAT_ID, CborLedgerCodec, DeclarationSnapshot, DiagnosticExport,
    LedgerCodec, MemoryManagerAuthorityRecord, StableCellLedgerRecord,
    decode_stable_cell_ledger_record, decode_stable_cell_payload,
    stable_structures::{
        DefaultMemoryImpl, Memory,
        cell::Cell,
        memory_manager::{MemoryId, VirtualMemory},
    },
};
use serde::Deserialize;
use std::cell::RefCell;

pub const MEMORY_LAYOUT_LEDGER_ID: u8 = ic_memory::MEMORY_MANAGER_LEDGER_ID;
pub const MEMORY_LAYOUT_LEDGER_OWNER: &str = ic_memory::IC_MEMORY_AUTHORITY_OWNER;
pub const MEMORY_LAYOUT_LEDGER_LABEL: &str = ic_memory::IC_MEMORY_LEDGER_LABEL;
pub const MEMORY_LAYOUT_LEDGER_STABLE_KEY: &str = ic_memory::IC_MEMORY_LEDGER_STABLE_KEY;
const LEGACY_CANIC_LEDGER_MAGIC: u64 = 0x4341_4E49_434D_454D;
const LEGACY_CANIC_LEDGER_ERROR: &str = "legacy Canic memory ledger format detected; this build uses ic-memory-native allocation persistence and cannot boot from the old format";

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

pub struct NativeMemoryLedgerSnapshot {
    pub export: DiagnosticExport,
    pub authorities: Vec<MemoryManagerAuthorityRecord>,
}

pub fn validate_bootstrap_state_before_cell_init(
    raw_state: RawStableMemoryState,
) -> Result<(), MemoryRegistryError> {
    match raw_state {
        RawStableMemoryState::Empty => Ok(()),
        RawStableMemoryState::ForeignOrCorrupt => Err(MemoryRegistryError::LedgerCorrupt {
            reason: "foreign or corrupt raw stable memory state",
        }),
        RawStableMemoryState::MemoryManager => {
            let memory = open_memory(MEMORY_LAYOUT_LEDGER_ID);
            validate_existing_ledger_memory(&memory)
        }
    }
}

pub(super) fn bootstrap_declarations<P>(
    declaration_snapshot: DeclarationSnapshot,
    policy: &P,
) -> Result<BootstrapCommit, BootstrapError<<CborLedgerCodec as LedgerCodec>::Error, P::Error>>
where
    P: AllocationPolicy,
{
    MEMORY_LAYOUT_LEDGER.with_borrow_mut(|cell| {
        let mut record = cell.get().clone();
        let mut bootstrap = AllocationBootstrap::new(record.store_mut());
        let commit = bootstrap.initialize_validate_and_commit(
            &CborLedgerCodec,
            &genesis_ledger(),
            declaration_snapshot,
            policy,
            None,
        )?;
        cell.set(record);
        Ok(commit)
    })
}

#[cfg(target_arch = "wasm32")]
pub fn try_diagnostic_snapshot() -> Result<NativeMemoryLedgerSnapshot, MemoryRegistryError> {
    match manager::classify_raw_stable_memory() {
        RawStableMemoryState::Empty => snapshot_from_record(&StableCellLedgerRecord::default()),
        RawStableMemoryState::ForeignOrCorrupt => Err(MemoryRegistryError::LedgerCorrupt {
            reason: "foreign or corrupt raw stable memory state",
        }),
        RawStableMemoryState::MemoryManager => {
            let memory = open_memory(MEMORY_LAYOUT_LEDGER_ID);
            validate_existing_ledger_memory(&memory)?;
            MEMORY_LAYOUT_LEDGER.with_borrow(|cell| snapshot_from_record(cell.get()))
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn try_snapshot() -> Result<NativeMemoryLedgerSnapshot, MemoryRegistryError> {
    MEMORY_LAYOUT_LEDGER.with_borrow(|cell| snapshot_from_record(cell.get()))
}

#[cfg(test)]
pub fn reset_for_tests() {
    MEMORY_LAYOUT_LEDGER.with_borrow_mut(|cell| {
        cell.set(StableCellLedgerRecord::default());
    });
}

#[cfg(test)]
pub fn try_export_records() -> Result<Vec<ic_memory::AllocationRecord>, MemoryRegistryError> {
    Ok(try_snapshot()?
        .export
        .records
        .into_iter()
        .map(|record| record.allocation)
        .collect())
}

fn open_memory(id: u8) -> VirtualMemory<DefaultMemoryImpl> {
    MEMORY_MANAGER.with_borrow_mut(|mgr| mgr.get(MemoryId::new(id)))
}

fn validate_existing_ledger_memory<M: Memory>(memory: &M) -> Result<(), MemoryRegistryError> {
    if memory.size() == 0 {
        return Ok(());
    }

    let bytes = decode_stable_cell_payload(memory).map_err(stable_cell_error)?;
    if decode_stable_cell_ledger_record(&bytes).is_ok() {
        return Ok(());
    }

    if let Ok(probe) = canic_cdk::serialize::deserialize::<LegacyCanicLedgerProbe>(&bytes)
        && probe.magic == LEGACY_CANIC_LEDGER_MAGIC
    {
        return Err(MemoryRegistryError::LedgerCorrupt {
            reason: LEGACY_CANIC_LEDGER_ERROR,
        });
    }

    Err(MemoryRegistryError::LedgerCorrupt {
        reason: "foreign or corrupt native ic-memory ledger state",
    })
}

#[derive(Deserialize)]
struct LegacyCanicLedgerProbe {
    magic: u64,
}

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
            .recover(&CborLedgerCodec)
            .map_err(|_| MemoryRegistryError::LedgerCorrupt {
                reason: "native ic-memory ledger recovery failed",
            })?
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
    AllocationLedger::new_committed(
        CURRENT_LEDGER_SCHEMA_VERSION,
        CURRENT_PHYSICAL_FORMAT_ID,
        0,
        AllocationHistory::default(),
    )
    .expect("empty ic-memory genesis ledger is structurally valid")
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::registry::MemoryRegistryError;
    use ic_memory::{
        AllocationDeclaration, DeclarationSnapshot, STABLE_CELL_LAYOUT_VERSION, STABLE_CELL_MAGIC,
        STABLE_CELL_VALUE_OFFSET, SchemaMetadata, StableCellLedgerRecord,
    };
    use serde::Serialize;

    #[derive(Debug, Eq, PartialEq)]
    struct AllowAllPolicy;

    impl AllocationPolicy for AllowAllPolicy {
        type Error = MemoryRegistryError;

        fn validate_key(&self, _key: &ic_memory::StableKey) -> Result<(), Self::Error> {
            Ok(())
        }

        fn validate_slot(
            &self,
            _key: &ic_memory::StableKey,
            _slot: &AllocationSlotDescriptor,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        fn validate_reserved_slot(
            &self,
            _key: &ic_memory::StableKey,
            _slot: &AllocationSlotDescriptor,
        ) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    #[test]
    fn native_bootstrap_persists_and_recovers_allocations() {
        reset_for_tests();
        let snapshot = declaration_snapshot(vec![declaration(100, "app.test.users.v1")]);

        bootstrap_declarations(snapshot, &AllowAllPolicy).expect("bootstrap native ledger");

        let records = try_export_records().expect("ledger records");
        assert!(records.iter().any(|record| {
            record.stable_key().as_str() == "app.test.users.v1"
                && record.slot().memory_manager_id() == Ok(100)
        }));
        assert!(try_snapshot().expect("snapshot").export.current_generation > 0);
    }

    #[test]
    fn legacy_canic_ledger_payload_is_rejected() {
        #[derive(Serialize)]
        struct LegacyCanicLedgerProbeForTest {
            magic: u64,
        }

        let payload = canic_cdk::serialize::serialize(&LegacyCanicLedgerProbeForTest {
            magic: LEGACY_CANIC_LEDGER_MAGIC,
        })
        .expect("legacy probe payload");
        let memory = DefaultMemoryImpl::default();
        memory.grow(1);
        write_stable_cell_payload(&memory, &payload);

        let err = validate_existing_ledger_memory(&memory)
            .expect_err("legacy Canic ledger must fail hard-cut bootstrap");
        assert!(
            matches!(err, MemoryRegistryError::LedgerCorrupt { reason } if reason == LEGACY_CANIC_LEDGER_ERROR)
        );
    }

    #[test]
    fn validates_native_ledger_cell_payload() {
        let payload = canic_cdk::serialize::serialize(&StableCellLedgerRecord::default())
            .expect("native payload");
        let memory = DefaultMemoryImpl::default();
        memory.grow(1);
        write_stable_cell_payload(&memory, &payload);

        validate_existing_ledger_memory(&memory).expect("native payload should validate");
    }

    fn declaration(id: u8, stable_key: &str) -> AllocationDeclaration {
        AllocationDeclaration::memory_manager_with_schema(
            stable_key,
            id,
            stable_key,
            SchemaMetadata::default(),
        )
        .expect("declaration")
    }

    fn declaration_snapshot(declarations: Vec<AllocationDeclaration>) -> DeclarationSnapshot {
        DeclarationSnapshot::new(declarations).expect("snapshot")
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
