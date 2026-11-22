//! Two independent stores owned by one hub.

use ic_db::design::prelude::*;

///
/// Hub
/// Owns both witness stores.
///

#[canister]
pub struct Hub {}

///
/// Routes
/// Retains the route row.
///

#[store(canister = Hub, key = "witness.routes.v1")]
pub struct Routes {}

///
/// Catalog
/// Retains the project row.
///

#[store(canister = Hub, key = "witness.catalog.v1")]
pub struct Catalog {}

///
/// Route
/// Identifies the value written to the routes store.
///

#[entity(
    store = Routes,
    pk(field = "id"),
    fields(
        field(name = "id", value(item(prim = "Nat64"))),
        field(name = "value", value(item(prim = "Nat64"))),
    )
)]
pub struct Route {}

///
/// Project
/// Identifies the value written to the project catalog.
///

#[entity(
    store = Catalog,
    pk(field = "id"),
    fields(
        field(name = "id", value(item(prim = "Nat64"))),
        field(name = "value", value(item(prim = "Nat64"))),
    ),
    timestamps
)]
pub struct Project {}
