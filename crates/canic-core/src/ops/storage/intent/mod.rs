//! Module: ops::storage::intent
//!
//! Responsibility: provide mechanical intent store reservation and cleanup operations.
//! Does not own: business policy, workflow orchestration, or endpoint DTOs.
//! Boundary: storage ops facade over stable intent records.

#[cfg(test)]
mod tests;

#[cfg(test)]
use crate::storage::stable::intent::IntentPendingIndexEntryRecord;
use crate::{
    InternalError,
    ids::{IntentId, IntentResourceKey},
    model::{
        intent::{
            BeginReceiptBackedIntentInput, BeginReceiptBackedIntentResult,
            PAYLOAD_BINDING_SCHEMA_VERSION, PayloadBinding, RECEIPT_BACKED_INTENT_SCHEMA_VERSION,
            ReceiptBackedIntent, ReceiptBackedIntentState, RemoveTerminalReceiptBackedIntentInput,
            RemoveTerminalReceiptBackedIntentResult, SettleReceiptBackedIntentInput,
            SettleReceiptBackedIntentResult, TERMINAL_EVIDENCE_SCHEMA_VERSION, TerminalEvidence,
        },
        replay::OperationId,
    },
    ops::storage::StorageOpsError,
    storage::stable::intent::{
        INTENT_STORE_SCHEMA_VERSION, IntentPendingEntryRecord, IntentRecord,
        IntentResourceTotalsRecord, IntentState, IntentStore, IntentStoreMetaRecord,
        ReceiptBackedIntentRecord, ReceiptBackedIntentStore,
    },
    view::intent::ReceiptBackedIntentPage,
};
use std::collections::BTreeMap;
use std::ops::Bound;
use thiserror::Error as ThisError;

pub const RECEIPT_BACKED_INTENT_RECORD_LIMIT: u64 = 100_000;

// -----------------------------------------------------------------------------
// Errors
// -----------------------------------------------------------------------------

///
/// IntentStoreOpsError
///
/// Typed storage failure for mechanical intent store operations.
///

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

    #[error("intent {0} cannot appear twice in one settlement")]
    RepeatedSettlementIntent(IntentId),

    #[error("intent pending index missing for {0}")]
    PendingIndexMissing(IntentId),

    #[error("intent pending index already exists for {0}")]
    PendingIndexExists(IntentId),

    #[error("multiple pending intents exist for resource {0}")]
    MultiplePendingForResource(IntentResourceKey),

    #[error("intent store schema mismatch (expected {expected}, found {found})")]
    SchemaMismatch { expected: u32, found: u32 },

    #[error("intent totals missing for resource {0}")]
    TotalsMissing(IntentResourceKey),

    #[error("receipt-backed intent {0} conflicts with an existing record")]
    ReceiptBackedConflict(OperationId),

    #[error("receipt-backed intent {0} has contradictory terminal evidence")]
    ReceiptBackedEvidenceConflict(OperationId),

    #[error(
        "receipt-backed intent record schema mismatch for {operation_id} (expected {expected}, found {found})"
    )]
    ReceiptBackedRecordSchemaMismatch {
        operation_id: OperationId,
        expected: u32,
        found: u32,
    },

    #[error("unsupported payload binding schema version {found} (expected {expected})")]
    UnsupportedPayloadBindingSchema { expected: u32, found: u32 },

    #[error("unsupported terminal evidence schema version {found} (expected {expected})")]
    UnsupportedTerminalEvidenceSchema { expected: u32, found: u32 },
}

impl From<IntentStoreOpsError> for InternalError {
    fn from(err: IntentStoreOpsError) -> Self {
        StorageOpsError::from(err).into()
    }
}

// -----------------------------------------------------------------------------
// Ops
// -----------------------------------------------------------------------------

///
/// IntentStoreOps
///
/// Storage-ops facade for intent reservation, commit, abort, and cleanup.
///

pub struct IntentStoreOps;

#[derive(Clone, Copy)]
enum IntentSettlement {
    Commit,
    AbortIfPending,
}

impl IntentStoreOps {
    // -------------------------------------------------------------------------
    // Allocation
    // -------------------------------------------------------------------------

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

    // -------------------------------------------------------------------------
    // Commands
    // -------------------------------------------------------------------------

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

    // Commit two intents only after both terminal transitions validate.
    pub(crate) fn commit_pair_at(
        first_intent_id: IntentId,
        second_intent_id: IntentId,
        now: u64,
    ) -> Result<(), InternalError> {
        settle_pair_at(
            (first_intent_id, IntentSettlement::Commit),
            (second_intent_id, IntentSettlement::Commit),
            now,
        )
    }

