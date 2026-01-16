//! Mechanical intent store operations (no business policy).

use crate::{
    InternalError,
    ops::storage::StorageOpsError,
    storage::stable::intent::{INTENT_STORE_SCHEMA_VERSION, IntentStore},
};
use thiserror::Error as ThisError;

pub use crate::storage::stable::intent::{
    IntentId, IntentPendingEntry, IntentRecord, IntentResourceKey, IntentResourceTotals,
    IntentState, IntentStoreMeta,
};

///
/// IntentStoreOpsError
///

#[derive(Debug, ThisError)]
pub enum IntentStoreOpsError {
    #[error("intent store schema mismatch (expected {expected}, found {found})")]
    SchemaMismatch { expected: u32, found: u32 },

    #[error("intent {0} not found")]
    NotFound(IntentId),

    #[error("intent {0} conflicts with existing record")]
    Conflict(IntentId),

    #[error("intent {id} invalid transition {from:?} -> {to:?}")]
    InvalidTransition {
        id: IntentId,
        from: IntentState,
        to: IntentState,
    },

    #[error("intent pending index missing for {0}")]
    PendingIndexMissing(IntentId),

    #[error("intent pending index already exists for {0}")]
    PendingIndexExists(IntentId),

    #[error("intent totals missing for resource {0}")]
    TotalsMissing(IntentResourceKey),

    #[error("intent aggregate underflow for {field}: current={current}, delta={delta}")]
    AggregateUnderflow {
        field: &'static str,
        current: u64,
        delta: u64,
    },

    #[error("intent aggregate overflow for {field}: current={current}, delta={delta}")]
    AggregateOverflow {
        field: &'static str,
        current: u64,
        delta: u64,
    },

    #[error("intent id space exhausted")]
    IdOverflow,
}

impl From<IntentStoreOpsError> for InternalError {
    fn from(err: IntentStoreOpsError) -> Self {
        StorageOpsError::from(err).into()
    }
}

///
/// IntentStoreOps
///

pub struct IntentStoreOps;

impl IntentStoreOps {
    // -------------------------------------------------------------
    // Allocation
    // -------------------------------------------------------------

    pub fn allocate_intent_id() -> Result<IntentId, InternalError> {
        let mut meta = ensure_schema()?;
        let id = meta.next_intent_id;
        let next = meta
            .next_intent_id
            .0
            .checked_add(1)
            .ok_or(IntentStoreOpsError::IdOverflow)?;
        meta.next_intent_id = IntentId(next);
        IntentStore::set_meta(meta);

        Ok(id)
    }

    // -------------------------------------------------------------
    // Commands
    // -------------------------------------------------------------

    pub fn try_reserve(
        intent_id: IntentId,
        resource_key: IntentResourceKey,
        quantity: u64,
        created_at: u64,
        ttl_secs: Option<u64>,
    ) -> Result<IntentRecord, InternalError> {
        let meta = ensure_schema()?;

        if let Some(existing) = IntentStore::get_record(intent_id) {
            ensure_compatible(&existing, &resource_key, quantity, ttl_secs)?;
            return Ok(existing);
        }

        if IntentStore::get_pending(intent_id).is_some() {
            return Err(IntentStoreOpsError::PendingIndexExists(intent_id).into());
        }

        let totals = IntentStore::get_totals(&resource_key).unwrap_or_default();
        let new_totals = IntentResourceTotals {
            reserved_qty: checked_add(totals.reserved_qty, quantity, "reserved_qty")?,
            committed_qty: totals.committed_qty,
            pending_count: checked_add(totals.pending_count, 1, "pending_count")?,
        };

        let mut meta = meta;
        meta.pending_total = checked_add(meta.pending_total, 1, "pending_total")?;

        let record = IntentRecord {
            id: intent_id,
            resource_key: resource_key.clone(),
            quantity,
            state: IntentState::Pending,
            created_at,
            ttl_secs,
        };

        let pending = IntentPendingEntry {
            resource_key: resource_key.clone(),
            quantity,
            created_at,
            ttl_secs,
        };

        let old = IntentStore::insert_record(record.clone());
        if old.is_some() {
            return Err(IntentStoreOpsError::Conflict(intent_id).into());
        }

        IntentStore::insert_pending(intent_id, pending);
        IntentStore::set_totals(resource_key, new_totals);
        IntentStore::set_meta(meta);

        Ok(record)
    }

