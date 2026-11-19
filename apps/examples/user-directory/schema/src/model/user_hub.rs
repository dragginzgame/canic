//! Private hub routes.

use ic_db::design::prelude::*;

/// The directory hub.
#[canister]
pub struct Hub {}

/// Retained database for the hub.
#[store(canister = Hub, key = "directory.hub.db.v1")]
pub struct Routes {}

///
/// Route
/// Stores the shard and readiness of an issuer-qualified account.
///

#[entity(
    store = Routes,
    name = "Route",
    pk(field = "id"),
    index(fields = ["issuer", "subject"], unique),
    fields(
        field(name = "id", value(item(prim = "Ulid"))),
        field(name = "issuer", value(item(prim = "Principal"))),
        field(name = "subject", value(item(prim = "Principal"))),
        field(name = "shard", value(item(prim = "Principal"))),
        field(name = "ready", value(item(prim = "Bool"))),
    )
)]
pub struct Route {}
