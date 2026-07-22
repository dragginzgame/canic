#![cfg(test)]

//! Module: storage::stable::receipt_capacity_tests
//!
//! Responsibility: measure the bounded receipt-related stable allocations.
//! Does not own: product capacity policy, record mutation, or migration logic.
//! Boundary: explicit high-volume tests exercise pinned stable-memory dependencies.

use crate::{
    cdk::{
        structures::{
            BTreeMap as StableBtreeMap, Memory, Storable, VectorMemory,
            memory::{MemoryId, MemoryManager},
        },
        types::Principal,
    },
    ids::IntentResourceKey,
    impl_storable_bounded,
    model::{
        intent::{
            PayloadBinding, RECEIPT_BACKED_INTENT_SCHEMA_VERSION, ReceiptBackedIntentState,
            TerminalEvidence, TerminalEvidenceDecision,
        },
        replay::OperationId,
    },
    role_contract::allocation::memory::intent::{
        APPLICATION_RECEIPT_ELIGIBILITY_ID, APPLICATION_RECEIPT_REPLAY_ID, INTENT_TOTALS_ID,
        PLACEMENT_ACKNOWLEDGEMENT_INDEX_ID, RECEIPT_BACKED_INTENT_RECORDS_ID,
    },
    storage::stable::intent::{
        APPLICATION_RECEIPT_ELIGIBILITY_SCHEMA_VERSION, APPLICATION_RECEIPT_REPLAY_SCHEMA_VERSION,
        ApplicationReceiptEligibilityKeyRecord, ApplicationReceiptEligibilityRecord,
        ApplicationReceiptReplayRecord, IntentResourceTotalsRecord,
        PlacementAcknowledgementEntryRecord, ReceiptBackedIntentRecord,
    },
};
use serde::{Deserialize, Serialize};

const RECORDS: u64 = 1_000;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct PreviousTotalsRecord {
    reserved_qty: u64,
    committed_qty: u64,
    pending_count: u64,
}

impl_storable_bounded!(PreviousTotalsRecord, 64, false);

#[test]
fn intent_resource_totals_bound_covers_every_u64_value() {
    let record = max_totals();
    let encoded = record.to_bytes();
    assert_eq!(encoded.len(), 69);
    assert_eq!(
        encoded.len(),
        IntentResourceTotalsRecord::STORABLE_MAX_SIZE as usize
    );
}

#[test]
fn corrected_totals_bound_loads_the_existing_v2_map_without_a_reader() {
    let memory = VectorMemory::default();
    let mut previous =
        StableBtreeMap::<IntentResourceKey, PreviousTotalsRecord, _>::init(memory.clone());
    previous.insert(
        resource_key(0),
        PreviousTotalsRecord {
            reserved_qty: 1,
            committed_qty: 2,
            pending_count: 3,
        },
    );
    assert_eq!(btree_page_size(&memory), 1_727);
    drop(previous);

    let mut corrected =
        StableBtreeMap::<IntentResourceKey, IntentResourceTotalsRecord, _>::init(memory.clone());
    for seed in 1..=100 {
        corrected.insert(resource_key(seed), max_totals());
    }
    drop(corrected);

    let reloaded = StableBtreeMap::<IntentResourceKey, IntentResourceTotalsRecord, _>::init(memory);
    assert_eq!(reloaded.len(), 101);
    assert_eq!(reloaded.get(&resource_key(100)), Some(max_totals()));
}

#[test]
#[ignore = "explicit 1,000-row 0.96 stable-capacity measurement"]
fn receipt_backed_stable_capacity_envelope_is_measured_at_the_admission_limit() {
    assert_eq!(
        receipt_record(u64::MAX, ReceiptBackedIntentState::Pending)
            .to_bytes()
            .len(),
        441
    );
    assert_eq!(
        receipt_record(u64::MAX, terminal_state()).to_bytes().len(),
        617
    );

    let primary_ascending = measure_map(ascending().map(|seed| {
        let record = receipt_record(seed, terminal_state());
        (record.operation_id, record)
    }));
    let primary_permuted = measure_map(permuted().map(|seed| {
        let record = receipt_record(seed, terminal_state());
        (record.operation_id, record)
    }));
    let acknowledgements_ascending = measure_map(ascending().map(|seed| {
        let operation_id = operation_id(seed);
        (
            operation_id,
            PlacementAcknowledgementEntryRecord { operation_id },
        )
    }));
    let acknowledgements_permuted = measure_map(permuted().map(|seed| {
        let operation_id = operation_id(seed);
        (
            operation_id,
            PlacementAcknowledgementEntryRecord { operation_id },
        )
    }));
    let replay_ascending = measure_map(ascending().map(|seed| {
        let record = application_replay_record(seed);
        (record.operation_id, record)
    }));
    let replay_permuted = measure_map(permuted().map(|seed| {
        let record = application_replay_record(seed);
        (record.operation_id, record)
    }));
    let eligibility_ascending = measure_map(ascending().map(application_eligibility_entry));
    let eligibility_permuted = measure_map(permuted().map(application_eligibility_entry));
    let totals_ascending = measure_map(ascending().map(|seed| (resource_key(seed), max_totals())));
    let totals_permuted = measure_map(permuted().map(|seed| (resource_key(seed), max_totals())));

    assert_eq!(primary_ascending, (8_855, 27));
    assert_eq!(primary_permuted, (8_855, 20));
    assert_eq!(acknowledgements_ascending, (1_463, 5));
    assert_eq!(acknowledgements_permuted, (1_463, 4));
    assert_eq!(totals_ascending, (1_768, 6));
    assert_eq!(totals_permuted, (1_768, 5));
    assert_eq!(
        application_replay_record(u64::MAX).to_bytes().len(),
        ApplicationReceiptReplayRecord::STORABLE_MAX_SIZE as usize
    );
    assert_eq!(replay_ascending, (1_430, 5));
    assert_eq!(replay_permuted, (1_430, 4));
    assert_eq!(
        application_eligibility_entry(u64::MAX).1.to_bytes().len(),
        ApplicationReceiptEligibilityRecord::STORABLE_MAX_SIZE as usize
    );
    assert_eq!(eligibility_ascending, (2_362, 8));
    assert_eq!(eligibility_permuted, (2_362, 6));
    assert_eq!(managed_application_ascending_pages(), (27, 5, 8, 6, 513));
    assert_eq!(managed_reserved_application_ascending_pages(), (8, 513));
    assert_eq!(managed_placement_ascending_pages(), (27, 5, 6, 385));
}

