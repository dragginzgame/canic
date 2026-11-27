//! Installed witness for a schema-only role with two stable stores.

pub mod model;
ic_db::include_schema!();

use canic::db;
use model::{Hub, Project, Route};

canic::mount_db!(schema = Hub);

#[cfg(feature = "receiver")]
mod receiver;

#[cfg(not(feature = "receiver"))]
canic::mount_lifecycle!(install = install, reopen = reopen);

#[cfg(not(feature = "receiver"))]
fn install() {
    // Fixed nonce for this disposable fixture.
    open(Some((
        [3; 16],
        ic_platform_runtime::system::canister_self(),
    )));
}

#[cfg(not(feature = "receiver"))]
fn reopen() {
    open(None);
}

#[cfg(not(feature = "receiver"))]
fn open(install: Option<([u8; 16], ic_db::Principal)>) {
    let host = canic::storage::bootstrap(
        Default::default(),
        install.is_some(),
        "witness",
        &database::keys(),
        65536,
    );
    database::attach(&host, install);
}

// seed
// Insert the same primary key with distinct values in the two declared stores.
// limits : default
#[canic::update]
fn seed() {
    db!().insert(&Route { id: 1, value: 7 }).unwrap();
    let db = db!();
    let mut batch = db.batch();
    let first = batch
        .insert_input(Project::insert().id(1).value(19))
        .unwrap();
    let second = batch
        .insert_input(Project::insert().id(2).value(20))
        .unwrap();
    batch.commit().unwrap();
    assert_eq!(first.created_at, second.created_at);
    assert_eq!(db.get::<Project>(2_u64).unwrap(), Some(second));
}

// replace_project
// Replace one timestamped row and compare the returned row with the stored value.
// limits : default
#[canic::update]
fn replace_project() -> u64 {
    let db = db!();
    let mut project = db.get::<Project>(1_u64).unwrap().unwrap();
    project.value += 1;
    let stored = db.replace(&project).unwrap();
    assert_eq!(stored.created_at, project.created_at);
    assert!(stored.updated_at > project.updated_at);
    assert_eq!(db.get::<Project>(1_u64).unwrap(), Some(stored.clone()));
    stored.value
}

// values
// Read one row from each store by entity type.
// limits : default
#[canic::query]
fn values() -> (u64, u64) {
    (
        db!().get::<Route>(1_u64).unwrap().unwrap().value,
        db!().get::<Project>(1_u64).unwrap().unwrap().value,
    )
}

// echo_id
// Round-trip the supplier's identifier across the mounted actor's Wire boundary.
// limits : default
#[canic::query]
fn echo_id(id: ic_db::Ulid) -> ic_db::Ulid {
    id
}
