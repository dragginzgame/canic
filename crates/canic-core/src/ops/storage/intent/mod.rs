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
            BeginPlacementReceiptBackedIntentInput, BeginReceiptBackedIntentInput,
            BeginReceiptBackedIntentResult, MAX_RECEIPT_BACKED_INTENT_REPLAY_WINDOW_NS,
            PAYLOAD_BINDING_SCHEMA_VERSION, PayloadBinding, RECEIPT_BACKED_INTENT_SCHEMA_VERSION,
            RECEIPT_TERMINAL_OBSERVATION_GRACE_NS, ReceiptBackedIntent, ReceiptBackedIntentState,
            ReceiptReplayWindowDecision, RemoveTerminalReceiptBackedIntentInput,
            RemoveTerminalReceiptBackedIntentResult, SettleReceiptBackedIntentInput,
            SettleReceiptBackedIntentResult, TERMINAL_EVIDENCE_SCHEMA_VERSION, TerminalEvidence,
            is_canic_owned_intent_resource_key, receipt_terminal_eligible_at,
        },
        placement::allocation::is_placement_resource_key,
        replay::OperationId,
    },
    ops::storage::StorageOpsError,
    storage::stable::intent::{
        APPLICATION_RECEIPT_ELIGIBILITY_SCHEMA_VERSION, APPLICATION_RECEIPT_REPLAY_SCHEMA_VERSION,
        ApplicationReceiptEligibilityKeyRecord, ApplicationReceiptEligibilityRecord,
        ApplicationReceiptReplayRecord, INTENT_STORE_SCHEMA_VERSION, IntentExpiryEntryRecord,
        IntentExpiryKeyRecord, IntentPendingEntryRecord, IntentRecord, IntentResourceTotalsRecord,
        IntentState, IntentStore, IntentStoreMetaRecord, PlacementAcknowledgementEntryRecord,
        ReceiptBackedIntentRecord, ReceiptBackedIntentStore,
    },
    view::intent::{
        ApplicationReceiptCapacityView, ApplicationReceiptReclamationBatch,
        PlacementAcknowledgementPage,
    },
};
use std::collections::BTreeMap;
use std::ops::Bound;
use thiserror::Error as ThisError;

