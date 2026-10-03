//! Exact owner/store-qualified pagination and bounded decoding of canonical accounting.

use super::*;
use canic_core::dto::{
    release_intents::{IntentReleaseReceipt, IntentReleaseReceiptState},
    release_receipts::{ReplayReleaseIntent, ReplayReleaseIntentState},
};

pub(in crate::fleet_ensure::ops::release) fn fixture(owner: Principal) -> IntentReleaseResponse {
    IntentReleaseResponse {
        owner,
        entry: Some(IntentReleaseEntry::Local {
            intent_id: 1,
            record: ReplayReleaseIntent {
                resource_key: "canic:cycles:orphan".into(),
                quantity: u64::MAX,
                state: ReplayReleaseIntentState::Pending,
                created_at_secs: 1,
                ttl_secs: Some(1),
            },
        }),
        next_after: None,
    }
}

pub(in crate::fleet_ensure::ops::release) fn wire(page: IntentReleaseResponse) -> Vec<u8> {
    candid::encode_one(Ok::<_, CanicError>(Response::IntentRelease(page))).unwrap()
}

#[test]
fn pages_keep_original_accounting_across_the_store_boundary() {
    let owner = Principal::from_slice(&[1]);
    let mut local = fixture(owner);
    local.next_after = Some(IntentReleaseKey::Local(1));
    let receipt = IntentReleaseResponse {
        owner,
        entry: Some(IntentReleaseEntry::ReceiptBacked(IntentReleaseReceipt {
            operation_id: [0; 32],
            payload_hash_schema_version: 1,
            payload_hash: [2; 32],
            resource_key: "application:pending".into(),
            quantity: 9,
            state: IntentReleaseReceiptState::Pending,
            revision: 3,
            created_at_ns: 1,
            updated_at_ns: 2,
            application_replay_deadline_ns: Some(3),
        })),
        next_after: None,
    };
    let mut pages = Pages::new(owner);
    let mut remaining = MAXIMUM_CENSUS_BYTES;
    assert!(
        !pages
            .push(decode_response(owner, &wire(local.clone()), &mut remaining).unwrap())
            .unwrap()
    );
    assert!(
        pages
            .push(decode_response(owner, &wire(receipt.clone()), &mut remaining).unwrap())
            .unwrap()
    );
    assert_eq!(pages.pages, vec![local, receipt]);
}

#[test]
fn rejects_wrong_owner_cursor_regression_and_false_completion() {
    let owner = Principal::from_slice(&[1]);
    let mut wrong_owner = fixture(Principal::anonymous());
    assert_eq!(
        Pages::new(owner).push(wrong_owner.clone()),
        Err(ReleaseIntentsStage::Binding)
    );
    wrong_owner.owner = owner;
    wrong_owner.next_after = Some(IntentReleaseKey::ReceiptBacked([1; 32]));
    assert_eq!(
        Pages::new(owner).push(wrong_owner),
        Err(ReleaseIntentsStage::Pagination)
    );
    let mut first = fixture(owner);
    first.next_after = Some(IntentReleaseKey::Local(1));
    let mut pages = Pages::new(owner);
    assert!(!pages.push(first).unwrap());
    assert_eq!(
        pages.push(fixture(owner)),
        Err(ReleaseIntentsStage::Pagination)
    );
    assert_eq!(
        pages.push(IntentReleaseResponse {
            owner,
            entry: None,
            next_after: None
        }),
        Err(ReleaseIntentsStage::Pagination)
    );
    pages.cursor = Some(IntentReleaseKey::ReceiptBacked([0; 32]));
    assert_eq!(
        pages.push(fixture(owner)),
        Err(ReleaseIntentsStage::Pagination)
    );
}

#[test]
fn decoding_and_collection_bounds_refuse_without_returning_partial_evidence() {
    let owner = Principal::anonymous();
    let bytes = wire(fixture(owner));
    for (bytes, mut budget, stage) in [
        (
            vec![0; RESPONSE_BYTES + 1],
            MAXIMUM_CENSUS_BYTES,
            ReleaseIntentsStage::Decode,
        ),
        (vec![0], MAXIMUM_CENSUS_BYTES, ReleaseIntentsStage::Decode),
        (bytes.clone(), bytes.len() - 1, ReleaseIntentsStage::Budget),
    ] {
        assert!(
            matches!(decode_response(owner, &bytes, &mut budget), Err(ReleaseIntentsError::Observation { stage: actual, .. }) if actual == stage)
        );
    }
    let mut pages = Pages::new(owner);
    pages.pages = vec![fixture(owner); MAXIMUM_INTENTS - 1];
    let mut page = fixture(owner);
    page.next_after = Some(IntentReleaseKey::Local(1));
    assert_eq!(pages.push(page), Err(ReleaseIntentsStage::Budget));
    let empty = IntentReleaseResponse {
        owner,
        entry: None,
        next_after: None,
    };
    assert!(Pages::new(owner).push(empty).unwrap());
}