    pub fn commit(intent_id: IntentId) -> Result<IntentRecord, InternalError> {
        let meta = ensure_schema()?;
        let record =
            IntentStore::get_record(intent_id).ok_or(IntentStoreOpsError::NotFound(intent_id))?;

        match record.state {
            IntentState::Committed => return Ok(record),
            IntentState::Aborted => {
                return Err(IntentStoreOpsError::InvalidTransition {
                    id: intent_id,
                    from: IntentState::Aborted,
                    to: IntentState::Committed,
                }
                .into());
            }
            IntentState::Pending => {}
        }

        if IntentStore::get_pending(intent_id).is_none() {
            return Err(IntentStoreOpsError::PendingIndexMissing(intent_id).into());
        }

        let totals = IntentStore::get_totals(&record.resource_key)
            .ok_or_else(|| IntentStoreOpsError::TotalsMissing(record.resource_key.clone()))?;
        let new_totals = IntentResourceTotals {
            reserved_qty: checked_sub(totals.reserved_qty, record.quantity, "reserved_qty")?,
            committed_qty: checked_add(totals.committed_qty, record.quantity, "committed_qty")?,
            pending_count: checked_sub(totals.pending_count, 1, "pending_count")?,
        };

        let mut meta = meta;
        meta.pending_total = checked_sub(meta.pending_total, 1, "pending_total")?;
        meta.committed_total = checked_add(meta.committed_total, 1, "committed_total")?;

        let updated = IntentRecord {
            state: IntentState::Committed,
            ..record.clone()
        };

        IntentStore::remove_pending(intent_id);
        IntentStore::set_totals(record.resource_key, new_totals);
        IntentStore::set_meta(meta);
        IntentStore::insert_record(updated.clone());

        Ok(updated)
    }

    pub fn abort(intent_id: IntentId) -> Result<IntentRecord, InternalError> {
        let meta = ensure_schema()?;
        let record =
            IntentStore::get_record(intent_id).ok_or(IntentStoreOpsError::NotFound(intent_id))?;

        match record.state {
            IntentState::Aborted => return Ok(record),
            IntentState::Committed => {
                return Err(IntentStoreOpsError::InvalidTransition {
                    id: intent_id,
                    from: IntentState::Committed,
                    to: IntentState::Aborted,
                }
                .into());
            }
            IntentState::Pending => {}
        }

        if IntentStore::get_pending(intent_id).is_none() {
            return Err(IntentStoreOpsError::PendingIndexMissing(intent_id).into());
        }

        let totals = IntentStore::get_totals(&record.resource_key)
            .ok_or_else(|| IntentStoreOpsError::TotalsMissing(record.resource_key.clone()))?;
        let new_totals = IntentResourceTotals {
            reserved_qty: checked_sub(totals.reserved_qty, record.quantity, "reserved_qty")?,
            committed_qty: totals.committed_qty,
            pending_count: checked_sub(totals.pending_count, 1, "pending_count")?,
        };

        let mut meta = meta;
        meta.pending_total = checked_sub(meta.pending_total, 1, "pending_total")?;
        meta.aborted_total = checked_add(meta.aborted_total, 1, "aborted_total")?;

        let updated = IntentRecord {
            state: IntentState::Aborted,
            ..record.clone()
        };

        IntentStore::remove_pending(intent_id);
        IntentStore::set_totals(record.resource_key, new_totals);
        IntentStore::set_meta(meta);
        IntentStore::insert_record(updated.clone());

        Ok(updated)
    }

    // -------------------------------------------------------------
    // Data (read-only)
    // -------------------------------------------------------------

