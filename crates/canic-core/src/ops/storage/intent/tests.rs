use super::*;
use crate::storage::stable::intent::IntentStore;

const CREATED_AT: u64 = 10;
const NOW: u64 = 100;

#[derive(Clone, Copy, Debug)]
enum Op {
    Commit,
    Abort,
}

fn reset() {
    IntentStore::reset_for_tests();
}

fn key() -> IntentResourceKey {
    IntentResourceKey::new("resource")
}

fn reserve(
    intent_id: IntentId,
    resource_key: IntentResourceKey,
    quantity: u64,
    ttl_secs: Option<u64>,
) -> Result<IntentRecord, InternalError> {
    IntentStoreOps::try_reserve(intent_id, resource_key, quantity, CREATED_AT, ttl_secs, NOW)
}

fn totals_at(key: &IntentResourceKey, now: u64) -> IntentResourceTotalsRecord {
    IntentStoreOps::totals_at(key, now)
}

fn meta() -> IntentStoreMetaRecord {
    IntentStore::meta()
}

fn apply(op: Op, intent_id: IntentId) -> Result<IntentRecord, InternalError> {
    match op {
        Op::Commit => IntentStoreOps::commit_at(intent_id, NOW),
        Op::Abort => IntentStoreOps::abort(intent_id),
    }
}

#[test]
fn idempotent_ops_do_not_double_count() {
    ///
    /// Case
    ///

    struct Case {
        name: &'static str,
        op: Option<Op>,
        expected_state: IntentState,
        expected_totals: IntentResourceTotalsRecord,
        pending_total: u64,
        committed_total: u64,
        aborted_total: u64,
    }

    let cases = [
        Case {
            name: "reserve only",
            op: None,
            expected_state: IntentState::Pending,
            expected_totals: IntentResourceTotalsRecord {
                reserved_qty: 5,
                committed_qty: 0,
                pending_count: 1,
            },
            pending_total: 1,
            committed_total: 0,
            aborted_total: 0,
        },
        Case {
            name: "commit",
            op: Some(Op::Commit),
            expected_state: IntentState::Committed,
            expected_totals: IntentResourceTotalsRecord {
                reserved_qty: 0,
                committed_qty: 5,
                pending_count: 0,
            },
            pending_total: 0,
            committed_total: 1,
            aborted_total: 0,
        },
        Case {
            name: "abort",
            op: Some(Op::Abort),
            expected_state: IntentState::Aborted,
            expected_totals: IntentResourceTotalsRecord {
                reserved_qty: 0,
                committed_qty: 0,
                pending_count: 0,
            },
            pending_total: 0,
            committed_total: 0,
            aborted_total: 1,
        },
    ];

    for case in cases {
        reset();
        let resource_key = key();
        let intent_id = IntentId(1);

        let first = reserve(intent_id, resource_key.clone(), 5, None).unwrap();
        let first = match case.op {
            Some(op) => apply(op, intent_id).unwrap(),
            None => first,
        };

        assert_eq!(first.state, case.expected_state, "{}", case.name);

        let totals_after_first = totals_at(&resource_key, NOW);
        let meta_after_first = meta();

        let second = match case.op {
            Some(op) => apply(op, intent_id).unwrap(),
            None => reserve(intent_id, resource_key.clone(), 5, None).unwrap(),
        };

        assert_eq!(second.state, case.expected_state, "{}", case.name);
        assert_eq!(totals_at(&resource_key, NOW), totals_after_first);
        assert_eq!(meta(), meta_after_first);

        let meta = meta();
        assert_eq!(totals_at(&resource_key, NOW), case.expected_totals);
        assert_eq!(meta.pending_total, case.pending_total);
        assert_eq!(meta.committed_total, case.committed_total);
        assert_eq!(meta.aborted_total, case.aborted_total);
    }
}

