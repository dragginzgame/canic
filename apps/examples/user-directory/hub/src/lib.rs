//! The user hub owns its account routes.
use canic::{
    auth::protocol::SessionView,
    sharding::provisioning::{Account, Binding, HubRoute, Record},
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

/// Maps the application's retained row into Canic provisioning.
pub struct Hub;

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

/// Reserve this session’s account and confirm its shard insertion.
#[canic::update(scope = Hub::SCOPE)]
async fn register(session: SessionView) -> canic::sharding::provisioning::Progress {
    canic::sharding::provisioning::provision::<Hub>(&session, (), || {
        Ulid::generate_prand().to_bytes()
    })
    .await
}

/// Return the signed-in user's confirmed shard.
#[canic::query(scope = "read")]
fn my_shard(session: SessionView) -> Option<Principal> {
    database::store::with(|db| {
        db.query::<Route>()
            .eq(Route::ISSUER, session.issuer)
            .eq(Route::SUBJECT, session.subject)
            .work_limit(1)
            .try_one()
    })
    .expect("lookup route")
    .filter(|row| row.ready)
    .map(|row| row.shard)
}
