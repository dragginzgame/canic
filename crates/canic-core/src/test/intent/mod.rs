//! Test-only intent store wrappers for PocketIC canisters.

use crate::ops::storage::intent::IntentStoreOps;

#[doc(hidden)]
pub use crate::ops::storage::intent::{
    IntentId, IntentPendingEntry, IntentRecord, IntentResourceKey, IntentResourceTotals,
    IntentState, IntentStoreMeta,
};

///
/// IntentTestOps
///

pub struct IntentTestOps;

impl IntentTestOps {
    pub fn allocate_intent_id() -> Result<IntentId, String> {
        IntentStoreOps::allocate_intent_id().map_err(|err| err.to_string())
    }

    pub fn try_reserve(
        intent_id: IntentId,
        resource_key: IntentResourceKey,
        quantity: u64,
        created_at: u64,
        ttl_secs: Option<u64>,
    ) -> Result<IntentRecord, String> {
        IntentStoreOps::try_reserve(intent_id, resource_key, quantity, created_at, ttl_secs)
            .map_err(|err| err.to_string())
    }

    pub fn commit(intent_id: IntentId) -> Result<IntentRecord, String> {
        IntentStoreOps::commit(intent_id).map_err(|err| err.to_string())
    }

    pub fn abort(intent_id: IntentId) -> Result<IntentRecord, String> {
        IntentStoreOps::abort(intent_id).map_err(|err| err.to_string())
    }

    #[must_use]
    pub fn totals(resource_key: &IntentResourceKey) -> IntentResourceTotals {
        IntentStoreOps::totals(resource_key).unwrap_or_default()
    }

    #[must_use]
    pub fn pending_entries() -> Vec<(IntentId, IntentPendingEntry)> {
        IntentStoreOps::pending_entries()
    }

    pub fn meta() -> Result<IntentStoreMeta, String> {
        IntentStoreOps::meta().map_err(|err| err.to_string())
    }
}