    // Commit one intent and abort the other only after the pair validates.
    pub(crate) fn commit_and_abort_pending_pair_at(
        commit_intent_id: IntentId,
        abort_intent_id: IntentId,
        now: u64,
    ) -> Result<(), InternalError> {
        settle_pair_at(
            (commit_intent_id, IntentSettlement::Commit),
            (abort_intent_id, IntentSettlement::AbortIfPending),
            now,
        )
    }

    // -------------------------------------------------------------------------
    // Read-only views (TTL authoritative)
    // -------------------------------------------------------------------------

    pub fn load(intent_id: IntentId) -> Result<Option<IntentRecord>, InternalError> {
        ensure_schema()?;
        Ok(IntentStore::get_record(intent_id))
    }

    pub fn totals(resource_key: &IntentResourceKey) -> IntentResourceTotalsRecord {
        IntentStore::get_totals(resource_key).unwrap_or_default()
    }

    /// Reset both intent stores through the ops authority for isolated unit tests.
    #[cfg(test)]
    pub(crate) fn reset_for_tests() {
        IntentStore::reset_for_tests();
        ReceiptBackedIntentStore::reset_for_tests();
    }

    /// Return whether one local intent remains pending without exposing its storage record.
    #[cfg(test)]
    pub(crate) fn is_pending_for_tests(intent_id: IntentId) -> Result<bool, InternalError> {
        Ok(Self::load(intent_id)?.is_some_and(|record| record.state == IntentState::Pending))
    }

    /// Return whether one local intent committed without exposing its storage record.
    #[cfg(test)]
    pub(crate) fn is_committed_for_tests(intent_id: IntentId) -> Result<bool, InternalError> {
        Ok(Self::load(intent_id)?.is_some_and(|record| record.state == IntentState::Committed))
    }

    /// Return whether one local intent aborted without exposing its storage record.
    #[cfg(test)]
    pub(crate) fn is_aborted_for_tests(intent_id: IntentId) -> Result<bool, InternalError> {
        Ok(Self::load(intent_id)?.is_some_and(|record| record.state == IntentState::Aborted))
    }

