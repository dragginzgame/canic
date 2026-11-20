//! The user hub owns its account routes.

use canic::{
    db,
    sharding::provisioning::{self, Account, Binding, Progress, Request},
};
use ic_db::{Principal, Ulid};
use user_directory_schema::{Users, model::user_hub::Route};

// Open the hub database and mount its Auth session and readiness endpoints.
canic::mount_db_role! {
    hub,
    database = user_directory_schema::model::user_hub::Routes,
    config = "directory.hub.config.v1",
    authority = "directory",
    pages = 65536,
    auth_read = "read",
}

// register
// Reserve the signed-in account and confirm insertion on its assigned shard.
// scope  : write
// limits : default
#[canic::update(scope = WRITE)]
async fn register() -> Progress {
    let account = Account::from(&auth);
    let lookup = || provisioning::account_row::<Route>(&db!(), account);
    let existing = lookup();
    if existing.as_ref().is_some_and(|row| row.ready) {
        return Progress::Ready;
    }
    let assignment =
        match provisioning::place::<Users>(&auth, existing.as_ref().map(|row| row.shard)).await {
            Ok(assignment) => assignment,
            Err(progress) => return progress,
        };
    // Another registration may have won during placement. Only the winner allocates an ID.
    let row = existing.or_else(lookup).unwrap_or_else(|| {
        let row = Route {
            id: Ulid::generate_prand(),
            issuer: account.issuer,
            subject: account.subject,
            shard: assignment.shard(),
            ready: false,
        };
        db!().insert(&row).expect("reserve account");

        row
    });
    let request = Request {
        binding: binding(&row),
        data: (),
    };
    let receipt = match assignment.send(&request).await {
        Ok(receipt) => receipt,
        Err(progress) => return progress,
    };
    let mut retained = lookup().expect("missing reservation");
    receipt.require_same(&binding(&retained));
    if !retained.ready {
        retained.ready = true;
        db!().replace(&retained).expect("confirm reservation");
    }

    Progress::Ready
}

// my_shard
// Look up at most one confirmed route for the signed-in account.
// scope  : read
// limits : default
#[canic::query(scope = "read")]
fn my_shard() -> Option<Principal> {
    provisioning::session_row::<Route>(&db!(), &auth)
        .filter(|row| row.ready)
        .map(|row| row.shard)
}

const WRITE: &str = "write";

fn binding(row: &Route) -> Binding<[u8; 16]> {
    Binding {
        id: row.id.to_bytes(),
        account: Account {
            issuer: row.issuer,
            subject: row.subject,
        },
        shard: row.shard,
    }
}
