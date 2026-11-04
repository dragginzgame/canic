#![cfg(test)]

//! Module: storage::stable::receipt_capacity_tests
//!
//! Responsibility: measure the bounded receipt-related stable allocations.
//! Does not own: product capacity policy, record mutation, or migration logic.
//! Boundary: explicit high-volume tests exercise pinned stable-memory dependencies.

use crate::{
    cdk::{
        structures::{Memory, StableBTreeMap as StableBtreeMap, Storable, VectorMemory},
        types::Principal,
    },
    ids::IntentResourceKey,
    model::{
        intent::{
            PayloadBinding, RECEIPT_BACKED_INTENT_SCHEMA_VERSION, ReceiptBackedIntentState,
            TerminalEvidence, TerminalEvidenceDecision,
        },
        replay::OperationId,
    },
    storage::stable::intent::{
        APPLICATION_RECEIPT_ELIGIBILITY_SCHEMA_VERSION, ApplicationReceiptEligibilityKeyRecord,
        ApplicationReceiptEligibilityRecord, ApplicationReceiptRetentionRecord,
        IntentResourceTotalsRecord, PlacementAcknowledgementEntryRecord, ReceiptBackedIntentRecord,
    },
};

const RECORDS: u64 = 1_000;

#[test]
fn intent_resource_totals_bound_covers_every_u64_value() {
    let record = max_totals();
    let encoded = record.to_bytes().expect("encode stable fixture");
    assert_eq!(
        encoded.len(),
        IntentResourceTotalsRecord::STORABLE_MAX_SIZE as usize
    );
}

#[test]
fn receipt_stable_capacity_fits_with_reserved_cleanup_at_the_admission_limit() {
    for state in [ReceiptBackedIntentState::Pending, terminal_state()] {
        assert!(
            receipt_record(u64::MAX, state)
                .to_bytes()
                .expect("encode stable fixture")
                .len()
                <= ReceiptBackedIntentRecord::STORABLE_MAX_SIZE as usize
        );
    }
    for entries in [ascending().collect::<Vec<_>>(), permuted().collect()] {
        let pages = measure_map(entries.into_iter().map(|seed| {
            let record = receipt_record(seed, terminal_state());
            (record.operation_id, record)
        }));
        assert!(pages <= 32, "receipt primary exceeds two 1 MiB buckets");
    }
    let (_, _, _, allocated) = managed_placement_ascending_pages();
    // Includes the runtime ledger's additional 16-page bucket.
    assert!(allocated <= 81);
    let (_, eligibility, _, allocated) = managed_application_ascending_pages_with_reservation(true);
    let reserved = super::intent::application_eligibility_required_pages(RECORDS).unwrap();
    assert!(eligibility >= reserved);
    // Includes the runtime ledger's additional 16-page bucket.
    assert!(allocated <= 81);
}

fn measure_map<K, V>(entries: impl IntoIterator<Item = (K, V)>) -> u64
where
    K: Storable + Ord + Clone,
    V: Storable,
{
    let runtime = crate::memory::fixtures::runtime(&["test.rows.v1"]);
    let mut map = {
        let allocation = runtime
            .open_memory(&ic_storage::StableKey::parse("test.rows.v1").unwrap())
            .unwrap();
        if ic_storage::Memory::size(&allocation) == 0 {
            StableBtreeMap::<K, V, _>::create(allocation).expect("open fixture collection")
        } else {
            StableBtreeMap::<K, V, _>::load(allocation).expect("open fixture collection")
        }
    };
    for (key, value) in entries {
        assert!(
            map.insert(key, value)
                .expect("Storage operation failed")
                .is_none()
        );
    }
    assert_eq!(map.len(), RECORDS);
    map.allocated_pages().1
}