    #[must_use]
    #[allow(dead_code)]
    pub fn get(intent_id: IntentId) -> Option<IntentRecord> {
        IntentStore::get_record(intent_id)
    }

    #[allow(dead_code)]
    pub fn meta() -> Result<IntentStoreMeta, InternalError> {
        ensure_schema().map_err(InternalError::from)
    }

    #[must_use]
    #[allow(dead_code)]
    pub fn totals(resource_key: &IntentResourceKey) -> Option<IntentResourceTotals> {
        IntentStore::get_totals(resource_key)
    }

    #[must_use]
    #[allow(dead_code)]
    pub fn pending_entries() -> Vec<(IntentId, IntentPendingEntry)> {
        IntentStore::pending_entries()
    }

    #[must_use]
    #[expect(dead_code)]
    pub fn pending_snapshot() -> Vec<IntentRecord> {
        IntentStore::pending_entries()
            .into_iter()
            .filter_map(|(id, _)| IntentStore::get_record(id))
            .collect()
    }
}

//
// ─────────────────────────────────────────────────────────────
// Internal helpers (mechanical)
// ─────────────────────────────────────────────────────────────
//

fn ensure_schema() -> Result<IntentStoreMeta, IntentStoreOpsError> {
    let meta = IntentStore::meta();
    if meta.schema_version != INTENT_STORE_SCHEMA_VERSION {
        return Err(IntentStoreOpsError::SchemaMismatch {
            expected: INTENT_STORE_SCHEMA_VERSION,
            found: meta.schema_version,
        });
    }

    Ok(meta)
}

fn ensure_compatible(
    existing: &IntentRecord,
    resource_key: &IntentResourceKey,
    quantity: u64,
    ttl_secs: Option<u64>,
) -> Result<(), IntentStoreOpsError> {
    if &existing.resource_key != resource_key || existing.quantity != quantity {
        return Err(IntentStoreOpsError::Conflict(existing.id));
    }

    if existing.ttl_secs != ttl_secs {
        return Err(IntentStoreOpsError::Conflict(existing.id));
    }

    Ok(())
}

fn checked_add(current: u64, delta: u64, field: &'static str) -> Result<u64, IntentStoreOpsError> {
    current
        .checked_add(delta)
        .ok_or(IntentStoreOpsError::AggregateOverflow {
            field,
            current,
            delta,
        })
}

