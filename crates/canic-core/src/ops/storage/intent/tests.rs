use super::*;
use crate::{
    InternalErrorClass, InternalErrorOrigin,
    cdk::types::Principal,
    model::intent::{
        BeginReceiptBackedIntentInput, BeginReceiptBackedIntentResult, PayloadBinding,
        ReceiptBackedIntentState, RemoveTerminalReceiptBackedIntentInput,
        RemoveTerminalReceiptBackedIntentResult, SettleReceiptBackedIntentInput,
        SettleReceiptBackedIntentResult, TerminalEvidence, TerminalEvidenceDecision,
    },
    storage::stable::intent::{IntentStore, ReceiptBackedIntentStore},
};

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

fn totals(key: &IntentResourceKey) -> IntentResourceTotalsRecord {
    IntentStoreOps::totals(key)
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

fn operation_id(byte: u8) -> OperationId {
    OperationId::from_bytes([byte; 32])
}

fn receipt_input(byte: u8) -> BeginReceiptBackedIntentInput {
    BeginReceiptBackedIntentInput {
        operation_id: operation_id(byte),
        payload_binding: PayloadBinding::new([byte.wrapping_add(1); 32]),
        resource_key: key(),
        quantity: 5,
        reservation_limit: 20,
    }
}

fn terminal_evidence(decision: TerminalEvidenceDecision, byte: u8) -> TerminalEvidence {
    TerminalEvidence::new(Principal::from_slice(&[1; 29]), decision, [byte; 32])
}

#[test]
fn idempotent_ops_do_not_double_count() {
    // -------------------------------------------------------------------------
    // Case
    // -------------------------------------------------------------------------

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

        let totals_after_first = totals(&resource_key);
        let meta_after_first = meta();

        let second = match case.op {
            Some(op) => apply(op, intent_id).unwrap(),
            None => reserve(intent_id, resource_key.clone(), 5, None).unwrap(),
        };

        assert_eq!(second.state, case.expected_state, "{}", case.name);
        assert_eq!(totals(&resource_key), totals_after_first);
        assert_eq!(meta(), meta_after_first);

        let meta = meta();
        assert_eq!(totals(&resource_key), case.expected_totals);
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
fn expired_intents_remain_reserved_until_cleanup() {
    reset();
    let resource_key = key();
    let intent_id = IntentId(10);

    reserve(intent_id, resource_key.clone(), 3, Some(5)).unwrap();

    let now = CREATED_AT + 10;
    let reserved_totals = totals(&resource_key);

    assert_eq!(reserved_totals.reserved_qty, 3);
    assert_eq!(reserved_totals.pending_count, 1);

    let pending = IntentStoreOps::pending_entries_at(now);
    assert!(pending.is_empty());

    let expired = IntentStoreOps::list_expired_pending_intents(now);
    assert_eq!(expired, vec![intent_id]);

    IntentStoreOps::commit_at(intent_id, now).unwrap_err();

    let record = IntentStore::get_record(intent_id).expect("record should exist");
    assert_eq!(record.state, IntentState::Pending);
    assert!(IntentStore::get_pending(intent_id).is_some());

    assert!(IntentStoreOps::abort_intent_if_pending(intent_id).expect("cleanup abort"));
    assert_eq!(totals(&resource_key), IntentResourceTotalsRecord::default());
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

    assert_eq!(totals(&resource_key).reserved_qty, u64::MAX);
}

#[test]
fn receipt_backed_begin_replays_without_second_reservation_or_ttl_entry() {
    reset();
    let input = receipt_input(1);

    assert_eq!(
        ReceiptBackedIntentOps::begin_or_load(&input, 100).expect("create"),
        BeginReceiptBackedIntentResult::Created { revision: 1 }
    );
    let totals_after_create = totals(&input.resource_key);
    let local_meta_after_create = meta();

    assert_eq!(
        ReceiptBackedIntentOps::begin_or_load(&input, 200).expect("replay"),
        BeginReceiptBackedIntentResult::ExistingPending { revision: 1 }
    );
    assert_eq!(totals(&input.resource_key), totals_after_create);
    assert_eq!(totals_after_create.reserved_qty, input.quantity);
    assert_eq!(totals_after_create.pending_count, 1);
    assert_eq!(meta(), local_meta_after_create);
    assert_eq!(local_meta_after_create.pending_total, 0);
    assert!(IntentStoreOps::list_expired_pending_intents(u64::MAX).is_empty());
    assert_eq!(ReceiptBackedIntentStore::len(), 1);

    assert_eq!(
        ReceiptBackedIntentOps::begin_or_load_with_limit(&input, 300, 1)
            .expect("existing at capacity"),
        BeginReceiptBackedIntentResult::ExistingPending { revision: 1 }
    );

    let evidence = terminal_evidence(TerminalEvidenceDecision::Committed, 2);
    assert_eq!(
        ReceiptBackedIntentOps::settle_if_pending(
            &SettleReceiptBackedIntentInput {
                operation_id: input.operation_id,
                expected_revision: 1,
                expected_payload_binding: input.payload_binding,
                evidence,
            },
            400,
        )
        .expect("settle existing at capacity"),
        SettleReceiptBackedIntentResult::Settled {
            revision: 2,
            state: ReceiptBackedIntentState::Committed { evidence },
        }
    );
}

#[test]
fn receipt_backed_begin_conflicts_without_mutation() {
    reset();
    let input = receipt_input(2);
    ReceiptBackedIntentOps::begin_or_load(&input, 100).expect("create");
    let totals_after_create = totals(&input.resource_key);
    let receipt_count_after_create = ReceiptBackedIntentStore::len();

    for conflict in [
        BeginReceiptBackedIntentInput {
            payload_binding: PayloadBinding::new([9; 32]),
            ..input.clone()
        },
        BeginReceiptBackedIntentInput {
            resource_key: IntentResourceKey::new("other"),
            ..input.clone()
        },
        BeginReceiptBackedIntentInput {
            quantity: input.quantity + 1,
            ..input.clone()
        },
    ] {
        assert_eq!(
            ReceiptBackedIntentOps::begin_or_load(&conflict, 200).expect("conflict"),
            BeginReceiptBackedIntentResult::BindingConflict
        );
    }

    assert_eq!(totals(&input.resource_key), totals_after_create);
    assert_eq!(ReceiptBackedIntentStore::len(), receipt_count_after_create);
}

#[test]
fn receipt_backed_rejects_unsupported_schemas_without_mutation() {
    reset();
    let mut unsupported = receipt_input(6);
    unsupported.payload_binding.schema_version = PAYLOAD_BINDING_SCHEMA_VERSION + 1;

    let begin_error = ReceiptBackedIntentOps::begin_or_load(&unsupported, 100)
        .expect_err("unsupported binding schema");
    assert_eq!(
        begin_error.log_fields(),
        (InternalErrorClass::Ops, InternalErrorOrigin::Ops)
    );
    assert!(
        ReceiptBackedIntentOps::load(unsupported.operation_id)
            .expect("load absent operation")
            .is_none()
    );
    assert_eq!(
        totals(&unsupported.resource_key),
        IntentResourceTotalsRecord::default()
    );
    assert_eq!(ReceiptBackedIntentStore::len(), 0);

    let input = receipt_input(6);
    ReceiptBackedIntentOps::begin_or_load(&input, 200).expect("create supported intent");
    let mut evidence = terminal_evidence(TerminalEvidenceDecision::Committed, 6);
    evidence.schema_version = TERMINAL_EVIDENCE_SCHEMA_VERSION + 1;
    let settle_error = ReceiptBackedIntentOps::settle_if_pending(
        &SettleReceiptBackedIntentInput {
            operation_id: input.operation_id,
            expected_revision: 1,
            expected_payload_binding: input.payload_binding,
            evidence,
        },
        300,
    )
    .expect_err("unsupported evidence schema");
    assert_eq!(
        settle_error.log_fields(),
        (InternalErrorClass::Ops, InternalErrorOrigin::Ops)
    );
    assert_eq!(
        ReceiptBackedIntentOps::load(input.operation_id)
            .expect("load pending intent")
            .expect("pending intent")
            .state,
        ReceiptBackedIntentState::Pending
    );
    assert_eq!(totals(&input.resource_key).reserved_qty, input.quantity);
}

#[test]
fn terminal_receipt_cleanup_is_scoped_exact_and_preserves_totals() {
    reset();
    let mut placement = receipt_input(20);
    placement.resource_key = IntentResourceKey::new("placement:test");
    let mut other = receipt_input(21);
    other.resource_key = IntentResourceKey::new("other:test");

    for input in [&placement, &other] {
        ReceiptBackedIntentOps::begin_or_load(input, 100).expect("create intent");
        ReceiptBackedIntentOps::settle_if_pending(
            &SettleReceiptBackedIntentInput {
                operation_id: input.operation_id,
                expected_revision: 1,
                expected_payload_binding: input.payload_binding,
                evidence: terminal_evidence(TerminalEvidenceDecision::Committed, 22),
            },
            200,
        )
        .expect("settle intent");
    }

    let listed = ReceiptBackedIntentOps::list_page(None, 10).expect("list intent page");
    assert_eq!(listed.intents.len(), 2);
    assert_eq!(listed.next_cursor, None);
    assert!(listed.intents.iter().any(|intent| {
        intent.operation_id == placement.operation_id
            && intent.resource_key.starts_with("placement:")
            && !matches!(intent.state, ReceiptBackedIntentState::Pending)
    }));
    let first_page = ReceiptBackedIntentOps::list_page(None, 1).expect("first bounded page");
    let first_cursor = first_page
        .next_cursor
        .expect("another record must require a continuation cursor");
    assert_eq!(first_page.intents.len(), 1);
    assert_eq!(first_page.intents[0].operation_id, first_cursor);
    let second_page =
        ReceiptBackedIntentOps::list_page(Some(first_cursor), 1).expect("second bounded page");
    assert_eq!(second_page.intents.len(), 1);
    assert_ne!(
        second_page.intents[0].operation_id,
        first_page.intents[0].operation_id
    );
    assert_eq!(second_page.next_cursor, None);

    let totals_before = totals(&placement.resource_key);
    assert_eq!(
        ReceiptBackedIntentOps::remove_terminal(&RemoveTerminalReceiptBackedIntentInput {
            operation_id: placement.operation_id,
            expected_revision: 2,
            expected_payload_binding: PayloadBinding::new([99; 32]),
        })
        .expect("binding conflict is typed"),
        RemoveTerminalReceiptBackedIntentResult::BindingConflict
    );
    assert_eq!(
        ReceiptBackedIntentOps::remove_terminal(&RemoveTerminalReceiptBackedIntentInput {
            operation_id: placement.operation_id,
            expected_revision: 2,
            expected_payload_binding: placement.payload_binding,
        })
        .expect("terminal removal succeeds"),
        RemoveTerminalReceiptBackedIntentResult::Removed
    );
    assert_eq!(totals(&placement.resource_key), totals_before);
    assert!(
        ReceiptBackedIntentOps::load(placement.operation_id)
            .expect("load removed intent")
            .is_none()
    );
    assert!(
        ReceiptBackedIntentOps::load(other.operation_id)
            .expect("load unrelated intent")
            .is_some()
    );
}

#[test]
fn receipt_backed_begin_enforces_shared_resource_and_store_capacity() {
    reset();
    let resource_key = key();
    reserve(IntentId(1), resource_key.clone(), 4, Some(60)).expect("local reserve");

    let mut input = receipt_input(3);
    input.resource_key = resource_key.clone();
    input.quantity = 2;
    input.reservation_limit = 5;
    assert_eq!(
        ReceiptBackedIntentOps::begin_or_load(&input, 100).expect("capacity result"),
        BeginReceiptBackedIntentResult::CapacityExceeded {
            current_quantity: 4,
            requested_quantity: 2,
            limit: 5,
        }
    );
    assert_eq!(totals(&resource_key).reserved_qty, 4);

    input.reservation_limit = 10;
    assert_eq!(
        ReceiptBackedIntentOps::begin_or_load_with_limit(&input, 100, 0)
            .expect("store capacity result"),
        BeginReceiptBackedIntentResult::StoreCapacityReached {
            current_records: 0,
            limit: 0,
        }
    );
    assert_eq!(totals(&resource_key).reserved_qty, 4);
}

#[test]
fn receipt_backed_commit_is_compare_and_set_and_idempotent() {
    reset();
    let input = receipt_input(4);
    ReceiptBackedIntentOps::begin_or_load(&input, 100).expect("create");
    let evidence = terminal_evidence(TerminalEvidenceDecision::Committed, 7);
    let settle = SettleReceiptBackedIntentInput {
        operation_id: input.operation_id,
        expected_revision: 1,
        expected_payload_binding: input.payload_binding,
        evidence,
    };

    assert_eq!(
        ReceiptBackedIntentOps::settle_if_pending(&settle, 200).expect("settle"),
        SettleReceiptBackedIntentResult::Settled {
            revision: 2,
            state: ReceiptBackedIntentState::Committed { evidence },
        }
    );
    let totals_after_settle = totals(&input.resource_key);
    assert_eq!(totals_after_settle.reserved_qty, 0);
    assert_eq!(totals_after_settle.committed_qty, input.quantity);
    assert_eq!(totals_after_settle.pending_count, 0);

    assert_eq!(
        ReceiptBackedIntentOps::settle_if_pending(&settle, 300).expect("idempotent settle"),
        SettleReceiptBackedIntentResult::AlreadySettled {
            revision: 2,
            state: ReceiptBackedIntentState::Committed { evidence },
        }
    );
    assert_eq!(totals(&input.resource_key), totals_after_settle);
    assert_eq!(
        ReceiptBackedIntentOps::begin_or_load(&input, 350).expect("terminal replay"),
        BeginReceiptBackedIntentResult::ExistingCommitted {
            revision: 2,
            evidence,
        }
    );

    let contradictory = SettleReceiptBackedIntentInput {
        evidence: terminal_evidence(TerminalEvidenceDecision::RolledBack, 8),
        ..settle
    };
    ReceiptBackedIntentOps::settle_if_pending(&contradictory, 400)
        .expect_err("contradictory evidence must fail");
    assert_eq!(totals(&input.resource_key), totals_after_settle);
}

#[test]
fn receipt_backed_rollback_and_revision_conflict_preserve_exact_totals() {
    reset();
    let input = receipt_input(5);
    ReceiptBackedIntentOps::begin_or_load(&input, 100).expect("create");
    let evidence = terminal_evidence(TerminalEvidenceDecision::RolledBack, 9);
    let stale = SettleReceiptBackedIntentInput {
        operation_id: input.operation_id,
        expected_revision: 0,
        expected_payload_binding: input.payload_binding,
        evidence,
    };

    assert_eq!(
        ReceiptBackedIntentOps::settle_if_pending(&stale, 200).expect("stale result"),
        SettleReceiptBackedIntentResult::RevisionConflict { actual_revision: 1 }
    );
    assert_eq!(totals(&input.resource_key).reserved_qty, input.quantity);

    let settle = SettleReceiptBackedIntentInput {
        expected_revision: 1,
        ..stale
    };
    assert_eq!(
        ReceiptBackedIntentOps::settle_if_pending(&settle, 300).expect("rollback"),
        SettleReceiptBackedIntentResult::Settled {
            revision: 2,
            state: ReceiptBackedIntentState::RolledBack { evidence },
        }
    );
    assert_eq!(
        totals(&input.resource_key),
        IntentResourceTotalsRecord::default()
    );

    let loaded = ReceiptBackedIntentOps::load(input.operation_id)
        .expect("load")
        .expect("stored intent");
    assert_eq!(loaded.revision, 2);
    assert_eq!(loaded.updated_at_ns, 300);
    assert_eq!(
        loaded.state,
        ReceiptBackedIntentState::RolledBack { evidence }
    );
}
