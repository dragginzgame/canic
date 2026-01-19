//! Mechanical intent store operations (no business policy).

use crate::{
    InternalError,
    ids::IntentResourceKey,
    ops::{ic::IcOps, storage::StorageOpsError},
    storage::stable::intent::{
        INTENT_STORE_SCHEMA_VERSION, IntentId, IntentPendingEntryRecord, IntentRecord,
        IntentResourceTotalsRecord, IntentState, IntentStore, IntentStoreMetaRecord,
    },
};
use thiserror::Error as ThisError;

/// -----------------------------------------------------------------------------
/// Errors
/// -----------------------------------------------------------------------------

#[derive(Debug, ThisError)]
pub enum IntentStoreOpsError {
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

    #[error("intent {0} conflicts with existing record")]
    Conflict(IntentId),

    #[error("intent {id} expired at {expires_at:?}")]
    Expired {
        id: IntentId,
        expires_at: Option<u64>,
    },

    #[error("intent id space exhausted")]
    IdOverflow,

    #[error("intent {id} invalid transition {from:?} -> {to:?}")]
    InvalidTransition {
        id: IntentId,
        from: IntentState,
        to: IntentState,
    },

    #[error("intent {0} not found")]
    NotFound(IntentId),

    #[error("intent pending index missing for {0}")]
    PendingIndexMissing(IntentId),

    #[error("intent pending index already exists for {0}")]
    PendingIndexExists(IntentId),

    #[error("intent store schema mismatch (expected {expected}, found {found})")]
    SchemaMismatch { expected: u32, found: u32 },

    #[error("intent totals missing for resource {0}")]
    TotalsMissing(IntentResourceKey),
}

impl From<IntentStoreOpsError> for InternalError {
    fn from(err: IntentStoreOpsError) -> Self {
        StorageOpsError::from(err).into()
    }
}