fn checked_sub(current: u64, delta: u64, field: &'static str) -> Result<u64, IntentStoreOpsError> {
    current
        .checked_sub(delta)
        .ok_or(IntentStoreOpsError::AggregateUnderflow {
            field,
            current,
            delta,
        })
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::stable::intent::IntentStore;

    const CREATED_AT: u64 = 10;

    #[derive(Clone, Copy, Debug)]
    enum Op {
        Commit,
        Abort,
    }

    fn reset_store() {
        IntentStore::reset_for_tests();
    }

    fn key() -> IntentResourceKey {
        IntentResourceKey::new("resource")
    }

    fn reserve(
        intent_id: IntentId,
        resource_key: IntentResourceKey,
        quantity: u64,
    ) -> IntentRecord {
        IntentStoreOps::try_reserve(intent_id, resource_key, quantity, CREATED_AT, None)
            .expect("reserve should succeed")
    }

    fn totals_for(key: &IntentResourceKey) -> IntentResourceTotals {
        IntentStoreOps::totals(key).unwrap_or_default()
    }

    fn meta() -> IntentStoreMeta {
        IntentStoreOps::meta().expect("meta should be readable")
    }

    fn apply(op: Op, intent_id: IntentId) -> Result<IntentRecord, InternalError> {
        match op {
            Op::Commit => IntentStoreOps::commit(intent_id),
            Op::Abort => IntentStoreOps::abort(intent_id),
        }
    }

    #[test]
    fn idempotent_ops_do_not_double_count() {
        struct Case {
            name: &'static str,
            op: Option<Op>,
            expected_state: IntentState,
            expected_totals: IntentResourceTotals,
            expected_pending_total: u64,
            expected_committed_total: u64,
            expected_aborted_total: u64,
        }

        let cases = [
            Case {
                name: "try_reserve",
                op: None,
                expected_state: IntentState::Pending,
                expected_totals: IntentResourceTotals {
                    reserved_qty: 5,
                    committed_qty: 0,
                    pending_count: 1,
                },
                expected_pending_total: 1,
                expected_committed_total: 0,
                expected_aborted_total: 0,
            },
            Case {
                name: "commit",
                op: Some(Op::Commit),
                expected_state: IntentState::Committed,
                expected_totals: IntentResourceTotals {
                    reserved_qty: 0,
                    committed_qty: 5,
                    pending_count: 0,
                },
                expected_pending_total: 0,
                expected_committed_total: 1,
                expected_aborted_total: 0,
            },
            Case {
                name: "abort",
                op: Some(Op::Abort),
                expected_state: IntentState::Aborted,
                expected_totals: IntentResourceTotals {
                    reserved_qty: 0,
                    committed_qty: 0,
                    pending_count: 0,
                },
                expected_pending_total: 0,
                expected_committed_total: 0,
                expected_aborted_total: 1,
            },
        ];

        for case in cases {
            reset_store();
            let resource_key = key();
            let intent_id = IntentId(1);

            let first = reserve(intent_id, resource_key.clone(), 5);

            let first = match case.op {
                Some(op) => apply(op, intent_id).expect("operation should succeed"),
                None => first,
            };

            assert_eq!(first.state, case.expected_state, "case {}", case.name);

            let totals_after_first = totals_for(&resource_key);
            let meta_after_first = meta();

            let second = match case.op {
                Some(op) => apply(op, intent_id).expect("idempotent op should succeed"),
                None => reserve(intent_id, resource_key.clone(), 5),
            };

            assert_eq!(second.state, case.expected_state, "case {}", case.name);
            assert_eq!(
                totals_for(&resource_key),
                totals_after_first,
                "case {}",
                case.name
            );
            assert_eq!(meta(), meta_after_first, "case {}", case.name);
            assert_eq!(
                totals_for(&resource_key),
                case.expected_totals,
                "case {}",
                case.name
            );

            let meta = meta();
            assert_eq!(
                meta.pending_total, case.expected_pending_total,
                "case {}",
                case.name
            );
            assert_eq!(
                meta.committed_total, case.expected_committed_total,
                "case {}",
                case.name
            );
            assert_eq!(
                meta.aborted_total, case.expected_aborted_total,
                "case {}",
                case.name
            );
        }
    }

    #[test]
    fn pending_transitions_are_allowed() {
        struct Case {
            name: &'static str,
            op: Op,
            expected_state: IntentState,
        }

        let cases = [
            Case {
                name: "pending_to_committed",
                op: Op::Commit,
                expected_state: IntentState::Committed,
            },
            Case {
                name: "pending_to_aborted",
                op: Op::Abort,
                expected_state: IntentState::Aborted,
            },
        ];

        for case in cases {
            reset_store();
            let resource_key = key();
            let intent_id = IntentId(1);
            reserve(intent_id, resource_key, 3);

            let record = apply(case.op, intent_id).expect("transition should succeed");
            assert_eq!(record.state, case.expected_state, "case {}", case.name);
        }
    }

    #[test]
    fn rejects_invalid_transitions() {
        struct Case {
            name: &'static str,
            first: Op,
            second: Op,
        }

        let cases = [
            Case {
                name: "committed_to_aborted",
                first: Op::Commit,
                second: Op::Abort,
            },
            Case {
                name: "aborted_to_committed",
                first: Op::Abort,
                second: Op::Commit,
            },
        ];

        for case in cases {
            reset_store();
            let resource_key = key();
            let intent_id = IntentId(1);
            reserve(intent_id, resource_key, 4);
            apply(case.first, intent_id).expect("first transition should succeed");

            let err = apply(case.second, intent_id).expect_err("invalid transition should fail");
            assert!(
                err.to_string().contains("invalid transition"),
                "case {}: {err}",
                case.name
            );
        }
    }

    #[test]
    fn prevents_aggregate_underflow() {
        reset_store();
        let resource_key = key();
        let intent_id = IntentId(42);

        let record = IntentRecord {
            id: intent_id,
            resource_key: resource_key.clone(),
            quantity: 9,
            state: IntentState::Pending,
            created_at: CREATED_AT,
            ttl_secs: None,
        };
        let pending = IntentPendingEntry {
            resource_key: resource_key.clone(),
            quantity: 9,
            created_at: CREATED_AT,
            ttl_secs: None,
        };

        IntentStore::insert_record(record);
        IntentStore::insert_pending(intent_id, pending);
        IntentStore::set_totals(
            resource_key,
            IntentResourceTotals {
                reserved_qty: 0,
                committed_qty: 0,
                pending_count: 1,
            },
        );

        let err = IntentStoreOps::commit(intent_id).expect_err("underflow should fail");
        assert!(err.to_string().contains("reserved_qty"), "{err}");
    }

    #[test]
    fn prevents_aggregate_overflow() {
        reset_store();
        let resource_key = key();
        let intent_id = IntentId(7);

        IntentStore::set_totals(
            resource_key.clone(),
            IntentResourceTotals {
                reserved_qty: u64::MAX,
                committed_qty: 0,
                pending_count: 0,
            },
        );

        let err = IntentStoreOps::try_reserve(intent_id, resource_key.clone(), 1, CREATED_AT, None)
            .expect_err("overflow should fail");
        assert!(err.to_string().contains("reserved_qty"), "{err}");
        assert!(IntentStoreOps::get(intent_id).is_none());
        assert!(IntentStoreOps::pending_entries().is_empty());
        assert_eq!(
            totals_for(&resource_key).reserved_qty,
            u64::MAX,
            "totals should be unchanged"
        );
    }

    #[test]
    fn workflow_reserve_commit_increments_once() {
        reset_store();
        let resource_key = key();
        let intent_id = IntentId(100);

        let _ = reserve(intent_id, resource_key.clone(), 2);
        let _ = IntentStoreOps::commit(intent_id).expect("commit should succeed");

        let totals = totals_for(&resource_key);
        assert_eq!(totals.reserved_qty, 0);
        assert_eq!(totals.committed_qty, 2);
        assert_eq!(totals.pending_count, 0);

        let meta = meta();
        assert_eq!(meta.pending_total, 0);
        assert_eq!(meta.committed_total, 1);
        assert_eq!(meta.aborted_total, 0);
        assert!(IntentStoreOps::pending_entries().is_empty());
    }

    #[test]
    fn workflow_reserve_abort_cleans_up() {
        reset_store();
        let resource_key = key();
        let intent_id = IntentId(101);

        let _ = reserve(intent_id, resource_key.clone(), 3);
        let _ = IntentStoreOps::abort(intent_id).expect("abort should succeed");

        let totals = totals_for(&resource_key);
        assert_eq!(totals.reserved_qty, 0);
        assert_eq!(totals.committed_qty, 0);
        assert_eq!(totals.pending_count, 0);

        let meta = meta();
        assert_eq!(meta.pending_total, 0);
        assert_eq!(meta.committed_total, 0);
        assert_eq!(meta.aborted_total, 1);
        assert!(IntentStoreOps::pending_entries().is_empty());
    }

    #[test]
    fn workflow_duplicate_reserve_is_idempotent() {
        reset_store();
        let resource_key = key();
        let intent_id = IntentId(102);

        let first = reserve(intent_id, resource_key.clone(), 1);
        let totals_after_first = totals_for(&resource_key);
        let meta_after_first = meta();

        let second = reserve(intent_id, resource_key.clone(), 1);
        assert_eq!(first.state, IntentState::Pending);
        assert_eq!(second.state, IntentState::Pending);
        assert_eq!(totals_for(&resource_key), totals_after_first);
        assert_eq!(meta(), meta_after_first);
    }
}
