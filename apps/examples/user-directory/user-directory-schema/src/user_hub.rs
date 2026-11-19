//! Private hub routes.
use ic_db::design::prelude::*;

/// One record per issuer-qualified account.
#[entity(
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
