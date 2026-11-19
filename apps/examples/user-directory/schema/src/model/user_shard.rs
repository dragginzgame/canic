//! Private shard accounts.

use ic_db::design::prelude::*;

/// The directory shard.
#[canister]
pub struct Shard {}

/// Retained database for the shard.
#[store(canister = Shard, key = "directory.shard.db.v1")]
pub struct Accounts {}

///
/// User
/// Stores the local user ID for an issuer-qualified account.
///

#[entity(
    store = Accounts,
    name = "User",
    pk(field = "id"),
    index(fields = ["issuer", "subject"], unique),
    fields(
        field(name = "id", value(item(prim = "Ulid"))),
        field(name = "issuer", value(item(prim = "Principal"))),
        field(name = "subject", value(item(prim = "Principal"))),
    )
)]
pub struct User {}
