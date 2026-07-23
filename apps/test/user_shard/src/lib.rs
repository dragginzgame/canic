#![expect(clippy::unused_async)]

use canic::{Error, dto::auth::DelegatedToken, ids::cap, prelude::*};
use std::cell::RefCell;

thread_local! {
    static RECOVERY_GENERATION: RefCell<String> = const { RefCell::new(String::new()) };
}

canic::start!();

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

#[canic_query(requires(auth::authenticated(cap::VERIFY)))]
async fn hello(token: DelegatedToken) -> Result<(), Error> {
    let _ = token;

    Ok(())
}

/// Set deterministic fixture state for the disposable backup/restore journey.
#[canic_update(public)]
async fn test_set_recovery_generation(generation: String) -> Result<(), Error> {
    RECOVERY_GENERATION.with_borrow_mut(|current| *current = generation);
    Ok(())
}

/// Return deterministic fixture state for the disposable backup/restore journey.
#[canic_query(public)]
async fn test_recovery_generation() -> Result<String, Error> {
    Ok(RECOVERY_GENERATION.with_borrow(Clone::clone))
}

canic::finish!();
