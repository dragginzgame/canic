#![cfg(test)]

use super::*;
use ic_memory::{
    AllocationDeclaration, AllocationHistory, AllocationLedger, AllocationSlotDescriptor,
    SchemaMetadata,
    ic_stable_structures::{
        Memory, VectorMemory,
        memory_manager::{MemoryId, MemoryManager},
    },
};

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
    assert_eq!(report.bucket_size_pages, 128);
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
        AllocationSlotDescriptor::memory_manager(100).expect("usable slot"),
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
        memory_size: Some(DiagnosticMemorySizeOutcome::Measured(
            DiagnosticMemorySize::from_wasm_pages(3),
        )),
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
fn memory_allocation_record_response_omits_failed_size_measurements() {
    let declaration = AllocationDeclaration::new(
        "app.users.v1",
        AllocationSlotDescriptor::memory_manager(100).expect("usable slot"),
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
        memory_size: Some(DiagnosticMemorySizeOutcome::Failed(
            ic_memory::DiagnosticFailure::new(
                ic_memory::DiagnosticCode::MemorySize,
                "slot could not be measured",
            ),
        )),
    };

    let response = memory_allocation_record_response(record);

    assert_eq!(response.memory_size, None);
    assert_eq!(memory_ledger_memory_entry_response(&response), None);
}
