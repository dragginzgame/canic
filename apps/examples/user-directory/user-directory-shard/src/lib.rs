//! The user shard owns its account records.
use canic::{
    auth::protocol::SessionView,
    sharding::provisioning::{Account, Record, ShardRecord, Ticket},
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

/// Maps the application's retained row into Canic provisioning.
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

/// Accept an insertion or exact replay from the installed parent hub.
#[cfg(target_arch = "wasm32")]
#[canic::update(
    contract = canic::sharding::provisioning::Provision<Users>,
    guard = canic::topology::require_parent,
)]
fn provision_user(request: Ticket<Shard>) -> canic::sharding::provisioning::Binding<[u8; 16]> {
    canic::sharding::provisioning::receive::<Shard>(request)
}

/// Return only this session's private user ID.
#[canic::query(scope = "read")]
fn my_id(session: SessionView) -> Option<[u8; 16]> {
    database::store::with(|db| {
        db.query::<User>()
            .eq(User::ISSUER, session.issuer)
            .eq(User::SUBJECT, session.subject)
            .work_limit(1)
            .try_one()
    })
    .expect("lookup user")
    .map(|row| row.id.to_bytes())
}
