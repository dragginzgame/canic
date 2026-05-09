//! Mechanical intent store operations (no business policy).

use crate::{
    InternalError,
    ids::{IntentId, IntentResourceKey},
    ops::storage::StorageOpsError,
    storage::stable::intent::{
        INTENT_STORE_SCHEMA_VERSION, IntentPendingEntryRecord, IntentRecord,
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
        now_secs: u64,
    ) -> Result<IntentRecord, InternalError> {
        let meta = ensure_schema()?;

        if let Some(existing) = IntentStore::get_record(intent_id) {
            if is_record_expired(now_secs, &existing) {
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

        IntentStore::with_pending_entries(|pending| {
            for entry in pending.iter() {
                let record = entry.value();
                if record.resource_key.as_ref() != resource_key.as_ref() {
                    continue;
                }
                if is_pending_entry_expired(now, &record) {
                    continue;
                }
                reserved_qty = reserved_qty.saturating_add(record.quantity);
                pending_count = pending_count.saturating_add(1);
            }
        });

        IntentResourceTotalsRecord {
            reserved_qty,
            committed_qty,
            pending_count,
        }
    }

    #[cfg(test)]
    pub fn pending_entries_at(now: u64) -> Vec<(IntentId, IntentPendingEntryRecord)> {
        let mut entries = Vec::new();

        IntentStore::with_pending_entries(|pending| {
            for entry in pending.iter() {
                let record = entry.value();
                if is_pending_entry_expired(now, &record) {
                    continue;
                }
                entries.push((*entry.key(), record));
            }
        });

        entries
    }

    pub fn list_expired_pending_intents(now: u64) -> Vec<IntentId> {
        let mut expired = Vec::new();

        IntentStore::with_pending_entries(|pending| {
            for entry in pending.iter() {
                let record = entry.value();
                if is_pending_entry_expired(now, &record) {
                    expired.push(*entry.key());
                }
            }
        });

        expired
    }

    /// Return the stored pending-intent count without scanning the pending index.
    pub fn pending_total() -> Result<u64, InternalError> {
        Ok(ensure_schema()?.pending_total)
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

#[cfg(test)]
mod tests;
