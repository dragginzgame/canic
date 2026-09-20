//! Controlled IcyDB schema, using Canic's existing participant memory range.

use icydb::model::prelude::*;

/// Shared lifecycle metadata; features select zero, one or ten entities.
#[canister(memory_namespace = "canic_composed_audit")]
pub struct EmptyCanister {}

/// One empty journaled store matching the existing IcyDB audit shape.
#[store(canister = "EmptyCanister", storage(journaled(key = "empty")))]
pub struct EmptyStore {}

/// Matching two-field entity 1 for controlled specialization attribution.
#[cfg(feature = "entity")]
#[entity(
    store = "EmptyStore",
    version = 1,
    pk(fields = ["id"]),
    fields(
        field(name = "id", value(item(prim = "Nat64"))),
        field(name = "value", value(item(prim = "Nat64")))
    )
)]
pub struct AuditEntity01 {}

/// Matching two-field entity 2 for controlled specialization attribution.
#[cfg(feature = "ten")]
#[entity(
    store = "EmptyStore",
    version = 1,
    pk(fields = ["id"]),
    fields(
        field(name = "id", value(item(prim = "Nat64"))),
        field(name = "value", value(item(prim = "Nat64")))
    )
)]
pub struct AuditEntity02 {}

/// Matching two-field entity 3 for controlled specialization attribution.
#[cfg(feature = "ten")]
#[entity(
    store = "EmptyStore",
    version = 1,
    pk(fields = ["id"]),
    fields(
        field(name = "id", value(item(prim = "Nat64"))),
        field(name = "value", value(item(prim = "Nat64")))
    )
)]
pub struct AuditEntity03 {}

/// Matching two-field entity 4 for controlled specialization attribution.
#[cfg(feature = "ten")]
#[entity(
    store = "EmptyStore",
    version = 1,
    pk(fields = ["id"]),
    fields(
        field(name = "id", value(item(prim = "Nat64"))),
        field(name = "value", value(item(prim = "Nat64")))
    )
)]
pub struct AuditEntity04 {}

/// Matching two-field entity 5 for controlled specialization attribution.
#[cfg(feature = "ten")]
#[entity(
    store = "EmptyStore",
    version = 1,
    pk(fields = ["id"]),
    fields(
        field(name = "id", value(item(prim = "Nat64"))),
        field(name = "value", value(item(prim = "Nat64")))
    )
)]
pub struct AuditEntity05 {}

/// Matching two-field entity 6 for controlled specialization attribution.
#[cfg(feature = "ten")]
#[entity(
    store = "EmptyStore",
    version = 1,
    pk(fields = ["id"]),
    fields(
        field(name = "id", value(item(prim = "Nat64"))),
        field(name = "value", value(item(prim = "Nat64")))
    )
)]
pub struct AuditEntity06 {}

/// Matching two-field entity 7 for controlled specialization attribution.
#[cfg(feature = "ten")]
#[entity(
    store = "EmptyStore",
    version = 1,
    pk(fields = ["id"]),
    fields(
        field(name = "id", value(item(prim = "Nat64"))),
        field(name = "value", value(item(prim = "Nat64")))
    )
)]
pub struct AuditEntity07 {}

/// Matching two-field entity 8 for controlled specialization attribution.
#[cfg(feature = "ten")]
#[entity(
    store = "EmptyStore",
    version = 1,
    pk(fields = ["id"]),
    fields(
        field(name = "id", value(item(prim = "Nat64"))),
        field(name = "value", value(item(prim = "Nat64")))
    )
)]
pub struct AuditEntity08 {}

/// Matching two-field entity 9 for controlled specialization attribution.
#[cfg(feature = "ten")]
#[entity(
    store = "EmptyStore",
    version = 1,
    pk(fields = ["id"]),
    fields(
        field(name = "id", value(item(prim = "Nat64"))),
        field(name = "value", value(item(prim = "Nat64")))
    )
)]
pub struct AuditEntity09 {}

/// Matching two-field entity 10 for controlled specialization attribution.
#[cfg(feature = "ten")]
#[entity(
    store = "EmptyStore",
    version = 1,
    pk(fields = ["id"]),
    fields(
        field(name = "id", value(item(prim = "Nat64"))),
        field(name = "value", value(item(prim = "Nat64")))
    )
)]
pub struct AuditEntity10 {}