fn managed_placement_ascending_pages() -> (u64, u64, u64, u64) {
    let physical = VectorMemory::default();
    let runtime = ic_storage::testing::fixture_runtime(
        physical.clone(),
        "test",
        "test.",
        &[
            "test.primary.v1",
            "test.acknowledgement.v1",
            "test.totals.v1",
        ],
        ic_storage::StorageConfig::default(),
    )
    .unwrap();
    let mut primary = {
        let allocation = runtime
            .open_memory(&ic_storage::StableKey::parse("test.primary.v1").unwrap())
            .unwrap();
        if ic_storage::Memory::size(&allocation) == 0 {
            StableBtreeMap::create(allocation).expect("open admitted stable collection")
        } else {
            StableBtreeMap::load(allocation).expect("open admitted stable collection")
        }
    };
    let mut acknowledgements = {
        let allocation = runtime
            .open_memory(&ic_storage::StableKey::parse("test.acknowledgement.v1").unwrap())
            .unwrap();
        if ic_storage::Memory::size(&allocation) == 0 {
            StableBtreeMap::create(allocation).expect("open admitted stable collection")
        } else {
            StableBtreeMap::load(allocation).expect("open admitted stable collection")
        }
    };
    let mut totals = {
        let allocation = runtime
            .open_memory(&ic_storage::StableKey::parse("test.totals.v1").unwrap())
            .unwrap();
        if ic_storage::Memory::size(&allocation) == 0 {
            StableBtreeMap::create(allocation).expect("open admitted stable collection")
        } else {
            StableBtreeMap::load(allocation).expect("open admitted stable collection")
        }
    };

    for seed in ascending() {
        let record = receipt_record(seed, terminal_state());
        let operation_id = record.operation_id;
        primary
            .insert(operation_id, record)
            .expect("commit stable fixture");
        acknowledgements
            .insert(
                operation_id,
                PlacementAcknowledgementEntryRecord { operation_id },
            )
            .expect("commit stable fixture");
        totals
            .insert(resource_key(seed), max_totals())
            .expect("commit stable fixture");
    }
    (
        primary.allocated_pages().1,
        acknowledgements.allocated_pages().1,
        totals.allocated_pages().1,
        physical.size(),
    )
}

fn managed_application_ascending_pages_with_reservation(
    reserve_eligibility: bool,
) -> (u64, u64, u64, u64) {
    let physical = VectorMemory::default();
    let runtime = ic_storage::testing::fixture_runtime(
        physical.clone(),
        "test",
        "test.",
        &["test.primary.v1", "test.eligibility.v1", "test.totals.v1"],
        ic_storage::StorageConfig::default(),
    )
    .unwrap();
    let mut primary = {
        let allocation = runtime
            .open_memory(&ic_storage::StableKey::parse("test.primary.v1").unwrap())
            .unwrap();
        if ic_storage::Memory::size(&allocation) == 0 {
            StableBtreeMap::create(allocation).expect("open admitted stable collection")
        } else {
            StableBtreeMap::load(allocation).expect("open admitted stable collection")
        }
    };
    let mut eligibility = {
        let allocation = runtime
            .open_memory(&ic_storage::StableKey::parse("test.eligibility.v1").unwrap())
            .unwrap();
        if ic_storage::Memory::size(&allocation) == 0 {
            StableBtreeMap::create(allocation).expect("open admitted stable collection")
        } else {
            StableBtreeMap::load(allocation).expect("open admitted stable collection")
        }
    };
    let mut totals = {
        let allocation = runtime
            .open_memory(&ic_storage::StableKey::parse("test.totals.v1").unwrap())
            .unwrap();
        if ic_storage::Memory::size(&allocation) == 0 {
            StableBtreeMap::create(allocation).expect("open admitted stable collection")
        } else {
            StableBtreeMap::load(allocation).expect("open admitted stable collection")
        }
    };

    if reserve_eligibility {
        let required_pages = super::intent::application_eligibility_required_pages(RECORDS)
            .expect("admission-limit eligibility reservation must be representable");
        let (reserved, result) = eligibility.reserve_pages(required_pages);
        result.unwrap();
        eligibility = reserved;
    }

    for seed in ascending() {
        let record = receipt_record(seed, terminal_state());
        let operation_id = record.operation_id;
        primary
            .insert(operation_id, record)
            .expect("commit stable fixture");
        let (eligibility_key, eligibility_record) = application_eligibility_entry(seed);
        eligibility
            .insert(eligibility_key, eligibility_record)
            .expect("commit stable fixture");
        totals
            .insert(resource_key(seed), max_totals())
            .expect("commit stable fixture");
    }
    (
        primary.allocated_pages().1,
        eligibility.allocated_pages().1,
        totals.allocated_pages().1,
        physical.size(),
    )
}

