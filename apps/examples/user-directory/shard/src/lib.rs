//! The user shard owns its account records.

use canic::{
    auth::protocol::SessionView,
    db,
    sharding::provisioning::{self, Account, Record, ShardRecord, Ticket},
};
use ic_db::{Database, Ulid};
use user_directory_schema::{Users, user_shard::User};

// Open the shard database and mount its Auth session and readiness endpoints.
canic::mount_db_role! {
    shard,
    database = "directory.shard.db.v1",
    authority = "directory",
    prefix = "directory.",
    auth_read = "read",
}

// provision_user
// Insert the private account row or confirm an exact replay.
// guard  : canic::topology::require_parent
// limits : Provision<Users>
#[canic::update(
    contract = canic::sharding::provisioning::Provision<Users>,
    guard = canic::topology::require_parent,
)]
fn provision_user(request: Ticket<Shard>) -> canic::sharding::provisioning::Binding<[u8; 16]> {
    canic::sharding::provisioning::receive::<Shard>(request)
}

// my_id
// Look up at most one private user ID for the signed-in account.
// scope  : read
// limits : default
#[canic::query(scope = "read")]
fn my_id(session: SessionView) -> Option<[u8; 16]> {
    provisioning::session_row::<User>(&db!(), &session).map(|row| row.id.to_bytes())
}

///
/// Shard
/// Maps the application's retained row into Canic provisioning.
///

pub struct Shard;

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
        }
    }
}
