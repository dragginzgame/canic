//! The user shard owns its account records.

use canic::{
    db,
    sharding::provisioning::{self, Binding, Provision, Request},
};
use ic_db::Ulid;
use user_directory_schema::{Users, model::user_shard::User};

// Open the shard database and mount its Auth session and readiness endpoints.
canic::mount_db_role! {
    shard,
    database = user_directory_schema::model::user_shard::Accounts,
    authority = "directory",
    prefix = "directory.",
    auth_read = "read",
}

// provision_user
// Insert the private account row or confirm an exact replay.
// guard  : canic::topology::require_parent
// limits : Provision<Users>
#[canic::update(
    contract = Provision<Users>,
    guard = canic::topology::require_parent,
)]
fn provision_user(request: Request<[u8; 16], ()>) -> Binding<[u8; 16]> {
    let binding = request.binding;
    binding.require_target();
    let account = binding.account;
    if let Some(user) = provisioning::account_row::<User>(&db!(), account) {
        if user.id.to_bytes() != binding.id {
            canic::method::fail("BindingConflict");
        }
    } else {
        db!()
            .insert(&User {
                id: Ulid::from_bytes(binding.id),
                issuer: account.issuer,
                subject: account.subject,
            })
            .expect("insert user");
    }

    binding
}

// my_id
// Look up at most one private user ID for the signed-in account.
// scope  : read
// limits : default
#[canic::query(scope = "read")]
fn my_id() -> Option<[u8; 16]> {
    provisioning::session_row::<User>(&db!(), &auth).map(|row| row.id.to_bytes())
}