pub const RECEIPT_BACKED_INTENT_RECORD_LIMIT: u64 = 1_000;
const NANOS_PER_SECOND: u64 = 1_000_000_000;

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

    #[error("intent pending index does not match primary record for {0}")]
    PendingIndexMismatch(IntentId),

    #[error("intent pending total mismatch: metadata={metadata}, index={index}")]
    PendingTotalMismatch { metadata: u64, index: u64 },

    #[error("intent {id} finite expiry overflows: created_at={created_at}, ttl_secs={ttl_secs}")]
    ExpiryDeadlineOverflow {
        id: IntentId,
        created_at: u64,
        ttl_secs: u64,
    },

    #[error("intent expiry index already exists for {id} at {due_at_secs}")]
    ExpiryIndexExists { id: IntentId, due_at_secs: u64 },

    #[error("intent expiry index missing for {id} at {due_at_secs}")]
    ExpiryIndexMissing { id: IntentId, due_at_secs: u64 },

    #[error("intent expiry index value mismatch at {due_at_secs}: key={key_id}, value={value_id}")]
    ExpiryIndexValueMismatch {
        due_at_secs: u64,
        key_id: IntentId,
        value_id: IntentId,
    },

    #[error(
        "intent expiry index key mismatch for {id}: expected={expected_due_at_secs}, found={found_due_at_secs}"
    )]
    ExpiryIndexKeyMismatch {
        id: IntentId,
        expected_due_at_secs: u64,
        found_due_at_secs: u64,
    },

    #[error("TTL-free intent {0} appears in the finite-expiry index")]
    TtlFreeIntentInExpiryIndex(IntentId),

    #[error("multiple pending intents exist for resource {0}")]
    MultiplePendingForResource(IntentResourceKey),

    #[error("placement acknowledgement index already exists for {0}")]
    PlacementAcknowledgementIndexExists(OperationId),

    #[error("placement acknowledgement index missing for {0}")]
    PlacementAcknowledgementIndexMissing(OperationId),

    #[error("placement acknowledgement index key {key} contains operation {value}")]
    PlacementAcknowledgementIndexValueMismatch {
        key: OperationId,
        value: OperationId,
    },

    #[error("unexpected placement acknowledgement index entry for {0}")]
    PlacementAcknowledgementUnexpectedIndex(OperationId),

    #[error("placement acknowledgement index references missing intent {0}")]
    PlacementAcknowledgementPrimaryMissing(OperationId),

    #[error("placement acknowledgement intent {0} is not a terminal placement record")]
    PlacementAcknowledgementPrimaryMismatch(OperationId),

    #[error("intent store schema mismatch (expected {expected}, found {found})")]
    SchemaMismatch { expected: u32, found: u32 },

    #[error("intent totals missing for resource {0}")]
    TotalsMissing(IntentResourceKey),

    #[error("receipt-backed intent {0} conflicts with an existing record")]
    ReceiptBackedConflict(OperationId),

    #[error("receipt-backed intent {0} has contradictory terminal evidence")]
    ReceiptBackedEvidenceConflict(OperationId),

    #[error("receipt-backed intent record limit exceeded: records={records}, limit={limit}")]
    ReceiptBackedRecordLimitExceeded { records: u64, limit: u64 },

    #[error("application receipt replay metadata missing for {0}")]
    ApplicationReceiptReplayMissing(OperationId),

    #[error("application receipt replay metadata exists without primary receipt {0}")]
    ApplicationReceiptReplayPrimaryMissing(OperationId),

    #[error("application receipt replay metadata is not permitted for Canic-owned receipt {0}")]
    ApplicationReceiptReplayUnexpected(OperationId),

    #[error("application receipt replay metadata key {key} contains operation {value}")]
    ApplicationReceiptReplayIdentityMismatch {
        key: OperationId,
        value: OperationId,
    },

    #[error(
        "application receipt replay schema mismatch for {operation_id} (expected {expected}, found {found})"
    )]
    ApplicationReceiptReplaySchemaMismatch {
        operation_id: OperationId,
        expected: u32,
        found: u32,
    },

    #[error("application receipt terminal eligibility missing for {0}")]
    ApplicationReceiptEligibilityMissing(OperationId),

    #[error("application receipt terminal eligibility already exists for {0}")]
    ApplicationReceiptEligibilityExists(OperationId),

    #[error("application receipt terminal eligibility exists without a terminal primary {0}")]
    ApplicationReceiptEligibilityPrimaryMismatch(OperationId),

    #[error("application receipt terminal eligibility key {key} contains operation {value}")]
    ApplicationReceiptEligibilityIdentityMismatch {
        key: OperationId,
        value: OperationId,
    },

    #[error(
        "application receipt terminal eligibility schema mismatch for {operation_id} (expected {expected}, found {found})"
    )]
    ApplicationReceiptEligibilitySchemaMismatch {
        operation_id: OperationId,
        expected: u32,
        found: u32,
    },

    #[error("application receipt terminal eligibility binding mismatch for {0}")]
    ApplicationReceiptEligibilityBindingMismatch(OperationId),

    #[error(
        "application receipt terminal eligibility revision mismatch for {operation_id} (expected {expected}, found {found})"
    )]
    ApplicationReceiptEligibilityRevisionMismatch {
        operation_id: OperationId,
        expected: u64,
        found: u64,
    },

    #[error(
        "application receipt terminal eligibility overflows for {operation_id}: terminal_timestamp_ns={terminal_timestamp_ns}, observation_grace_ns={observation_grace_ns}"
    )]
    ApplicationReceiptEligibilityOverflow {
        operation_id: OperationId,
        terminal_timestamp_ns: u64,
        observation_grace_ns: u64,
    },

    #[error("application receipt terminal-capacity reservation count overflow")]
    ApplicationReceiptEligibilityReservationOverflow,

    #[error("application receipt reclamation count exceeds u64")]
    ApplicationReceiptReclamationCountOverflow,

    #[error(
        "application receipt terminal-capacity reservation unavailable for {required_records} records"
    )]
    ApplicationReceiptEligibilityCapacityUnavailable { required_records: u64 },

    #[error("receipt-backed intent {operation_id} has incompatible {owner} resource ownership")]
    ReceiptBackedOwnershipMismatch {
        operation_id: OperationId,
        owner: &'static str,
    },

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
        let expiry_key = expiry_key(intent_id, created_at, ttl_secs)?;

        if let Some(existing) = IntentStore::get_record(intent_id) {
            if is_record_expired(now_secs, &existing)? {
                return Err(expired_err(intent_id, &existing).into());
            }
            ensure_compatible(&existing, &resource_key, quantity, ttl_secs)?;
            if existing.state == IntentState::Pending {
                ensure_pending_indexes(&existing)?;
            }
            return Ok(existing);
        }

        if IntentStore::get_pending(intent_id).is_some() {
            return Err(IntentStoreOpsError::PendingIndexExists(intent_id).into());
        }
        if let Some(expiry_key) = expiry_key
            && IntentStore::get_expiry(expiry_key).is_some()
        {
            return Err(IntentStoreOpsError::ExpiryIndexExists {
                id: intent_id,
                due_at_secs: expiry_key.due_at_secs,
            }
            .into());
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
        if let Some(expiry_key) = expiry_key {
            let previous =
                IntentStore::insert_expiry(expiry_key, IntentExpiryEntryRecord { intent_id });
            assert!(
                previous.is_none(),
                "validated intent expiry index insertion replaced an entry"
            );
        }
        persist_resource_totals(resource_key, new_totals);
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

        if is_record_expired(now, &record)? {
            return Err(expired_err(intent_id, &record).into());
        }

        ensure_pending_indexes(&record)?;

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

        remove_pending_and_apply(record.resource_key, new_totals, meta, updated.clone());
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

        ensure_pending_indexes(&record)?;

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

        remove_pending_and_apply(record.resource_key, new_totals, meta, updated.clone());
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

    /// Return the exact cleanup deadline for a validated finite local intent.
    pub fn cleanup_due_at_secs(intent_id: IntentId) -> Result<Option<u64>, InternalError> {
        let record =
            IntentStore::get_record(intent_id).ok_or(IntentStoreOpsError::NotFound(intent_id))?;
        if record.state != IntentState::Pending {
            return Ok(None);
        }
        ensure_pending_indexes(&record)?;
        expiry_key(record.id, record.created_at, record.ttl_secs)
            .map(|key| key.map(|key| key.due_at_secs))
            .map_err(InternalError::from)
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
    pub(crate) fn clear_totals_for_tests() {
        IntentStore::import_totals(crate::storage::stable::intent::IntentTotalsData::default());
    }

    #[cfg(test)]
    pub fn pending_entries_at(
        now: u64,
    ) -> Result<Vec<IntentPendingIndexEntryRecord>, InternalError> {
        let mut entries = Vec::new();

        IntentStore::with_pending_entries(|pending| -> Result<(), IntentStoreOpsError> {
            for entry in pending.iter() {
                let record = entry.value();
                if is_pending_entry_expired(*entry.key(), now, &record)? {
                    continue;
                }
                entries.push(IntentPendingIndexEntryRecord {
                    intent_id: *entry.key(),
                    record,
                });
            }
            Ok(())
        })?;

        Ok(entries)
    }

    /// Return at most `limit` expiry-ordered local intents whose cleanup deadline is due.
    pub fn list_due_expiry_intents(
        now_secs: u64,
        limit: usize,
    ) -> Result<Vec<IntentId>, InternalError> {
        IntentStore::with_expiry_entries(|index| -> Result<Vec<IntentId>, IntentStoreOpsError> {
            let mut due = Vec::with_capacity(limit);
            for entry in index.iter() {
                let key = *entry.key();
                if key.due_at_secs > now_secs || due.len() == limit {
                    break;
                }
                validate_expiry_entry(key, entry.value())?;
                due.push(key.intent_id);
            }
            Ok::<Vec<IntentId>, IntentStoreOpsError>(due)
        })
        .map_err(InternalError::from)
    }

    /// Return the earliest validated finite local-intent cleanup deadline.
    pub fn next_expiry_at_secs() -> Result<Option<u64>, InternalError> {
        IntentStore::with_expiry_entries(|index| -> Result<Option<u64>, IntentStoreOpsError> {
            let Some(entry) = index.iter().next() else {
                return Ok(None);
            };
            let key = *entry.key();
            validate_expiry_entry(key, entry.value())?;
            Ok(Some(key.due_at_secs))
        })
        .map_err(InternalError::from)
    }

    /// Rebuild the derived finite-expiry index from canonical pending intent state.
    pub fn rebuild_expiry_index() -> Result<(), InternalError> {
        let meta = ensure_schema()?;
        let entries = IntentStore::with_pending_entries(|pending| {
            let index_total = pending.len();
            if index_total != meta.pending_total {
                return Err(IntentStoreOpsError::PendingTotalMismatch {
                    metadata: meta.pending_total,
                    index: index_total,
                });
            }

            let mut entries = Vec::new();
            for pending_entry in pending.iter() {
                let intent_id = *pending_entry.key();
                let pending_record = pending_entry.value();
                let record = IntentStore::get_record(intent_id)
                    .ok_or(IntentStoreOpsError::NotFound(intent_id))?;
                ensure_pending_record_matches(&record, &pending_record)?;
                if let Some(key) = expiry_key(intent_id, record.created_at, record.ttl_secs)? {
                    entries.push(key);
                }
            }
            Ok::<Vec<IntentExpiryKeyRecord>, IntentStoreOpsError>(entries)
        })?;

        IntentStore::clear_expiry_index();
        for key in entries {
            let previous = IntentStore::insert_expiry(
                key,
                IntentExpiryEntryRecord {
                    intent_id: key.intent_id,
                },
            );
            assert!(
                previous.is_none(),
                "rebuilt intent expiry index contains a duplicate key"
            );
        }
        Ok(())
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

    /// Return the stored total for all pending local intents.
    #[cfg(test)]
    pub(crate) fn pending_total() -> Result<u64, InternalError> {
        Ok(ensure_schema()?.pending_total)
    }

    #[cfg(test)]
    pub(crate) fn expiry_index_total_for_tests() -> u64 {
        IntentStore::with_expiry_entries(crate::cdk::structures::btreemap::BTreeMap::len)
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

        if target_state == IntentState::Committed && is_record_expired(now, &record)? {
            return Err(expired_err(intent_id, &record).into());
        }
        ensure_pending_indexes(&record)?;

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
        remove_pending_indexes(&record);
        IntentStore::insert_record(record);
    }
    for (resource_key, totals) in totals_by_resource {
        persist_resource_totals(resource_key, totals);
    }
    IntentStore::set_meta(meta);
    Ok(())
}

/// Exact-key receipt-backed reservation and settlement operations.
pub struct ReceiptBackedIntentOps;

#[derive(Clone, Copy)]
enum ReceiptAdmissionOwner {
    Application {
        replay_deadline_ns: u64,
        replay_window: ReceiptReplayWindowDecision,
    },
    Placement,
}

struct ReceiptAdmission<'a> {
    operation_id: OperationId,
    payload_binding: PayloadBinding,
    resource_key: &'a IntentResourceKey,
    quantity: u64,
    reservation_limit: u64,
    owner: ReceiptAdmissionOwner,
}

impl ReceiptBackedIntentOps {
    pub fn begin_or_load(
        input: &BeginReceiptBackedIntentInput,
        now_ns: u64,
        replay_window: ReceiptReplayWindowDecision,
    ) -> Result<BeginReceiptBackedIntentResult, InternalError> {
        Self::begin_or_load_with_limit(
            input,
            now_ns,
            replay_window,
            RECEIPT_BACKED_INTENT_RECORD_LIMIT,
        )
    }

    pub(crate) fn begin_placement_or_load(
        input: &BeginPlacementReceiptBackedIntentInput,
        now_ns: u64,
    ) -> Result<BeginReceiptBackedIntentResult, InternalError> {
        Self::begin_or_load_authorized(
            ReceiptAdmission {
                operation_id: input.operation_id,
                payload_binding: input.payload_binding,
                resource_key: &input.resource_key,
                quantity: input.quantity,
                reservation_limit: input.reservation_limit,
                owner: ReceiptAdmissionOwner::Placement,
            },
            now_ns,
            RECEIPT_BACKED_INTENT_RECORD_LIMIT,
        )
    }

    fn begin_or_load_with_limit(
        input: &BeginReceiptBackedIntentInput,
        now_ns: u64,
        replay_window: ReceiptReplayWindowDecision,
        record_limit: u64,
    ) -> Result<BeginReceiptBackedIntentResult, InternalError> {
        Self::begin_or_load_authorized(
            ReceiptAdmission {
                operation_id: input.operation_id,
                payload_binding: input.payload_binding,
                resource_key: &input.resource_key,
                quantity: input.quantity,
                reservation_limit: input.reservation_limit,
                owner: ReceiptAdmissionOwner::Application {
                    replay_deadline_ns: input.replay_deadline_ns,
                    replay_window,
                },
            },
            now_ns,
            record_limit,
        )
    }

    fn begin_or_load_authorized(
        input: ReceiptAdmission<'_>,
        now_ns: u64,
        record_limit: u64,
    ) -> Result<BeginReceiptBackedIntentResult, InternalError> {
        validate_payload_binding(input.payload_binding)?;
        validate_receipt_admission_owner(&input)?;

        if let Some(result) = retained_begin_result(&input)? {
            return Ok(result);
        }

        validate_absent_receipt_indexes(input.operation_id)?;
        if let Some(result) = replay_window_rejection(input.owner) {
            return Ok(result);
        }
        create_receipt(input, now_ns, record_limit)
    }

    pub fn load(operation_id: OperationId) -> Result<Option<ReceiptBackedIntent>, InternalError> {
        let Some(record) = ReceiptBackedIntentStore::get(operation_id) else {
            if ReceiptBackedIntentStore::get_application_replay(operation_id).is_some() {
                return Err(IntentStoreOpsError::ApplicationReceiptReplayPrimaryMissing(
                    operation_id,
                )
                .into());
            }
            if ReceiptBackedIntentStore::get_placement_acknowledgement(operation_id).is_some() {
                return Err(IntentStoreOpsError::PlacementAcknowledgementPrimaryMissing(
                    operation_id,
                )
                .into());
            }
            return Ok(None);
        };
        ensure_receipt_backed_record_schema(&record)?;
        validate_application_replay_for_record(&record)?;
        validate_placement_acknowledgement_index_for_record(&record)?;
        Ok(Some(record.into_intent()))
    }

    /// Return the maintained application receipt capacity and retention projection.
    pub(crate) fn application_capacity() -> Result<ApplicationReceiptCapacityView, InternalError> {
        let total_records = ReceiptBackedIntentStore::len();
        let application_records = ReceiptBackedIntentStore::application_replay_len();
        let canic_owned_records = checked_sub(
            total_records,
            application_records,
            "Canic-owned receipt records",
        )?;
        let terminal_records = ReceiptBackedIntentStore::with_application_eligibility(
            crate::cdk::structures::btreemap::BTreeMap::len,
        );
        let pending_records = checked_sub(
            application_records,
            terminal_records,
            "application pending records",
        )?;
        let next_eligibility_at_ns = validate_first_application_eligibility()?;
        let remaining_record_headroom = RECEIPT_BACKED_INTENT_RECORD_LIMIT
            .checked_sub(total_records)
            .ok_or(IntentStoreOpsError::ReceiptBackedRecordLimitExceeded {
                records: total_records,
                limit: RECEIPT_BACKED_INTENT_RECORD_LIMIT,
            })?;

        Ok(ApplicationReceiptCapacityView {
            total_records,
            application_records,
            canic_owned_records,
            pending_records,
            terminal_records,
            record_limit: RECEIPT_BACKED_INTENT_RECORD_LIMIT,
            remaining_record_headroom,
            reserved_terminal_slots: application_records,
            reserved_terminal_pages:
                ReceiptBackedIntentStore::application_eligibility_reserved_pages(),
            next_eligibility_at_ns,
        })
    }

    /// Remove one validated bounded prefix of due terminal application receipts.
    pub(crate) fn reclaim_due_application_receipts(
        now_ns: u64,
        limit: usize,
    ) -> Result<ApplicationReceiptReclamationBatch, InternalError> {
        let candidates = collect_due_application_reclamation_candidates(now_ns, limit)?;
        let removed_records = u64::try_from(candidates.len())
            .map_err(|_| IntentStoreOpsError::ApplicationReceiptReclamationCountOverflow)?;

        for candidate in candidates {
            let removed = ReceiptBackedIntentStore::remove_application_eligibility(candidate.key);
            assert_eq!(
                removed,
                Some(candidate.eligibility),
                "validated application eligibility changed before reclamation"
            );
            let removed =
                ReceiptBackedIntentStore::remove_application_replay(candidate.primary.operation_id);
            assert_eq!(
                removed,
                Some(candidate.replay),
                "validated application replay metadata changed before reclamation"
            );
            let removed = ReceiptBackedIntentStore::remove(candidate.primary.operation_id);
            assert_eq!(
                removed,
                Some(candidate.primary),
                "validated application receipt changed before reclamation"
            );
        }

        Ok(ApplicationReceiptReclamationBatch {
            removed_records,
            next_eligibility_at_ns: validate_first_application_eligibility()?,
        })
    }

    pub fn settle_if_pending(
        input: &SettleReceiptBackedIntentInput,
        now_ns: u64,
    ) -> Result<SettleReceiptBackedIntentResult, InternalError> {
        validate_payload_binding(input.expected_payload_binding)?;
        validate_terminal_evidence(input.evidence.schema_version)?;
        let Some(record) = ReceiptBackedIntentStore::get(input.operation_id) else {
            if ReceiptBackedIntentStore::get_application_replay(input.operation_id).is_some() {
                return Err(IntentStoreOpsError::ApplicationReceiptReplayPrimaryMissing(
                    input.operation_id,
                )
                .into());
            }
            return Ok(SettleReceiptBackedIntentResult::NotFound);
        };
        ensure_receipt_backed_record_schema(&record)?;
        let replay = validate_application_replay_for_record(&record)?;
        validate_placement_acknowledgement_index_for_record(&record)?;

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

        insert_application_settlement_eligibility(&updated, replay)?;
        persist_resource_totals(updated.resource_key.clone(), new_totals);
        ReceiptBackedIntentStore::insert(updated.clone());
        if is_placement_resource_key(&updated.resource_key) {
            let acknowledgement = PlacementAcknowledgementEntryRecord {
                operation_id: updated.operation_id,
            };
            if ReceiptBackedIntentStore::insert_placement_acknowledgement(acknowledgement).is_some()
            {
                return Err(IntentStoreOpsError::PlacementAcknowledgementIndexExists(
                    updated.operation_id,
                )
                .into());
            }
        }

        Ok(SettleReceiptBackedIntentResult::Settled { revision, state })
    }

    /// Return whether exact terminal placement acknowledgement work exists.
    pub(crate) fn has_placement_acknowledgements() -> Result<bool, InternalError> {
        ReceiptBackedIntentStore::with_placement_acknowledgements(|index| {
            let Some(entry) = index.iter().next() else {
                return Ok(false);
            };
            validate_placement_acknowledgement_entry(*entry.key(), entry.value())?;
            Ok::<bool, IntentStoreOpsError>(true)
        })
        .map_err(InternalError::from)
    }

    /// Return one bounded placement-only page after an exact index cursor.
    pub(crate) fn list_placement_acknowledgement_page(
        cursor: Option<OperationId>,
        limit: usize,
    ) -> Result<PlacementAcknowledgementPage, InternalError> {
        ReceiptBackedIntentStore::with_placement_acknowledgements(|index| {
            let mut entries = match cursor {
                Some(cursor) => index.range((Bound::Excluded(cursor), Bound::Unbounded)),
                None => index.iter(),
            };
            let mut intents = Vec::with_capacity(limit);
            for entry in entries.by_ref().take(limit) {
                let record = validate_placement_acknowledgement_entry(*entry.key(), entry.value())?;
                intents.push(record.into_intent());
            }
            let next_cursor = entries
                .next()
                .and_then(|_| intents.last().map(|intent| intent.operation_id));
            Ok::<PlacementAcknowledgementPage, IntentStoreOpsError>(PlacementAcknowledgementPage {
                intents,
                next_cursor,
            })
        })
        .map_err(InternalError::from)
    }

    /// Validate receipt adjunct ownership and rebuild the placement acknowledgement index.
    pub fn reconcile_receipt_indexes() -> Result<(), InternalError> {
        validate_receipt_record_limit()?;
        let (operation_ids, expected_eligibility) = collect_expected_receipt_indexes()?;
        validate_application_eligibility_index(expected_eligibility)?;

        ReceiptBackedIntentStore::clear_placement_acknowledgement_index();
        for operation_id in operation_ids {
            let record = PlacementAcknowledgementEntryRecord { operation_id };
            let previous = ReceiptBackedIntentStore::insert_placement_acknowledgement(record);
            assert!(
                previous.is_none(),
                "rebuilt placement acknowledgement index contains a duplicate operation"
            );
        }
        Ok(())
    }

    /// Delete one exact terminal record without changing its already-settled totals.
    pub(crate) fn remove_terminal(
        input: &RemoveTerminalReceiptBackedIntentInput,
    ) -> Result<RemoveTerminalReceiptBackedIntentResult, InternalError> {
        validate_payload_binding(input.expected_payload_binding)?;
        let Some(record) = ReceiptBackedIntentStore::get(input.operation_id) else {
            if ReceiptBackedIntentStore::get_application_replay(input.operation_id).is_some() {
                return Err(IntentStoreOpsError::ApplicationReceiptReplayPrimaryMissing(
                    input.operation_id,
                )
                .into());
            }
            return Ok(RemoveTerminalReceiptBackedIntentResult::NotFound);
        };
        ensure_receipt_backed_record_schema(&record)?;
        validate_application_replay_for_record(&record)?;
        validate_placement_acknowledgement_index_for_record(&record)?;

        if !is_placement_resource_key(&record.resource_key) {
            return Err(IntentStoreOpsError::ReceiptBackedOwnershipMismatch {
                operation_id: record.operation_id,
                owner: "placement cleanup",
            }
            .into());
        }

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
        let removed =
            ReceiptBackedIntentStore::remove_placement_acknowledgement(input.operation_id);
        if removed
            != Some(PlacementAcknowledgementEntryRecord {
                operation_id: input.operation_id,
            })
        {
            return Err(IntentStoreOpsError::PlacementAcknowledgementIndexMissing(
                input.operation_id,
            )
            .into());
        }

        Ok(RemoveTerminalReceiptBackedIntentResult::Removed)
    }
}

// -----------------------------------------------------------------------------
// Internal helpers
// -----------------------------------------------------------------------------

type ExpectedApplicationEligibility = (
    ApplicationReceiptEligibilityKeyRecord,
    ApplicationReceiptEligibilityRecord,
);

#[derive(Clone, Debug, Eq, PartialEq)]
struct ApplicationReceiptReclamationCandidate {
    key: ApplicationReceiptEligibilityKeyRecord,
    eligibility: ApplicationReceiptEligibilityRecord,
    replay: ApplicationReceiptReplayRecord,
    primary: ReceiptBackedIntentRecord,
}

fn validate_receipt_record_limit() -> Result<(), IntentStoreOpsError> {
    let records = ReceiptBackedIntentStore::len();
    if records <= RECEIPT_BACKED_INTENT_RECORD_LIMIT {
        return Ok(());
    }
    Err(IntentStoreOpsError::ReceiptBackedRecordLimitExceeded {
        records,
        limit: RECEIPT_BACKED_INTENT_RECORD_LIMIT,
    })
}

fn insert_application_settlement_eligibility(
    updated: &ReceiptBackedIntentRecord,
    replay: Option<ApplicationReceiptReplayRecord>,
) -> Result<(), IntentStoreOpsError> {
    let Some(replay) = replay else {
        return Ok(());
    };
    let (key, eligibility) = expected_application_eligibility(updated, replay)?.ok_or(
        IntentStoreOpsError::ApplicationReceiptEligibilityPrimaryMismatch(updated.operation_id),
    )?;
    if ReceiptBackedIntentStore::get_application_eligibility(key).is_some() {
        return Err(IntentStoreOpsError::ApplicationReceiptEligibilityExists(
            updated.operation_id,
        ));
    }
    let previous = ReceiptBackedIntentStore::insert_application_eligibility(key, eligibility);
    assert!(
        previous.is_none(),
        "prevalidated application eligibility insertion replaced an existing row"
    );
    Ok(())
}

fn validate_first_application_eligibility() -> Result<Option<u64>, IntentStoreOpsError> {
    let Some((key, actual)) = ReceiptBackedIntentStore::first_application_eligibility() else {
        return Ok(None);
    };
    validate_application_eligibility_identity(key.operation_id, actual)?;
    let primary = ReceiptBackedIntentStore::get(key.operation_id).ok_or(
        IntentStoreOpsError::ApplicationReceiptEligibilityPrimaryMismatch(key.operation_id),
    )?;
    ensure_receipt_backed_record_schema(&primary)?;
    if is_canic_owned_intent_resource_key(&primary.resource_key) {
        return Err(
            IntentStoreOpsError::ApplicationReceiptEligibilityPrimaryMismatch(key.operation_id),
        );
    }
    let replay = ReceiptBackedIntentStore::get_application_replay(key.operation_id).ok_or(
        IntentStoreOpsError::ApplicationReceiptReplayMissing(key.operation_id),
    )?;
    validate_application_replay_identity(key.operation_id, replay)?;
    let (expected_key, expected) = expected_application_eligibility(&primary, replay)?.ok_or(
        IntentStoreOpsError::ApplicationReceiptEligibilityPrimaryMismatch(key.operation_id),
    )?;
    validate_expected_application_eligibility(expected_key, expected, key, actual)?;
    Ok(Some(key.eligible_at_ns))
}

fn collect_due_application_reclamation_candidates(
    now_ns: u64,
    limit: usize,
) -> Result<Vec<ApplicationReceiptReclamationCandidate>, IntentStoreOpsError> {
    let due = ReceiptBackedIntentStore::with_application_eligibility(|eligibility| {
        eligibility
            .iter()
            .take_while(|entry| entry.key().eligible_at_ns <= now_ns)
            .take(limit)
            .map(|entry| (*entry.key(), entry.value()))
            .collect::<Vec<_>>()
    });

    due.into_iter()
        .map(|(key, eligibility)| validate_application_reclamation_candidate(key, eligibility))
        .collect()
}

fn validate_application_reclamation_candidate(
    key: ApplicationReceiptEligibilityKeyRecord,
    eligibility: ApplicationReceiptEligibilityRecord,
) -> Result<ApplicationReceiptReclamationCandidate, IntentStoreOpsError> {
    validate_application_eligibility_identity(key.operation_id, eligibility)?;
    let primary = ReceiptBackedIntentStore::get(key.operation_id).ok_or(
        IntentStoreOpsError::ApplicationReceiptEligibilityPrimaryMismatch(key.operation_id),
    )?;
    ensure_receipt_backed_record_schema(&primary)?;
    if is_canic_owned_intent_resource_key(&primary.resource_key)
        || matches!(primary.state, ReceiptBackedIntentState::Pending)
    {
        return Err(
            IntentStoreOpsError::ApplicationReceiptEligibilityPrimaryMismatch(key.operation_id),
        );
    }
    validate_placement_acknowledgement_index_for_record(&primary)?;

    let replay = ReceiptBackedIntentStore::get_application_replay(key.operation_id).ok_or(
        IntentStoreOpsError::ApplicationReceiptReplayMissing(key.operation_id),
    )?;
    validate_application_replay_identity(key.operation_id, replay)?;
    let (expected_key, expected) = expected_application_eligibility(&primary, replay)?.ok_or(
        IntentStoreOpsError::ApplicationReceiptEligibilityPrimaryMismatch(key.operation_id),
    )?;
    validate_expected_application_eligibility(expected_key, expected, key, eligibility)?;

    Ok(ApplicationReceiptReclamationCandidate {
        key,
        eligibility,
        replay,
        primary,
    })
}

fn collect_expected_receipt_indexes()
-> Result<(Vec<OperationId>, Vec<ExpectedApplicationEligibility>), IntentStoreOpsError> {
    ReceiptBackedIntentStore::with_records(|records| {
        ReceiptBackedIntentStore::with_application_replay(|replay_records| {
            let mut placement = Vec::new();
            let mut eligibility = Vec::new();
            let mut replay_entries = replay_records.iter().peekable();

            for entry in records.iter() {
                let record = entry.value();
                ensure_receipt_backed_record_schema(&record)?;
                let next_replay_id = replay_entries.peek().map(|entry| *entry.key());
                if next_replay_id.is_some_and(|operation_id| operation_id < record.operation_id) {
                    let orphan = replay_entries
                        .next()
                        .expect("peeked application replay entry must remain available");
                    validate_application_replay_identity(*orphan.key(), orphan.value())?;
                    return Err(IntentStoreOpsError::ApplicationReceiptReplayPrimaryMissing(
                        *orphan.key(),
                    ));
                }

                let replay =
                    replay_for_reconciliation(&record, next_replay_id, &mut replay_entries)?;
                if let Some(replay) = replay
                    && let Some(entry) = expected_application_eligibility(&record, replay)?
                {
                    eligibility.push(entry);
                }
                if receipt_requires_placement_acknowledgement(&record) {
                    placement.push(record.operation_id);
                }
            }

            if let Some(orphan) = replay_entries.next() {
                validate_application_replay_identity(*orphan.key(), orphan.value())?;
                return Err(IntentStoreOpsError::ApplicationReceiptReplayPrimaryMissing(
                    *orphan.key(),
                ));
            }
            Ok((placement, eligibility))
        })
    })
}

fn replay_for_reconciliation<M: crate::cdk::structures::Memory>(
    record: &ReceiptBackedIntentRecord,
    next_replay_id: Option<OperationId>,
    replay_entries: &mut std::iter::Peekable<
        crate::cdk::structures::btreemap::Iter<'_, OperationId, ApplicationReceiptReplayRecord, M>,
    >,
) -> Result<Option<ApplicationReceiptReplayRecord>, IntentStoreOpsError> {
    if is_canic_owned_intent_resource_key(&record.resource_key) {
        if next_replay_id == Some(record.operation_id) {
            let replay = replay_entries
                .next()
                .expect("matched application replay entry must remain available");
            validate_application_replay_identity(record.operation_id, replay.value())?;
            return Err(IntentStoreOpsError::ApplicationReceiptReplayUnexpected(
                record.operation_id,
            ));
        }
        return Ok(None);
    }
    if next_replay_id != Some(record.operation_id) {
        return Err(IntentStoreOpsError::ApplicationReceiptReplayMissing(
            record.operation_id,
        ));
    }
    let replay = replay_entries
        .next()
        .expect("matched application replay entry must remain available")
        .value();
    validate_application_replay_identity(record.operation_id, replay)?;
    Ok(Some(replay))
}

fn validate_application_eligibility_index(
    mut expected: Vec<ExpectedApplicationEligibility>,
) -> Result<(), IntentStoreOpsError> {
    expected.sort_unstable_by_key(|(key, _)| *key);
    ReceiptBackedIntentStore::with_application_eligibility(|eligibility| {
        let mut actual = eligibility.iter();
        for (expected_key, expected_record) in expected {
            let Some(actual_entry) = actual.next() else {
                return Err(IntentStoreOpsError::ApplicationReceiptEligibilityMissing(
                    expected_key.operation_id,
                ));
            };
            validate_expected_application_eligibility(
                expected_key,
                expected_record,
                *actual_entry.key(),
                actual_entry.value(),
            )?;
        }
        if let Some(actual_entry) = actual.next() {
            return Err(
                IntentStoreOpsError::ApplicationReceiptEligibilityPrimaryMismatch(
                    actual_entry.key().operation_id,
                ),
            );
        }
        Ok(())
    })
}

fn validate_expected_application_eligibility(
    expected_key: ApplicationReceiptEligibilityKeyRecord,
    expected: ApplicationReceiptEligibilityRecord,
    actual_key: ApplicationReceiptEligibilityKeyRecord,
    actual: ApplicationReceiptEligibilityRecord,
) -> Result<(), IntentStoreOpsError> {
    if actual_key < expected_key {
        return Err(
            IntentStoreOpsError::ApplicationReceiptEligibilityPrimaryMismatch(
                actual_key.operation_id,
            ),
        );
    }
    if actual_key > expected_key {
        return Err(IntentStoreOpsError::ApplicationReceiptEligibilityMissing(
            expected_key.operation_id,
        ));
    }
    validate_application_eligibility_identity(expected_key.operation_id, actual)?;
    if actual.payload_binding != expected.payload_binding {
        return Err(
            IntentStoreOpsError::ApplicationReceiptEligibilityBindingMismatch(
                expected_key.operation_id,
            ),
        );
    }
    if actual.terminal_revision != expected.terminal_revision {
        return Err(
            IntentStoreOpsError::ApplicationReceiptEligibilityRevisionMismatch {
                operation_id: expected_key.operation_id,
                expected: expected.terminal_revision,
                found: actual.terminal_revision,
            },
        );
    }
    Ok(())
}

fn retained_begin_result(
    input: &ReceiptAdmission<'_>,
) -> Result<Option<BeginReceiptBackedIntentResult>, InternalError> {
    let Some(record) = ReceiptBackedIntentStore::get(input.operation_id) else {
        return Ok(None);
    };

    ensure_receipt_backed_record_schema(&record)?;
    validate_placement_acknowledgement_index_for_record(&record)?;
    let replay = validate_application_replay_for_record(&record)?;
    if record.payload_binding != input.payload_binding
        || record.resource_key != *input.resource_key
        || record.quantity != input.quantity
    {
        return Ok(Some(BeginReceiptBackedIntentResult::BindingConflict));
    }

    match (input.owner, replay) {
        (
            ReceiptAdmissionOwner::Application {
                replay_deadline_ns, ..
            },
            Some(replay),
        ) if replay.replay_deadline_ns != replay_deadline_ns => {
            return Ok(Some(BeginReceiptBackedIntentResult::BindingConflict));
        }
        (ReceiptAdmissionOwner::Application { .. }, Some(_))
        | (ReceiptAdmissionOwner::Placement, None) => {}
        (ReceiptAdmissionOwner::Application { .. }, None)
        | (ReceiptAdmissionOwner::Placement, Some(_)) => {
            return Ok(Some(BeginReceiptBackedIntentResult::BindingConflict));
        }
    }
    Ok(Some(begin_result_for_existing(&record)))
}

fn validate_absent_receipt_indexes(operation_id: OperationId) -> Result<(), InternalError> {
    if ReceiptBackedIntentStore::get_application_replay(operation_id).is_some() {
        return Err(
            IntentStoreOpsError::ApplicationReceiptReplayPrimaryMissing(operation_id).into(),
        );
    }
    if ReceiptBackedIntentStore::get_placement_acknowledgement(operation_id).is_some() {
        return Err(
            IntentStoreOpsError::PlacementAcknowledgementPrimaryMissing(operation_id).into(),
        );
    }
    Ok(())
}

const fn replay_window_rejection(
    owner: ReceiptAdmissionOwner,
) -> Option<BeginReceiptBackedIntentResult> {
    let ReceiptAdmissionOwner::Application {
        replay_deadline_ns,
        replay_window,
    } = owner
    else {
        return None;
    };
    match replay_window {
        ReceiptReplayWindowDecision::Open => None,
        ReceiptReplayWindowDecision::Closed => {
            Some(BeginReceiptBackedIntentResult::ReplayWindowClosed { replay_deadline_ns })
        }
        ReceiptReplayWindowDecision::TooLong { remaining_ns } => {
            Some(BeginReceiptBackedIntentResult::ReplayWindowTooLong {
                remaining_ns,
                maximum_ns: MAX_RECEIPT_BACKED_INTENT_REPLAY_WINDOW_NS,
            })
        }
    }
}

fn create_receipt(
    input: ReceiptAdmission<'_>,
    now_ns: u64,
    record_limit: u64,
) -> Result<BeginReceiptBackedIntentResult, InternalError> {
    let record_count = ReceiptBackedIntentStore::len();
    if record_count >= record_limit {
        return Ok(BeginReceiptBackedIntentResult::StoreCapacityReached {
            current_records: record_count,
            limit: record_limit,
        });
    }

    let totals = IntentStore::get_totals(input.resource_key).unwrap_or_default();
    let current_quantity = checked_add(totals.reserved_qty, totals.committed_qty, "accounted_qty")?;
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
    let replay = match input.owner {
        ReceiptAdmissionOwner::Application {
            replay_deadline_ns, ..
        } => Some(ApplicationReceiptReplayRecord {
            schema_version: APPLICATION_RECEIPT_REPLAY_SCHEMA_VERSION,
            operation_id: input.operation_id,
            replay_deadline_ns,
        }),
        ReceiptAdmissionOwner::Placement => None,
    };

    if replay.is_some() {
        let required_records = ReceiptBackedIntentStore::application_replay_len()
            .checked_add(1)
            .ok_or(IntentStoreOpsError::ApplicationReceiptEligibilityReservationOverflow)?;
        if !ReceiptBackedIntentStore::reserve_application_eligibility_capacity(required_records) {
            return Err(
                IntentStoreOpsError::ApplicationReceiptEligibilityCapacityUnavailable {
                    required_records,
                }
                .into(),
            );
        }
    }

    assert!(
        ReceiptBackedIntentStore::insert(record).is_none(),
        "validated receipt-backed intent insertion replaced an entry"
    );
    if let Some(replay) = replay {
        assert!(
            ReceiptBackedIntentStore::insert_application_replay(replay).is_none(),
            "validated application receipt replay insertion replaced an entry"
        );
    }
    persist_resource_totals(input.resource_key.clone(), new_totals);

    Ok(BeginReceiptBackedIntentResult::Created { revision })
}

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

fn validate_receipt_admission_owner(
    input: &ReceiptAdmission<'_>,
) -> Result<(), IntentStoreOpsError> {
    match (
        input.owner,
        is_canic_owned_intent_resource_key(input.resource_key),
    ) {
        (ReceiptAdmissionOwner::Application { .. }, false)
        | (ReceiptAdmissionOwner::Placement, true) => Ok(()),
        (ReceiptAdmissionOwner::Application { .. }, true) => {
            Err(IntentStoreOpsError::ReceiptBackedOwnershipMismatch {
                operation_id: input.operation_id,
                owner: "application",
            })
        }
        (ReceiptAdmissionOwner::Placement, false) => {
            Err(IntentStoreOpsError::ReceiptBackedOwnershipMismatch {
                operation_id: input.operation_id,
                owner: "placement",
            })
        }
    }
}

fn validate_application_replay_for_record(
    record: &ReceiptBackedIntentRecord,
) -> Result<Option<ApplicationReceiptReplayRecord>, IntentStoreOpsError> {
    let replay = ReceiptBackedIntentStore::get_application_replay(record.operation_id);
    match (
        is_canic_owned_intent_resource_key(&record.resource_key),
        replay,
    ) {
        (true, Some(_)) => Err(IntentStoreOpsError::ApplicationReceiptReplayUnexpected(
            record.operation_id,
        )),
        (true, None) => Ok(None),
        (false, None) => Err(IntentStoreOpsError::ApplicationReceiptReplayMissing(
            record.operation_id,
        )),
        (false, Some(replay)) => {
            validate_application_replay_identity(record.operation_id, replay)?;
            validate_application_eligibility_for_record(record, replay)?;
            Ok(Some(replay))
        }
    }
}

fn expected_application_eligibility(
    record: &ReceiptBackedIntentRecord,
    replay: ApplicationReceiptReplayRecord,
) -> Result<
    Option<(
        ApplicationReceiptEligibilityKeyRecord,
        ApplicationReceiptEligibilityRecord,
    )>,
    IntentStoreOpsError,
> {
    if matches!(record.state, ReceiptBackedIntentState::Pending) {
        return Ok(None);
    }
    let eligible_at_ns =
        receipt_terminal_eligible_at(replay.replay_deadline_ns, record.updated_at_ns).ok_or(
            IntentStoreOpsError::ApplicationReceiptEligibilityOverflow {
                operation_id: record.operation_id,
                terminal_timestamp_ns: record.updated_at_ns,
                observation_grace_ns: RECEIPT_TERMINAL_OBSERVATION_GRACE_NS,
            },
        )?;
    Ok(Some((
        ApplicationReceiptEligibilityKeyRecord {
            eligible_at_ns,
            operation_id: record.operation_id,
        },
        ApplicationReceiptEligibilityRecord {
            schema_version: APPLICATION_RECEIPT_ELIGIBILITY_SCHEMA_VERSION,
            operation_id: record.operation_id,
            payload_binding: record.payload_binding,
            terminal_revision: record.revision,
        },
    )))
}

fn validate_application_eligibility_for_record(
    record: &ReceiptBackedIntentRecord,
    replay: ApplicationReceiptReplayRecord,
) -> Result<(), IntentStoreOpsError> {
    let Some((key, expected)) = expected_application_eligibility(record, replay)? else {
        return Ok(());
    };
    let actual = ReceiptBackedIntentStore::get_application_eligibility(key).ok_or(
        IntentStoreOpsError::ApplicationReceiptEligibilityMissing(record.operation_id),
    )?;
    validate_application_eligibility_identity(record.operation_id, actual)?;
    if actual.payload_binding != expected.payload_binding {
        return Err(
            IntentStoreOpsError::ApplicationReceiptEligibilityBindingMismatch(record.operation_id),
        );
    }
    if actual.terminal_revision != expected.terminal_revision {
        return Err(
            IntentStoreOpsError::ApplicationReceiptEligibilityRevisionMismatch {
                operation_id: record.operation_id,
                expected: expected.terminal_revision,
                found: actual.terminal_revision,
            },
        );
    }
    Ok(())
}

fn validate_application_eligibility_identity(
    operation_id: OperationId,
    eligibility: ApplicationReceiptEligibilityRecord,
) -> Result<(), IntentStoreOpsError> {
    if eligibility.operation_id != operation_id {
        return Err(
            IntentStoreOpsError::ApplicationReceiptEligibilityIdentityMismatch {
                key: operation_id,
                value: eligibility.operation_id,
            },
        );
    }
    if eligibility.schema_version != APPLICATION_RECEIPT_ELIGIBILITY_SCHEMA_VERSION {
        return Err(
            IntentStoreOpsError::ApplicationReceiptEligibilitySchemaMismatch {
                operation_id,
                expected: APPLICATION_RECEIPT_ELIGIBILITY_SCHEMA_VERSION,
                found: eligibility.schema_version,
            },
        );
    }
    Ok(())
}

fn validate_application_replay_identity(
    operation_id: OperationId,
    replay: ApplicationReceiptReplayRecord,
) -> Result<(), IntentStoreOpsError> {
    if replay.operation_id != operation_id {
        return Err(
            IntentStoreOpsError::ApplicationReceiptReplayIdentityMismatch {
                key: operation_id,
                value: replay.operation_id,
            },
        );
    }
    if replay.schema_version != APPLICATION_RECEIPT_REPLAY_SCHEMA_VERSION {
        return Err(
            IntentStoreOpsError::ApplicationReceiptReplaySchemaMismatch {
                operation_id,
                expected: APPLICATION_RECEIPT_REPLAY_SCHEMA_VERSION,
                found: replay.schema_version,
            },
        );
    }
    Ok(())
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

fn receipt_requires_placement_acknowledgement(record: &ReceiptBackedIntentRecord) -> bool {
    !matches!(record.state, ReceiptBackedIntentState::Pending)
        && is_placement_resource_key(&record.resource_key)
}

fn validate_placement_acknowledgement_index_for_record(
    record: &ReceiptBackedIntentRecord,
) -> Result<(), IntentStoreOpsError> {
    let indexed = ReceiptBackedIntentStore::get_placement_acknowledgement(record.operation_id);
    match (receipt_requires_placement_acknowledgement(record), indexed) {
        (true, None) => Err(IntentStoreOpsError::PlacementAcknowledgementIndexMissing(
            record.operation_id,
        )),
        (true, Some(entry)) if entry.operation_id != record.operation_id => Err(
            IntentStoreOpsError::PlacementAcknowledgementIndexValueMismatch {
                key: record.operation_id,
                value: entry.operation_id,
            },
        ),
        (false, Some(_)) => {
            Err(IntentStoreOpsError::PlacementAcknowledgementUnexpectedIndex(record.operation_id))
        }
        (true, Some(_)) | (false, None) => Ok(()),
    }
}

fn validate_placement_acknowledgement_entry(
    operation_id: OperationId,
    entry: PlacementAcknowledgementEntryRecord,
) -> Result<ReceiptBackedIntentRecord, IntentStoreOpsError> {
    if entry.operation_id != operation_id {
        return Err(
            IntentStoreOpsError::PlacementAcknowledgementIndexValueMismatch {
                key: operation_id,
                value: entry.operation_id,
            },
        );
    }
    let record = ReceiptBackedIntentStore::get(operation_id).ok_or(
        IntentStoreOpsError::PlacementAcknowledgementPrimaryMissing(operation_id),
    )?;
    ensure_receipt_backed_record_schema(&record)?;
    if !receipt_requires_placement_acknowledgement(&record) {
        return Err(IntentStoreOpsError::PlacementAcknowledgementPrimaryMismatch(operation_id));
    }
    Ok(record)
}

fn ensure_pending_indexes(record: &IntentRecord) -> Result<(), IntentStoreOpsError> {
    let pending = IntentStore::get_pending(record.id)
        .ok_or(IntentStoreOpsError::PendingIndexMissing(record.id))?;
    ensure_pending_record_matches(record, &pending)?;

    if let Some(key) = expiry_key(record.id, record.created_at, record.ttl_secs)? {
        let indexed =
            IntentStore::get_expiry(key).ok_or(IntentStoreOpsError::ExpiryIndexMissing {
                id: record.id,
                due_at_secs: key.due_at_secs,
            })?;
        if indexed.intent_id != record.id {
            return Err(IntentStoreOpsError::ExpiryIndexValueMismatch {
                due_at_secs: key.due_at_secs,
                key_id: record.id,
                value_id: indexed.intent_id,
            });
        }
    }
    Ok(())
}

fn ensure_pending_record_matches(
    record: &IntentRecord,
    pending: &IntentPendingEntryRecord,
) -> Result<(), IntentStoreOpsError> {
    if record.state != IntentState::Pending
        || record.resource_key != pending.resource_key
        || record.quantity != pending.quantity
        || record.created_at != pending.created_at
        || record.ttl_secs != pending.ttl_secs
    {
        return Err(IntentStoreOpsError::PendingIndexMismatch(record.id));
    }
    Ok(())
}

fn validate_expiry_entry(
    key: IntentExpiryKeyRecord,
    record: IntentExpiryEntryRecord,
) -> Result<(), IntentStoreOpsError> {
    if record.intent_id != key.intent_id {
        return Err(IntentStoreOpsError::ExpiryIndexValueMismatch {
            due_at_secs: key.due_at_secs,
            key_id: key.intent_id,
            value_id: record.intent_id,
        });
    }
    let pending = IntentStore::get_pending(key.intent_id)
        .ok_or(IntentStoreOpsError::PendingIndexMissing(key.intent_id))?;
    let record = IntentStore::get_record(key.intent_id)
        .ok_or(IntentStoreOpsError::NotFound(key.intent_id))?;
    ensure_pending_record_matches(&record, &pending)?;
    let expected = expiry_key(record.id, record.created_at, record.ttl_secs)?
        .ok_or(IntentStoreOpsError::TtlFreeIntentInExpiryIndex(record.id))?;
    if expected != key {
        return Err(IntentStoreOpsError::ExpiryIndexKeyMismatch {
            id: record.id,
            expected_due_at_secs: expected.due_at_secs,
            found_due_at_secs: key.due_at_secs,
        });
    }
    Ok(())
}

fn remove_pending_and_apply(
    resource_key: IntentResourceKey,
    totals: IntentResourceTotalsRecord,
    meta: IntentStoreMetaRecord,
    record: IntentRecord,
) {
    remove_pending_indexes(&record);
    persist_resource_totals(resource_key, totals);
    IntentStore::set_meta(meta);
    IntentStore::insert_record(record);
}

fn remove_pending_indexes(record: &IntentRecord) {
    let removed_pending = IntentStore::remove_pending(record.id);
    assert!(
        removed_pending.is_some(),
        "validated pending intent index disappeared before removal"
    );
    if let Some(key) = expiry_key(record.id, record.created_at, record.ttl_secs)
        .unwrap_or_else(|err| panic!("validated intent expiry became invalid: {err}"))
    {
        let removed_expiry = IntentStore::remove_expiry(key);
        assert_eq!(
            removed_expiry,
            Some(IntentExpiryEntryRecord {
                intent_id: record.id
            }),
            "validated intent expiry index changed before removal"
        );
    }
}

fn persist_resource_totals(resource_key: IntentResourceKey, totals: IntentResourceTotalsRecord) {
    if totals == IntentResourceTotalsRecord::default() {
        let removed = IntentStore::remove_totals(&resource_key);
        assert!(
            removed.is_some(),
            "validated zero resource totals lost their stored authority before removal"
        );
    } else {
        IntentStore::set_totals(resource_key, totals);
    }
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

fn expiry_key(
    intent_id: IntentId,
    created_at: u64,
    ttl_secs: Option<u64>,
) -> Result<Option<IntentExpiryKeyRecord>, IntentStoreOpsError> {
    let Some(ttl_secs) = ttl_secs else {
        return Ok(None);
    };
    let due_at_secs = created_at
        .checked_add(ttl_secs)
        .and_then(|expires_at| expires_at.checked_add(1))
        .filter(|due_at_secs| *due_at_secs <= u64::MAX / NANOS_PER_SECOND)
        .ok_or(IntentStoreOpsError::ExpiryDeadlineOverflow {
            id: intent_id,
            created_at,
            ttl_secs,
        })?;
    Ok(Some(IntentExpiryKeyRecord {
        due_at_secs,
        intent_id,
    }))
}

fn is_expired(
    intent_id: IntentId,
    now: u64,
    created_at: u64,
    ttl_secs: Option<u64>,
) -> Result<bool, IntentStoreOpsError> {
    match expiry_key(intent_id, created_at, ttl_secs)? {
        Some(key) => Ok(now >= key.due_at_secs),
        None => Ok(false),
    }
}

fn is_record_expired(now: u64, record: &IntentRecord) -> Result<bool, IntentStoreOpsError> {
    is_expired(record.id, now, record.created_at, record.ttl_secs)
}

#[cfg(test)]
fn is_pending_entry_expired(
    intent_id: IntentId,
    now: u64,
    entry: &IntentPendingEntryRecord,
) -> Result<bool, IntentStoreOpsError> {
    is_expired(intent_id, now, entry.created_at, entry.ttl_secs)
}
