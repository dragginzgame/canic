#![cfg(test)]

use super::*;
use crate::storage::stable::env::{Env, EnvData, EnvRecord};
use ic_memory::{
    AllocationDeclaration, AllocationHistory, AllocationLedger, MemoryManagerSlot, SchemaMetadata,
    ic_stable_structures::{
        Memory, VectorMemory,
        memory_manager::{MemoryId, MemoryManager},
    },
};

#[test]
fn bootstrap_retains_declarations_and_opens_only_the_accessed_store() {
    MemoryRegistryOps::bootstrap_registry().expect("bootstrap declared allocations");
    let before = MemoryRegistryOps::allocation_snapshot().unwrap();
    let declared = before
        .memories
        .iter()
        .filter(|entry| matches!(entry.binding, MemoryAllocationBinding::Current { .. }))
        .collect::<Vec<_>>();
    assert!(!declared.is_empty());
    assert!(
        declared
            .iter()
            .all(|entry| entry.virtual_extent.wasm_pages == 0)
    );

    let parent = candid::Principal::from_slice(&[73]);
    Env::import(EnvData {
        record: EnvRecord {
            parent_pid: Some(parent),
            ..EnvRecord::default()
        },
    });
    let after = MemoryRegistryOps::allocation_snapshot().unwrap();
    let bindings_id = crate::role_contract::allocation::memory::runtime::RUNTIME_BINDINGS_ID;
    for entry in &after.memories {
        if entry.memory_manager_id == bindings_id {
            assert!(entry.virtual_extent.wasm_pages > 0);
        } else if matches!(entry.binding, MemoryAllocationBinding::Current { .. }) {
            assert_eq!(entry.virtual_extent.wasm_pages, 0);
        }
    }
    assert_eq!(before.current_generation, after.current_generation);
    assert_eq!(Env::export().record.parent_pid, Some(parent));
    assert_eq!(MemoryRegistryOps::allocation_snapshot().unwrap(), after);
}

#[test]
fn allocation_snapshot_preserves_unopened_memories_and_ledger_generation() {
    MemoryRegistryOps::init_registry().expect("bootstrap canonical memory runtime");
    let before = MemoryRegistryOps::ledger_snapshot().expect("ledger before observation");
    let report = MemoryRegistryOps::allocation_snapshot().expect("current allocations");
    let replay = MemoryRegistryOps::allocation_snapshot().expect("repeat current allocations");
    let after = MemoryRegistryOps::ledger_snapshot().expect("ledger after observation");

    assert_eq!(report, replay);
    assert_eq!(before.current_generation, report.current_generation);
    assert_eq!(before.current_generation, after.current_generation);
    assert_eq!(before.memories, after.memories);
    assert_eq!(
        report.memories.len(),
        usize::from(ic_memory::MEMORY_MANAGER_INVALID_ID)
    );
    assert!(
        report
            .memories
            .windows(2)
            .all(|pair| pair[0].memory_manager_id < pair[1].memory_manager_id)
    );
    assert!(
        report
            .memories
            .iter()
            .any(|entry| entry.virtual_extent.wasm_pages == 0)
    );
    for entry in &report.memories {
        assert_eq!(
            entry.virtual_extent.bytes,
            entry.virtual_extent.wasm_pages * 65_536
        );
        assert_eq!(entry.payload_bytes, None);
        if let MemoryAllocationBinding::Current { stable_key, .. } = &entry.binding {
            let prior = before
                .memories
                .iter()
                .find(|prior| prior.memory_manager_id == entry.memory_manager_id)
                .expect("current binding is in the ledger");
            assert_eq!(&prior.stable_key, stable_key);
            assert_eq!(prior.size, entry.virtual_extent);
        }
    }
    assert_eq!(
        report.bucket_size_pages,
        crate::memory::configured_bucket_pages()
    );
    assert_eq!(report.metadata_bytes_read, 34_848);
    assert!(matches!(
        report.memories[0].binding,
        MemoryAllocationBinding::Ledger { .. }
    ));
    assert_eq!(
        report.physical_extent.bytes,
        report.manager_metadata_bytes + report.allocated_bucket_bytes + report.unmanaged_bytes
    );
    assert_eq!(
        report.allocated_bucket_bytes,
        report
            .memories
            .iter()
            .map(|entry| entry.allocated_bytes)
            .sum::<u64>()
    );
    assert_eq!(
        report.allocated_bucket_bytes,
        report.known_binding_bytes + report.unknown_binding_bytes
    );
    assert_eq!(
        report.allocated_bucket_bytes,
        report.virtual_extent.bytes + report.bucket_slack_bytes
    );
    let bytes = candid::encode_one(&report).expect("encode bounded report");
    let decoded: MemoryAllocationsResponse =
        candid::decode_one(&bytes).expect("decode bounded report");
    assert_eq!(decoded, report);
}

