//! Private shard accounts.

use ic_db::design::prelude::*;

///
/// User
/// Stores the local user ID for an issuer-qualified account.
///

#[entity(
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
