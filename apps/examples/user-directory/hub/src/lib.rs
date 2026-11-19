//! The user hub owns its account routes.

use canic::{
    auth::protocol::SessionView,
    db,
    sharding::provisioning::{self, Account, Binding, HubRoute, Record},
};
use ic_db::{Database, Principal, Ulid};
use user_directory_schema::{Users, user_hub::Route};

// Open the hub database and mount its Auth session and readiness endpoints.
canic::mount_db_role! {
    hub,
    database = "directory.hub.db.v1",
    config = "directory.hub.config.v1",
    authority = "directory",
    pages = 65536,
    auth_read = "read",
}

// register
// Reserve the signed-in account and confirm insertion on its assigned shard.
// scope  : write
// limits : default
#[canic::update(scope = Hub::SCOPE)]
async fn register(session: SessionView) -> canic::sharding::provisioning::Progress {
    provisioning::provision::<Hub>(&session, (), provisioning::new_ulid).await
}

// my_shard
// Look up at most one confirmed route for the signed-in account.
// scope  : read
// limits : default
#[canic::query(scope = "read")]
fn my_shard(session: SessionView) -> Option<Principal> {
    provisioning::session_row::<Route>(&db!(), &session)
        .filter(|row| row.ready)
        .map(|row| row.shard)
}

///
/// Hub
/// Maps the application's retained row into Canic provisioning.
///

pub struct Hub;

impl HubRoute for Hub {
    const SCOPE: &'static str = "write";
    type Initial = ();

    fn normalize((): ()) {}

    fn pending(binding: Binding<[u8; 16]>, (): ()) -> Route {
        Route {
            id: Ulid::from_bytes(binding.id),
            issuer: binding.account.issuer,
            subject: binding.account.subject,
            shard: binding.shard,
            ready: false,
        }
    }

    fn retained(row: &Route) -> (Principal, ()) {
        (row.shard, ())
    }

    fn ready(row: &Route) -> bool {
        row.ready
    }

    fn mark_ready(row: &mut Route) {
        row.ready = true;
    }
}

impl Record for Hub {
    type Row = Route;
    type Resource = Users;

    fn with_store<T>(f: impl FnOnce(&mut Database) -> T) -> T {
        database::store::with(f)
    }

    fn identity(row: &Route) -> (Account, [u8; 16]) {
        (
            Account {
                issuer: row.issuer,
                subject: row.subject,
            },
            row.id.to_bytes(),
        )
    }
}