fn measure_map<K, V>(entries: impl IntoIterator<Item = (K, V)>) -> (u32, u64)
where
    K: Storable + Ord + Clone,
    V: Storable,
{
    let memory = VectorMemory::default();
    let mut map = StableBtreeMap::<K, V, _>::init(memory.clone());
    for (key, value) in entries {
        assert!(map.insert(key, value).is_none());
    }
    assert_eq!(map.len(), RECORDS);
    (btree_page_size(&memory), memory.size())
}

fn managed_placement_ascending_pages() -> (u64, u64, u64, u64) {
    let physical = VectorMemory::default();
    let manager = MemoryManager::init(physical.clone());
    let primary_memory = manager.get(MemoryId::new(RECEIPT_BACKED_INTENT_RECORDS_ID));
    let acknowledgement_memory = manager.get(MemoryId::new(PLACEMENT_ACKNOWLEDGEMENT_INDEX_ID));
    let totals_memory = manager.get(MemoryId::new(INTENT_TOTALS_ID));
    let mut primary = StableBtreeMap::init(primary_memory.clone());
    let mut acknowledgements = StableBtreeMap::init(acknowledgement_memory.clone());
    let mut totals = StableBtreeMap::init(totals_memory.clone());

    for seed in ascending() {
        let record = receipt_record(seed, terminal_state());
        let operation_id = record.operation_id;
        primary.insert(operation_id, record);
        acknowledgements.insert(
            operation_id,
            PlacementAcknowledgementEntryRecord { operation_id },
        );
        totals.insert(resource_key(seed), max_totals());
    }
    (
        primary_memory.size(),
        acknowledgement_memory.size(),
        totals_memory.size(),
        physical.size(),
    )
}

fn managed_application_ascending_pages() -> (u64, u64, u64, u64, u64) {
    managed_application_ascending_pages_with_reservation(false)
}

fn managed_reserved_application_ascending_pages() -> (u64, u64) {
    let measured = managed_application_ascending_pages_with_reservation(true);
    (measured.2, measured.4)
}

fn managed_application_ascending_pages_with_reservation(
    reserve_eligibility: bool,
) -> (u64, u64, u64, u64, u64) {
    let physical = VectorMemory::default();
    let manager = MemoryManager::init(physical.clone());
    let primary_memory = manager.get(MemoryId::new(RECEIPT_BACKED_INTENT_RECORDS_ID));
    let replay_memory = manager.get(MemoryId::new(APPLICATION_RECEIPT_REPLAY_ID));
    let eligibility_memory = manager.get(MemoryId::new(APPLICATION_RECEIPT_ELIGIBILITY_ID));
    let totals_memory = manager.get(MemoryId::new(INTENT_TOTALS_ID));
    let mut primary = StableBtreeMap::init(primary_memory.clone());
    let mut replay = StableBtreeMap::init(replay_memory.clone());
    let mut eligibility = StableBtreeMap::init(eligibility_memory.clone());
    let mut totals = StableBtreeMap::init(totals_memory.clone());

    if reserve_eligibility {
        let required_pages = super::intent::application_eligibility_required_pages(RECORDS)
            .expect("admission-limit eligibility reservation must be representable");
        let current_pages = eligibility_memory.size();
        assert!(eligibility_memory.grow(required_pages - current_pages) >= 0);
    }

    for seed in ascending() {
        let record = receipt_record(seed, terminal_state());
        let operation_id = record.operation_id;
        primary.insert(operation_id, record);
        replay.insert(operation_id, application_replay_record(seed));
        let (eligibility_key, eligibility_record) = application_eligibility_entry(seed);
        eligibility.insert(eligibility_key, eligibility_record);
        totals.insert(resource_key(seed), max_totals());
    }
    (
        primary_memory.size(),
        replay_memory.size(),
        eligibility_memory.size(),
        totals_memory.size(),
        physical.size(),
    )
}

fn btree_page_size(memory: &VectorMemory) -> u32 {
    let bytes = memory.borrow();
    assert_eq!(&bytes[0..3], b"BTR");
    assert_eq!(
        u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
        u32::MAX
    );
    u32::from_le_bytes(bytes[4..8].try_into().unwrap())
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

fn application_replay_record(seed: u64) -> ApplicationReceiptReplayRecord {
    ApplicationReceiptReplayRecord {
        schema_version: APPLICATION_RECEIPT_REPLAY_SCHEMA_VERSION,
        operation_id: operation_id(seed),
        replay_deadline_ns: u64::MAX,
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
