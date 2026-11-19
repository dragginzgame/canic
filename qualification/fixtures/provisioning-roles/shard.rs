//! Optional role-metadata test endpoint to append to the actor root.
use ic_db::{Entity, Schema};

/// Report the configured role and original retained allocation for the upgrade witness.
#[canic::query]
fn role_metadata() -> (String, String) {
    let expected = Schema::new(&[User::SCHEMA]).unwrap().fingerprint();
    assert_eq!(
        database::store::with(|db| db.schema().fingerprint()),
        expected
    );
    (
        option_env!(concat!("CANIC_APP_ROLE_", env!("CARGO_PKG_NAME")))
            .unwrap_or("account_archive")
            .to_owned(),
        database::DATABASE_KEY.to_owned(),
    )
}