#[test]
fn allocation_snapshot_requires_existing_bootstrap() {
    let error = MemoryRegistryOps::allocation_snapshot().expect_err("uninitialized runtime");
    assert_eq!(error.code(), crate::diagnostics::codes::STATE_INVALID);
    assert!(!MemoryRegistryOps::is_initialized().expect("runtime state"));
}

#[test]
fn allocation_projection_preserves_unknown_owners_and_measured_nondefault_buckets() {
    let backing = VectorMemory::default();
    let manager = MemoryManager::init_with_bucket_size(backing.clone(), 16);
    assert_eq!(manager.get(MemoryId::new(200)).grow(1), 0);
    drop(manager);
    let runtime = ic_memory::MemoryRuntime::new(backing).expect("reopen owned manager");
    let report = runtime
        .memory_allocations()
        .expect("bounded unbound allocations");
    assert_eq!(report.bucket_size_pages, 16);
    assert_eq!(report.unknown_binding_bytes, 1_048_576);
    let entry = memory_allocation_entry_response(report.memories[200].clone());
    assert_eq!(entry.binding, MemoryAllocationBinding::Unknown);
    assert_eq!(entry.virtual_extent.bytes, 65_536);
    assert_eq!(entry.allocated_bytes, 1_048_576);
    assert_eq!(entry.bucket_slack_bytes, 983_040);
    assert_eq!(entry.payload_bytes, None);
    let error =
        memory_allocations_response(report).expect_err("Canic requires committed bootstrap");
    assert_eq!(error.code(), crate::diagnostics::codes::STATE_INVALID);
}

#[test]
fn ledger_snapshot_reads_the_bootstrapped_ic_memory_runtime() {
    MemoryRegistryOps::init_registry().expect("bootstrap canonical memory runtime");

    let snapshot = MemoryRegistryOps::ledger_snapshot().expect("runtime diagnostic export");

    assert!(snapshot.current_generation > 0);
    assert!(
        snapshot
            .authorities
            .iter()
            .any(|authority| authority.owner == "canic-core")
    );
    assert!(
        snapshot
            .memories
            .iter()
            .any(|memory| memory.memory_manager_id >= 30)
    );
}

#[test]
fn commit_slot_response_maps_runtime_variants_exactly() {
    assert_eq!(
        commit_slot_response(CommitSlotDiagnostic::Empty),
        MemoryCommitSlotResponse {
            present: false,
            generation: None,
            valid: false,
        }
    );
    assert_eq!(
        commit_slot_response(CommitSlotDiagnostic::Valid { generation: 7 }),
        MemoryCommitSlotResponse {
            present: true,
            generation: Some(7),
            valid: true,
        }
    );
    assert_eq!(
        commit_slot_response(CommitSlotDiagnostic::Invalid { generation: 8 }),
        MemoryCommitSlotResponse {
            present: true,
            generation: Some(8),
            valid: false,
        }
    );
}

#[test]
fn commit_recovery_response_maps_invalid_slots_without_unknown_fallback() {
    let response = commit_recovery_response(Some(CommitStoreDiagnostic {
        slot0: CommitSlotDiagnostic::Invalid { generation: 3 },
        slot1: CommitSlotDiagnostic::Empty,
        recovery: Err(CommitRecoveryError::InvalidCommitSlots {
            slot0_invalid: true,
            slot1_invalid: false,
        }),
    }));

    assert_eq!(response.authoritative_generation, None);
    assert_eq!(
        response.recovery_error,
        Some(MemoryCommitRecoveryErrorResponse::InvalidCommitSlots)
    );
}

#[test]
fn memory_allocation_record_response_includes_live_backing_memory_size() {
    let declaration = AllocationDeclaration::new(
        "app.users.v1",
        MemoryManagerSlot::new(100).expect("usable slot"),
        None,
        SchemaMetadata::default(),
    )
    .expect("declaration");
    let ledger = AllocationLedger::new_committed(0, AllocationHistory::default())
        .expect("genesis ledger")
        .stage_reservation_generation(&[declaration], None)
        .expect("reservation generation");
    let record = DiagnosticRecord {
        allocation: ledger.allocation_history().records()[0].clone(),
        memory_size: Some(DiagnosticMemorySize::from_wasm_pages(3)),
    };

    let response = memory_allocation_record_response(record);

    assert_eq!(
        response.memory_size,
        Some(MemoryAllocationSizeEntry {
            wasm_pages: 3,
            bytes: 196_608,
        })
    );
    assert_eq!(
        memory_ledger_memory_entry_response(&response),
        Some(MemoryLedgerMemoryEntry {
            memory_manager_id: 100,
            stable_key: "app.users.v1".to_string(),
            state: MemoryAllocationState::Reserved,
            size: MemoryAllocationSizeEntry {
                wasm_pages: 3,
                bytes: 196_608,
            },
        })
    );
}

