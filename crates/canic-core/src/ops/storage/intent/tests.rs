use super::*;
use crate::{
    InternalErrorClass, InternalErrorOrigin,
    cdk::types::Principal,
    ids::CanisterRole,
    model::{
        intent::{
            BeginPlacementReceiptBackedIntentInput, BeginReceiptBackedIntentInput,
            BeginReceiptBackedIntentResult, MAX_RECEIPT_BACKED_INTENT_REPLAY_WINDOW_NS,
            PayloadBinding, RECEIPT_TERMINAL_OBSERVATION_GRACE_NS, ReceiptBackedIntentState,
            RemoveTerminalReceiptBackedIntentInput, RemoveTerminalReceiptBackedIntentResult,
            SettleReceiptBackedIntentInput, SettleReceiptBackedIntentResult, TerminalEvidence,
            TerminalEvidenceDecision,
        },
        placement::allocation::PlacementAllocationIdentity,
    },
    storage::stable::intent::{
        APPLICATION_RECEIPT_ELIGIBILITY_SCHEMA_VERSION, APPLICATION_RECEIPT_REPLAY_SCHEMA_VERSION,
        ApplicationReceiptEligibilityData, ApplicationReceiptEligibilityEntryRecord,
        ApplicationReceiptEligibilityKeyRecord, ApplicationReceiptEligibilityRecord,
        ApplicationReceiptReplayData, ApplicationReceiptReplayEntryRecord,
        ApplicationReceiptReplayRecord, IntentStore, IntentTotalsData,
        PlacementAcknowledgementEntryRecord, PlacementAcknowledgementIndexData,
        PlacementAcknowledgementIndexEntryRecord, ReceiptBackedIntentStore,
    },
};

const CREATED_AT: u64 = 10;
const NOW: u64 = 100;
const REPLAY_DEADLINE: u64 = 10_000;

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
        replay_deadline_ns: REPLAY_DEADLINE,
    }
}

fn begin_receipt(
    input: &BeginReceiptBackedIntentInput,
    now_ns: u64,
) -> Result<BeginReceiptBackedIntentResult, InternalError> {
    ReceiptBackedIntentOps::begin_or_load(input, now_ns, ReceiptReplayWindowDecision::Open)
}

fn begin_receipt_with_limit(
    input: &BeginReceiptBackedIntentInput,
    now_ns: u64,
    record_limit: u64,
) -> Result<BeginReceiptBackedIntentResult, InternalError> {
    ReceiptBackedIntentOps::begin_or_load_with_limit(
        input,
        now_ns,
        ReceiptReplayWindowDecision::Open,
        record_limit,
    )
}

fn placement_receipt_input(sequence: u64) -> BeginPlacementReceiptBackedIntentInput {
    let identity = PlacementAllocationIdentity::scaling(
        Principal::from_slice(&[7; 29]),
        "workers",
        sequence,
        &CanisterRole::new("worker"),
        None,
    );
    BeginPlacementReceiptBackedIntentInput {
        operation_id: identity.operation_id,
        payload_binding: identity.payload_binding,
        resource_key: identity.resource_key,
        quantity: 1,
        reservation_limit: 20,
    }
}

fn terminal_evidence(decision: TerminalEvidenceDecision, byte: u8) -> TerminalEvidence {
    TerminalEvidence::new(Principal::from_slice(&[1; 29]), decision, [byte; 32])
}

