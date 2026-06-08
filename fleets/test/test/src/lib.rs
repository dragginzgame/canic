#![expect(clippy::unused_async)]

use canic::{Error, dto::auth::DelegatedToken, ids::cap, prelude::*};

canic::start!();

async fn canic_setup() {}

async fn canic_install(_: Option<Vec<u8>>) {}

async fn canic_upgrade() {}

#[canic_update(requires(auth::authenticated(cap::VERIFY)))]
async fn test_verify_delegated_token(token: DelegatedToken) -> Result<(), Error> {
    let _ = token;

    if let Err(err) = canic::access::env::build_network_local() {
        return Err(Error::forbidden(err.to_string()));
    }

    Ok(())
}

canic::finish!();