#[test]
fn memory_allocation_record_response_omits_unmeasured_sizes() {
    let declaration = AllocationDeclaration::new(
        "app.users.v1",
        MemoryManagerSlot::new(100).expect("usable slot"),
        None,
        SchemaMetadata::default(),
    )
    .expect("declaration");
    let ledger = AllocationLedger::new_committed(0, AllocationHistory::default())
        .expect("genesis ledger")
        .stage_reservation_generation(&[declaration], None)
        .expect("reservation generation");
    let record = DiagnosticRecord {
        allocation: ledger.allocation_history().records()[0].clone(),
        memory_size: None,
    };

    let response = memory_allocation_record_response(record);

    assert_eq!(response.memory_size, None);
    assert_eq!(memory_ledger_memory_entry_response(&response), None);
}

#[derive(Clone, Default)]
struct RefusableMemory {
    bytes: VectorMemory,
    refuse: std::rc::Rc<std::cell::Cell<bool>>,
}

impl Memory for RefusableMemory {
    fn size(&self) -> u64 {
        self.bytes.size()
    }

    fn grow(&self, pages: u64) -> i64 {
        if self.refuse.get() {
            -1
        } else {
            self.bytes.grow(pages)
        }
    }

    fn read(&self, offset: u64, bytes: &mut [u8]) {
        self.bytes.read(offset, bytes);
    }

    fn write(&self, offset: u64, bytes: &[u8]) {
        self.bytes.write(offset, bytes);
    }
}

#[test]
fn early_default_access_preserves_configured_bootstrap_and_authority() {
    // A fresh thread isolates the default runtime from other native store tests.
    std::thread::spawn(|| {
        use ic_memory::{RuntimeAdoptionError, RuntimeDiagnosticError, RuntimeOpenError};
        let key = "canic.core.runtime.bindings.v1";
        let id = crate::role_contract::allocation::memory::runtime::RUNTIME_BINDINGS_ID;
        let declarations = ic_memory::sealed_declaration_snapshot().unwrap();
        assert!(matches!(
            ic_memory::open_default_memory_manager_memory(key, id),
            Err(RuntimeOpenError::NotBootstrapped)
        ));
        assert!(matches!(
            ic_memory::open_default_memory_manager_memory_by_key(key),
            Err(RuntimeOpenError::NotBootstrapped)
        ));
        assert_eq!(
            ic_memory::default_memory_manager_memory_id(key),
            Err(RuntimeOpenError::NotBootstrapped)
        );
        assert_eq!(
            ic_memory::verify_default_memory_manager_authority(
                &declarations,
                memory::CANIC_CORE_MEMORY_AUTHORITY,
            ),
            Err(RuntimeAdoptionError::Open(
                RuntimeOpenError::NotBootstrapped
            ))
        );
        assert!(matches!(
            ic_memory::default_memory_manager_memory_allocation_summary(),
            Err(RuntimeDiagnosticError::NotBootstrapped)
        ));
        assert!(!MemoryRegistryOps::is_initialized().unwrap());
        ic_memory::bootstrap_default_memory_manager_with_config(
            ic_memory::MemoryManagerConfig::new(1).unwrap(),
            &memory::CanicMemoryManagerPolicy::new(),
        )
        .unwrap();
        let before = MemoryRegistryOps::allocation_snapshot().unwrap();
        assert_eq!(before.bucket_size_pages, 1);
        assert_eq!(ic_memory::default_memory_manager_memory_id(key), Ok(id));
        ic_memory::verify_default_memory_manager_authority(
            &declarations,
            memory::CANIC_CORE_MEMORY_AUTHORITY,
        )
        .unwrap();
        assert_eq!(MemoryRegistryOps::allocation_snapshot().unwrap(), before);
    })
    .join()
    .unwrap();
}

#[test]
fn ledger_growth_refusal_publishes_no_authority_and_retries_canic_bootstrap() {
    let backing = RefusableMemory::default();
    let mut runtime = ic_memory::MemoryRuntime::new_with_config(
        backing.clone(),
        ic_memory::MemoryManagerConfig::new(1).unwrap(),
    )
    .unwrap();
    let declarations = ic_memory::sealed_declaration_snapshot().unwrap();
    let policy = memory::CanicMemoryManagerPolicy::new();
    let before = backing.bytes.borrow().clone();
    backing.refuse.set(true);
    assert!(matches!(
        runtime.bootstrap(&declarations, &policy),
        Err(ic_memory::RuntimeBootstrapError::LedgerGrowth(
            ic_memory::RuntimeGrowError::BackingRefused { .. }
        ))
    ));
    assert!(!runtime.is_bootstrapped());
    assert!(matches!(
        runtime.committed_allocations(),
        Err(ic_memory::RuntimeOpenError::NotBootstrapped)
    ));
    assert_eq!(*backing.bytes.borrow(), before);
    backing.refuse.set(false);
    assert_eq!(
        runtime
            .bootstrap(&declarations, &policy)
            .unwrap()
            .generation(),
        1
    );
    runtime
        .verify_authority(&declarations, memory::CANIC_CORE_MEMORY_AUTHORITY)
        .unwrap();
}