#[test]
fn valid_pending_transitions() {
    reset();
    let resource_key = key();
    let intent_id = IntentId(2);

    reserve(intent_id, resource_key.clone(), 3, None).unwrap();
    let committed = IntentStoreOps::commit_at(intent_id, NOW).unwrap();
    assert_eq!(committed.state, IntentState::Committed);

    reset();
    reserve(intent_id, resource_key, 3, None).unwrap();
    let aborted = IntentStoreOps::abort(intent_id).unwrap();
    assert_eq!(aborted.state, IntentState::Aborted);
}

#[test]
fn rejects_invalid_transitions() {
    reset();
    let resource_key = key();
    let intent_id = IntentId(3);

    reserve(intent_id, resource_key, 4, None).unwrap();
    IntentStoreOps::commit_at(intent_id, NOW).unwrap();

    IntentStoreOps::abort(intent_id).unwrap_err();

    let record = IntentStore::get_record(intent_id).expect("record should exist");
    assert_eq!(record.state, IntentState::Committed);
    assert!(IntentStore::get_pending(intent_id).is_none());
}

#[test]
fn expired_intents_are_logically_ignored() {
    reset();
    let resource_key = key();
    let intent_id = IntentId(10);

    reserve(intent_id, resource_key.clone(), 3, Some(5)).unwrap();

    let now = CREATED_AT + 10;
    let totals = totals_at(&resource_key, now);

    assert_eq!(totals.reserved_qty, 0);
    assert_eq!(totals.pending_count, 0);

    let pending = IntentStoreOps::pending_entries_at(now);
    assert!(pending.is_empty());

    let expired = IntentStoreOps::list_expired_pending_intents(now);
    assert_eq!(expired, vec![intent_id]);

    IntentStoreOps::commit_at(intent_id, now).unwrap_err();

    let record = IntentStore::get_record(intent_id).expect("record should exist");
    assert_eq!(record.state, IntentState::Pending);
    assert!(IntentStore::get_pending(intent_id).is_some());
}

#[test]
fn prevents_aggregate_underflow() {
    reset();
    let resource_key = key();
    let intent_id = IntentId(42);

    IntentStore::insert_record(IntentRecord {
        id: intent_id,
        resource_key: resource_key.clone(),
        quantity: 9,
        state: IntentState::Pending,
        created_at: CREATED_AT,
        ttl_secs: None,
    });

    IntentStore::insert_pending(
        intent_id,
        IntentPendingEntryRecord {
            resource_key: resource_key.clone(),
            quantity: 9,
            created_at: CREATED_AT,
            ttl_secs: None,
        },
    );

    IntentStore::set_totals(
        resource_key.clone(),
        IntentResourceTotalsRecord {
            reserved_qty: 0,
            committed_qty: 0,
            pending_count: 1,
        },
    );

    IntentStoreOps::commit_at(intent_id, NOW).unwrap_err();

    let record = IntentStore::get_record(intent_id).expect("record should exist");
    assert_eq!(record.state, IntentState::Pending);
    let totals = IntentStore::get_totals(&resource_key).expect("totals should exist");
    assert_eq!(totals.reserved_qty, 0);
    assert_eq!(totals.pending_count, 1);
}

#[test]
fn prevents_aggregate_overflow() {
    reset();
    let resource_key = key();
    let intent_id = IntentId(7);

    IntentStore::set_totals(
        resource_key.clone(),
        IntentResourceTotalsRecord {
            reserved_qty: u64::MAX,
            committed_qty: 0,
            pending_count: 0,
        },
    );

    reserve(intent_id, resource_key.clone(), 1, None).expect_err("overflow should fail");

    assert!(IntentStore::get_record(intent_id).is_none());
    assert!(IntentStoreOps::pending_entries_at(NOW).is_empty());

    let raw = IntentStore::get_totals(&resource_key).expect("raw totals should exist");
    assert_eq!(raw.reserved_qty, u64::MAX, "totals should be unchanged");

    let logical = totals_at(&resource_key, NOW);
    assert_eq!(logical.reserved_qty, 0);
}
