#![expect(clippy::unused_async)]

use canic::{Error, dto::auth::DelegatedToken, ids::cap, prelude::*};

canic::start!();

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

#[canic_query(requires(auth::authenticated(cap::VERIFY)))]
async fn hello(token: DelegatedToken) -> Result<(), Error> {
    let _ = token;

    Ok(())
}

canic::finish!();