#[test]
fn application_growth_refusal_preserves_shared_extents_and_retry() {
    let backing = RefusableMemory::default();
    let mut runtime = ic_memory::MemoryRuntime::new_with_config(
        backing.clone(),
        ic_memory::MemoryManagerConfig::new(1).unwrap(),
    )
    .unwrap();
    runtime
        .bootstrap(
            &ic_memory::sealed_declaration_snapshot().unwrap(),
            &memory::CanicMemoryManagerPolicy::new(),
        )
        .unwrap();
    let rows = runtime
        .open_memory_by_key("canic.core.runtime.bindings.v1")
        .unwrap();
    let clone = rows.clone();
    let before = backing.bytes.borrow().clone();
    let summary = runtime.memory_allocation_summary().unwrap();
    backing.refuse.set(true);
    assert_eq!(
        clone.grow(1),
        Err(ic_memory::RuntimeGrowError::BackingRefused {
            additional_pages: 1
        })
    );
    assert_eq!(rows.size(), 0);
    assert_eq!(*backing.bytes.borrow(), before);
    assert_eq!(runtime.memory_allocation_summary().unwrap(), summary);
    backing.refuse.set(false);
    assert_eq!(clone.grow(1), Ok(0));
    rows.write(0, b"retained");
    let before = backing.bytes.borrow().clone();
    assert_eq!(
        rows.grow(u64::MAX),
        Err(ic_memory::RuntimeGrowError::ArithmeticOverflow)
    );
    assert!(matches!(
        rows.grow(32_768),
        Err(ic_memory::RuntimeGrowError::BucketExhausted { .. })
    ));
    assert_eq!(*backing.bytes.borrow(), before);
    drop(runtime);
    assert_eq!(rows.grow(1), Ok(1));
    let mut bytes = [0; 8];
    clone.read(0, &mut bytes);
    assert_eq!(&bytes, b"retained");
}

#[test]
fn receipt_capacity_growth_exhaustion_preserves_store_and_typed_ops_failure() {
    use crate::{
        ops::storage::intent::IntentStoreOpsError,
        storage::stable::intent::ReceiptBackedIntentStore,
    };
    ReceiptBackedIntentStore::reserve_application_eligibility_capacity(1).unwrap();
    let before = MemoryRegistryOps::allocation_snapshot().unwrap();
    let required_records = u64::from(u32::MAX);
    let source =
        ReceiptBackedIntentStore::reserve_application_eligibility_capacity(required_records)
            .unwrap_err();
    assert!(matches!(
        source,
        ic_memory::RuntimeGrowError::BucketExhausted { .. }
    ));
    assert_eq!(MemoryRegistryOps::allocation_snapshot().unwrap(), before);
    let error = IntentStoreOpsError::ApplicationReceiptEligibilityCapacityUnavailable {
        required_records,
        source,
    };
    assert!(matches!(
        error,
        IntentStoreOpsError::ApplicationReceiptEligibilityCapacityUnavailable {
            source: ic_memory::RuntimeGrowError::BucketExhausted { .. },
            ..
        }
    ));
    let internal: InternalError = error.into();
    assert_eq!(
        internal.code(),
        crate::diagnostics::codes::CAPACITY_UNAVAILABLE
    );
    ReceiptBackedIntentStore::reserve_application_eligibility_capacity(1).unwrap();
    assert_eq!(MemoryRegistryOps::allocation_snapshot().unwrap(), before);
}

#[test]
fn memory_ledger_generation_response_preserves_current_fields() {
    let response = memory_ledger_generation_response(
        GenerationRecord::new(7, 6, Some("host-build".to_string()), 4, Some(123))
            .expect("current generation record"),
    );
    let bytes = candid::encode_one(&response).expect("generation response Candid");
    let decoded: MemoryLedgerGenerationEntry = candid::decode_one(&bytes).expect("current Candid");
    assert_eq!(decoded.generation, 7);
    assert_eq!(decoded.parent_generation, Some(6));
    assert_eq!(decoded.runtime_fingerprint.as_deref(), Some("host-build"));
    assert_eq!(decoded.declaration_count, 4);
    assert_eq!(decoded.committed_at, Some(123));
}
