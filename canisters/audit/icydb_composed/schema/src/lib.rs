//! Empty journaled IcyDB schema, using Canic's existing participant memory range.

use icydb::model::prelude::*;

/// Lifecycle metadata for the empty participant; contains no entity registrations.
#[canister(
    memory_namespace = "canic_composed_audit",
    memory_min = 100,
    memory_max = 106,
    commit_memory_id = 104,
    startup_memory_id = 106,
    integrity_progress_memory_id = 105
)]
pub struct EmptyCanister {}

/// One empty journaled store matching the existing IcyDB audit shape.
#[store(
    canister = "EmptyCanister",
    storage(journaled(
        data_memory_id = 100,
        index_memory_id = 101,
        schema_memory_id = 102,
        journal_memory_id = 103
    ))
)]
pub struct EmptyStore {}
