//! Canonical accounting discovery retains orphan work and never consults cleanup indexes.

use super::*;
use crate::{
    ids::IntentResourceKey,
    model::intent::{PayloadBinding, TerminalEvidenceDecision},
    ops::storage::intent::IntentStoreOps,
    storage::stable::intent::{
        ApplicationReceiptRetentionRecord, IntentRecord, IntentState, IntentStore,
        ReceiptBackedIntentRecord, ReceiptBackedIntentStore,
    },
};

fn local(id: u64, state: IntentState) -> IntentRecord {
    IntentRecord {
        id: IntentId(id),
        resource_key: IntentResourceKey::try_from("canic:cycles:orphan").unwrap(),
        quantity: u64::MAX,
        state,
        created_at: 1,
        ttl_secs: Some(1),
    }
}

fn receipt(id: u8, state: ReceiptBackedIntentState) -> ReceiptBackedIntentRecord {
    ReceiptBackedIntentRecord {
        application_retention: Some(ApplicationReceiptRetentionRecord {
            replay_deadline_ns: 3,
        }),
        schema_version: 1,
        operation_id: OperationId::from_bytes([id; 32]),
        payload_binding: PayloadBinding::new([4; 32]),
        resource_key: IntentResourceKey::try_from("application:obligation").unwrap(),
        quantity: 99,
        state,
        revision: 7,
        created_at_ns: 1,
        updated_at_ns: 2,
    }
}

#[test]
fn canonical_census_keeps_orphan_pending_terminal_and_expired_rows_without_indexes() {
    IntentStoreOps::reset_for_tests();
    let owner = Principal::from_slice(&[1]);
    assert_eq!(observe(owner, None).unwrap().entry, None);
    for (id, state) in [
        (0, IntentState::Pending),
        (1, IntentState::Committed),
        (u64::MAX, IntentState::Aborted),
    ] {
        IntentStore::insert_record(local(id, state));
    }
    for (id, state) in [
        (0, ReceiptBackedIntentState::Pending),
        (
            1,
            ReceiptBackedIntentState::Committed {
                evidence: TerminalEvidence::new(
                    owner,
                    TerminalEvidenceDecision::Committed,
                    [8; 32],
                ),
            },
        ),
        (
            255,
            ReceiptBackedIntentState::RolledBack {
                evidence: TerminalEvidence::new(
                    owner,
                    TerminalEvidenceDecision::RolledBack,
                    [9; 32],
                ),
            },
        ),
    ] {
        ReceiptBackedIntentStore::insert(receipt(id, state));
    }
    let before = ReceiptBackedIntentStore::export_records();
    let mut cursor = None;
    let keys = [
        IntentReleaseKey::Local(0),
        IntentReleaseKey::Local(1),
        IntentReleaseKey::Local(u64::MAX),
        IntentReleaseKey::ReceiptBacked([0; 32]),
        IntentReleaseKey::ReceiptBacked([1; 32]),
        IntentReleaseKey::ReceiptBacked([255; 32]),
    ];
    for (index, key) in keys.iter().enumerate() {
        let page = observe(owner, cursor).unwrap();
        assert_eq!(page.owner, owner);
        assert_eq!(observe(owner, cursor).unwrap(), page);
        match page.entry.as_ref().unwrap() {
            IntentReleaseEntry::Local { intent_id, record } => {
                assert_eq!(*key, IntentReleaseKey::Local(*intent_id));
                assert_eq!(record.ttl_secs, Some(1));
                assert_eq!(record.quantity, u64::MAX);
                assert!(IntentStore::get_pending(IntentId(*intent_id)).is_none());
            }
            IntentReleaseEntry::ReceiptBacked(record) => {
                assert_eq!(*key, IntentReleaseKey::ReceiptBacked(record.operation_id));
                assert_eq!(record.application_replay_deadline_ns, Some(3));
                assert_eq!(record.payload_hash, [4; 32]);
                assert_eq!(record.revision, 7);
            }
        }
        let bytes = candid::encode_one(&page).unwrap();
        assert!(bytes.len() < 4096);
        assert_eq!(
            candid::decode_one::<IntentReleaseResponse>(&bytes).unwrap(),
            page
        );
        assert_eq!(
            page.next_after,
            if index + 1 < keys.len() {
                Some(*key)
            } else {
                None
            }
        );
        cursor = page.next_after;
    }
    assert_eq!(ReceiptBackedIntentStore::export_records(), before);
    assert_eq!(
        IntentStore::get_record(IntentId(0)),
        Some(local(0, IntentState::Pending))
    );
    assert_eq!(
        observe(owner, Some(IntentReleaseKey::ReceiptBacked([255; 32])))
            .unwrap()
            .entry,
        None
    );
}

#[test]
fn receipt_only_census_includes_internal_reservations_without_application_deadline() {
    IntentStoreOps::reset_for_tests();
    let mut row = receipt(0, ReceiptBackedIntentState::Pending);
    row.application_retention = None;
    row.resource_key = IntentResourceKey::try_from("canic:placement:test").unwrap();
    ReceiptBackedIntentStore::insert(row.clone());
    let page = observe(Principal::anonymous(), None).unwrap();
    let Some(IntentReleaseEntry::ReceiptBacked(entry)) = page.entry else {
        panic!("canonical receipt")
    };
    assert_eq!(entry.application_replay_deadline_ns, None);
    assert_eq!(entry.resource_key, row.resource_key.as_str());
    assert_eq!(entry.state, IntentReleaseReceiptState::Pending);
    assert_eq!(page.next_after, None);
}

#[test]
fn lookahead_does_not_validate_later_values_and_bad_primary_refuses_without_mutation() {
    IntentStoreOps::reset_for_tests();
    IntentStore::insert_record(local(1, IntentState::Pending));
    let mut invalid = receipt(1, ReceiptBackedIntentState::Pending);
    invalid.schema_version = 2;
    ReceiptBackedIntentStore::insert(invalid);
    let before = ReceiptBackedIntentStore::export_records();
    let page = observe(Principal::anonymous(), None).unwrap();
    assert_eq!(page.next_after, Some(IntentReleaseKey::Local(1)));
    assert!(observe(Principal::anonymous(), page.next_after).is_err());
    assert_eq!(ReceiptBackedIntentStore::export_records(), before);
}

#[test]
fn contradictory_terminal_evidence_cannot_be_presented_as_completion() {
    IntentStoreOps::reset_for_tests();
    ReceiptBackedIntentStore::insert(receipt(
        1,
        ReceiptBackedIntentState::Committed {
            evidence: TerminalEvidence::new(
                Principal::anonymous(),
                TerminalEvidenceDecision::RolledBack,
                [1; 32],
            ),
        },
    ));
    let before = ReceiptBackedIntentStore::export_records();
    assert_eq!(
        observe(Principal::anonymous(), None).unwrap_err().code(),
        InternalError::conflict().code()
    );
    assert_eq!(ReceiptBackedIntentStore::export_records(), before);
}