    #[cfg(test)]
    pub fn pending_entries_at(now: u64) -> Vec<IntentPendingIndexEntryRecord> {
        let mut entries = Vec::new();

        IntentStore::with_pending_entries(|pending| {
            for entry in pending.iter() {
                let record = entry.value();
                if is_pending_entry_expired(now, &record) {
                    continue;
                }
                entries.push(IntentPendingIndexEntryRecord {
                    intent_id: *entry.key(),
                    record,
                });
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

    /// Return the sole pending local intent for one exact resource key.
    ///
    /// Callers that use one resource as a durable recovery identity require
    /// uniqueness. Competing pending records are therefore an invariant error,
    /// not an arbitrary first-match selection.
    pub(crate) fn unique_pending_intent_id(
        resource_key: &IntentResourceKey,
    ) -> Result<Option<IntentId>, InternalError> {
        ensure_schema()?;
        IntentStore::with_pending_entries(|pending| {
            let mut found = None;
            for entry in pending.iter() {
                if entry.value().resource_key != *resource_key {
                    continue;
                }
                if found.replace(*entry.key()).is_some() {
                    return Err(IntentStoreOpsError::MultiplePendingForResource(
                        resource_key.clone(),
                    )
                    .into());
                }
            }
            Ok(found)
        })
    }

    /// Return the stored local TTL-index count without scanning that index.
    pub fn expirable_pending_total() -> Result<u64, InternalError> {
        Ok(ensure_schema()?.pending_total)
    }

    // -------------------------------------------------------------------------
    // Cleanup / repair helpers
    // -------------------------------------------------------------------------

    pub fn abort_intent_if_pending(intent_id: IntentId) -> Result<bool, InternalError> {
        let Some(record) = IntentStore::get_record(intent_id) else {
            return Ok(false);
        };
        if record.state != IntentState::Pending {
            return Ok(false);
        }

        Self::abort(intent_id)?;
        Ok(true)
    }
}

fn settle_pair_at(
    first: (IntentId, IntentSettlement),
    second: (IntentId, IntentSettlement),
    now: u64,
) -> Result<(), InternalError> {
    if first.0 == second.0 {
        return Err(IntentStoreOpsError::RepeatedSettlementIntent(first.0).into());
    }

    let mut meta = ensure_schema()?;
    let mut totals_by_resource = BTreeMap::<IntentResourceKey, IntentResourceTotalsRecord>::new();
    let mut updates = Vec::<IntentRecord>::with_capacity(2);

    for (intent_id, settlement) in [first, second] {
        let Some(record) = IntentStore::get_record(intent_id) else {
            if matches!(settlement, IntentSettlement::AbortIfPending) {
                continue;
            }
            return Err(IntentStoreOpsError::NotFound(intent_id).into());
        };

        let target_state = match (settlement, record.state) {
            (IntentSettlement::Commit, IntentState::Committed)
            | (IntentSettlement::AbortIfPending, IntentState::Committed | IntentState::Aborted) => {
                continue;
            }
            (IntentSettlement::Commit, IntentState::Aborted) => {
                return Err(IntentStoreOpsError::InvalidTransition {
                    id: intent_id,
                    from: IntentState::Aborted,
                    to: IntentState::Committed,
                }
                .into());
            }
            (IntentSettlement::Commit, IntentState::Pending) => IntentState::Committed,
            (IntentSettlement::AbortIfPending, IntentState::Pending) => IntentState::Aborted,
        };

        if target_state == IntentState::Committed && is_record_expired(now, &record) {
            return Err(expired_err(intent_id, &record).into());
        }
        ensure_pending_exists(intent_id)?;

        let current_totals = match totals_by_resource.get(&record.resource_key) {
            Some(totals) => *totals,
            None => IntentStore::get_totals(&record.resource_key)
                .ok_or_else(|| IntentStoreOpsError::TotalsMissing(record.resource_key.clone()))?,
        };
        let committed_qty = if target_state == IntentState::Committed {
            checked_add(
                current_totals.committed_qty,
                record.quantity,
                "committed_qty",
            )?
        } else {
            current_totals.committed_qty
        };
        let new_totals = IntentResourceTotalsRecord {
            reserved_qty: checked_sub(
                current_totals.reserved_qty,
                record.quantity,
                "reserved_qty",
            )?,
            committed_qty,
            pending_count: checked_sub(current_totals.pending_count, 1, "pending_count")?,
        };
        totals_by_resource.insert(record.resource_key.clone(), new_totals);

        meta.pending_total = checked_sub(meta.pending_total, 1, "pending_total")?;
        match target_state {
            IntentState::Committed => {
                meta.committed_total = checked_add(meta.committed_total, 1, "committed_total")?;
            }
            IntentState::Aborted => {
                meta.aborted_total = checked_add(meta.aborted_total, 1, "aborted_total")?;
            }
            IntentState::Pending => unreachable!("settlement target must be terminal"),
        }

        updates.push(IntentRecord {
            state: target_state,
            ..record
        });
    }

    for record in updates {
        IntentStore::remove_pending(record.id);
        IntentStore::insert_record(record);
    }
    for (resource_key, totals) in totals_by_resource {
        IntentStore::set_totals(resource_key, totals);
    }
    IntentStore::set_meta(meta);
    Ok(())
}

/// Exact-key receipt-backed reservation and settlement operations.
pub struct ReceiptBackedIntentOps;

impl ReceiptBackedIntentOps {
    pub fn begin_or_load(
        input: &BeginReceiptBackedIntentInput,
        now_ns: u64,
    ) -> Result<BeginReceiptBackedIntentResult, InternalError> {
        Self::begin_or_load_with_limit(input, now_ns, RECEIPT_BACKED_INTENT_RECORD_LIMIT)
    }

    fn begin_or_load_with_limit(
        input: &BeginReceiptBackedIntentInput,
        now_ns: u64,
        record_limit: u64,
    ) -> Result<BeginReceiptBackedIntentResult, InternalError> {
        validate_payload_binding(input.payload_binding)?;

        if let Some(record) = ReceiptBackedIntentStore::get(input.operation_id) {
            ensure_receipt_backed_record_schema(&record)?;
            if record.payload_binding != input.payload_binding
                || record.resource_key != input.resource_key
                || record.quantity != input.quantity
            {
                return Ok(BeginReceiptBackedIntentResult::BindingConflict);
            }
            return Ok(begin_result_for_existing(&record));
        }

        let record_count = ReceiptBackedIntentStore::len();
        if record_count >= record_limit {
            return Ok(BeginReceiptBackedIntentResult::StoreCapacityReached {
                current_records: record_count,
                limit: record_limit,
            });
        }

        let totals = IntentStore::get_totals(&input.resource_key).unwrap_or_default();
        let current_quantity =
            checked_add(totals.reserved_qty, totals.committed_qty, "accounted_qty")?;
        let next_quantity = checked_add(current_quantity, input.quantity, "accounted_qty")?;
        if next_quantity > input.reservation_limit {
            return Ok(BeginReceiptBackedIntentResult::CapacityExceeded {
                current_quantity,
                requested_quantity: input.quantity,
                limit: input.reservation_limit,
            });
        }

        let new_totals = IntentResourceTotalsRecord {
            reserved_qty: checked_add(totals.reserved_qty, input.quantity, "reserved_qty")?,
            committed_qty: totals.committed_qty,
            pending_count: checked_add(totals.pending_count, 1, "pending_count")?,
        };
        let revision = 1;
        let record = ReceiptBackedIntentRecord {
            schema_version: RECEIPT_BACKED_INTENT_SCHEMA_VERSION,
            operation_id: input.operation_id,
            payload_binding: input.payload_binding,
            resource_key: input.resource_key.clone(),
            quantity: input.quantity,
            state: ReceiptBackedIntentState::Pending,
            revision,
            created_at_ns: now_ns,
            updated_at_ns: now_ns,
        };

        if ReceiptBackedIntentStore::insert(record).is_some() {
            return Err(IntentStoreOpsError::ReceiptBackedConflict(input.operation_id).into());
        }
        IntentStore::set_totals(input.resource_key.clone(), new_totals);

        Ok(BeginReceiptBackedIntentResult::Created { revision })
    }

    pub fn load(operation_id: OperationId) -> Result<Option<ReceiptBackedIntent>, InternalError> {
        let Some(record) = ReceiptBackedIntentStore::get(operation_id) else {
            return Ok(None);
        };
        ensure_receipt_backed_record_schema(&record)?;
        Ok(Some(record.into_intent()))
    }

    pub fn settle_if_pending(
        input: &SettleReceiptBackedIntentInput,
        now_ns: u64,
    ) -> Result<SettleReceiptBackedIntentResult, InternalError> {
        validate_payload_binding(input.expected_payload_binding)?;
        validate_terminal_evidence(input.evidence.schema_version)?;
        let Some(record) = ReceiptBackedIntentStore::get(input.operation_id) else {
            return Ok(SettleReceiptBackedIntentResult::NotFound);
        };
        ensure_receipt_backed_record_schema(&record)?;

        if record.payload_binding != input.expected_payload_binding {
            return Ok(SettleReceiptBackedIntentResult::BindingConflict);
        }

        if record.state != ReceiptBackedIntentState::Pending {
            if terminal_evidence(&record.state) == Some(input.evidence) {
                return Ok(SettleReceiptBackedIntentResult::AlreadySettled {
                    revision: record.revision,
                    state: record.state,
                });
            }
            return Err(
                IntentStoreOpsError::ReceiptBackedEvidenceConflict(input.operation_id).into(),
            );
        }

        if record.revision != input.expected_revision {
            return Ok(SettleReceiptBackedIntentResult::RevisionConflict {
                actual_revision: record.revision,
            });
        }

        let totals = IntentStore::get_totals(&record.resource_key)
            .ok_or_else(|| IntentStoreOpsError::TotalsMissing(record.resource_key.clone()))?;
        let committed_qty = match input.evidence.decision {
            crate::model::intent::TerminalEvidenceDecision::Committed => {
                checked_add(totals.committed_qty, record.quantity, "committed_qty")?
            }
            crate::model::intent::TerminalEvidenceDecision::RolledBack => totals.committed_qty,
        };
        let new_totals = IntentResourceTotalsRecord {
            reserved_qty: checked_sub(totals.reserved_qty, record.quantity, "reserved_qty")?,
            committed_qty,
            pending_count: checked_sub(totals.pending_count, 1, "pending_count")?,
        };
        let revision = checked_add(record.revision, 1, "revision")?;
        let state = match input.evidence.decision {
            crate::model::intent::TerminalEvidenceDecision::Committed => {
                ReceiptBackedIntentState::Committed {
                    evidence: input.evidence,
                }
            }
            crate::model::intent::TerminalEvidenceDecision::RolledBack => {
                ReceiptBackedIntentState::RolledBack {
                    evidence: input.evidence,
                }
            }
        };
        let updated = ReceiptBackedIntentRecord {
            state,
            revision,
            updated_at_ns: now_ns,
            ..record
        };

        IntentStore::set_totals(updated.resource_key.clone(), new_totals);
        ReceiptBackedIntentStore::insert(updated);

        Ok(SettleReceiptBackedIntentResult::Settled { revision, state })
    }

    /// Return one bounded page of receipt-backed intents after an exact map cursor.
    pub(crate) fn list_page(
        cursor: Option<OperationId>,
        limit: usize,
    ) -> Result<ReceiptBackedIntentPage, InternalError> {
        ReceiptBackedIntentStore::with_records(|records| {
            let mut entries = match cursor {
                Some(cursor) => records.range((Bound::Excluded(cursor), Bound::Unbounded)),
                None => records.iter(),
            };
            let mut intents = Vec::with_capacity(limit);
            for entry in entries.by_ref().take(limit) {
                let record = entry.value();
                ensure_receipt_backed_record_schema(&record)?;
                intents.push(record.into_intent());
            }
            let next_cursor = entries
                .next()
                .and_then(|_| intents.last().map(|intent| intent.operation_id));
            Ok::<ReceiptBackedIntentPage, IntentStoreOpsError>(ReceiptBackedIntentPage {
                intents,
                next_cursor,
            })
        })
        .map_err(InternalError::from)
    }

    /// Delete one exact terminal record without changing its already-settled totals.
    pub(crate) fn remove_terminal(
        input: &RemoveTerminalReceiptBackedIntentInput,
    ) -> Result<RemoveTerminalReceiptBackedIntentResult, InternalError> {
        validate_payload_binding(input.expected_payload_binding)?;
        let Some(record) = ReceiptBackedIntentStore::get(input.operation_id) else {
            return Ok(RemoveTerminalReceiptBackedIntentResult::NotFound);
        };
        ensure_receipt_backed_record_schema(&record)?;

        if record.payload_binding != input.expected_payload_binding {
            return Ok(RemoveTerminalReceiptBackedIntentResult::BindingConflict);
        }
        if record.revision != input.expected_revision {
            return Ok(RemoveTerminalReceiptBackedIntentResult::RevisionConflict {
                actual_revision: record.revision,
            });
        }
        if matches!(record.state, ReceiptBackedIntentState::Pending) {
            return Ok(RemoveTerminalReceiptBackedIntentResult::NotTerminal);
        }

        let removed = ReceiptBackedIntentStore::remove(input.operation_id).ok_or(
            IntentStoreOpsError::ReceiptBackedConflict(input.operation_id),
        )?;
        if removed != record {
            return Err(IntentStoreOpsError::ReceiptBackedConflict(input.operation_id).into());
        }

        Ok(RemoveTerminalReceiptBackedIntentResult::Removed)
    }
}

// -----------------------------------------------------------------------------
// Internal helpers
// -----------------------------------------------------------------------------

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

const fn ensure_receipt_backed_record_schema(
    record: &ReceiptBackedIntentRecord,
) -> Result<(), IntentStoreOpsError> {
    if record.schema_version == RECEIPT_BACKED_INTENT_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(IntentStoreOpsError::ReceiptBackedRecordSchemaMismatch {
            operation_id: record.operation_id,
            expected: RECEIPT_BACKED_INTENT_SCHEMA_VERSION,
            found: record.schema_version,
        })
    }
}

const fn validate_payload_binding(binding: PayloadBinding) -> Result<(), IntentStoreOpsError> {
    if binding.schema_version == PAYLOAD_BINDING_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(IntentStoreOpsError::UnsupportedPayloadBindingSchema {
            expected: PAYLOAD_BINDING_SCHEMA_VERSION,
            found: binding.schema_version,
        })
    }
}

const fn validate_terminal_evidence(schema_version: u32) -> Result<(), IntentStoreOpsError> {
    if schema_version == TERMINAL_EVIDENCE_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(IntentStoreOpsError::UnsupportedTerminalEvidenceSchema {
            expected: TERMINAL_EVIDENCE_SCHEMA_VERSION,
            found: schema_version,
        })
    }
}

const fn begin_result_for_existing(
    record: &ReceiptBackedIntentRecord,
) -> BeginReceiptBackedIntentResult {
    match record.state {
        ReceiptBackedIntentState::Pending => BeginReceiptBackedIntentResult::ExistingPending {
            revision: record.revision,
        },
        ReceiptBackedIntentState::Committed { evidence } => {
            BeginReceiptBackedIntentResult::ExistingCommitted {
                revision: record.revision,
                evidence,
            }
        }
        ReceiptBackedIntentState::RolledBack { evidence } => {
            BeginReceiptBackedIntentResult::ExistingRolledBack {
                revision: record.revision,
                evidence,
            }
        }
    }
}

const fn terminal_evidence(state: &ReceiptBackedIntentState) -> Option<TerminalEvidence> {
    match state {
        ReceiptBackedIntentState::Pending => None,
        ReceiptBackedIntentState::Committed { evidence }
        | ReceiptBackedIntentState::RolledBack { evidence } => Some(*evidence),
    }
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
