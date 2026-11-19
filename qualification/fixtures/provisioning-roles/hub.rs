//! Application-owned hub mapping for Toko's existing shared schema.
mod endpoints;
mod ops;
mod register;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

use canic::sharding::provisioning::{Account, Binding, HubRoute, Record};
use ic_db::{Database, Entity, Principal, Schema, Ulid};
use toko::user::{display_name, provisioning::Users};
use toko_schema::entity::user_hub::UserRoute;
use toko_schema::types::signup::DisplayName;

pub use ops::get_my_route;
pub use toko::user::{Error, Identity};
pub use toko_schema::types::signup::{Route, Status};

canic::mount_db_role! {
    hub,
    database = "toko.user.hub.principal.v1",
    config = "toko.user.hub.config.v1",
    authority = "toko",
    pages = 65536,
    auth_read = "read",
}

struct Hub;

impl Record for Hub {
    type Row = UserRoute;
    type Resource = Users;

    fn with_store<T>(f: impl FnOnce(&mut Database) -> T) -> T {
        database::store::with(f)
    }

    fn identity(row: &UserRoute) -> (Account, [u8; 16]) {
        (
            Account {
                issuer: row.issuer,
                subject: row.subject,
            },
            row.id.to_bytes(),
        )
    }
}

impl HubRoute for Hub {
    const SCOPE: &'static str = "write";
    type Initial = DisplayName;

    fn normalize(name: String) -> DisplayName {
        display_name(&name).unwrap_or_else(|error| canic::method::fail(error))
    }

    fn pending(binding: Binding<[u8; 16]>, display_name: DisplayName) -> UserRoute {
        UserRoute {
            id: Ulid::from_bytes(binding.id),
            issuer: binding.account.issuer,
            subject: binding.account.subject,
            shard: binding.shard,
            display_name,
            status: Status::Pending,
        }
    }

    fn retained(row: &UserRoute) -> (Principal, String) {
        (row.shard, row.display_name.0.clone())
    }

    fn ready(row: &UserRoute) -> bool {
        row.status == Status::Ready
    }

    fn mark_ready(row: &mut UserRoute) {
        row.status = Status::Ready;
    }
}

/// Report the configured role and original retained allocation for the upgrade witness.
#[canic::query]
fn role_metadata() -> (String, String) {
    let expected = Schema::new(&[UserRoute::SCHEMA]).unwrap().fingerprint();
    assert_eq!(Hub::with_store(|db| db.schema().fingerprint()), expected);
    (
        option_env!(concat!("CANIC_APP_ROLE_", env!("CARGO_PKG_NAME")))
            .unwrap_or("")
            .to_owned(),
        database::DATABASE_KEY.to_owned(),
    )
}
