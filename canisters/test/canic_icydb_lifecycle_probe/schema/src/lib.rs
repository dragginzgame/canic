//! Minimal published-IcyDB schema for Canic lifecycle composition evidence.

use icydb_model::prelude::*;

/// IcyDB model whose application memory stays above Canic's reserved range.
#[canister(
    memory_namespace = "canic_icydb_lifecycle",
    memory_min = 100,
    memory_max = 106,
    commit_memory_id = 104,
    startup_memory_id = 106,
    integrity_progress_memory_id = 105
)]
pub struct CanicIcydbLifecycleCanister {}

/// One journaled store sufficient to exercise durable startup recovery.
#[store(
    canister = "CanicIcydbLifecycleCanister",
    storage(journaled(
        data_memory_id = 100,
        index_memory_id = 101,
        schema_memory_id = 102,
        journal_memory_id = 103
    ))
)]
pub struct CanicIcydbLifecycleStore {}

/// One generated-identity row that makes the accepted schema non-empty.
#[entity(
    store = "CanicIcydbLifecycleStore",
    version = 1,
    pk(fields = ["id"]),
    fields(
        field(name = "id", value(item(prim = "Ulid")), generated(insert = "Ulid::generate")),
        field(name = "name", value(item(prim = "Text", unbounded)))
    ),
    timestamps
)]
pub struct LifecycleProbeRow {}