/// -----------------------------------------------------------------------------
/// Ops
/// -----------------------------------------------------------------------------

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
        let now = IcOps::now_secs();

        if let Some(existing) = IntentStore::get_record(intent_id) {
            if is_record_expired(now, &existing) {
                return Err(expired_err(intent_id, &existing).into());
            }
            ensure_compatible(&existing, &resource_key, quantity, ttl_secs)?;
            return Ok(existing);
        }

        if IntentStore::get_pending(intent_id).is_some() {
            return Err(IntentStoreOpsError::PendingIndexExists(intent_id).into());
        }

        let totals = IntentStore::get_totals(&resource_key).unwrap_or_default();
        let new_totals = IntentResourceTotalsRecord {
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

        let pending = IntentPendingEntryRecord {
            resource_key: resource_key.clone(),
            quantity,
            created_at,
            ttl_secs,
        };

        if IntentStore::insert_record(record.clone()).is_some() {
            return Err(IntentStoreOpsError::Conflict(intent_id).into());
        }

        IntentStore::insert_pending(intent_id, pending);
        IntentStore::set_totals(resource_key, new_totals);
        IntentStore::set_meta(meta);

        Ok(record)
    }

    pub fn commit_at(intent_id: IntentId, now: u64) -> Result<IntentRecord, InternalError> {
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

        if is_record_expired(now, &record) {
            return Err(expired_err(intent_id, &record).into());
        }

        ensure_pending_exists(intent_id)?;

        let totals = IntentStore::get_totals(&record.resource_key)
            .ok_or_else(|| IntentStoreOpsError::TotalsMissing(record.resource_key.clone()))?;

        let new_totals = IntentResourceTotalsRecord {
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

        remove_pending_and_apply(
            intent_id,
            record.resource_key,
            new_totals,
            meta,
            updated.clone(),
        );
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

        ensure_pending_exists(intent_id)?;

        let totals = IntentStore::get_totals(&record.resource_key)
            .ok_or_else(|| IntentStoreOpsError::TotalsMissing(record.resource_key.clone()))?;

        let new_totals = IntentResourceTotalsRecord {
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

        remove_pending_and_apply(
            intent_id,
            record.resource_key,
            new_totals,
            meta,
            updated.clone(),
        );
        Ok(updated)
    }

    // -------------------------------------------------------------
    // Read-only views (TTL authoritative)
    // -------------------------------------------------------------

    pub fn totals_at(resource_key: &IntentResourceKey, now: u64) -> IntentResourceTotalsRecord {
        let committed_qty = IntentStore::get_totals(resource_key).map_or(0, |t| t.committed_qty);

        let mut reserved_qty: u64 = 0;
        let mut pending_count: u64 = 0;

        for (_, entry) in IntentStore::pending_entries() {
            if entry.resource_key.as_ref() != resource_key.as_ref() {
                continue;
            }
            if is_pending_entry_expired(now, &entry) {
                continue;
            }
            reserved_qty = reserved_qty.saturating_add(entry.quantity);
            pending_count = pending_count.saturating_add(1);
        }

        IntentResourceTotalsRecord {
            reserved_qty,
            committed_qty,
            pending_count,
        }
    }

    #[allow(dead_code)]
    pub fn pending_entries_at(now: u64) -> Vec<(IntentId, IntentPendingEntryRecord)> {
        IntentStore::pending_entries()
            .into_iter()
            .filter(|(_, e)| !is_pending_entry_expired(now, e))
            .collect()
    }

    pub fn list_expired_pending_intents(now: u64) -> Vec<IntentId> {
        IntentStore::pending_entries()
            .into_iter()
            .filter(|(_, e)| is_pending_entry_expired(now, e))
            .map(|(id, _)| id)
            .collect()
    }

    // -------------------------------------------------------------
    // Cleanup / repair helpers
    // -------------------------------------------------------------

    pub fn abort_intent_if_pending(intent_id: IntentId) -> Result<bool, InternalError> {
        let Some(record) = IntentStore::get_record(intent_id) else {
            return Ok(false);
        };
        if record.state != IntentState::Pending {
            return Ok(false);
        }

        let mut meta = ensure_schema()?;
        let totals = IntentStore::get_totals(&record.resource_key).unwrap_or_default();

        let new_totals = IntentResourceTotalsRecord {
            reserved_qty: totals.reserved_qty.saturating_sub(record.quantity),
            committed_qty: totals.committed_qty,
            pending_count: totals.pending_count.saturating_sub(1),
        };

        meta.pending_total = meta.pending_total.saturating_sub(1);
        meta.aborted_total = meta.aborted_total.saturating_add(1);

        let updated = IntentRecord {
            state: IntentState::Aborted,
            ..record
        };

        remove_pending_and_apply(
            intent_id,
            updated.resource_key.clone(),
            new_totals,
            meta,
            updated,
        );
        Ok(true)
    }
}

/// -----------------------------------------------------------------------------
/// Internal helpers (mechanical)
/// -----------------------------------------------------------------------------

fn ensure_schema() -> Result<IntentStoreMetaRecord, IntentStoreOpsError> {
    let meta = IntentStore::meta();
    if meta.schema_version != INTENT_STORE_SCHEMA_VERSION {
        return Err(IntentStoreOpsError::SchemaMismatch {
            expected: INTENT_STORE_SCHEMA_VERSION,
            found: meta.schema_version,
        });
    }
    Ok(meta)
}

fn ensure_pending_exists(intent_id: IntentId) -> Result<(), IntentStoreOpsError> {
    if IntentStore::get_pending(intent_id).is_none() {
        Err(IntentStoreOpsError::PendingIndexMissing(intent_id))
    } else {
        Ok(())
    }
}

fn remove_pending_and_apply(
    intent_id: IntentId,
    resource_key: IntentResourceKey,
    totals: IntentResourceTotalsRecord,
    meta: IntentStoreMetaRecord,
    record: IntentRecord,
) {
    IntentStore::remove_pending(intent_id);
    IntentStore::set_totals(resource_key, totals);
    IntentStore::set_meta(meta);
    IntentStore::insert_record(record);
}

fn expired_err(id: IntentId, r: &IntentRecord) -> IntentStoreOpsError {
    IntentStoreOpsError::Expired {
        id,
        expires_at: expires_at(r.created_at, r.ttl_secs),
    }
}

fn ensure_compatible(
    existing: &IntentRecord,
    key: &IntentResourceKey,
    quantity: u64,
    ttl_secs: Option<u64>,
) -> Result<(), IntentStoreOpsError> {
    if &existing.resource_key != key
        || existing.quantity != quantity
        || existing.ttl_secs != ttl_secs
    {
        Err(IntentStoreOpsError::Conflict(existing.id))
    } else {
        Ok(())
    }
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

fn expires_at(created_at: u64, ttl_secs: Option<u64>) -> Option<u64> {
    ttl_secs.and_then(|ttl| created_at.checked_add(ttl))
}

fn is_expired(now: u64, created_at: u64, ttl_secs: Option<u64>) -> bool {
    match ttl_secs.and_then(|t| created_at.checked_add(t)) {
        Some(exp) => now > exp,
        None => false,
    }
}

fn is_record_expired(now: u64, record: &IntentRecord) -> bool {
    is_expired(now, record.created_at, record.ttl_secs)
}

fn is_pending_entry_expired(now: u64, entry: &IntentPendingEntryRecord) -> bool {
    is_expired(now, entry.created_at, entry.ttl_secs)
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
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
        IntentStoreOps::try_reserve(intent_id, resource_key, quantity, CREATED_AT, ttl_secs)
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

    // -------------------------------------------------------------------------
    // Core invariants
    // -------------------------------------------------------------------------

    #[test]
    fn idempotent_ops_do_not_double_count() {
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

        let err = IntentStoreOps::abort(intent_id).unwrap_err();
        assert!(err.to_string().contains("invalid transition"));
    }

    // -------------------------------------------------------------------------
    // TTL semantics
    // -------------------------------------------------------------------------

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

        let err = IntentStoreOps::commit_at(intent_id, now).unwrap_err();
        assert!(err.to_string().contains("expired"));
    }

    // -------------------------------------------------------------------------
    // Arithmetic safety
    // -------------------------------------------------------------------------

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
            resource_key,
            IntentResourceTotalsRecord {
                reserved_qty: 0,
                committed_qty: 0,
                pending_count: 1,
            },
        );

        let err = IntentStoreOps::commit_at(intent_id, NOW).unwrap_err();
        assert!(err.to_string().contains("reserved_qty"));
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

        let err =
            reserve(intent_id, resource_key.clone(), 1, None).expect_err("overflow should fail");
        assert!(err.to_string().contains("reserved_qty"), "{err}");

        // sanity: no partial insertions
        assert!(IntentStore::get_record(intent_id).is_none());
        assert!(IntentStoreOps::pending_entries_at(NOW).is_empty());

        let raw = IntentStore::get_totals(&resource_key).expect("raw totals should exist");
        assert_eq!(raw.reserved_qty, u64::MAX, "totals should be unchanged");

        let logical = totals_at(&resource_key, NOW);
        assert_eq!(logical.reserved_qty, 0);
    }
}