fn application_replay_contradiction_cases(
    canonical: ApplicationReceiptReplayEntryRecord,
    placement_operation_id: OperationId,
) -> [(&'static str, ApplicationReceiptReplayData); 5] {
    let orphan_id = operation_id(251);
    [
        ("missing", ApplicationReceiptReplayData::default()),
        (
            "wrong schema",
            ApplicationReceiptReplayData {
                entries: vec![ApplicationReceiptReplayEntryRecord {
                    record: ApplicationReceiptReplayRecord {
                        schema_version: APPLICATION_RECEIPT_REPLAY_SCHEMA_VERSION + 1,
                        ..canonical.record
                    },
                    ..canonical
                }],
            },
        ),
        (
            "wrong identity",
            ApplicationReceiptReplayData {
                entries: vec![ApplicationReceiptReplayEntryRecord {
                    record: ApplicationReceiptReplayRecord {
                        operation_id: orphan_id,
                        ..canonical.record
                    },
                    ..canonical
                }],
            },
        ),
        (
            "orphan",
            ApplicationReceiptReplayData {
                entries: vec![
                    canonical,
                    ApplicationReceiptReplayEntryRecord {
                        operation_id: orphan_id,
                        record: ApplicationReceiptReplayRecord {
                            schema_version: APPLICATION_RECEIPT_REPLAY_SCHEMA_VERSION,
                            operation_id: orphan_id,
                            replay_deadline_ns: REPLAY_DEADLINE,
                        },
                    },
                ],
            },
        ),
        (
            "Canic-owned",
            ApplicationReceiptReplayData {
                entries: vec![
                    canonical,
                    ApplicationReceiptReplayEntryRecord {
                        operation_id: placement_operation_id,
                        record: ApplicationReceiptReplayRecord {
                            schema_version: APPLICATION_RECEIPT_REPLAY_SCHEMA_VERSION,
                            operation_id: placement_operation_id,
                            replay_deadline_ns: REPLAY_DEADLINE,
                        },
                    },
                ],
            },
        ),
    ]
}

fn application_eligibility_contradiction_cases(
    canonical: ApplicationReceiptEligibilityEntryRecord,
) -> [(&'static str, ApplicationReceiptEligibilityData); 7] {
    let mut wrong_key = canonical;
    wrong_key.key.eligible_at_ns += 1;
    let mut wrong_schema = canonical;
    wrong_schema.record.schema_version += 1;
    let mut wrong_identity = canonical;
    wrong_identity.record.operation_id = operation_id(247);
    let mut wrong_binding = canonical;
    wrong_binding.record.payload_binding = PayloadBinding::new([248; 32]);
    let mut wrong_revision = canonical;
    wrong_revision.record.terminal_revision += 1;
    let extra_operation_id = operation_id(249);
    let extra = ApplicationReceiptEligibilityEntryRecord {
        key: ApplicationReceiptEligibilityKeyRecord {
            eligible_at_ns: canonical.key.eligible_at_ns + 1,
            operation_id: extra_operation_id,
        },
        record: ApplicationReceiptEligibilityRecord {
            operation_id: extra_operation_id,
            ..canonical.record
        },
    };

    [
        ("missing", ApplicationReceiptEligibilityData::default()),
        (
            "wrong key",
            ApplicationReceiptEligibilityData {
                entries: vec![wrong_key],
            },
        ),
        (
            "wrong schema",
            ApplicationReceiptEligibilityData {
                entries: vec![wrong_schema],
            },
        ),
        (
            "wrong identity",
            ApplicationReceiptEligibilityData {
                entries: vec![wrong_identity],
            },
        ),
        (
            "wrong binding",
            ApplicationReceiptEligibilityData {
                entries: vec![wrong_binding],
            },
        ),
        (
            "wrong revision",
            ApplicationReceiptEligibilityData {
                entries: vec![wrong_revision],
            },
        ),
        (
            "extra",
            ApplicationReceiptEligibilityData {
                entries: vec![canonical, extra],
            },
        ),
    ]
}

fn assert_reconciliation_rejects_application_replay_contradictions(
    canonical: ApplicationReceiptReplayEntryRecord,
    placement_operation_id: OperationId,
    sentinel: &PlacementAcknowledgementIndexData,
) {
    for (name, replay) in application_replay_contradiction_cases(canonical, placement_operation_id)
    {
        ReceiptBackedIntentStore::import_application_replay(replay);
        ReceiptBackedIntentOps::reconcile_receipt_indexes()
            .expect_err("contradictory receipt indexes must reject");
        assert_eq!(
            ReceiptBackedIntentStore::export_placement_acknowledgement_index(),
            *sentinel,
            "placement index mutated for {name} application replay contradiction"
        );
    }
}

fn assert_reconciliation_rejects_application_eligibility_contradictions(
    canonical: ApplicationReceiptEligibilityEntryRecord,
    sentinel: &PlacementAcknowledgementIndexData,
) {
    for (name, eligibility) in application_eligibility_contradiction_cases(canonical) {
        ReceiptBackedIntentStore::import_application_eligibility(eligibility);
        ReceiptBackedIntentOps::reconcile_receipt_indexes()
            .expect_err("contradictory terminal eligibility must reject");
        assert_eq!(
            ReceiptBackedIntentStore::export_placement_acknowledgement_index(),
            *sentinel,
            "placement index mutated for {name} terminal eligibility contradiction"
        );
    }
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

    let pending = IntentStoreOps::pending_entries_at(now).expect("pending view");
    assert!(pending.is_empty());

    let expired = IntentStoreOps::list_due_expiry_intents(now, 32).expect("expiry index");
    assert_eq!(expired, vec![intent_id]);

    IntentStoreOps::commit_at(intent_id, now).unwrap_err();

    let record = IntentStore::get_record(intent_id).expect("record should exist");
    assert_eq!(record.state, IntentState::Pending);
    assert!(IntentStore::get_pending(intent_id).is_some());

    assert!(IntentStoreOps::abort_intent_if_pending(intent_id).expect("cleanup abort"));
    assert_eq!(totals(&resource_key), IntentResourceTotalsRecord::default());
    assert!(IntentStore::get_totals(&resource_key).is_none());
    assert_eq!(IntentStoreOps::expiry_index_total_for_tests(), 0);
}

#[test]
fn finite_expiry_index_is_ordered_bounded_and_excludes_ttl_free_intents() {
    reset();
    let ttl_free = IntentId(20);
    let later = IntentId(21);
    let earlier_high_id = IntentId(23);
    let earlier_low_id = IntentId(22);

    IntentStoreOps::try_reserve(ttl_free, key(), 1, CREATED_AT, None, CREATED_AT)
        .expect("TTL-free reservation");
    IntentStoreOps::try_reserve(later, key(), 1, CREATED_AT, Some(9), CREATED_AT)
        .expect("later finite reservation");
    IntentStoreOps::try_reserve(earlier_high_id, key(), 1, CREATED_AT, Some(4), CREATED_AT)
        .expect("earlier high-ID reservation");
    IntentStoreOps::try_reserve(earlier_low_id, key(), 1, CREATED_AT, Some(4), CREATED_AT)
        .expect("earlier low-ID reservation");

    assert_eq!(IntentStoreOps::pending_total().expect("pending total"), 4);
    assert_eq!(IntentStoreOps::expiry_index_total_for_tests(), 3);
    assert_eq!(
        IntentStoreOps::next_expiry_at_secs().expect("next expiry"),
        Some(15)
    );
    assert!(
        IntentStoreOps::list_due_expiry_intents(14, 32)
            .expect("not yet due")
            .is_empty()
    );
    assert_eq!(
        IntentStoreOps::list_due_expiry_intents(15, 1).expect("bounded due page"),
        vec![earlier_low_id]
    );
    assert_eq!(
        IntentStoreOps::list_due_expiry_intents(15, 32).expect("ordered due page"),
        vec![earlier_low_id, earlier_high_id]
    );
}

#[test]
fn terminal_transition_removes_exact_finite_expiry_entry() {
    reset();
    let committed_id = IntentId(24);
    let aborted_id = IntentId(25);
    reserve(committed_id, key(), 1, Some(200)).expect("committed reservation");
    reserve(aborted_id, key(), 1, Some(300)).expect("aborted reservation");
    assert_eq!(IntentStoreOps::expiry_index_total_for_tests(), 2);

    IntentStoreOps::commit_at(committed_id, NOW).expect("commit before due deadline");
    assert_eq!(IntentStoreOps::expiry_index_total_for_tests(), 1);
    IntentStoreOps::abort(aborted_id).expect("abort reservation");
    assert_eq!(IntentStoreOps::expiry_index_total_for_tests(), 0);
    assert_eq!(
        IntentStoreOps::next_expiry_at_secs().expect("empty index"),
        None
    );
}

#[test]
fn finite_expiry_overflow_rejects_without_partial_reservation() {
    reset();
    let intent_id = IntentId(26);
    let error = IntentStoreOps::try_reserve(
        intent_id,
        key(),
        1,
        u64::MAX / NANOS_PER_SECOND,
        Some(1),
        u64::MAX / NANOS_PER_SECOND,
    )
    .expect_err("unrepresentable cleanup deadline must reject");

    assert_eq!(error.log_fields().0, InternalErrorClass::Ops);
    assert!(IntentStore::get_record(intent_id).is_none());
    assert!(IntentStore::get_pending(intent_id).is_none());
    assert_eq!(IntentStoreOps::pending_total().expect("pending total"), 0);
    assert_eq!(IntentStoreOps::expiry_index_total_for_tests(), 0);
}

#[test]
fn expiry_index_rebuild_uses_pending_authority_and_rejects_mismatch() {
    reset();
    let first = IntentId(27);
    let second = IntentId(28);
    reserve(first, key(), 1, Some(20)).expect("first reservation");
    reserve(second, key(), 1, None).expect("TTL-free reservation");
    IntentStore::clear_expiry_index();

    IntentStoreOps::rebuild_expiry_index().expect("rebuild finite expiry index");
    assert_eq!(IntentStoreOps::expiry_index_total_for_tests(), 1);
    assert_eq!(
        IntentStoreOps::next_expiry_at_secs().expect("rebuilt deadline"),
        Some(31)
    );

    let mut pending = IntentStore::get_pending(first).expect("pending row");
    pending.quantity += 1;
    IntentStore::insert_pending(first, pending);
    IntentStoreOps::rebuild_expiry_index()
        .expect_err("contradictory pending row must reject rebuild");
    assert_eq!(IntentStoreOps::expiry_index_total_for_tests(), 1);
}

#[test]
fn cleanup_abort_rejects_missing_totals_without_mutation() {
    reset();
    let resource_key = key();
    let intent_id = IntentId(11);

    reserve(intent_id, resource_key, 3, Some(5)).expect("reserve intent");
    IntentStore::import_totals(IntentTotalsData::default());

    IntentStoreOps::abort_intent_if_pending(intent_id)
        .expect_err("missing totals must fail closed");

    assert_eq!(
        IntentStore::get_record(intent_id)
            .expect("record remains pending")
            .state,
        IntentState::Pending
    );
    assert!(IntentStore::get_pending(intent_id).is_some());
    assert_eq!(meta().pending_total, 1);
    assert_eq!(meta().aborted_total, 0);
}

#[test]
fn unique_pending_intent_lookup_rejects_ambiguous_resource_ownership() {
    reset();
    let resource_key = key();
    let first = IntentId(12);
    let second = IntentId(13);

    reserve(first, resource_key.clone(), 1, None).expect("first reservation");
    assert_eq!(
        IntentStoreOps::unique_pending_intent_id(&resource_key).expect("unique pending intent"),
        Some(first)
    );

    reserve(second, resource_key.clone(), 1, None).expect("second reservation");
    IntentStoreOps::unique_pending_intent_id(&resource_key)
        .expect_err("ambiguous recovery identity must fail closed");
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
    assert!(
        IntentStoreOps::pending_entries_at(NOW)
            .expect("pending view")
            .is_empty()
    );

    let raw = IntentStore::get_totals(&resource_key).expect("raw totals should exist");
    assert_eq!(raw.reserved_qty, u64::MAX, "totals should be unchanged");

    assert_eq!(totals(&resource_key).reserved_qty, u64::MAX);
}

#[test]
fn receipt_backed_begin_replays_without_second_reservation_or_ttl_entry() {
    reset();
    let input = receipt_input(1);

    assert_eq!(
        begin_receipt(&input, 100).expect("create"),
        BeginReceiptBackedIntentResult::Created { revision: 1 }
    );
    let totals_after_create = totals(&input.resource_key);
    let local_meta_after_create = meta();

    assert_eq!(
        begin_receipt(&input, 200).expect("replay"),
        BeginReceiptBackedIntentResult::ExistingPending { revision: 1 }
    );
    assert_eq!(totals(&input.resource_key), totals_after_create);
    assert_eq!(totals_after_create.reserved_qty, input.quantity);
    assert_eq!(totals_after_create.pending_count, 1);
    assert_eq!(meta(), local_meta_after_create);
    assert_eq!(local_meta_after_create.pending_total, 0);
    assert_eq!(IntentStoreOps::expiry_index_total_for_tests(), 0);
    assert_eq!(ReceiptBackedIntentStore::len(), 1);
    assert!(
        ReceiptBackedIntentStore::application_eligibility_reserved_pages_for_tests() >= 1,
        "application admission must reserve terminal-index memory"
    );

    assert_eq!(
        begin_receipt_with_limit(&input, 300, 1).expect("existing at capacity"),
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
fn application_receipt_replay_window_is_checked_only_after_exact_lookup_misses() {
    reset();
    let mut retained = receipt_input(40);
    retained.replay_deadline_ns = 101;
    assert_eq!(
        begin_receipt(&retained, 100).expect("create retained receipt"),
        BeginReceiptBackedIntentResult::Created { revision: 1 }
    );
    assert_eq!(
        ReceiptBackedIntentOps::begin_or_load(&retained, 101, ReceiptReplayWindowDecision::Closed,)
            .expect("retained receipt remains observable at deadline"),
        BeginReceiptBackedIntentResult::ExistingPending { revision: 1 }
    );

    let mut closed = receipt_input(41);
    closed.replay_deadline_ns = 200;
    assert_eq!(
        ReceiptBackedIntentOps::begin_or_load_with_limit(
            &closed,
            200,
            ReceiptReplayWindowDecision::Closed,
            0,
        )
        .expect("closed replay decision"),
        BeginReceiptBackedIntentResult::ReplayWindowClosed {
            replay_deadline_ns: 200,
        }
    );
    assert!(ReceiptBackedIntentStore::get(closed.operation_id).is_none());
    assert!(ReceiptBackedIntentStore::get_application_replay(closed.operation_id).is_none());

    let mut maximum = receipt_input(42);
    maximum.replay_deadline_ns = 300 + MAX_RECEIPT_BACKED_INTENT_REPLAY_WINDOW_NS;
    assert_eq!(
        begin_receipt(&maximum, 300).expect("maximum replay window"),
        BeginReceiptBackedIntentResult::Created { revision: 1 }
    );

    let mut overlong = receipt_input(43);
    overlong.replay_deadline_ns = 400 + MAX_RECEIPT_BACKED_INTENT_REPLAY_WINDOW_NS + 1;
    assert_eq!(
        ReceiptBackedIntentOps::begin_or_load_with_limit(
            &overlong,
            400,
            ReceiptReplayWindowDecision::TooLong {
                remaining_ns: MAX_RECEIPT_BACKED_INTENT_REPLAY_WINDOW_NS + 1,
            },
            0,
        )
        .expect("overlong replay decision"),
        BeginReceiptBackedIntentResult::ReplayWindowTooLong {
            remaining_ns: MAX_RECEIPT_BACKED_INTENT_REPLAY_WINDOW_NS + 1,
            maximum_ns: MAX_RECEIPT_BACKED_INTENT_REPLAY_WINDOW_NS,
        }
    );
    assert!(ReceiptBackedIntentStore::get(overlong.operation_id).is_none());
}

#[test]
fn application_replay_metadata_contradictions_fail_closed() {
    reset();
    let input = receipt_input(44);
    begin_receipt(&input, 100).expect("create application receipt");

    ReceiptBackedIntentStore::import_application_replay(ApplicationReceiptReplayData::default());
    let missing = ReceiptBackedIntentOps::load(input.operation_id)
        .expect_err("application receipt without replay metadata must reject");
    assert_eq!(
        missing.log_fields(),
        (InternalErrorClass::Ops, InternalErrorOrigin::Ops)
    );

    reset();
    let orphan_id = operation_id(45);
    ReceiptBackedIntentStore::import_application_replay(ApplicationReceiptReplayData {
        entries: vec![ApplicationReceiptReplayEntryRecord {
            operation_id: orphan_id,
            record: ApplicationReceiptReplayRecord {
                schema_version: APPLICATION_RECEIPT_REPLAY_SCHEMA_VERSION,
                operation_id: orphan_id,
                replay_deadline_ns: REPLAY_DEADLINE,
            },
        }],
    });
    let orphan = ReceiptBackedIntentOps::load(orphan_id)
        .expect_err("orphan application replay metadata must reject");
    assert_eq!(
        orphan.log_fields(),
        (InternalErrorClass::Ops, InternalErrorOrigin::Ops)
    );
}

#[test]
fn receipt_index_reconciliation_is_ordered_fail_closed_and_non_mutating_on_error() {
    reset();
    let application = receipt_input(46);
    begin_receipt(&application, 100).expect("create application receipt");
    ReceiptBackedIntentOps::settle_if_pending(
        &SettleReceiptBackedIntentInput {
            operation_id: application.operation_id,
            expected_revision: 1,
            expected_payload_binding: application.payload_binding,
            evidence: terminal_evidence(TerminalEvidenceDecision::Committed, 45),
        },
        150,
    )
    .expect("settle application receipt");
    let canonical_application = ReceiptBackedIntentStore::export_application_replay().entries[0];
    let canonical_eligibility = ReceiptBackedIntentStore::export_application_eligibility();

    let placement = placement_receipt_input(46);
    ReceiptBackedIntentOps::begin_placement_or_load(&placement, 100)
        .expect("create placement receipt");
    ReceiptBackedIntentOps::settle_if_pending(
        &SettleReceiptBackedIntentInput {
            operation_id: placement.operation_id,
            expected_revision: 1,
            expected_payload_binding: placement.payload_binding,
            evidence: terminal_evidence(TerminalEvidenceDecision::Committed, 46),
        },
        200,
    )
    .expect("settle placement receipt");

    let sentinel_id = operation_id(250);
    let sentinel_index = PlacementAcknowledgementIndexData {
        entries: vec![PlacementAcknowledgementIndexEntryRecord {
            operation_id: sentinel_id,
            record: PlacementAcknowledgementEntryRecord {
                operation_id: sentinel_id,
            },
        }],
    };
    ReceiptBackedIntentStore::import_placement_acknowledgement_index(sentinel_index.clone());

    assert_reconciliation_rejects_application_replay_contradictions(
        canonical_application,
        placement.operation_id,
        &sentinel_index,
    );

    ReceiptBackedIntentStore::import_application_replay(ApplicationReceiptReplayData {
        entries: vec![canonical_application],
    });
    let canonical_entry = canonical_eligibility.entries[0];
    assert_reconciliation_rejects_application_eligibility_contradictions(
        canonical_entry,
        &sentinel_index,
    );

    ReceiptBackedIntentStore::import_application_eligibility(canonical_eligibility);
    ReceiptBackedIntentOps::reconcile_receipt_indexes().expect("canonical indexes reconcile");
    let reconciled = ReceiptBackedIntentStore::export_placement_acknowledgement_index();
    assert_eq!(reconciled.entries.len(), 1);
    assert_eq!(reconciled.entries[0].operation_id, placement.operation_id);
}

#[test]
fn receipt_backed_begin_conflicts_without_mutation() {
    reset();
    let input = receipt_input(2);
    begin_receipt(&input, 100).expect("create");
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
        BeginReceiptBackedIntentInput {
            replay_deadline_ns: input.replay_deadline_ns + 1,
            ..input.clone()
        },
    ] {
        assert_eq!(
            begin_receipt(&conflict, 200).expect("conflict"),
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

    let begin_error = begin_receipt(&unsupported, 100).expect_err("unsupported binding schema");
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
    begin_receipt(&input, 200).expect("create supported intent");
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
    let target = placement_receipt_input(20);
    let other = placement_receipt_input(21);

    for input in [&target, &other] {
        ReceiptBackedIntentOps::begin_placement_or_load(input, 100).expect("create intent");
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

    let totals_before = totals(&target.resource_key);
    assert_eq!(
        ReceiptBackedIntentOps::remove_terminal(&RemoveTerminalReceiptBackedIntentInput {
            operation_id: target.operation_id,
            expected_revision: 2,
            expected_payload_binding: PayloadBinding::new([99; 32]),
        })
        .expect("binding conflict is typed"),
        RemoveTerminalReceiptBackedIntentResult::BindingConflict
    );
    assert_eq!(
        ReceiptBackedIntentOps::remove_terminal(&RemoveTerminalReceiptBackedIntentInput {
            operation_id: target.operation_id,
            expected_revision: 2,
            expected_payload_binding: target.payload_binding,
        })
        .expect("terminal removal succeeds"),
        RemoveTerminalReceiptBackedIntentResult::Removed
    );
    assert_eq!(totals(&target.resource_key), totals_before);
    assert!(
        ReceiptBackedIntentOps::load(target.operation_id)
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
fn placement_acknowledgement_index_is_exact_bounded_and_rebuilt_from_terminal_authority() {
    reset();
    let first = placement_receipt_input(1);
    let second = placement_receipt_input(2);
    let generic = receipt_input(23);

    for input in [&first, &second] {
        ReceiptBackedIntentOps::begin_placement_or_load(input, 100)
            .expect("create placement intent");
    }
    begin_receipt(&generic, 100).expect("create application intent");

    for (operation_id, payload_binding) in [
        (first.operation_id, first.payload_binding),
        (second.operation_id, second.payload_binding),
        (generic.operation_id, generic.payload_binding),
    ] {
        ReceiptBackedIntentOps::settle_if_pending(
            &SettleReceiptBackedIntentInput {
                operation_id,
                expected_revision: 1,
                expected_payload_binding: payload_binding,
                evidence: terminal_evidence(TerminalEvidenceDecision::Committed, 24),
            },
            200,
        )
        .expect("settle intent");
    }

    assert!(
        ReceiptBackedIntentOps::has_placement_acknowledgements()
            .expect("placement acknowledgement presence")
    );
    let first_page = ReceiptBackedIntentOps::list_placement_acknowledgement_page(None, 1)
        .expect("first bounded page");
    let cursor = first_page.next_cursor.expect("second placement row exists");
    assert_eq!(first_page.intents.len(), 1);
    assert_eq!(first_page.intents[0].operation_id, cursor);
    let second_page = ReceiptBackedIntentOps::list_placement_acknowledgement_page(Some(cursor), 1)
        .expect("second bounded page");
    assert_eq!(second_page.intents.len(), 1);
    assert_eq!(second_page.next_cursor, None);
    assert_ne!(
        first_page.intents[0].operation_id,
        second_page.intents[0].operation_id
    );
    assert!(
        [first.operation_id, second.operation_id].contains(&first_page.intents[0].operation_id)
    );
    assert!(
        [first.operation_id, second.operation_id].contains(&second_page.intents[0].operation_id)
    );

    ReceiptBackedIntentStore::import_placement_acknowledgement_index(
        PlacementAcknowledgementIndexData {
            entries: vec![PlacementAcknowledgementIndexEntryRecord {
                operation_id: operation_id(99),
                record: PlacementAcknowledgementEntryRecord {
                    operation_id: operation_id(99),
                },
            }],
        },
    );
    ReceiptBackedIntentOps::reconcile_receipt_indexes().expect("rebuild from canonical records");
    let rebuilt = ReceiptBackedIntentOps::list_placement_acknowledgement_page(None, 10)
        .expect("rebuilt placement page");
    assert_eq!(rebuilt.intents.len(), 2);
    assert_eq!(
        ReceiptBackedIntentStore::export_placement_acknowledgement_index()
            .entries
            .len(),
        2
    );
    assert!(
        rebuilt
            .intents
            .iter()
            .all(|intent| intent.operation_id != generic.operation_id)
    );
}

#[test]
fn placement_acknowledgement_index_corruption_fails_closed() {
    reset();
    let input = placement_receipt_input(3);
    ReceiptBackedIntentOps::begin_placement_or_load(&input, 100).expect("create placement intent");
    ReceiptBackedIntentOps::settle_if_pending(
        &SettleReceiptBackedIntentInput {
            operation_id: input.operation_id,
            expected_revision: 1,
            expected_payload_binding: input.payload_binding,
            evidence: terminal_evidence(TerminalEvidenceDecision::Committed, 25),
        },
        200,
    )
    .expect("settle placement intent");

    ReceiptBackedIntentStore::import_placement_acknowledgement_index(
        PlacementAcknowledgementIndexData {
            entries: vec![PlacementAcknowledgementIndexEntryRecord {
                operation_id: input.operation_id,
                record: PlacementAcknowledgementEntryRecord {
                    operation_id: operation_id(98),
                },
            }],
        },
    );

    let error = ReceiptBackedIntentOps::list_placement_acknowledgement_page(None, 1)
        .expect_err("mismatched derived identity must reject");
    assert_eq!(
        error.log_fields(),
        (InternalErrorClass::Ops, InternalErrorOrigin::Ops)
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
        begin_receipt(&input, 100).expect("capacity result"),
        BeginReceiptBackedIntentResult::CapacityExceeded {
            current_quantity: 4,
            requested_quantity: 2,
            limit: 5,
        }
    );
    assert_eq!(totals(&resource_key).reserved_qty, 4);

    input.reservation_limit = 10;
    assert_eq!(
        begin_receipt_with_limit(&input, 100, 0).expect("store capacity result"),
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
    begin_receipt(&input, 100).expect("create");
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
        ReceiptBackedIntentOps::begin_or_load(
            &input,
            input.replay_deadline_ns,
            ReceiptReplayWindowDecision::Closed,
        )
        .expect("terminal replay at deadline"),
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
fn application_terminal_eligibility_is_exact_and_overflow_is_non_mutating() {
    reset();
    let input = receipt_input(47);
    begin_receipt(&input, 100).expect("create application receipt");
    let settle = SettleReceiptBackedIntentInput {
        operation_id: input.operation_id,
        expected_revision: 1,
        expected_payload_binding: input.payload_binding,
        evidence: terminal_evidence(TerminalEvidenceDecision::Committed, 47),
    };
    ReceiptBackedIntentOps::settle_if_pending(&settle, 200).expect("settle application receipt");

    let eligibility = ReceiptBackedIntentStore::export_application_eligibility();
    assert_eq!(eligibility.entries.len(), 1);
    assert_eq!(
        eligibility.entries[0],
        ApplicationReceiptEligibilityEntryRecord {
            key: ApplicationReceiptEligibilityKeyRecord {
                eligible_at_ns: 200 + RECEIPT_TERMINAL_OBSERVATION_GRACE_NS,
                operation_id: input.operation_id,
            },
            record: ApplicationReceiptEligibilityRecord {
                schema_version: APPLICATION_RECEIPT_ELIGIBILITY_SCHEMA_VERSION,
                operation_id: input.operation_id,
                payload_binding: input.payload_binding,
                terminal_revision: 2,
            },
        }
    );

    reset();
    let overflow = receipt_input(48);
    begin_receipt(&overflow, 100).expect("create overflow fixture");
    let totals_before = totals(&overflow.resource_key);
    let error = ReceiptBackedIntentOps::settle_if_pending(
        &SettleReceiptBackedIntentInput {
            operation_id: overflow.operation_id,
            expected_revision: 1,
            expected_payload_binding: overflow.payload_binding,
            evidence: terminal_evidence(TerminalEvidenceDecision::RolledBack, 48),
        },
        u64::MAX,
    )
    .expect_err("overflowing terminal retention deadline must reject");
    assert_eq!(
        error.log_fields(),
        (InternalErrorClass::Ops, InternalErrorOrigin::Ops)
    );
    assert_eq!(totals(&overflow.resource_key), totals_before);
    assert!(matches!(
        ReceiptBackedIntentOps::load(overflow.operation_id)
            .expect("pending receipt remains readable")
            .expect("pending receipt remains present")
            .state,
        ReceiptBackedIntentState::Pending
    ));
    assert_eq!(
        ReceiptBackedIntentStore::export_application_eligibility(),
        ApplicationReceiptEligibilityData::default()
    );
}

#[test]
fn receipt_backed_rollback_and_revision_conflict_preserve_exact_totals() {
    reset();
    let input = receipt_input(5);
    begin_receipt(&input, 100).expect("create");
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
    assert!(IntentStore::get_totals(&input.resource_key).is_none());

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
