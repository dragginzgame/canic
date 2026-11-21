//! Optional role-metadata test endpoint to append to the actor root.
use ic_db::{CompiledCanister, Entity, Schema, Store};

// role_metadata
// Check the mounted session and report the entity’s declared allocation.
// limits : default
#[canic::query]
fn role_metadata() -> (String, String) {
    type EntityStore = <UserRoute as Entity>::Store;
    let _session = canic::db!();
    let store = <<EntityStore as Store>::Canister as CompiledCanister>::STORES.iter()
        .find(|store| store.key == EntityStore::KEY).unwrap();
    let expected = Schema::new(&[UserRoute::SCHEMA]).unwrap().fingerprint();
    assert_eq!(
        (store.schema)().fingerprint(),
        expected
    );
    (
        option_env!(concat!("CANIC_APP_ROLE_", env!("CARGO_PKG_NAME")))
            .unwrap_or("")
            .to_owned(),
        EntityStore::KEY.to_owned(),
    )
}