fn ascending() -> std::ops::Range<u64> {
    0..RECORDS
}

fn permuted() -> impl Iterator<Item = u64> {
    ascending().map(|seed| seed.wrapping_mul(0x9e37_79b9_7f4a_7c15))
}

fn operation_id(seed: u64) -> OperationId {
    let mut bytes = [u8::MAX; 32];
    bytes[..8].copy_from_slice(&seed.to_be_bytes());
    OperationId::from_bytes(bytes)
}

fn resource_key(seed: u64) -> IntentResourceKey {
    IntentResourceKey::new(format!("{seed:016x}{}", "r".repeat(112)))
}

fn max_totals() -> IntentResourceTotalsRecord {
    IntentResourceTotalsRecord {
        reserved_qty: u64::MAX,
        committed_qty: u64::MAX,
        pending_count: u64::MAX,
        retain_until_secs: Some(u64::MAX),
    }
}

fn terminal_state() -> ReceiptBackedIntentState {
    ReceiptBackedIntentState::RolledBack {
        evidence: TerminalEvidence::new(
            Principal::from_slice(&[u8::MAX; 29]),
            TerminalEvidenceDecision::RolledBack,
            [u8::MAX; 32],
        ),
    }
}

fn receipt_record(seed: u64, state: ReceiptBackedIntentState) -> ReceiptBackedIntentRecord {
    ReceiptBackedIntentRecord {
        application_retention: Some(ApplicationReceiptRetentionRecord {
            replay_deadline_ns: u64::MAX,
        }),
        schema_version: RECEIPT_BACKED_INTENT_SCHEMA_VERSION,
        operation_id: operation_id(seed),
        payload_binding: PayloadBinding::new([u8::MAX; 32]),
        resource_key: IntentResourceKey::new("r".repeat(128)),
        quantity: u64::MAX,
        state,
        revision: u64::MAX,
        created_at_ns: u64::MAX,
        updated_at_ns: u64::MAX,
    }
}

fn application_eligibility_entry(
    seed: u64,
) -> (
    ApplicationReceiptEligibilityKeyRecord,
    ApplicationReceiptEligibilityRecord,
) {
    let operation_id = operation_id(seed);
    (
        ApplicationReceiptEligibilityKeyRecord {
            eligible_at_ns: seed,
            operation_id,
        },
        ApplicationReceiptEligibilityRecord {
            schema_version: APPLICATION_RECEIPT_ELIGIBILITY_SCHEMA_VERSION,
            operation_id,
            payload_binding: PayloadBinding::new([u8::MAX; 32]),
            terminal_revision: u64::MAX,
        },
    )
}

#[test]
fn empty_bounded_singletons_allocate_only_their_cell_header() {
    fn check<T: Storable>() {
        let runtime = crate::memory::fixtures::runtime(&["test.cell.v1"]);
        let cell = {
            let allocation = runtime
                .open_memory(&ic_storage::StableKey::parse("test.cell.v1").unwrap())
                .unwrap();
            if ic_storage::Memory::size(&allocation) == 0 {
                crate::cdk::structures::StableCell::create(allocation, None::<T>)
                    .expect("open fixture collection")
            } else {
                crate::cdk::structures::StableCell::load(
                    allocation,
                    canic_contracts::serialization::STORAGE_LIMITS.bytes,
                )
                .expect("retained stable cell")
            }
        };
        assert!(cell.get().is_none());
        let allocations = runtime.memory_allocations().unwrap();
        let entry = allocations
            .iter()
            .find(|entry| entry.stable_key.as_str() == "test.cell.v1")
            .unwrap();
        assert_eq!(entry.pages, 1);
    }
    check::<crate::storage::stable::fleet_activation::FleetActivationRecord>();
    check::<crate::storage::stable::authority_restore::AuthorityRestoreFenceRecord>();
    check::<crate::storage::stable::fleet_admission_projection::FleetAdmissionProjectionRecord>();
}
