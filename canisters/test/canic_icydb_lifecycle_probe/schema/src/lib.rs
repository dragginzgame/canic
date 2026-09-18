//! Minimal published-IcyDB schema for Canic lifecycle composition evidence.

use icydb::model::prelude::*;

/// IcyDB model whose application memory stays above Canic's reserved range.
#[canister(memory_namespace = "canic_icydb_lifecycle")]
pub struct CanicIcydbLifecycleCanister {}

/// One journaled store sufficient to exercise durable startup recovery.
#[store(
    canister = "CanicIcydbLifecycleCanister",
    storage(journaled(key = "lifecycle"))
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

/// Deterministic application rows used only by the fixture commit qualification.
#[entity(
    store = "CanicIcydbLifecycleStore",
    version = 1,
    pk(fields = ["id"]),
    fields(
        field(name = "id", value(item(prim = "Nat64"))),
        field(name = "value", value(item(prim = "Nat64")))
    )
)]
pub struct FixtureProbeRow {}
