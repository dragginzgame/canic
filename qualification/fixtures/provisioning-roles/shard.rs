//! Application-owned shard mapping for Toko's existing shared schema.
mod endpoints;
mod ops;
mod provision_user;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

use canic::sharding::provisioning::{Account, Record, ShardRecord, Ticket};
use ic_db::{Database, Entity, Schema, Ulid};
use toko::user::{display_name, provisioning::Users};
use toko_schema::entity::user_shard::User;

pub use ops::user::{get_me, set_display_name};
pub use toko::user::{Error, Identity};
pub use toko_schema::types::signup::Profile;

canic::mount_db_role! {
    shard,
    database = "toko.user.shard.user.v1",
    authority = "user_shard",
    prefix = "toko.user.shard.",
    auth_read = "read",
}

struct Shard;

impl Record for Shard {
    type Row = User;
    type Resource = Users;

    fn with_store<T>(f: impl FnOnce(&mut Database) -> T) -> T {
        database::store::with(f)
    }

    fn identity(row: &User) -> (Account, [u8; 16]) {
        (
            Account {
                issuer: row.issuer,
                subject: row.subject,
            },
            row.id.to_bytes(),
        )
    }
}

impl ShardRecord for Shard {
    fn create(request: Ticket<Self>) -> User {
        let binding = request.binding;
        User {
            id: Ulid::from_bytes(binding.id),
            issuer: binding.account.issuer,
            subject: binding.account.subject,
            display_name: display_name(&request.data)
                .unwrap_or_else(|error| canic::method::fail(error)),
        }
    }
}

/// Report the configured role and original retained allocation for the upgrade witness.
#[canic::query]
fn role_metadata() -> (String, String) {
    let expected = Schema::new(&[User::SCHEMA]).unwrap().fingerprint();
    assert_eq!(Shard::with_store(|db| db.schema().fingerprint()), expected);
    (
        option_env!(concat!("CANIC_APP_ROLE_", env!("CARGO_PKG_NAME")))
            .unwrap_or("account_archive")
            .to_owned(),
        database::DATABASE_KEY.to_owned(),
    )
}
