//! The user hub owns its account routes.

use canic::{
    db,
    sharding::provisioning::{self, AuthSubject, Binding, Progress, Request, SendError},
};
use ic_db::{Principal, Ulid};
use user_directory_schema::{ProvisionRefusal, Users, model::user_hub::Route};

canic::mount_db!(schema = user_directory_schema::model::user_hub::Hub);

canic::mount_receiver_lifecycle! {
    config_key = "directory.hub.config.v1",
    authority = "directory",
    pages = 65536,
    auth_read = "read",
}

// register
// Reserve the signed-in account and confirm insertion on its assigned shard.
// scope  : write
// limits : default
#[canic::update(scope = WRITE)]
async fn register() -> Result<Progress, ProvisionRefusal> {
    let account = AuthSubject::from(&auth);
    let lookup = || provisioning::account_row::<Route>(&db!(), account);
    let existing = lookup();
    if existing.as_ref().is_some_and(|row| row.ready) {
        return Ok(Progress::Ready);
    }
    let assignment = match provisioning::place::<Users>(
        &auth,
        canic::shard_pool!("ex-user-directory-shard"),
        existing.as_ref().map(|row| row.shard),
    )
    .await
    {
        Ok(assignment) => assignment,
        Err(progress) => return Ok(progress),
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
        Err(SendError::Progress(progress)) => return Ok(progress),
        Err(SendError::Refused(reason)) => return Err(reason),
    };
    let mut retained = lookup().expect("missing reservation");
    receipt.require_same(&binding(&retained));
    if !retained.ready {
        retained.ready = true;
        db!().replace(&retained).expect("confirm reservation");
    }

    Ok(Progress::Ready)
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
        account: AuthSubject {
            issuer: row.issuer,
            subject: row.subject,
        },
        shard: row.shard,
    }
}

// reserve_project
// Create a reservation or return its ID only to the shard that owns it.
// guard  : attested user_shard (ReserveProject::ROLES)
// limits : default
#[cfg(feature = "role-service")]
#[canic::update(attested_method = user_directory_schema::ReserveProject)]
fn reserve_project(account: AuthSubject) -> [u8; 16] {
    let shard = ic_platform_runtime::system::caller();
    if let Some(row) = provisioning::account_row::<Route>(&db!(), account) {
        assert_eq!(row.shard, shard, "reservation belongs to another shard");
        return row.id.to_bytes();
    }
    let id = Ulid::generate_prand();
    db!()
        .insert(&Route {
            id,
            issuer: account.issuer,
            subject: account.subject,
            shard,
            ready: true,
        })
        .expect("reserve project");
    id.to_bytes()
}

// confirmed_shard
// Return the confirmed shard for the registry's account lookup.
// guard  : attested project_registry
// limits : default
#[cfg(feature = "role-service")]
#[canic::update(attested_method = user_directory_schema::confirmed_shard)]
fn confirmed_shard(account: AuthSubject) -> Option<Principal> {
    provisioning::account_row::<Route>(&db!(), account)
        .filter(|row| row.ready)
        .map(|row| row.shard)
}

// find_my_shard
// Resolve the signed-in account through the fleet's unique user hub.
// scope  : read
// limits : default
#[cfg(feature = "role-service")]
#[canic::update(scope = "read")]
async fn find_my_shard() -> Result<Option<Principal>, String> {
    canic::topology::call_service_by_role::<user_directory_schema::confirmed_shard>(
        "user_hub",
        5,
        AuthSubject::from(&auth),
        || auth.reauthorize().map_err(|e| format!("{e:?}")),
    )
    .await
    .map_err(|e| format!("{e:?}"))
}
