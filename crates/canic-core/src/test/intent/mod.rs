//! Test-only intent store wrappers for PocketIC canisters.

use crate::{
    ids::IntentResourceKey, ops::storage::intent::IntentStoreOps,
    storage::stable::intent::IntentStore,
};

#[doc(hidden)]
pub(crate) use crate::storage::stable::intent::{
    IntentId, IntentPendingEntryRecord, IntentRecord, IntentResourceTotalsRecord,
    IntentStoreMetaRecord,
};

///
/// IntentTestOps
///
/// Test-only convenience facade over IntentStoreOps.
/// Owns time explicitly and converts errors to strings for assertions.
///

pub struct IntentTestOps;

impl IntentTestOps {
    // -------------------------------------------------------------------------
    // Time helpers
    // -------------------------------------------------------------------------

    #[must_use]
    pub fn now() -> u64 {
        // Tests should override this explicitly where needed
        crate::ops::ic::IcOps::now_secs()
    }

    // -------------------------------------------------------------------------
    // Allocation
    // -------------------------------------------------------------------------

    pub fn allocate_intent_id() -> Result<IntentId, String> {
        IntentStoreOps::allocate_intent_id().map_err(|e| e.to_string())
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
    ) -> Result<IntentRecord, String> {
        IntentStoreOps::try_reserve(intent_id, resource_key, quantity, created_at, ttl_secs)
            .map_err(|e| e.to_string())
    }

    pub fn commit_at(intent_id: IntentId, now: u64) -> Result<IntentRecord, String> {
        IntentStoreOps::commit_at(intent_id, now).map_err(|e| e.to_string())
    }

    pub fn abort(intent_id: IntentId) -> Result<IntentRecord, String> {
        IntentStoreOps::abort(intent_id).map_err(|e| e.to_string())
    }

    // -------------------------------------------------------------------------
    // Read-only views (TTL authoritative)
    // -------------------------------------------------------------------------

    #[must_use]
    pub fn totals_at(resource_key: &IntentResourceKey, now: u64) -> IntentResourceTotalsRecord {
        IntentStoreOps::totals_at(resource_key, now)
    }

    #[must_use]
    pub fn pending_entries_at(now: u64) -> Vec<(IntentId, IntentPendingEntryRecord)> {
        IntentStoreOps::pending_entries_at(now)
    }

    #[must_use]
    pub fn expired_pending_ids(now: u64) -> Vec<IntentId> {
        IntentStoreOps::list_expired_pending_intents(now)
    }

    // -------------------------------------------------------------------------
    // Storage-level inspection (tests only)
    // -------------------------------------------------------------------------

    #[must_use]
    pub fn meta() -> IntentStoreMetaRecord {
        // Tests are allowed to read storage internals directly
        IntentStore::meta()
    }

    #[must_use]
    pub fn record(intent_id: IntentId) -> Option<IntentRecord> {
        IntentStore::get_record(intent_id)
    }
}
