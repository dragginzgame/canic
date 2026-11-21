//! Installed witness for a schema-only role with two stable stores.

pub mod model;
ic_db::include_schema!();

use canic::db;
use model::{Hub, Project, Route};

canic::mount_db!(schema = Hub);

canic::mount_lifecycle!(install = install, reopen = reopen);

fn install() {
    // Fixed nonce for this disposable fixture.
    open(Some((
        [3; 16],
        canic::lifecycle::runtime::system::canister_self(),
    )));
}

fn reopen() {
    open(None);
}

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
    db!().insert(&Project { id: 1, value: 19 }).unwrap();
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
