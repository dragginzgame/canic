#![expect(clippy::unused_async)]

use canic::{
    Error, api::auth::AuthApi, cdk::types::Principal, dto::auth::DelegatedToken, ids::cap,
    prelude::*,
};

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

#[canic_update(public)]
async fn test_set_delegated_session_subject(
    delegated_subject: Principal,
    bootstrap_token: DelegatedToken,
    requested_ttl_secs: Option<u64>,
) -> Result<(), Error> {
    AuthApi::set_delegated_session_subject(delegated_subject, bootstrap_token, requested_ttl_secs)
}

#[canic_query(public)]
async fn test_delegated_session_subject() -> Result<Option<Principal>, Error> {
    Ok(AuthApi::delegated_session_subject())
}

canic::finish!();
